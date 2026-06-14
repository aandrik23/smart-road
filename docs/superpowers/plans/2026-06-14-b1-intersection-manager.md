# B1 IntersectionManager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a reservation-based intersection controller with a grid-cell-computed conflict table, clean grant/deny/release API, and trigger zone detection.

**Architecture:** `IntersectionManager::new()` calls `build_path_map()`, rasterizes all 12 paths into a 5×5 grid, and derives a `[[bool; 12]; 12]` conflict table. Active reservations are stored in a `HashMap<u32, (Direction, Route)>`. No SDL2. No allocations after startup.

**Tech Stack:** Rust, `std::collections::{HashMap, HashSet}`, existing `types.rs` constants, existing `build_path_map()` in `intersection.rs`.

**Test command throughout:** `LIBRARY_PATH=/opt/homebrew/lib cargo test intersection`

---

## Task 1: Add TRIGGER_DIST to types.rs

**Files:**
- Modify: `src/types.rs`

- [ ] **Step 1: Add the constant**

Open `src/types.rs`. After the `DECEL_RATE` constant (currently the last constant), add:

```rust
pub const TRIGGER_DIST: f32 = 200.0;
```

The block should now end:

```rust
pub const ACCEL_RATE:         f32 = 60.0;
pub const DECEL_RATE:         f32 = 120.0;
pub const TRIGGER_DIST:       f32 = 200.0;
```

- [ ] **Step 2: Verify it compiles**

```bash
LIBRARY_PATH=/opt/homebrew/lib cargo build 2>&1
```

Expected: `Finished` with zero errors.

- [ ] **Step 3: Commit**

```bash
git add src/types.rs
git commit -m "feat(types): add TRIGGER_DIST constant (200 px)"
```

---

## Task 2: Replace stub struct, update imports, add path_index

**Files:**
- Modify: `src/intersection.rs`

- [ ] **Step 1: Write the failing path_index test**

The test module already exists at the bottom of `src/intersection.rs`. Add this test inside the existing `mod tests { use super::*; ... }` block:

```rust
#[test]
fn path_index_all_12_unique_and_in_range() {
    let dirs   = [Direction::North, Direction::South, Direction::West, Direction::East];
    let routes = [Route::Right, Route::Straight, Route::Left];
    let mut seen = std::collections::HashSet::new();
    for dir in dirs {
        for route in routes {
            let idx = path_index(dir, route);
            assert!(idx < 12, "index {} out of range", idx);
            assert!(seen.insert(idx), "duplicate index {} for {:?} {:?}", idx, dir, route);
        }
    }
}
```

- [ ] **Step 2: Run — confirm it fails to compile (path_index not yet defined)**

```bash
LIBRARY_PATH=/opt/homebrew/lib cargo test intersection 2>&1 | head -20
```

Expected: compile error `cannot find function 'path_index'`.

- [ ] **Step 3: Replace the top of intersection.rs**

Replace the existing imports and struct at the top of `src/intersection.rs` (lines 1–22) with:

```rust
use std::collections::{HashMap, HashSet};
use crate::types::{
    Direction, Route, Vec2, Vehicle,
    INTER_X, INTER_Y, INTER_W, INTER_H, LANE_WIDTH, TRIGGER_DIST,
};

pub struct IntersectionManager {
    conflicts: [[bool; 12]; 12],
    active:    HashMap<u32, (Direction, Route)>,
}

fn path_index(dir: Direction, route: Route) -> usize {
    let d = match dir {
        Direction::North => 0,
        Direction::South => 1,
        Direction::West  => 2,
        Direction::East  => 3,
    };
    let r = match route {
        Route::Right    => 0,
        Route::Straight => 1,
        Route::Left     => 2,
    };
    d * 3 + r
}

impl IntersectionManager {
    pub fn new() -> Self {
        IntersectionManager {
            conflicts: [[false; 12]; 12],
            active: HashMap::new(),
        }
    }

    pub fn request_reservation(&mut self, _id: u32, _dir: Direction, _route: Route) -> bool {
        todo!("B1: reservation grant/deny with conflict table")
    }

    pub fn release_reservation(&mut self, _id: u32) {
        todo!("B1: release after full intersection clearance")
    }

    pub fn is_in_trigger_zone(&self, _vehicle: &Vehicle) -> bool {
        todo!("B1: 200 px trigger zone check")
    }

    pub fn active_count(&self) -> usize {
        todo!("B1: active reservation count")
    }
}
```

- [ ] **Step 4: Run — confirm path_index test passes, existing path map tests still pass**

```bash
LIBRARY_PATH=/opt/homebrew/lib cargo test intersection 2>&1
```

Expected: `path_index_all_12_unique_and_in_range ... ok` plus the 5 existing path map tests. The `todo!()` methods are not called yet so no panic.

- [ ] **Step 5: Commit**

```bash
git add src/intersection.rs
git commit -m "feat(intersection): replace stub with typed struct + path_index helper"
```

---

## Task 3: Write all failing reservation tests

**Files:**
- Modify: `src/intersection.rs` (tests only)

- [ ] **Step 1: Add the six reservation tests to the test module**

Inside `mod tests { use super::*; ... }`, add:

```rust
#[test]
fn all_four_right_turns_non_conflicting() {
    let mut mgr = IntersectionManager::new();
    assert!(mgr.request_reservation(1, Direction::North, Route::Right));
    assert!(mgr.request_reservation(2, Direction::South, Route::Right));
    assert!(mgr.request_reservation(3, Direction::West,  Route::Right));
    assert!(mgr.request_reservation(4, Direction::East,  Route::Right));
    assert_eq!(mgr.active_count(), 4);
}

#[test]
fn conflicting_request_denied() {
    let mut mgr = IntersectionManager::new();
    // (N,St) and (S,L) both traverse x=480 inside the intersection
    assert!(mgr.request_reservation(1, Direction::North, Route::Straight));
    assert!(!mgr.request_reservation(2, Direction::South, Route::Left));
    assert_eq!(mgr.active_count(), 1);
}

#[test]
fn release_frees_slot() {
    let mut mgr = IntersectionManager::new();
    assert!(mgr.request_reservation(1, Direction::North, Route::Straight));
    mgr.release_reservation(1);
    assert!(mgr.request_reservation(2, Direction::South, Route::Left));
    assert_eq!(mgr.active_count(), 1);
}

#[test]
fn active_count_tracks_grants_and_releases() {
    let mut mgr = IntersectionManager::new();
    mgr.request_reservation(1, Direction::North, Route::Right);
    mgr.request_reservation(2, Direction::South, Route::Right);
    mgr.request_reservation(3, Direction::West,  Route::Right);
    assert_eq!(mgr.active_count(), 3);
    mgr.release_reservation(2);
    assert_eq!(mgr.active_count(), 2);
}

#[test]
fn spec_confirmed_north_right_west_straight_no_conflict() {
    let mut mgr = IntersectionManager::new();
    assert!(mgr.request_reservation(1, Direction::North, Route::Right));
    assert!(mgr.request_reservation(2, Direction::West,  Route::Straight));
    assert_eq!(mgr.active_count(), 2);
}

#[test]
fn idempotent_re_request() {
    let mut mgr = IntersectionManager::new();
    assert!(mgr.request_reservation(1, Direction::North, Route::Straight));
    // Same vehicle requests again (re-enters trigger zone after deny loop)
    assert!(mgr.request_reservation(1, Direction::North, Route::Straight));
    assert_eq!(mgr.active_count(), 1);
}
```

- [ ] **Step 2: Run — confirm all 6 new tests fail with "not yet implemented"**

```bash
LIBRARY_PATH=/opt/homebrew/lib cargo test intersection 2>&1
```

Expected: 6 tests fail with `panicked at 'not yet implemented'`. The 6 existing path map tests and `path_index` test still pass.

---

## Task 4: Implement rasterization + new() + reservation methods

**Files:**
- Modify: `src/intersection.rs`

- [ ] **Step 1: Add rasterize_segment and build_conflict_table before the impl block**

Insert the following two private functions immediately before `impl IntersectionManager {`:

```rust
fn rasterize_segment(a: Vec2, b: Vec2, cells: &mut HashSet<(u8, u8)>) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let dist = (dx * dx + dy * dy).sqrt();
    let steps = (dist / 5.0).ceil() as u32;
    for i in 0..=steps {
        let t = if steps == 0 { 0.0 } else { i as f32 / steps as f32 };
        let x = a.x + t * dx;
        let y = a.y + t * dy;
        if x >= INTER_X && x < INTER_X + INTER_W
            && y >= INTER_Y && y < INTER_Y + INTER_H
        {
            let col = ((x - INTER_X) / LANE_WIDTH) as u8;
            let row = ((y - INTER_Y) / LANE_WIDTH) as u8;
            cells.insert((col, row));
        }
    }
}

fn build_conflict_table() -> [[bool; 12]; 12] {
    let path_map = build_path_map();
    let mut cell_sets: Vec<HashSet<(u8, u8)>> = (0..12).map(|_| HashSet::new()).collect();

    for ((dir, route), path) in &path_map {
        let idx = path_index(*dir, *route);
        for segment in path.windows(2) {
            rasterize_segment(segment[0], segment[1], &mut cell_sets[idx]);
        }
    }

    let mut conflicts = [[false; 12]; 12];
    for i in 0..12 {
        for j in (i + 1)..12 {
            if !cell_sets[i].is_disjoint(&cell_sets[j]) {
                conflicts[i][j] = true;
                conflicts[j][i] = true;
            }
        }
    }
    conflicts
}
```

- [ ] **Step 2: Replace new(), request_reservation, release_reservation, and active_count**

Inside `impl IntersectionManager`, replace the four method bodies:

```rust
pub fn new() -> Self {
    IntersectionManager {
        conflicts: build_conflict_table(),
        active: HashMap::new(),
    }
}

pub fn request_reservation(&mut self, id: u32, dir: Direction, route: Route) -> bool {
    if self.active.contains_key(&id) {
        return true;
    }
    let req_idx = path_index(dir, route);
    for (_, (a_dir, a_route)) in &self.active {
        if self.conflicts[req_idx][path_index(*a_dir, *a_route)] {
            return false;
        }
    }
    self.active.insert(id, (dir, route));
    true
}

pub fn release_reservation(&mut self, id: u32) {
    self.active.remove(&id);
}

pub fn is_in_trigger_zone(&self, _vehicle: &Vehicle) -> bool {
    todo!("B1: 200 px trigger zone check")
}

pub fn active_count(&self) -> usize {
    self.active.len()
}
```

- [ ] **Step 3: Run — confirm all 6 reservation tests now pass**

```bash
LIBRARY_PATH=/opt/homebrew/lib cargo test intersection 2>&1
```

Expected: 13 tests pass (6 path map tests, `path_index` test, 6 reservation tests). `is_in_trigger_zone` is still `todo!()` but not called by any test yet.

- [ ] **Step 4: Commit**

```bash
git add src/intersection.rs
git commit -m "feat(intersection): implement conflict table via grid-cell rasterization + reservation API"
```

---

## Task 5: Implement is_in_trigger_zone

**Files:**
- Modify: `src/intersection.rs`

- [ ] **Step 1: Add a vehicle helper and trigger zone tests to the test module**

At the top of `mod tests { use super::*; ... }`, add an import and a helper:

```rust
use crate::types::VehicleState; // Vehicle is already in scope via use super::*

fn make_vehicle(dir: Direction, x: f32, y: f32) -> Vehicle {
    Vehicle {
        id: 99,
        direction: dir,
        route: Route::Straight,
        state: VehicleState::Approaching,
        pos: Vec2 { x, y },
        velocity: 0.0,
        target_vel: 0.0,
        angle_deg: 0.0,
        path: vec![],
        path_index: 0,
        entry_time_ms: 0,
        exit_time_ms: 0,
        distance_travelled: 0.0,
    }
}
```

Then add the trigger zone tests:

```rust
#[test]
fn trigger_zone_south_inside() {
    // South vehicle (traveling north) — stop line y=600, zone y∈(600, 800]
    let mgr = IntersectionManager::new();
    assert!(mgr.is_in_trigger_zone(&make_vehicle(Direction::South, 420.0, 750.0)));
}

#[test]
fn trigger_zone_south_too_far() {
    let mgr = IntersectionManager::new();
    assert!(!mgr.is_in_trigger_zone(&make_vehicle(Direction::South, 420.0, 850.0)));
}

#[test]
fn trigger_zone_south_past_stop_line() {
    let mgr = IntersectionManager::new();
    // y=590 is inside the intersection box — no longer in approach trigger zone
    assert!(!mgr.is_in_trigger_zone(&make_vehicle(Direction::South, 420.0, 590.0)));
}

#[test]
fn trigger_zone_north_inside() {
    // North vehicle (traveling south) — stop line y=300, zone y∈[100, 300)
    let mgr = IntersectionManager::new();
    assert!(mgr.is_in_trigger_zone(&make_vehicle(Direction::North, 480.0, 150.0)));
}

#[test]
fn trigger_zone_west_inside() {
    // West vehicle (traveling east) — stop line x=300, zone x∈[100, 300)
    let mgr = IntersectionManager::new();
    assert!(mgr.is_in_trigger_zone(&make_vehicle(Direction::West, 200.0, 420.0)));
}

#[test]
fn trigger_zone_east_inside() {
    // East vehicle (traveling west) — stop line x=600, zone x∈(600, 800]
    let mgr = IntersectionManager::new();
    assert!(mgr.is_in_trigger_zone(&make_vehicle(Direction::East, 650.0, 480.0)));
}
```

- [ ] **Step 2: Run — confirm all 6 new tests fail with "not yet implemented"**

```bash
LIBRARY_PATH=/opt/homebrew/lib cargo test intersection 2>&1
```

Expected: 6 new trigger zone tests fail with `panicked at 'not yet implemented'`. 13 existing tests still pass.

- [ ] **Step 3: Replace is_in_trigger_zone body**

Inside `impl IntersectionManager`, replace `is_in_trigger_zone`:

```rust
pub fn is_in_trigger_zone(&self, vehicle: &Vehicle) -> bool {
    match vehicle.direction {
        Direction::South => {
            vehicle.pos.y > INTER_Y + INTER_H
                && vehicle.pos.y <= INTER_Y + INTER_H + TRIGGER_DIST
        }
        Direction::North => {
            vehicle.pos.y < INTER_Y
                && vehicle.pos.y >= INTER_Y - TRIGGER_DIST
        }
        Direction::West => {
            vehicle.pos.x < INTER_X
                && vehicle.pos.x >= INTER_X - TRIGGER_DIST
        }
        Direction::East => {
            vehicle.pos.x > INTER_X + INTER_W
                && vehicle.pos.x <= INTER_X + INTER_W + TRIGGER_DIST
        }
    }
}
```

- [ ] **Step 4: Run — confirm all 19 tests pass**

```bash
LIBRARY_PATH=/opt/homebrew/lib cargo test intersection 2>&1
```

Expected: `test result: ok. 18 passed` (5 path map + 1 path_index + 6 reservation + 6 trigger zone).

- [ ] **Step 5: Commit**

```bash
git add src/intersection.rs
git commit -m "feat(intersection): implement is_in_trigger_zone for all four approach directions"
```

---

## Task 6: Final verification

**Files:** none (read-only check)

- [ ] **Step 1: Run the full test suite**

```bash
LIBRARY_PATH=/opt/homebrew/lib cargo test 2>&1
```

Expected: `test result: ok. 18 passed`.

- [ ] **Step 2: Run clippy**

```bash
LIBRARY_PATH=/opt/homebrew/lib cargo clippy -- -D warnings 2>&1
```

Expected: `Finished` with zero warnings.

- [ ] **Step 3: Confirm no SDL2 import in intersection.rs**

```bash
grep -n "sdl2" src/intersection.rs
```

Expected: no output.

- [ ] **Step 4: Confirm TRIGGER_DIST is not hardcoded (no magic 200)**

```bash
grep -n "200" src/intersection.rs
```

Expected: no output (the value lives in `types.rs` only).

- [ ] **Step 5: Mark B1 done in the ticket tracker**

In `docs/SMART_ROAD_ticket_breakdown.md`:
1. Change the B1 row status from `[ ]` to `[x]`
2. Update the Summary Snapshot: `Done: 2` → `Done: 3`, `Not Started: 6` → `Not Started: 5`

- [ ] **Step 6: Final commit**

```bash
git add docs/SMART_ROAD_ticket_breakdown.md
git commit -m "chore: mark B1 complete in ticket tracker"
```
