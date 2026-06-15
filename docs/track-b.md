# Track B — Vehicles & Motion
**Developer:** Dev 2
**Modules owned:** `intersection.rs` (reservation manager), `vehicle.rs` (physics + state machine)

---

## Overview

Track B is the simulation's brain. B1 delivers the conflict-aware reservation
manager; B2 delivers the per-frame vehicle physics update that drives everything
else. Both tickets are large because the logic is concentrated in two modules —
read AGENTS.md §3 and §4 completely before writing a single line.

Track B has no rendering responsibilities. `vehicle.rs` must never touch the
SDL2 canvas. All types come from `types.rs` (Track A).

Dependency order: **A1 + A2 → B1 → B2 → (C3, X1 unblocked)**

---

## Tickets

---

### B1 — `IntersectionManager` — reservation system
**Wave:** 2 (P1) | **Depends on:** A1, A2 | **Blocks:** B2

#### What to build

A pure-logic module. No SDL2. No rendering. No vehicle spawning.

**Struct:**
```rust
pub struct IntersectionManager {
    // internal fields — track active reservations
}
```

**Public API:**

```rust
impl IntersectionManager {
    pub fn new() -> Self { ... }

    // Returns true if reservation was granted.
    // A vehicle may hold at most one reservation at a time.
    // Non-conflicting reservations may be held simultaneously.
    pub fn request_reservation(&mut self, id: u32, dir: Direction, route: Route) -> bool

    // Releases the reservation held by `id`.
    // Must only be called after the vehicle has fully cleared the intersection box —
    // not at the first exit waypoint.
    pub fn release_reservation(&mut self, id: u32)

    // Returns true if the vehicle's current position is within TRIGGER_DIST (200 px)
    // of the stop line for its direction.
    pub fn is_in_trigger_zone(&self, vehicle: &Vehicle) -> bool

    // Returns the count of currently active (granted) reservations.
    pub fn active_count(&self) -> usize
}
```

**Conflict table (SDS §5.2):**

Pre-compute a static or `once_cell` conflict table at startup — do **not**
evaluate conflicts dynamically per frame. Two reservations conflict if their
paths share grid cells in the intersection box.

Known non-conflicting pairs (may proceed simultaneously):
- Two vehicles both turning `Right` from perpendicular directions
- `(North, Right)` and `(West, Straight)` — paths do not cross
- Refer to SDS §5.2 for the abbreviated table; build the complete 12×12 matrix

Known conflicting pairs (examples):
- Any two `Straight` paths from perpendicular directions
- `Left` turn vs. any conflicting `Straight` or opposing path

The full table has 12 paths × 12 paths = 144 entries (symmetric, so 66 unique
pairs). Write the table as a `HashSet<(Direction, Route, Direction, Route)>` or
as a 12×12 `bool` array indexed by a path enum. Either is acceptable.

**Trigger zone geometry (`is_in_trigger_zone`):**
- `TRIGGER_DIST = 200.0` (px) — define this constant in `types.rs` if it isn't there yet
- For `Direction::South` (travelling north): trigger when `vehicle.pos.y > INTER_Y + INTER_H - TRIGGER_DIST` ... adapt per direction
- The stop line for each direction is the edge of the intersection box on the approach side

**Unit tests (required):**
- Grant two non-conflicting reservations simultaneously — both return `true`
- Deny a conflicting reservation when one is already held
- Grant after `release_reservation` frees the slot
- `active_count()` reflects grants and releases correctly
- Cover all four conflict pairs mentioned in SDS §5.2

#### Verification gate
- [x] Unit tests pass: all conflict/non-conflict cases covered
- [x] Two non-conflicting vehicles can hold reservations simultaneously
- [x] A conflicting request is denied while the first reservation is active
- [x] `release_reservation` actually frees the slot (subsequent grant succeeds)
- [x] No SDL2 import in `intersection.rs`

---

### B2 — Vehicle physics & state machine
**Wave:** 2 (P1) | **Depends on:** A1, A2, B1 | **Blocks:** C3, X1

#### What to build

The core per-frame update function. This is the most complex ticket in the project.
Read AGENTS.md §3, §4, §5, and §9 (all pitfall sections) before starting.

**Function signature:**
```rust
pub fn update(
    vehicle:      &mut Vehicle,
    dt:           f32,                                        // seconds since last frame
    path_map:     &HashMap<(Direction, Route), Vec<Vec2>>,
    manager:      &mut IntersectionManager,
    all_vehicles: &[Vehicle],                                 // read-only snapshot for safe-distance
    now_ms:       u64,
) -> bool  // returns false when vehicle should be removed
```

**Waypoint traversal:**
- Move toward `vehicle.path[vehicle.path_index]` at current velocity
- When distance to current waypoint ≤ 2 px: advance `path_index`
- After advancing, recompute `angle_deg`:
  ```
  angle_deg = -atan2(next.y - pos.y, next.x - pos.x).to_degrees()
  ```
  The negation is mandatory — SDL2's `copy_ex` angle is clockwise in screen space
  (y-axis points down), while `atan2` is counter-clockwise. Failing this causes
  vehicles to face the wrong direction on half the routes.
- When `path_index >= path.len()`: return `false` (vehicle is off-screen → Removed)

**Position update (SDS §6.1):**
```
velocity += ACCEL_RATE * dt    // if velocity < target_vel
velocity -= DECEL_RATE * dt    // if velocity > target_vel
velocity = clamp(velocity, 0.0, SPEED_FAST)
pos.x += cos(angle_rad) * velocity * dt
pos.y += sin(angle_rad) * velocity * dt   // note: toward current waypoint, not a unit vec
distance_travelled += velocity * dt
```
Velocity must **never** snap instantly. Any instant speed change is a bug.

**State machine:**

```
Spawned with state = Approaching, target_vel = SPEED_FAST
│
├─ Approaching (outside trigger zone)
│   target_vel = SPEED_FAST
│
├─ Approaching (inside trigger zone, reservation not held)
│   call manager.request_reservation(id, dir, route)
│   ├─ GRANTED → target_vel = SPEED_MEDIUM
│   └─ DENIED  → target_vel = SPEED_SLOW (hold at stop line)
│   Set entry_time_ms = now_ms on FIRST trigger zone entry (not spawn)
│
├─ InIntersection
│   Hold reservation, traverse intersection waypoints
│   target_vel = SPEED_MEDIUM
│
├─ Exiting
│   Call manager.release_reservation(id) ONLY after last intersection waypoint
│   target_vel = SPEED_FAST
│
└─ Removed
    Return false from update()
    (caller records stats and removes from vehicle list)
```

**`entry_time_ms` rule (AGENTS.md §8):**
Set exactly once — when the vehicle first enters the trigger zone and
`manager.is_in_trigger_zone()` returns `true`. Do **not** set it at spawn.
Do **not** reset it if the vehicle is denied and re-requests.

**Safe-distance check (SDS §5.4 — independent of reservation system):**
Every frame, find the nearest vehicle ahead in the same lane and direction:
- Filter `all_vehicles` by matching `direction`
- Confirm the candidate is ahead along the travel axis (not behind or on a
  perpendicular lane near the intersection)
- If `distance_to_nearest < SAFE_DISTANCE`: clamp `target_vel` proportionally
  (e.g. `target_vel = SPEED_SLOW` or a fraction based on gap)
- This runs every frame regardless of `VehicleState`

Pitfall: do not compare against all vehicles globally — perpendicular vehicles
near the intersection will produce false positives (AGENTS.md §9, Pitfall 5).

**Reservation release timing (AGENTS.md §9, Pitfall 1):**
Release only after the vehicle has fully cleared the intersection box — after the
last waypoint whose position is inside `(INTER_X, INTER_Y, INTER_W, INTER_H)`.
Not at the first exit waypoint. Not when `state` changes to `Exiting`.
Early release is the most common source of phantom collisions.

#### Verification gate
- [x] Vehicles traverse all 12 route/direction paths to off-screen without freezing
- [x] `angle_deg` updates at every waypoint — no sideways sliding through turns
- [x] Velocity changes are smooth (gradual accel/decel), never instantaneous
- [x] `entry_time_ms` is set at trigger zone entry, not at spawn
- [x] Reservations are released only after full intersection clearance
- [x] Safe-distance check fires on approach roads, not just inside the intersection
- [x] No SDL2 import in `vehicle.rs`

---

## Pre-submission checklist for Track B

Before marking any ticket `[x]`:

- [ ] No struct or constant is defined in `intersection.rs` or `vehicle.rs` — all types live in `types.rs`
- [ ] No SDL2 call in either module
- [ ] `vehicle.rs` does not spawn vehicles (spawning is `input.rs`)
- [ ] `intersection.rs` does not draw anything
- [ ] Vehicles face their direction of travel through all waypoints (`angle_deg` updated at every waypoint)
- [ ] Reservations are released only after full intersection clearance, not at first exit waypoint
- [ ] `entry_time_ms` is set at first algorithm detection — not at spawn, not at box entry
- [ ] `SAFE_DISTANCE` is never zero and is enforced on approach roads
- [ ] Velocity changes are smooth — instant snaps are bugs
- [ ] `cargo clippy -- -D warnings` passes for changed files
