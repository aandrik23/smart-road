# Track A — Foundation & Types
**Developer:** Dev 1
**Modules owned:** `types.rs`, `main.rs`, `intersection.rs` (path map only), `stats.rs` (partial, shared with C on X1)

---

## Overview

Track A lands the shared data contract that every other module depends on.
Nothing compiles meaningfully until A1 is merged. A2 unlocks vehicle physics
(Track B) and rendering (Track C). X1 is jointly owned with Dev 3 (Track C);
coordinate on the stats overlay layout before starting X1. X2 closes the project.

Dependency order: **A1 → A2 → (B and C run) → X1 → X2**

---

## Tickets

---

### A1 — `types.rs` contract + module scaffold + `main.rs` skeleton
**Wave:** 1 (P0) | **Depends on:** None | **Blocks:** A2, B1, C1, C2

#### What to build

**`types.rs`** — the single source of truth. All constants, all enums, all structs.
No logic, no side effects, no imports from sibling modules. Future modules will
import this; nothing else is allowed to define types.

Constants to define (exact values from SDS §3.1):
```rust
pub const WINDOW_WIDTH:       u32 = 900;
pub const WINDOW_HEIGHT:      u32 = 900;
pub const LANE_WIDTH:         f32 = 60.0;
pub const TILE_SIZE:          u32 = 60;
pub const INTER_X:            f32 = 300.0;
pub const INTER_Y:            f32 = 300.0;
pub const INTER_W:            f32 = 300.0;
pub const INTER_H:            f32 = 300.0;
pub const SPEED_SLOW:         f32 = 40.0;
pub const SPEED_MEDIUM:       f32 = 100.0;
pub const SPEED_FAST:         f32 = 180.0;
pub const SAFE_DISTANCE:      f32 = 80.0;
pub const CLOSE_CALL_DIST:    f32 = 30.0;
pub const SPAWN_INTERVAL_MS:  u64 = 800;
pub const ACCEL_RATE:         f32 = 60.0;
pub const DECEL_RATE:         f32 = 120.0;
```

Enums to define (SDS §3.2):
```rust
pub enum Direction { North, South, East, West }
pub enum Route    { Right, Straight, Left }
pub enum VehicleState { Approaching, InIntersection, Exiting, Removed }
pub enum Speed    { Slow, Medium, Fast }
```

Structs to define (SDS §3.3):
```rust
pub struct Vec2 { pub x: f32, pub y: f32 }

pub struct Vehicle {
    pub id:                 u32,
    pub direction:          Direction,
    pub route:              Route,
    pub state:              VehicleState,
    pub pos:                Vec2,
    pub velocity:           f32,
    pub target_vel:         f32,
    pub angle_deg:          f32,
    pub path:               Vec<Vec2>,
    pub path_index:         usize,
    pub entry_time_ms:      u64,
    pub exit_time_ms:       u64,
    pub distance_travelled: f32,
}

pub struct IntersectionSlot {
    pub reserved_by: Option<u32>,
    pub route:       Option<Route>,
}

pub struct Stats {
    pub total_passed:  u32,
    pub max_velocity:  f32,
    pub min_velocity:  f32,
    pub max_time_ms:   u64,
    pub min_time_ms:   u64,
    pub close_calls:   u32,
}
```

**Module scaffold** — create stub files that compile cleanly. Each stub should
declare the module's public interface as empty or `todo!()` bodies:
- `src/intersection.rs` — empty `pub struct IntersectionManager`
- `src/vehicle.rs` — stub `pub fn update(...)` signature
- `src/renderer.rs` — stub `pub fn draw(...)` signature
- `src/input.rs` — stub `pub fn handle_events(...)` signature
- `src/stats.rs` — stub `pub fn record_passed(...)` etc.

**`main.rs`** — minimal but complete SDL2 skeleton:
- Create a `900×900` window titled "Smart Road"
- Initialise SDL2 video subsystem
- Run an event-pump loop: clear canvas to road-background colour (`#3a3a3a` or similar dark gray), present, cap at ~60 FPS
- Exit cleanly on `Esc` keydown

The loop must already have placeholder slots for `handle_events`, `update`, and
`draw` so wiring (C3) is a drop-in, not a rewrite.

#### Verification gate
- [x] `cargo build` succeeds with zero errors
- [x] `cargo clippy -- -D warnings` passes
- [x] Window opens, shows a uniform dark-gray canvas, exits on Esc
- [x] All types from SDS §3 are present and public in `types.rs`
- [x] No type or constant is defined in any file other than `types.rs`

---

### A2 — Waypoint path pre-computation
**Wave:** 1 (P0) | **Depends on:** A1 | **Blocks:** B1, B2, C2

#### What to build

`build_path_map() -> HashMap<(Direction, Route), Vec<Vec2>>`

Returns all 12 `(Direction, Route)` path vectors. Paths are stored once at
startup; nothing in the simulation recomputes them per frame.

**Lane geometry (SDS §4.1):**

| Arrow key | Travel direction | Lane x-coords (or y-coords) | Routes L→R / T→B |
|-----------|------------------|-----------------------------|------------------|
| Up        | South → North    | x = 360, 420, 480           | Right, Straight, Left |
| Down      | North → South    | x = 540, 480, 420           | Right, Straight, Left |
| Right     | West → East      | y = 360, 420, 480           | Right, Straight, Left |
| Left      | East → West      | y = 540, 480, 420           | Right, Straight, Left |

**Path rules:**
- Every path starts off-screen (outside the 900×900 canvas bounds).
- Every path ends off-screen on the departure side.
- Paths pass through the correct approach lane, through the intersection, and out
  the correct exit lane.
- The example from SDS §4.2 is canonical for `(South, Right)`:
  `(360, 900) → (360, 600) → (360, 540) → (300, 540) → (0, 540)`

**Waypoint density:** provide enough intermediate waypoints around curves that
`angle_deg = atan2(next.y - cur.y, next.x - cur.x)` produces smooth rotation.
Corners and turns need intermediate points; straight segments can be coarser.

**Tests (`#[cfg(test)]`):**
- Assert that the first waypoint of every path is off-screen (outside 0–900 on
  the relevant axis).
- Assert that the last waypoint of every path is off-screen on the exit side.
- Cover at least 4 distinct `(Direction, Route)` pairs.
- Assert that no path is empty and no two consecutive waypoints are identical.

#### Verification gate
- [x] `cargo test` passes with all path-map assertions green
- [x] All 12 paths are present in the returned map
- [x] First/last waypoints of every path are outside canvas bounds
- [x] No path geometry is recomputed after startup (no allocation inside the game loop)

---

### X1 — Stats collection & end-screen *(joint A / C)*
**Wave:** 4 (P3) | **Depends on:** B2, C3 | **Blocks:** X2

> **Ownership split:** Dev 1 (Track A) owns `stats.rs` update methods and the
> `Stats` data logic. Dev 3 (Track C) owns the SDL2 overlay rendering.
> Coordinate on the `Stats` API before starting so both sides compile cleanly.

#### What to build (Dev 1 portion — `stats.rs`)

`Stats` update methods (pure data, no SDL2):
- `record_passed(&mut self)` — increments `total_passed`
- `record_velocity(&mut self, v: f32)` — updates `max_velocity` / `min_velocity`;
  only call this while the vehicle is in motion (`velocity > 0`)
- `record_transit(&mut self, entry_ms: u64, exit_ms: u64)` — computes duration,
  updates `max_time_ms` / `min_time_ms`

Close-call detection (in `vehicle.rs` or `main.rs` — pick one, document it):
- Each frame, for every pair of vehicles compute Euclidean distance between `pos`.
- If distance < `CLOSE_CALL_DIST`, increment `Stats.close_calls`.
- Decide and document: count each frame of violation separately, or deduplicate
  per pair per "incident". Either is acceptable; be consistent.

**Bonus — `VehicleKind` enum** (SDS §11):
Add to `types.rs`:
```rust
pub enum VehicleKind { Standard, Sports, Truck }
```
Each kind gets its own `ACCEL_RATE`/`DECEL_RATE` values. Document how `vehicle.rs`
should read these (e.g. a `match` on `VehicleKind` that returns the pair).
Keep all constants in `types.rs`.

#### Verification gate
- [ ] `record_passed`, `record_velocity`, `record_transit` compile and have basic unit tests
- [ ] Close-call detection triggers and increments the counter
- [ ] Stats overlay shows: total passed, max/min velocity, max/min transit time, close-call count
- [ ] Stats appear only after Esc — never during live simulation
- [ ] `VehicleKind` (bonus) is in `types.rs` with distinct accel/decel per kind

---

### X2 — Project-wide hygiene sweep
**Wave:** 4 (P3) | **Depends on:** X1, C3, all other tickets | **Blocks:** None

#### What to do

This is the final gating ticket. It must run after all other tickets are merged.

1. **Zero warnings:** `cargo build` and `cargo clippy -- -D warnings` must pass
   on the final combined codebase.

2. **Module boundary audit:** walk every `use` import and confirm:
   - No SDL2 types (`Canvas`, `Texture`, etc.) appear in `vehicle.rs`, `intersection.rs`, `stats.rs`, or `types.rs`.
   - No physics or reservation logic appears in `renderer.rs` or `input.rs`.
   - No type or constant is defined outside `types.rs`.

3. **Invariant audit:**
   - `CLOSE_CALL_DIST < SAFE_DISTANCE` — assert or document in `types.rs`.
   - `entry_time_ms` is not set at spawn; confirm by code inspection.
   - No magic numbers in any module other than `types.rs`.

4. **`Cargo.toml`:** confirm minimal, correct dependencies (sdl2 crate, feature
   flags only for what's used).

5. **`.gitignore`:** covers `target/`, editor files, OS junk.

6. **`README.md`:** brief prerequisites (Rust toolchain version, SDL2 dev libs,
   `cargo run` invocation). Not a design doc — just enough to build and run.

#### Verification gate
- [ ] `cargo clippy -- -D warnings` exits 0 on the full codebase
- [ ] No SDL2 call outside `renderer.rs` or `main.rs` SDL2 init block
- [ ] No struct, enum, or constant defined outside `types.rs`
- [ ] `CLOSE_CALL_DIST` (30.0) < `SAFE_DISTANCE` (80.0) — confirmed in code
- [ ] `entry_time_ms` is provably not set at spawn time
- [ ] README exists with build prerequisites

---

## Pre-submission checklist for Track A

Before marking any ticket `[x]`:

- [ ] All new types and constants are in `types.rs`, nowhere else
- [ ] `types.rs` contains no logic (no `impl` blocks with side effects, no `new()` with defaults)
- [ ] `CLOSE_CALL_DIST` < `SAFE_DISTANCE`
- [ ] `entry_time_ms` is set at first algorithm detection — not at spawn
- [ ] Stats are collected passively and displayed only on Esc
- [ ] `cargo clippy -- -D warnings` passes for changed files
