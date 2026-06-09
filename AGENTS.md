# AGENTS.md
## SMART ROAD — Agent Guidelines

This file is for any AI agent (or developer acting as one) touching this codebase.
Read it fully before writing, refactoring, or reviewing any code.

---

## 1. What This Project Is

A **real-time graphical simulation** in Rust + SDL2 of a smart cross-intersection
managing autonomous vehicles (AVs). The simulation is driven by keyboard input,
rendered with SDL2, and governed by a reservation-based intersection algorithm.
There are no human drivers, no emergency vehicles, no network layer.

This is simultaneously:
- A **physics simulation** (velocity, acceleration, deceleration, distance, time)
- A **collision avoidance system** (reservation grants, safe following distance)
- A **graphical animation** (sprite rotation, waypoint traversal, frame animation)
- A **statistics collector** (transit times, speeds, close calls)

Never treat it as just one of these. Every change you make likely touches all four.

---

## 2. Non-Negotiable Structural Rules

### `types.rs` is the single source of truth
All `pub const`, `enum`, and `struct` definitions **must** live in `types.rs`.
No logic. No methods with side effects. No imports from sibling modules.
Every other module imports from `types.rs`.

**Do not:**
- Define a struct in `vehicle.rs` because it felt convenient there.
- Duplicate a constant in `renderer.rs` because it was faster.
- Add a `new()` constructor with business logic into `types.rs`.

If you are unsure where something belongs: if it is a shape of data or a named constant, it is `types.rs`. If it does something, it is not.

### Module responsibilities are strict
| Module | Owns | Must NOT |
|---|---|---|
| `types.rs` | All types and constants | Contain logic |
| `main.rs` | Game loop, SDL2 init, FPS cap | Contain physics or rendering detail |
| `intersection.rs` | Reservation manager, conflict table, grant/release | Spawn vehicles or draw anything |
| `vehicle.rs` | Physics update, waypoint traversal, safe-distance check | Access SDL2 canvas |
| `renderer.rs` | All SDL2 draw calls, sprite rotation, HUD | Mutate vehicle or intersection state |
| `input.rs` | Event pump, spawn guard, random-mode toggle | Contain physics or reservation logic |
| `stats.rs` | Stats struct update, end-screen layout | Drive simulation state |

Crossing these boundaries creates circular dependencies and makes the simulation
untestable. If you feel you need to break a boundary, reconsider the design first.

---

## 3. The Intersection Algorithm — Critical Rules

The intersection uses a **reservation-based model**, not traffic lights.
This is intentional. Traffic lights are designed for human drivers.
AVs negotiate entry cell-by-cell through a conflict table.

### Reservation lifecycle
```
Approaching (> TRIGGER_DIST) → request reservation
  ├── GRANTED → target_vel = SPEED_MEDIUM, proceed
  └── DENIED  → target_vel = SPEED_SLOW, hold at stop line
InIntersection → hold reservation, traverse waypoints
Exiting → release reservation immediately on last waypoint
Removed → off-screen, stats recorded
```

**Never release a reservation before the vehicle has fully cleared the
intersection box.** Early release is the most common source of phantom collisions.

### Conflict table
Non-conflicting paths may proceed simultaneously (e.g. two perpendicular right turns).
The conflict table is pre-computed at startup — do not evaluate conflicts dynamically
per frame. If you modify routes or add lanes, regenerate the full conflict table.

### Two independent safety layers — keep them separate
1. **Reservation system** — prevents intersection-level collisions.
2. **Same-lane following distance** — prevents rear-end collisions on approach roads.

These are not redundant. Do not merge them. The following-distance check runs
every frame for every vehicle regardless of reservation state.

---

## 4. Vehicle Physics — What Must Be True

Every vehicle must always carry these live values (see `Vehicle` struct):
- `velocity` — current speed in px/s
- `target_vel` — desired speed (the algorithm writes here; physics reads here)
- `angle_deg` — current facing angle for sprite rotation
- `distance_travelled` — cumulative px moved (used for sprite frame index)
- `entry_time_ms` / `exit_time_ms` — for statistics

### Velocity levels
There are exactly **3 named speeds**: `SPEED_SLOW`, `SPEED_MEDIUM`, `SPEED_FAST`.
The smart intersection system controls `target_vel` by selecting one of these.
Do not invent intermediate magic numbers. If a new speed level is needed,
add it to `types.rs` as a named constant and update the `Speed` enum.

### Accel/decel (bonus — but treat as core once implemented)
```
velocity += ACCEL_RATE * dt   // if velocity < target_vel
velocity -= DECEL_RATE * dt   // if velocity > target_vel
velocity = clamp(velocity, 0.0, SPEED_FAST)
```
Velocity must **never** snap instantly. If you see an instant speed change anywhere
in the codebase, it is a bug. Different `VehicleKind` variants may have different
`ACCEL_RATE`/`DECEL_RATE` — this is intentional and must be preserved.

### Safe distance
`SAFE_DISTANCE` is a strictly positive constant. It must never be set to zero,
never be overridden per-vehicle, and must be enforced on approach roads, not just
inside the intersection. The stats system tracks **close calls**, defined as any
frame where two vehicles are within `CLOSE_CALL_DIST` of each other — this is
a separate, smaller threshold from `SAFE_DISTANCE`.

---

## 5. Waypoint Paths — Do Not Regenerate Per Frame

All 12 `(Direction, Route)` paths are pre-computed at startup as `Vec<Vec2>`
and stored in a `HashMap<(Direction, Route), Vec<Vec2>>`.

**Do not recalculate waypoints mid-simulation.** If you change the intersection
geometry (e.g. resize `INTER_W`), you must regenerate all 12 paths and verify
each one visually. Each path must:
- Start off-screen (outside the canvas)
- Pass through the correct approach lane
- Navigate the intersection following the correct geometric curve
- Exit into the correct departure lane
- End off-screen on the opposite side

Vehicle movement is always toward `path[path_index]`. When within 2 px of the
current waypoint, advance `path_index` and recompute `angle_deg` using `atan2`.

---

## 6. Sprite Animation — Rotation Is Mandatory

Rendering a vehicle as a static image facing one direction is **not acceptable**.
The spec explicitly requires the vehicle image to face the direction of travel
at all times, including during turns.

Rotation is achieved via SDL2's `copy_ex`:
```rust
canvas.copy_ex(&texture, src_rect, dst_rect, angle_deg, center, false, false)?;
```

The frame index for the sprite sheet is derived from `distance_travelled`:
```rust
let frame_x = (distance_travelled as u32 / FRAME_STRIDE % FRAME_COUNT) * SPRITE_W;
```

Do not use static frame indices. Do not hardcode facing directions.
If a vehicle appears to slide sideways through a turn, `angle_deg` is not
being updated at each waypoint — that is the bug to fix.

---

## 7. Keyboard Input — Spawn Guard Is Mandatory

When a key is held or spammed, vehicles must **not** be created on top of each other.

Each direction tracks `last_spawn_time`. A vehicle is only spawned if:
```
now - last_spawn_time > SPAWN_INTERVAL_MS
```

`SPAWN_INTERVAL_MS` is defined in `types.rs`. Do not lower it below a value
that would allow visual overlap at the spawn point. If you change vehicle size
or spawn positions, re-evaluate this constant.

### Key bindings — these are fixed by spec, do not reassign
| Key | Spawns from | Direction |
|---|---|---|
| Arrow Up | South | → North |
| Arrow Down | North | → South |
| Arrow Right | West | → East |
| Arrow Left | East | → West |
| R | Random | Toggle continuous mode |
| Esc | — | End simulation, show stats |

The `R` key toggles a mode where random vehicles are generated each game loop tick,
subject to the same spawn guard per direction.

---

## 8. Statistics — Timing Window Is Precisely Defined

The spec defines the timing window explicitly:

> "The time starts to count whenever the vehicle is detected by the smart
> intersection algorithm **until the end of the intersection**, which is when
> the vehicle is removed from the canvas."

This means:
- `entry_time_ms` is set when the vehicle enters `VehicleState::Approaching`
  (i.e. when it comes within `TRIGGER_DIST` and the reservation system first
  evaluates it — **not** when it enters the intersection box).
- `exit_time_ms` is set when the vehicle transitions to `VehicleState::Removed`
  (i.e. when it is off-screen and dequeued).

Do not measure from spawn time. Do not measure from intersection entry.
The stats window is: **first algorithm detection → fully off canvas**.

Stats displayed on Esc (in a separate SDL2 overlay window):
- Max vehicles that passed the intersection
- Max and min velocity across all vehicles
- Max and min transit time across all vehicles
- Close call count

---

## 9. Common Pitfalls

### Pitfall 1 — Releasing reservations too early
The most frequent source of simulated collisions. Release only after the
vehicle's last waypoint inside the intersection box is passed, not at the
first exit waypoint.

### Pitfall 2 — Forgetting that `angle_deg` uses SDL2 conventions
SDL2's `copy_ex` angle is **clockwise in screen space** (y-axis points down).
`atan2` in standard math is counter-clockwise. Apply the sign flip:
`angle_deg = -atan2(dy, dx).to_degrees()` or account for this in your
coordinate system. Failing to do so causes vehicles to face the wrong direction
on half the routes.

### Pitfall 3 — Mutating `types.rs` to add logic
Helpers, constructors with defaults, and match arms with side effects do not
belong in `types.rs`. The file must be safe to read as a data dictionary.

### Pitfall 4 — Using pixel distance as the close-call check
Close calls track when two vehicles are within `CLOSE_CALL_DIST` px of each other,
**not** when they share a cell or overlap sprites. Use Euclidean distance between
`pos` values. Count each frame of violation separately, or deduplicate per pair —
be consistent and document your choice.

### Pitfall 5 — Same-lane detection using global vehicle list
The following-distance check must only compare a vehicle to the **nearest vehicle
ahead in the same lane and same direction**. Comparing against all vehicles
produces false positives for vehicles on perpendicular lanes near the intersection.
Filter by `direction` and confirm the candidate is ahead along the travel axis.

### Pitfall 6 — Spawning vehicles at the intersection edge
Vehicles must spawn **off-screen** and drive into view. Spawning at the stop line
or at the intersection edge skips the approach physics and breaks the stats
timing window. Spawn coordinates must be outside the canvas bounds.

### Pitfall 7 — Hardcoding routes instead of reading from the lane
Each lane has exactly one route (`Right`, `Straight`, or `Left`). The vehicle
follows whatever route belongs to the lane it was spawned in. Route is assigned
at spawn time from the lane definition — never randomly overridden mid-journey.
When `R` key generates random vehicles, it randomises the lane (and thus the
route), not the route independently.

---

## 10. Things You Are Explicitly Not Responsible For

- Emergency vehicles — out of scope, do not model them.
- Lane changing — AVs cannot change lanes or routes mid-journey.
- Human driver behaviour — all vehicles are AVs.
- Network, file I/O, databases — this is a self-contained simulation.
- Multiple intersection topologies — only the standard 4-way cross is in scope.

If a request or refactor pulls you toward any of the above, push back.

---

## 11. Before You Submit Any Change

- [ ] All new types and constants are in `types.rs`, nowhere else.
- [ ] No module has gained a responsibility listed under another module in §2.
- [ ] Vehicles still face their direction of travel through all waypoints.
- [ ] Reservations are released only after full intersection clearance.
- [ ] `entry_time_ms` is set at first algorithm detection, not at spawn or box entry.
- [ ] Spawn guard is intact — no two vehicles can overlap at spawn.
- [ ] `SAFE_DISTANCE` is never zero and is enforced on approach roads.
- [ ] `CLOSE_CALL_DIST` < `SAFE_DISTANCE` (it is a violation threshold, not a safe one).
- [ ] Velocity changes are smooth (accel/decel), never instantaneous.
- [ ] Stats are collected passively and displayed only on Esc, not mid-simulation.
