# 6-Lane Intersection Refactor

**Goal:** Replace the current 5-lane road (300 px wide) with a 6-lane road (360 px wide) so that every (direction, route) pair has its own dedicated exit lane, eliminating vehicle collisions on shared exit roads. Also fix the close-call counter to track distinct events instead of raw frame ticks.

**Root problem:** The current 5-lane layout forces two (direction, route) pairs to share each exit lane (e.g. South-Left and North-Right both exit east at y=360). `nearest_ahead` filters by `(direction == v.direction && route == v.route)`, so vehicles from different origins on the same physical lane never brake for each other.

**Approach:** Widen the road to 6 lanes (3 northbound + 3 southbound, 3 eastbound + 3 westbound). Each route gets a unique exit lane, so the geometry fixes the collision at the source — no code change to `nearest_ahead` needed.

**Tech stack:** Rust, SDL2 (`sdl2` crate)

---

## New Layout Reference

```
Intersection: x ∈ [270, 630)   y ∈ [270, 630)
              INTER_X=270  INTER_Y=270  INTER_W=360  INTER_H=360

Northbound lanes (x): 300  360  420
Southbound lanes (x): 480  540  600
Eastbound  lanes (y): 300  360  420
Westbound  lanes (y): 480  540  600

Approach lanes:
  South (→N): Right=x300  Straight=x360  Left=x420
  North (→S): Right=x600  Straight=x540  Left=x480
  West  (→E): Right=y300  Straight=y360  Left=y420
  East  (→W): Right=y600  Straight=y540  Left=y480

Exit lanes (all unique — the fix):
  North exit: East-Left=x300   South-Straight=x360  West-Right=x420
  South exit: East-Right=x480  North-Straight=x540  West-Left=x600
  East  exit: South-Left=y300  West-Straight=y360   North-Right=y420
  West  exit: South-Right=y480 East-Straight=y540   North-Left=y600
```

---

## All 12 New Paths

Turn geometry rule: each right turn uses a 30 px approach + 45° diagonal (dx=±30, dy=±30); each left turn crosses most of the intersection before the same diagonal step.

```
South-Right  (x=300 → W y=480): (300,950)→(300,630)→(300,510)→(270,480)→(-50,480)
South-Straight (x=360 → N x=360): (360,950)→(360,630)→(360,270)→(360,-50)
South-Left   (x=420 → E y=300): (420,950)→(420,630)→(420,330)→(450,300)→(630,300)→(950,300)

North-Right  (x=600 → E y=420): (600,-50)→(600,270)→(600,390)→(630,420)→(950,420)
North-Straight (x=540 → S x=540): (540,-50)→(540,270)→(540,630)→(540,950)
North-Left   (x=480 → W y=600): (480,-50)→(480,270)→(480,570)→(450,600)→(270,600)→(-50,600)

West-Right   (y=300 → N x=420): (-50,300)→(270,300)→(390,300)→(420,270)→(420,-50)
West-Straight (y=360 → E y=360): (-50,360)→(270,360)→(630,360)→(950,360)
West-Left    (y=420 → S x=600): (-50,420)→(270,420)→(570,420)→(600,450)→(600,630)→(600,950)

East-Right   (y=600 → S x=480): (950,600)→(630,600)→(510,600)→(480,630)→(480,950)
East-Straight (y=540 → W y=540): (950,540)→(630,540)→(270,540)→(-50,540)
East-Left    (y=480 → N x=300): (950,480)→(630,480)→(330,480)→(300,450)→(300,270)→(300,-50)
```

Each 45° diagonal step is exactly (±30, ±30) — verifiable by inspection.

---

## Files Changed

| File | What changes |
|---|---|
| `src/types.rs` | 4 constants: INTER_X, INTER_Y, INTER_W, INTER_H |
| `src/intersection.rs` | All 12 paths in `build_path_map`; replace one test |
| `src/renderer.rs` | Stop-line rects use constants; lane-marking y-bounds use constants |
| `src/vehicle.rs` | One test: update x coordinate for South-Straight |
| `src/stats.rs` | `record_close_calls` counts events, not frames |
| `src/main.rs` | Maintain `active_close_pairs` set; pass to `record_close_calls` |

`src/input.rs` — **no changes needed** (spawns at `path[0]`, picks up new coords automatically).

---

## Task 1 — Update intersection constants (`types.rs`)

**Files:** `src/types.rs`

- [ ] In `src/types.rs`, replace the four intersection constants:

```rust
pub const INTER_X: f32 = 270.0;
pub const INTER_Y: f32 = 270.0;
pub const INTER_W: f32 = 360.0;
pub const INTER_H: f32 = 360.0;
```

- [ ] Run `cargo build` and confirm it compiles (tests will fail — that is expected until Tasks 2–4 are done):

```
cargo build
```

Expected: compiles. Some tests may fail — ignore for now.

- [ ] Commit:

```bash
git add src/types.rs
git commit -m "refactor: widen intersection to 6 lanes (360px) — update INTER constants"
```

---

## Task 2 — Rewrite all 12 paths (`intersection.rs`)

**Files:** `src/intersection.rs`

- [ ] Replace the entire body of `build_path_map()` with the new paths. The function signature stays the same. Use these exact waypoints (copy verbatim):

```rust
pub fn build_path_map() -> HashMap<(Direction, Route), Vec<Vec2>> {
    let mut map = HashMap::with_capacity(12);

    // ── South (spawn y=950, travel north) ───────────────────────────────────
    // Right turn → exits West at y=480 (innermost westbound lane)
    map.insert((Direction::South, Route::Right), vec![
        Vec2 { x: 300.0, y: 950.0 },
        Vec2 { x: 300.0, y: 630.0 },
        Vec2 { x: 300.0, y: 510.0 }, // turn entry
        Vec2 { x: 270.0, y: 480.0 }, // 45° diagonal → west edge
        Vec2 { x: -50.0, y: 480.0 },
    ]);

    // Straight → exits North at x=360
    map.insert((Direction::South, Route::Straight), vec![
        Vec2 { x: 360.0, y: 950.0 },
        Vec2 { x: 360.0, y: 630.0 },
        Vec2 { x: 360.0, y: 270.0 },
        Vec2 { x: 360.0, y: -50.0 },
    ]);

    // Left turn → exits East at y=300 (outermost eastbound lane)
    map.insert((Direction::South, Route::Left), vec![
        Vec2 { x: 420.0, y: 950.0 },
        Vec2 { x: 420.0, y: 630.0 },
        Vec2 { x: 420.0, y: 330.0 }, // turn entry
        Vec2 { x: 450.0, y: 300.0 }, // 45° diagonal
        Vec2 { x: 630.0, y: 300.0 }, // east edge
        Vec2 { x: 950.0, y: 300.0 },
    ]);

    // ── North (spawn y=-50, travel south) ───────────────────────────────────
    // Right turn → exits East at y=420 (innermost eastbound lane)
    map.insert((Direction::North, Route::Right), vec![
        Vec2 { x: 600.0, y: -50.0 },
        Vec2 { x: 600.0, y: 270.0 },
        Vec2 { x: 600.0, y: 390.0 }, // turn entry
        Vec2 { x: 630.0, y: 420.0 }, // 45° diagonal → east edge
        Vec2 { x: 950.0, y: 420.0 },
    ]);

    // Straight → exits South at x=540
    map.insert((Direction::North, Route::Straight), vec![
        Vec2 { x: 540.0, y: -50.0 },
        Vec2 { x: 540.0, y: 270.0 },
        Vec2 { x: 540.0, y: 630.0 },
        Vec2 { x: 540.0, y: 950.0 },
    ]);

    // Left turn → exits West at y=600 (outermost westbound lane)
    map.insert((Direction::North, Route::Left), vec![
        Vec2 { x: 480.0, y: -50.0 },
        Vec2 { x: 480.0, y: 270.0 },
        Vec2 { x: 480.0, y: 570.0 }, // turn entry
        Vec2 { x: 450.0, y: 600.0 }, // 45° diagonal
        Vec2 { x: 270.0, y: 600.0 }, // west edge
        Vec2 { x: -50.0, y: 600.0 },
    ]);

    // ── West (spawn x=-50, travel east) ─────────────────────────────────────
    // Right turn → exits North at x=420 (innermost northbound lane)
    map.insert((Direction::West, Route::Right), vec![
        Vec2 { x: -50.0, y: 300.0 },
        Vec2 { x: 270.0, y: 300.0 },
        Vec2 { x: 390.0, y: 300.0 }, // turn entry
        Vec2 { x: 420.0, y: 270.0 }, // 45° diagonal → north edge
        Vec2 { x: 420.0, y: -50.0 },
    ]);

    // Straight → exits East at y=360
    map.insert((Direction::West, Route::Straight), vec![
        Vec2 { x: -50.0, y: 360.0 },
        Vec2 { x: 270.0, y: 360.0 },
        Vec2 { x: 630.0, y: 360.0 },
        Vec2 { x: 950.0, y: 360.0 },
    ]);

    // Left turn → exits South at x=600 (outermost southbound lane)
    map.insert((Direction::West, Route::Left), vec![
        Vec2 { x: -50.0, y: 420.0 },
        Vec2 { x: 270.0, y: 420.0 },
        Vec2 { x: 570.0, y: 420.0 }, // turn entry
        Vec2 { x: 600.0, y: 450.0 }, // 45° diagonal
        Vec2 { x: 600.0, y: 630.0 }, // south edge
        Vec2 { x: 600.0, y: 950.0 },
    ]);

    // ── East (spawn x=950, travel west) ─────────────────────────────────────
    // Right turn → exits South at x=480 (innermost southbound lane)
    map.insert((Direction::East, Route::Right), vec![
        Vec2 { x: 950.0, y: 600.0 },
        Vec2 { x: 630.0, y: 600.0 },
        Vec2 { x: 510.0, y: 600.0 }, // turn entry
        Vec2 { x: 480.0, y: 630.0 }, // 45° diagonal → south edge
        Vec2 { x: 480.0, y: 950.0 },
    ]);

    // Straight → exits West at y=540
    map.insert((Direction::East, Route::Straight), vec![
        Vec2 { x: 950.0, y: 540.0 },
        Vec2 { x: 630.0, y: 540.0 },
        Vec2 { x: 270.0, y: 540.0 },
        Vec2 { x: -50.0, y: 540.0 },
    ]);

    // Left turn → exits North at x=300 (outermost northbound lane)
    map.insert((Direction::East, Route::Left), vec![
        Vec2 { x: 950.0, y: 480.0 },
        Vec2 { x: 630.0, y: 480.0 },
        Vec2 { x: 330.0, y: 480.0 }, // turn entry
        Vec2 { x: 300.0, y: 450.0 }, // 45° diagonal
        Vec2 { x: 300.0, y: 270.0 }, // north edge
        Vec2 { x: 300.0, y: -50.0 },
    ]);

    map
}
```

- [ ] In the `tests` module inside `intersection.rs`, remove the test `spec_confirmed_north_right_west_straight_no_conflict` (North-Right and West-Straight now share a cell at column 5, row 1 in the new geometry — they are correctly flagged as conflicting). Replace it with a test for a pair that is genuinely non-conflicting in the new geometry:

```rust
#[test]
fn spec_south_straight_north_right_no_conflict() {
    // South-Straight occupies column 1 (x=360); North-Right occupies column 5
    // (x=600). Paths share no intersection cells.
    let mut mgr = IntersectionManager::new();
    assert!(mgr.request_reservation(1, Direction::South, Route::Straight));
    assert!(mgr.request_reservation(2, Direction::North, Route::Right));
    assert_eq!(mgr.active_count(), 2);
}
```

- [ ] Run the intersection tests:

```
cargo test --lib intersection
```

Expected: all tests pass. If `conflicting_request_denied` fails, verify North-Straight (x=540, col 4) and South-Left (y=300 row 0, shares col 4 at row 0) still conflict — they do.

- [ ] Commit:

```bash
git add src/intersection.rs
git commit -m "refactor: rewrite all 12 paths for 6-lane layout, fix conflict test"
```

---

## Task 3 — Update renderer for new intersection size (`renderer.rs`)

**Files:** `src/renderer.rs`

The road-strip and intersection-box rects already use `INTER_X/Y/W/H` constants — those update automatically. Only the **stop lines** and **lane-marking y/x bounds** are hardcoded.

- [ ] In the `draw` function, replace the four hardcoded stop-line rects with constant-driven versions. Find:

```rust
    // Stop lines
    canvas.fill_rect(Rect::new(300, 295, 300, 5)).ok();
    canvas.fill_rect(Rect::new(300, 600, 300, 5)).ok();
    canvas.fill_rect(Rect::new(295, 300, 5, 300)).ok();
    canvas.fill_rect(Rect::new(600, 300, 5, 300)).ok();
```

Replace with:

```rust
    // Stop lines — driven by constants so they track any future resize
    canvas.fill_rect(Rect::new(INTER_X as i32, INTER_Y as i32 - 5, INTER_W as u32, 5)).ok();
    canvas.fill_rect(Rect::new(INTER_X as i32, (INTER_Y + INTER_H) as i32, INTER_W as u32, 5)).ok();
    canvas.fill_rect(Rect::new(INTER_X as i32 - 5, INTER_Y as i32, 5, INTER_H as u32)).ok();
    canvas.fill_rect(Rect::new((INTER_X + INTER_W) as i32, INTER_Y as i32, 5, INTER_H as u32)).ok();
```

- [ ] Replace the hardcoded lane-marking loops and center guides. Find:

```rust
    for x in [330, 390, 450, 510, 570] {
        draw_dashed_vertical(canvas, x, 0, 300);
        draw_dashed_vertical(canvas, x, 600, 900);
    }
    for y in [330, 390, 450, 510, 570] {
        draw_dashed_horizontal(canvas, y, 0, 300);
        draw_dashed_horizontal(canvas, y, 600, 900);
    }
    // Center guides inside intersection
    draw_dashed_vertical(canvas, 450, 300, 600);
    draw_dashed_horizontal(canvas, 450, 300, 600);
```

Replace with:

```rust
    // Dividers between the 6 lanes — same absolute pixel positions (270+60n)
    // but y/x bounds now driven by constants.
    for x in [330, 390, 450, 510, 570] {
        draw_dashed_vertical(canvas, x, 0, INTER_Y as i32);
        draw_dashed_vertical(canvas, x, (INTER_Y + INTER_H) as i32, WINDOW_HEIGHT as i32);
    }
    for y in [330, 390, 450, 510, 570] {
        draw_dashed_horizontal(canvas, y, 0, INTER_X as i32);
        draw_dashed_horizontal(canvas, y, (INTER_X + INTER_W) as i32, WINDOW_WIDTH as i32);
    }
    // Center guide inside intersection (x=450 = INTER_X + INTER_W/2)
    draw_dashed_vertical(canvas, 450, INTER_Y as i32, (INTER_Y + INTER_H) as i32);
    draw_dashed_horizontal(canvas, 450, INTER_X as i32, (INTER_X + INTER_W) as i32);
```

- [ ] Build and do a visual smoke-test by running the simulation. Confirm:
  - Road strip is visibly wider (270–630 px instead of 300–600 px)
  - Stop lines appear at the correct edges of the intersection box
  - Lane dividers are evenly spaced and align with vehicle paths

```
cargo run
```

- [ ] Commit:

```bash
git add src/renderer.rs
git commit -m "fix(renderer): drive stop lines and lane bounds from INTER constants"
```

---

## Task 4 — Update vehicle tests for new coordinates (`vehicle.rs`)

**Files:** `src/vehicle.rs`

The only test with a hardcoded lane coordinate is `safe_distance_slows_follower`, which places two South-Straight vehicles at x=420. South-Straight is now at x=360.

- [ ] Find in the test:

```rust
        leader.pos   = Vec2 { x: 420.0, y: 820.0 };
        follower.pos = Vec2 { x: 420.0, y: 820.0 + SAFE_DISTANCE * 0.5 };
```

Replace with:

```rust
        leader.pos   = Vec2 { x: 360.0, y: 820.0 };
        follower.pos = Vec2 { x: 360.0, y: 820.0 + SAFE_DISTANCE * 0.5 };
```

- [ ] Run all vehicle tests:

```
cargo test --lib vehicle
```

Expected: all 5 tests pass.

- [ ] Run all tests together to confirm nothing is broken:

```
cargo test
```

Expected: all tests pass.

- [ ] Commit:

```bash
git add src/vehicle.rs
git commit -m "fix(test): update South-Straight lane x from 420 to 360 after 6-lane refactor"
```

---

## Task 5 — Fix close-call counter (`stats.rs`, `main.rs`)

**Problem:** `record_close_calls` is called every frame and increments `stats.close_calls` for every pair within `CLOSE_CALL_DIST` on that frame. If two vehicles stay within range for 60 frames, the counter grows by 60 instead of 1.

**Fix:** Track which pairs are currently inside the close-call range. Only increment when a pair *enters* the range (transitions from not-close to close).

- [ ] In `src/stats.rs`, change the signature and body of `record_close_calls`:

```rust
use std::collections::HashSet;

pub fn record_close_calls(
    stats: &mut Stats,
    vehicles: &[Vehicle],
    active_pairs: &mut HashSet<(u32, u32)>,
) {
    let mut current: HashSet<(u32, u32)> = HashSet::new();

    for i in 0..vehicles.len() {
        for j in (i + 1)..vehicles.len() {
            let dx = vehicles[i].pos.x - vehicles[j].pos.x;
            let dy = vehicles[i].pos.y - vehicles[j].pos.y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > 0.0 && dist < CLOSE_CALL_DIST {
                let a = vehicles[i].id.min(vehicles[j].id);
                let b = vehicles[i].id.max(vehicles[j].id);
                current.insert((a, b));
            }
        }
    }

    for pair in &current {
        if !active_pairs.contains(pair) {
            stats.close_calls += 1;
        }
    }

    *active_pairs = current;
}
```

Also add `use std::collections::HashSet;` at the top of `stats.rs` if not already present.

- [ ] In `src/main.rs`, add the `active_close_pairs` state and pass it to `record_close_calls`. Find the line:

```rust
    let mut stats = Stats {
```

Add one line after the `stats` block closes:

```rust
    let mut active_close_pairs: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
```

Then find the call:

```rust
        stats::record_close_calls(&mut stats, &vehicles);
```

Replace with:

```rust
        stats::record_close_calls(&mut stats, &vehicles, &mut active_close_pairs);
```

- [ ] Add a unit test in `stats.rs` that verifies a single event is counted even when the pair stays in range for multiple calls:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Direction, Route, Vec2, VehicleState};
    use std::collections::HashSet;

    fn make_vehicle(id: u32, x: f32, y: f32) -> Vehicle {
        Vehicle {
            id,
            direction: Direction::South,
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

    #[test]
    fn close_call_counted_once_not_per_frame() {
        let mut stats = Stats {
            total_passed: 0,
            max_velocity: 0.0,
            min_velocity: f32::MAX,
            max_time_ms: 0,
            min_time_ms: u64::MAX,
            close_calls: 0,
        };
        let mut pairs: HashSet<(u32, u32)> = HashSet::new();

        // Two vehicles within CLOSE_CALL_DIST
        let vehicles = vec![
            make_vehicle(1, 0.0, 0.0),
            make_vehicle(2, CLOSE_CALL_DIST * 0.5, 0.0),
        ];

        // Simulate 5 consecutive frames — should count exactly 1 close call
        for _ in 0..5 {
            record_close_calls(&mut stats, &vehicles, &mut pairs);
        }
        assert_eq!(stats.close_calls, 1, "same pair in range for 5 frames = 1 event");
    }

    #[test]
    fn close_call_resets_when_vehicles_separate() {
        let mut stats = Stats {
            total_passed: 0,
            max_velocity: 0.0,
            min_velocity: f32::MAX,
            max_time_ms: 0,
            min_time_ms: u64::MAX,
            close_calls: 0,
        };
        let mut pairs: HashSet<(u32, u32)> = HashSet::new();

        let close = vec![
            make_vehicle(1, 0.0, 0.0),
            make_vehicle(2, CLOSE_CALL_DIST * 0.5, 0.0),
        ];
        let far = vec![
            make_vehicle(1, 0.0, 0.0),
            make_vehicle(2, CLOSE_CALL_DIST * 2.0, 0.0),
        ];

        record_close_calls(&mut stats, &close, &mut pairs); // enter: +1
        record_close_calls(&mut stats, &far,   &mut pairs); // exit
        record_close_calls(&mut stats, &close, &mut pairs); // re-enter: +1

        assert_eq!(stats.close_calls, 2, "pair that separates and re-enters = 2 events");
    }
}
```

- [ ] Run the stats tests:

```
cargo test --lib stats
```

Expected: both tests pass.

- [ ] Run the full test suite one final time:

```
cargo test
```

Expected: all tests pass.

- [ ] Commit:

```bash
git add src/stats.rs src/main.rs
git commit -m "fix: count close-call events not frames; track active pairs per frame"
```

---

## Verification Checklist

After all 5 tasks:

- [ ] `cargo test` — all pass
- [ ] `cargo run` — launch sim, press R for random mode, watch for ~30 seconds
  - No two vehicles visually overlap
  - Vehicles stop and queue correctly at the intersection
  - Close-calls HUD counter stays low and grows slowly (not by hundreds per second)
  - Stats overlay on Esc shows sensible close-call count
