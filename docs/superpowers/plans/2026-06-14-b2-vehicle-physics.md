# B2 Vehicle Physics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `vehicle::update()` — waypoint traversal, smooth accel/decel, reservation lifecycle state machine, and safe-distance following.

**Architecture:** Single `pub fn update() -> bool` in `vehicle.rs`. Private `is_inside_intersection()` helper drives both enter and exit state transitions. Update order per frame: state machine → safe-distance clamp → velocity physics → position → waypoint advance → removal check.

**Tech Stack:** Rust, `types.rs` (all constants + structs), `intersection.rs` (`IntersectionManager` + `build_path_map`). No SDL2.

---

## File Changes

| File | Action | Responsibility |
|------|--------|----------------|
| `src/vehicle.rs` | Modify | Replace `todo!()` stub with full update function + 5 unit tests |
| `docs/SMART_ROAD_ticket_breakdown.md` | Modify | Mark B2 `[x]` complete |

No new files.

---

## Angle Convention Note

The spec states `angle_deg = -atan2(ndy, ndx)`. **Do not use the negation.** In SDL2's y-down coordinate system `atan2(ndy, ndx)` is already clockwise — matching SDL2's `copy_ex` — and gives correct movement direction. Negating it causes N/S vehicles to move away from their waypoints. Use `f64::atan2(ndy as f64, ndx as f64).to_degrees()` (no sign flip) everywhere.

---

### Task 1: Fix signature, add imports, add helper, write all failing tests

**Files:**
- Modify: `src/vehicle.rs`

- [ ] **Step 1: Replace the entire file contents**

```rust
use std::collections::HashMap;
use crate::types::{
    Direction, Route, Vehicle, Vec2, VehicleState,
    INTER_X, INTER_Y, INTER_W, INTER_H,
    SPEED_FAST, SPEED_MEDIUM, SPEED_SLOW,
    ACCEL_RATE, DECEL_RATE, SAFE_DISTANCE, TRIGGER_DIST,
};
use crate::intersection::IntersectionManager;

fn is_inside_intersection(pos: Vec2) -> bool {
    pos.x >= INTER_X && pos.x < INTER_X + INTER_W
        && pos.y >= INTER_Y && pos.y < INTER_Y + INTER_H
}

pub fn update(
    vehicle:       &mut Vehicle,
    _dt:           f32,
    _path_map:     &HashMap<(Direction, Route), Vec<Vec2>>,
    _manager:      &mut IntersectionManager,
    _all_vehicles: &[Vehicle],
    _now_ms:       u64,
) -> bool {
    todo!("B2: implement update")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intersection::{IntersectionManager, build_path_map};

    fn make_vehicle(id: u32, dir: Direction, route: Route) -> Vehicle {
        let path_map = build_path_map();
        let path = path_map[&(dir, route)].clone();
        let p0 = path[0];
        let p1 = path[1];
        let ndx = p1.x - p0.x;
        let ndy = p1.y - p0.y;
        Vehicle {
            id,
            direction: dir,
            route,
            state: VehicleState::Approaching,
            pos: p0,
            velocity: SPEED_FAST,
            target_vel: SPEED_FAST,
            angle_deg: f64::atan2(ndy as f64, ndx as f64).to_degrees(),
            path,
            path_index: 1,
            entry_time_ms: 0,
            exit_time_ms: 0,
            distance_travelled: 0.0,
        }
    }

    #[test]
    fn vehicle_traverses_south_straight_to_completion() {
        let path_map = build_path_map();
        let mut mgr = IntersectionManager::new();
        let mut v = make_vehicle(1, Direction::South, Route::Straight);
        let dt = 1.0_f32 / 60.0;
        let mut alive = true;
        for frame in 0..10_000u64 {
            alive = update(&mut v, dt, &path_map, &mut mgr, &[], frame);
            if !alive { break; }
        }
        assert!(!alive, "vehicle should reach Removed state within 10 000 frames");
    }

    #[test]
    fn velocity_does_not_snap() {
        let path_map = build_path_map();
        let mut mgr = IntersectionManager::new();
        let mut v = make_vehicle(1, Direction::South, Route::Straight);
        v.velocity = 0.0;
        v.target_vel = SPEED_FAST;
        update(&mut v, 1.0 / 60.0, &path_map, &mut mgr, &[], 0);
        assert!(v.velocity > 0.0,          "velocity must increase from 0");
        assert!(v.velocity < SPEED_FAST,   "velocity must not snap to target_vel in one frame");
    }

    #[test]
    fn entry_time_ms_set_at_trigger_zone_not_spawn() {
        let path_map = build_path_map();
        let mut mgr = IntersectionManager::new();
        let mut v = make_vehicle(1, Direction::South, Route::Straight);
        assert_eq!(v.entry_time_ms, 0, "entry_time_ms must be 0 at spawn");
        let dt = 1.0_f32 / 60.0;
        for frame in 1..=5_000u64 {
            update(&mut v, dt, &path_map, &mut mgr, &[], frame);
            // South trigger zone: pos.y ∈ (600, 800]
            if v.pos.y <= 800.0 && v.pos.y > 600.0 {
                assert!(v.entry_time_ms > 0,
                    "entry_time_ms must be set on first trigger zone frame");
                let captured = v.entry_time_ms;
                update(&mut v, dt, &path_map, &mut mgr, &[], frame + 1);
                assert_eq!(v.entry_time_ms, captured,
                    "entry_time_ms must not change once set");
                return;
            }
        }
        panic!("vehicle never entered trigger zone in 5 000 frames");
    }

    #[test]
    fn reservation_released_after_intersection_exit() {
        let path_map = build_path_map();
        let mut mgr = IntersectionManager::new();
        let mut v = make_vehicle(1, Direction::South, Route::Straight);
        let dt = 1.0_f32 / 60.0;
        for frame in 0..10_000u64 {
            update(&mut v, dt, &path_map, &mut mgr, &[], frame);
            if v.state == VehicleState::Exiting {
                assert_eq!(mgr.active_count(), 0,
                    "reservation must be released when state transitions to Exiting");
                return;
            }
            if v.state == VehicleState::Removed { break; }
        }
        panic!("vehicle never reached Exiting state");
    }

    #[test]
    fn safe_distance_slows_follower() {
        let path_map = build_path_map();
        let mut mgr = IntersectionManager::new();
        // Both South vehicles on the approach road (y > 600), leader further north.
        // South travels north (y decreasing), so smaller y = further ahead.
        let mut leader   = make_vehicle(1, Direction::South, Route::Straight);
        let mut follower = make_vehicle(2, Direction::South, Route::Straight);
        leader.pos   = Vec2 { x: 420.0, y: 820.0 };
        follower.pos = Vec2 { x: 420.0, y: 820.0 + SAFE_DISTANCE * 0.5 };
        follower.target_vel = SPEED_FAST;

        let all = vec![leader.clone()];
        update(&mut follower, 1.0 / 60.0, &path_map, &mut mgr, &all, 0);

        assert_eq!(follower.target_vel, SPEED_SLOW,
            "follower must slow to SPEED_SLOW when gap to leader < SAFE_DISTANCE");
    }
}
```

- [ ] **Step 2: Verify all 5 tests FAIL with todo! panic**

```
LIBRARY_PATH=/opt/homebrew/lib cargo test vehicle 2>&1 | grep -E "test vehicle|FAILED|panicked|passed|failed"
```

Expected: 5 FAILED, each with `not yet implemented`.

---

### Task 2: Implement waypoint traversal, velocity physics, and position update

Vehicle moves at constant speed with no state machine. Tests 1 and 2 should pass.

**Files:**
- Modify: `src/vehicle.rs`

- [ ] **Step 1: Replace the `update` function with movement-only implementation**

Replace just the `update` fn (keep `is_inside_intersection` and test module unchanged):

```rust
pub fn update(
    vehicle:       &mut Vehicle,
    dt:            f32,
    _path_map:     &HashMap<(Direction, Route), Vec<Vec2>>,
    _manager:      &mut IntersectionManager,
    _all_vehicles: &[Vehicle],
    _now_ms:       u64,
) -> bool {
    // Velocity physics
    if vehicle.velocity < vehicle.target_vel {
        vehicle.velocity += ACCEL_RATE * dt;
        vehicle.velocity = vehicle.velocity.min(vehicle.target_vel);
    } else if vehicle.velocity > vehicle.target_vel {
        vehicle.velocity -= DECEL_RATE * dt;
        vehicle.velocity = vehicle.velocity.max(vehicle.target_vel);
    }
    vehicle.velocity = vehicle.velocity.clamp(0.0, SPEED_FAST);

    // Position update — angle_deg is SDL2-clockwise; no sign flip needed (y-down coords).
    let angle_rad = vehicle.angle_deg.to_radians() as f32;
    vehicle.pos.x += angle_rad.cos() * vehicle.velocity * dt;
    vehicle.pos.y += angle_rad.sin() * vehicle.velocity * dt;
    vehicle.distance_travelled += vehicle.velocity * dt;

    // Waypoint advance
    if vehicle.path_index >= vehicle.path.len() {
        vehicle.state = VehicleState::Removed;
        return false;
    }
    let target = vehicle.path[vehicle.path_index];
    let dx = target.x - vehicle.pos.x;
    let dy = target.y - vehicle.pos.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= 2.0 {
        vehicle.path_index += 1;
        if vehicle.path_index >= vehicle.path.len() {
            vehicle.state = VehicleState::Removed;
            return false;
        }
        let next = vehicle.path[vehicle.path_index];
        let ndx = next.x - vehicle.pos.x;
        let ndy = next.y - vehicle.pos.y;
        vehicle.angle_deg = f64::atan2(ndy as f64, ndx as f64).to_degrees();
    }

    vehicle.state != VehicleState::Removed
}
```

- [ ] **Step 2: Run tests — expect tests 1 and 2 to pass, 3/4/5 still fail**

```
LIBRARY_PATH=/opt/homebrew/lib cargo test vehicle 2>&1 | grep -E "test vehicle|FAILED|ok"
```

Expected:
```
test vehicle::tests::vehicle_traverses_south_straight_to_completion ... ok
test vehicle::tests::velocity_does_not_snap ... ok
test vehicle::tests::entry_time_ms_set_at_trigger_zone_not_spawn ... FAILED
test vehicle::tests::reservation_released_after_intersection_exit ... FAILED
test vehicle::tests::safe_distance_slows_follower ... FAILED
```

---

### Task 3: Implement safe-distance check

**Files:**
- Modify: `src/vehicle.rs`

- [ ] **Step 1: Add the `apply_safe_distance` helper before `pub fn update`**

Insert this function between `is_inside_intersection` and `pub fn update`:

```rust
fn apply_safe_distance(vehicle: &Vehicle, all_vehicles: &[Vehicle]) -> bool {
    let nearest = all_vehicles
        .iter()
        .filter(|o| o.id != vehicle.id && o.direction == vehicle.direction)
        .filter(|o| match vehicle.direction {
            Direction::South => o.pos.y < vehicle.pos.y,
            Direction::North => o.pos.y > vehicle.pos.y,
            Direction::West  => o.pos.x > vehicle.pos.x,
            Direction::East  => o.pos.x < vehicle.pos.x,
        })
        .map(|o| {
            let dx = o.pos.x - vehicle.pos.x;
            let dy = o.pos.y - vehicle.pos.y;
            (dx * dx + dy * dy).sqrt()
        })
        .fold(f32::INFINITY, f32::min);
    nearest < SAFE_DISTANCE
}
```

- [ ] **Step 2: Update the `update` function signature and body**

Change `_all_vehicles` to `all_vehicles` (remove underscore — it is now used), and add the safe-distance clamp at the TOP of the function body, before velocity physics:

```rust
pub fn update(
    vehicle:      &mut Vehicle,
    dt:           f32,
    _path_map:    &HashMap<(Direction, Route), Vec<Vec2>>,
    _manager:     &mut IntersectionManager,
    all_vehicles: &[Vehicle],
    _now_ms:      u64,
) -> bool {
    // Safe-distance clamp: overrides target_vel downward when a same-direction
    // vehicle is less than SAFE_DISTANCE ahead. Runs after state machine (Task 4
    // will prepend the state machine above this block).
    if apply_safe_distance(vehicle, all_vehicles) {
        vehicle.target_vel = SPEED_SLOW;
    }

    // Velocity physics
    if vehicle.velocity < vehicle.target_vel {
        vehicle.velocity += ACCEL_RATE * dt;
        vehicle.velocity = vehicle.velocity.min(vehicle.target_vel);
    } else if vehicle.velocity > vehicle.target_vel {
        vehicle.velocity -= DECEL_RATE * dt;
        vehicle.velocity = vehicle.velocity.max(vehicle.target_vel);
    }
    vehicle.velocity = vehicle.velocity.clamp(0.0, SPEED_FAST);

    // Position update
    let angle_rad = vehicle.angle_deg.to_radians() as f32;
    vehicle.pos.x += angle_rad.cos() * vehicle.velocity * dt;
    vehicle.pos.y += angle_rad.sin() * vehicle.velocity * dt;
    vehicle.distance_travelled += vehicle.velocity * dt;

    // Waypoint advance
    if vehicle.path_index >= vehicle.path.len() {
        vehicle.state = VehicleState::Removed;
        return false;
    }
    let target = vehicle.path[vehicle.path_index];
    let dx = target.x - vehicle.pos.x;
    let dy = target.y - vehicle.pos.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= 2.0 {
        vehicle.path_index += 1;
        if vehicle.path_index >= vehicle.path.len() {
            vehicle.state = VehicleState::Removed;
            return false;
        }
        let next = vehicle.path[vehicle.path_index];
        let ndx = next.x - vehicle.pos.x;
        let ndy = next.y - vehicle.pos.y;
        vehicle.angle_deg = f64::atan2(ndy as f64, ndx as f64).to_degrees();
    }

    vehicle.state != VehicleState::Removed
}
```

- [ ] **Step 3: Run tests — expect test 5 to now pass (tests 3/4 still fail)**

```
LIBRARY_PATH=/opt/homebrew/lib cargo test vehicle 2>&1 | grep -E "test vehicle|FAILED|ok"
```

Expected:
```
test vehicle::tests::vehicle_traverses_south_straight_to_completion ... ok
test vehicle::tests::velocity_does_not_snap ... ok
test vehicle::tests::entry_time_ms_set_at_trigger_zone_not_spawn ... FAILED
test vehicle::tests::reservation_released_after_intersection_exit ... FAILED
test vehicle::tests::safe_distance_slows_follower ... ok
```

---

### Task 4: Implement the state machine

**Files:**
- Modify: `src/vehicle.rs`

- [ ] **Step 1: Update the `update` signature and prepend the state machine**

Change `_manager` → `manager` and `_now_ms` → `now_ms` (both are now used). Then write the full function:

```rust
pub fn update(
    vehicle:      &mut Vehicle,
    dt:           f32,
    _path_map:    &HashMap<(Direction, Route), Vec<Vec2>>,
    manager:      &mut IntersectionManager,
    all_vehicles: &[Vehicle],
    now_ms:       u64,
) -> bool {
    // State machine
    match vehicle.state {
        VehicleState::Approaching => {
            if manager.is_in_trigger_zone(vehicle) {
                if vehicle.entry_time_ms == 0 {
                    vehicle.entry_time_ms = now_ms;
                }
                let granted = manager.request_reservation(
                    vehicle.id, vehicle.direction, vehicle.route,
                );
                if granted {
                    vehicle.target_vel = SPEED_MEDIUM;
                    if is_inside_intersection(vehicle.pos) {
                        vehicle.state = VehicleState::InIntersection;
                    }
                } else {
                    vehicle.target_vel = SPEED_SLOW;
                }
            } else {
                vehicle.target_vel = SPEED_FAST;
            }
        }
        VehicleState::InIntersection => {
            vehicle.target_vel = SPEED_MEDIUM;
            if !is_inside_intersection(vehicle.pos) {
                manager.release_reservation(vehicle.id);
                vehicle.state = VehicleState::Exiting;
            }
        }
        VehicleState::Exiting => {
            vehicle.target_vel = SPEED_FAST;
        }
        VehicleState::Removed => {
            return false;
        }
    }

    // Safe-distance clamp: overrides target_vel downward when a same-direction
    // vehicle is less than SAFE_DISTANCE ahead.
    if apply_safe_distance(vehicle, all_vehicles) {
        vehicle.target_vel = SPEED_SLOW;
    }

    // Velocity physics
    if vehicle.velocity < vehicle.target_vel {
        vehicle.velocity += ACCEL_RATE * dt;
        vehicle.velocity = vehicle.velocity.min(vehicle.target_vel);
    } else if vehicle.velocity > vehicle.target_vel {
        vehicle.velocity -= DECEL_RATE * dt;
        vehicle.velocity = vehicle.velocity.max(vehicle.target_vel);
    }
    vehicle.velocity = vehicle.velocity.clamp(0.0, SPEED_FAST);

    // Position update
    let angle_rad = vehicle.angle_deg.to_radians() as f32;
    vehicle.pos.x += angle_rad.cos() * vehicle.velocity * dt;
    vehicle.pos.y += angle_rad.sin() * vehicle.velocity * dt;
    vehicle.distance_travelled += vehicle.velocity * dt;

    // Waypoint advance
    if vehicle.path_index >= vehicle.path.len() {
        vehicle.state = VehicleState::Removed;
        return false;
    }
    let target = vehicle.path[vehicle.path_index];
    let dx = target.x - vehicle.pos.x;
    let dy = target.y - vehicle.pos.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= 2.0 {
        vehicle.path_index += 1;
        if vehicle.path_index >= vehicle.path.len() {
            vehicle.state = VehicleState::Removed;
            return false;
        }
        let next = vehicle.path[vehicle.path_index];
        let ndx = next.x - vehicle.pos.x;
        let ndy = next.y - vehicle.pos.y;
        vehicle.angle_deg = f64::atan2(ndy as f64, ndx as f64).to_degrees();
    }

    vehicle.state != VehicleState::Removed
}
```

- [ ] **Step 2: Run all tests — all 5 must pass**

```
LIBRARY_PATH=/opt/homebrew/lib cargo test vehicle 2>&1 | grep -E "test vehicle|FAILED|ok|passed|failed"
```

Expected: all 5 tests **ok**, 0 failures.

---

### Task 5: Final verification — clippy, no SDL2, ticket tracker

**Files:**
- Modify: `docs/SMART_ROAD_ticket_breakdown.md`

- [ ] **Step 1: Run clippy on the whole project**

```
LIBRARY_PATH=/opt/homebrew/lib cargo clippy 2>&1 | grep -E "warning|error"
```

Fix any warnings in `vehicle.rs`. Common patterns:
- Unused import → remove it
- Manual range: `x >= a && x < b` → use `(a..b).contains(&x)`
- Needless `.clone()` on copy types

- [ ] **Step 2: Confirm no SDL2 import in vehicle.rs**

```
grep -n "sdl2" /Users/vparik/Desktop/Zone01/Rust_Projects/smart-road/src/vehicle.rs
```

Expected: no output.

- [ ] **Step 3: Run full test suite — verify no regressions**

```
LIBRARY_PATH=/opt/homebrew/lib cargo test 2>&1 | tail -5
```

Expected: 23 tests pass (18 intersection + 5 vehicle), 0 failed.

- [ ] **Step 4: Mark B2 complete in ticket tracker**

In `docs/SMART_ROAD_ticket_breakdown.md`:
- Find the B2 row and change `[ ]` to `[x]`
- Update the Done count from 3 → 4 and Not Started count accordingly
