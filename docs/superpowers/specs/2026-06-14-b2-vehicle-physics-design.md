# B2 — Vehicle Physics & State Machine Design

**Date:** 2026-06-14  
**Ticket:** B2 (Wave 2, P1)  
**Module:** `vehicle.rs`  
**Depends on:** A1, A2, B1 (all complete)  
**Blocks:** C3, X1

---

## Overview

B2 delivers the per-frame vehicle update that drives the entire simulation. It is pure physics and logic — no SDL2, no spawning. Every vehicle is updated once per frame by calling `vehicle::update`. The function returns `false` when the vehicle is off-screen and should be removed by the caller.

---

## Function Signature

```rust
pub fn update(
    vehicle:      &mut Vehicle,
    dt:           f32,
    path_map:     &HashMap<(Direction, Route), Vec<Vec2>>,
    manager:      &mut IntersectionManager,
    all_vehicles: &[Vehicle],
    now_ms:       u64,
) -> bool
```

The current stub returns `()` — this must change to `-> bool`. Returns `false` when `vehicle.state == VehicleState::Removed`. The caller removes the vehicle from the list and records stats (`exit_time_ms = now_ms`) at that point.

**`path_map` usage:** `vehicle.path` is populated at spawn time (caller's responsibility, not B2's). `path_map` is passed in but B2 reads `vehicle.path` directly — it does not look up the map per frame.

---

## Private Helper

```rust
fn is_inside_intersection(pos: Vec2) -> bool {
    pos.x >= INTER_X && pos.x < INTER_X + INTER_W
        && pos.y >= INTER_Y && pos.y < INTER_Y + INTER_H
}
```

Used by the state machine for both enter and exit detection. One helper, two call sites, consistent boundary definition.

---

## Update Order Per Frame

Each call to `update` runs these steps in order:

1. **Safe-distance check** — may lower `target_vel` before physics
2. **State machine** — reads position, calls `manager`, updates `target_vel` and `state`
3. **Velocity physics** — applies accel/decel toward `target_vel`
4. **Position update** — moves vehicle toward current waypoint
5. **Waypoint advance** — checks if current waypoint is reached, increments index, recomputes `angle_deg`
6. **Removal check** — returns `false` if `Removed`

---

## Step 1 — Safe-Distance Check

Runs every frame, independent of `VehicleState`. Executes before the state machine so any speed reduction is visible to the physics step immediately.

```
filter all_vehicles where:
  - v.id != vehicle.id
  - v.direction == vehicle.direction
  - v is AHEAD along the travel axis (see table below)

find nearest ahead candidate by Euclidean distance between pos values

if nearest_distance < SAFE_DISTANCE:
  vehicle.target_vel = SPEED_SLOW
```

**"Ahead" definition by direction:**

| Direction | Travel axis | Ahead condition |
|-----------|-------------|-----------------|
| South (→N) | y decreasing | candidate.pos.y < vehicle.pos.y |
| North (→S) | y increasing | candidate.pos.y > vehicle.pos.y |
| West  (→E) | x increasing | candidate.pos.x > vehicle.pos.x |
| East  (→W) | x decreasing | candidate.pos.x < vehicle.pos.x |

Do not compare against all vehicles globally — perpendicular vehicles near the intersection will trigger false positives (AGENTS.md §9, Pitfall 5).

If no vehicle is ahead within `SAFE_DISTANCE`, `target_vel` is not modified here (the state machine sets it next).

---

## Step 2 — State Machine

### Approaching (outside trigger zone)

```
target_vel = SPEED_FAST
```

No reservation interaction until the trigger zone.

### Approaching (inside trigger zone)

On the first frame `manager.is_in_trigger_zone(vehicle)` returns `true`:
```
if entry_time_ms == 0:
    entry_time_ms = now_ms
```

Every frame in trigger zone:
```
granted = manager.request_reservation(vehicle.id, vehicle.direction, vehicle.route)
if granted:  target_vel = SPEED_MEDIUM
if !granted: target_vel = SPEED_SLOW
```

`request_reservation` is idempotent — safe to call every frame.

**Transition → InIntersection:**
```
if granted && is_inside_intersection(vehicle.pos):
    state = InIntersection
```

### InIntersection

```
target_vel = SPEED_MEDIUM
```

**Transition → Exiting:**
```
if !is_inside_intersection(vehicle.pos):
    manager.release_reservation(vehicle.id)
    state = Exiting
```

Reservation is held until the vehicle's `pos` fully exits the intersection box. This is the single correct release point — never release earlier (AGENTS.md §9, Pitfall 1).

### Exiting

```
target_vel = SPEED_FAST
```

No reservation interaction. Vehicle accelerates toward off-screen.

### Removed

```
return false
```

Triggered by waypoint advance (see Step 5).

---

## Step 3 — Velocity Physics

Applies every frame after the state machine has set `target_vel`:

```rust
if vehicle.velocity < vehicle.target_vel {
    vehicle.velocity += ACCEL_RATE * dt;
    vehicle.velocity = vehicle.velocity.min(vehicle.target_vel);
}
if vehicle.velocity > vehicle.target_vel {
    vehicle.velocity -= DECEL_RATE * dt;
    vehicle.velocity = vehicle.velocity.max(vehicle.target_vel);
}
vehicle.velocity = vehicle.velocity.clamp(0.0, SPEED_FAST);
```

Velocity **never** snaps instantly. The `.min`/`.max` guards prevent overshooting `target_vel`.

---

## Step 4 — Position Update

```rust
let angle_rad = vehicle.angle_deg.to_radians() as f32;  // angle_deg is f64
vehicle.pos.x += angle_rad.cos() * vehicle.velocity * dt;
vehicle.pos.y += angle_rad.sin() * vehicle.velocity * dt;
vehicle.distance_travelled += vehicle.velocity * dt;
```

`angle_deg` is `f64` in the `Vehicle` struct (matches SDL2's `copy_ex` expectation). Cast to `f32` for position arithmetic only.

---

## Step 5 — Waypoint Advance

```rust
if path_index >= path.len() {
    state = Removed; return false;
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
    // Recompute angle toward NEW waypoint
    let next = vehicle.path[vehicle.path_index];
    let ndx = next.x - vehicle.pos.x;
    let ndy = next.y - vehicle.pos.y;
    vehicle.angle_deg = -f64::atan2(ndy as f64, ndx as f64).to_degrees();
}
```

The sign flip on `atan2` is mandatory — SDL2's `copy_ex` angle is clockwise in screen space; `atan2` is counter-clockwise (AGENTS.md §9, Pitfall 2).

`angle_deg` is initialised at spawn (caller's responsibility) and recomputed at every waypoint advance here.

---

## Step 6 — Return Value

```rust
vehicle.state != VehicleState::Removed
```

Or equivalently, return `false` at each `Removed` transition point and `true` at the end of the function.

---

## Module Boundary Checklist

- [ ] No SDL2 import in `vehicle.rs`
- [ ] No struct or constant defined in `vehicle.rs` — all types from `types.rs`
- [ ] `vehicle.rs` does not spawn vehicles (spawning is `input.rs`)
- [ ] `vehicle.rs` does not draw anything
- [ ] `angle_deg` updated at every waypoint advance (no sideways sliding)
- [ ] `entry_time_ms` set at first trigger zone detection, not at spawn
- [ ] Reservation released only when `pos` exits intersection box, not at first exit waypoint
- [ ] `SAFE_DISTANCE` enforced on approach roads, not just inside intersection
- [ ] Velocity changes are smooth — no instant snaps

---

## Unit Tests (`#[cfg(test)]`)

B2 is harder to unit-test than B1 because it requires a running path and a real `IntersectionManager`. Tests should use `build_path_map()` and `IntersectionManager::new()` directly.

| # | Name | What it proves |
|---|------|----------------|
| 1 | `vehicle_traverses_south_straight_to_completion` | A South→Straight vehicle given enough dt steps returns false (Removed) |
| 2 | `velocity_does_not_snap` | velocity changes gradually over frames, never jumps to target_vel in one step |
| 3 | `entry_time_ms_set_at_trigger_zone_not_spawn` | entry_time_ms == 0 at spawn, > 0 only after trigger zone is entered |
| 4 | `reservation_released_after_intersection_exit` | active_count() drops to 0 only after vehicle pos exits the box |
| 5 | `safe_distance_slows_follower` | two South vehicles, leader ahead, follower's target_vel drops to SPEED_SLOW when gap < SAFE_DISTANCE |

All tests use `LIBRARY_PATH=/opt/homebrew/lib cargo test vehicle`.
