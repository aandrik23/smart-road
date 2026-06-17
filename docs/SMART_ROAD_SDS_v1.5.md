# Software Design Specification
## smart road
**Language:** Rust | **Graphics:** SDL2 | **Version:** 1.5

---

## 1. Overview

A real-time simulation of a smart cross-intersection managing autonomous vehicles (AVs). The system controls vehicle velocity to prevent collisions and minimize congestion. All vehicles are AVs; no human-driven or emergency vehicles are considered.

---

## 2. Module Structure

```
src/
├── main.rs          # Entry point, game loop, SDL2 init
├── types.rs         # All constants, enums, structs (required)
├── intersection.rs  # Intersection logic, reservation/slot algorithm
├── vehicle.rs       # AV physics, movement, route following
├── renderer.rs      # SDL2 drawing, sprite animation, rotation
├── input.rs         # Keyboard event handling
└── stats.rs         # Statistics collection and end-screen rendering
```

---

## 3. `types.rs` — Constants, Enums, Structs

`types.rs` is the **single source of truth** for all data shapes and named values.
It must contain only `pub const`, `enum`, and `struct` definitions — no logic,
no methods with side effects, no imports from sibling modules. Every other module
imports from `types.rs`. Treat it as a data dictionary.

### 3.1 Constants

```rust
pub const WINDOW_WIDTH:  u32 = 900;
pub const WINDOW_HEIGHT: u32 = 900;
pub const LANE_WIDTH:    f32 = 60.0;
pub const TILE_SIZE:     u32 = 60;

// Intersection bounding box (pixels)
pub const INTER_X: f32 = 300.0;
pub const INTER_Y: f32 = 300.0;
pub const INTER_W: f32 = 300.0;
pub const INTER_H: f32 = 300.0;

// Velocity levels — exactly 3 named speeds used as probe candidates by the scheduling
// algorithm. target_vel is a continuous f32; these are the preferred values but the
// scheduler may return an exact speed (dist/gap_seconds) when no named speed fits.
// SPEED_SLOW is the absolute minimum: velocity is never clamped below this.
pub const SPEED_SLOW:   f32 = 40.0;   // px/s — minimum; also scheduler floor
pub const SPEED_MEDIUM: f32 = 100.0;  // px/s
pub const SPEED_FAST:   f32 = 180.0;  // px/s

pub const SAFE_DISTANCE:     f32 = 80.0;  // px — following distance floor (strictly positive)
pub const CLOSE_CALL_DIST:   f32 = 30.0;  // px — violation threshold (< SAFE_DISTANCE)
pub const SPAWN_INTERVAL_MS: u64 = 800;   // min ms between spawns per direction
pub const TRIGGER_DIST:      f32 = 200.0; // px — reservation request distance from stop line
pub const TRANSIT_LENGTH:    f32 = 300.0; // px — length of path through intersection box (approx)
pub const ACCEL_RATE:        f32 = 60.0;  // px/s²
pub const DECEL_RATE:        f32 = 120.0; // px/s²
```

If a new speed level is ever needed, add it here as a named constant and update
the `Speed` enum. Never use a bare float literal for a speed value anywhere else
in the codebase.

### 3.2 Enums

```rust
pub enum Direction { North, South, East, West }

pub enum Route { Right, Straight, Left }

pub enum VehicleState {
    Approaching,    // outside intersection, moving toward stop line
    InIntersection,
    Exiting,        // past intersection box, on outgoing lane, heading offscreen
    Removed,
}

// Named speed levels — mirrors the three SPEED_* constants.
pub enum Speed { Slow, Medium, Fast }
```

### 3.3 Core Structs

```rust
pub struct Vec2 { pub x: f32, pub y: f32 }

pub struct Vehicle {
    pub id:               u32,
    pub direction:        Direction,
    pub route:            Route,       // assigned at spawn from lane; never changed mid-journey
    pub state:            VehicleState,
    pub pos:              Vec2,
    pub velocity:         f32,         // current speed in px/s; always >= SPEED_SLOW in motion
    pub target_vel:       f32,         // desired speed; always >= SPEED_SLOW; never set to 0
    pub angle_deg:        f32,         // clockwise screen-space degrees for SDL2 copy_ex
    pub path:             Vec<Vec2>,   // pre-computed waypoints: spawn → stop line → intersection → outgoing → offscreen
    pub path_index:       usize,       // index of the current target waypoint
    pub entry_time_ms:    u64,         // set when vehicle first enters Approaching (TRIGGER_DIST detection)
    pub exit_time_ms:     u64,         // set when vehicle transitions to Removed
    pub distance_travelled: f32,       // cumulative px moved; used for sprite frame index
}

pub struct IntersectionSlot {
    pub reserved_by:        Option<u32>,   // vehicle id
    pub route:              Option<Route>,
    pub scheduled_entry_ms: u64,           // absolute ms when vehicle should cross stop line
    pub scheduled_exit_ms:  u64,           // absolute ms when vehicle should fully clear the box
}

pub struct Stats {
    pub total_passed: u32,
    pub max_velocity: f32,
    pub min_velocity: f32,
    pub max_time_ms:  u64,
    pub min_time_ms:  u64,
    pub close_calls:  u32,
}
```

---

## 4. Intersection Layout

The cross-intersection occupies pixels (300, 300) to (600, 600). Each cardinal arm has **6 lanes** of 60 px each. The blueprint defines two lane classes per arm:

- **`spawn_lanes`** — the 3 lanes where vehicles originate and drive *toward* the intersection.
- **`inc_lanes`** — the 3 lanes where vehicles *arrive* after exiting the intersection. These are receive-only; no vehicle spawns here.

Vehicles travel from a `spawn_lane` through the intersection and deposit onto an `inc_lane` of a different arm. A vehicle on a `spawn_lane` can never enter another `spawn_lane` — it always exits onto an `inc_lane`. This is the fundamental rule of the blueprint.

### 4.1 Lane Coordinates and Route Assignment

The window is 900×900 px. The intersection box occupies x=300–600, y=300–600.

Each arm is 180 px wide (6 × 60 px lanes). The `spawn_lanes` and `inc_lanes` occupy opposite halves of each arm, as shown in the blueprint:

#### North arm (x = 120–300, vertical, y = 0–300)

```
x range     Class        Vehicles        Routes (left→right in blueprint)
120–180     spawn_lane   N→S, route r    r  (turns right → exits East arm)
180–240     spawn_lane   N→S, route s    s  (goes straight → exits South arm)
240–300     spawn_lane   N→S, route l    l  (turns left → exits West arm)
300–360     inc_lane     receives S→N Right arrivals
360–420     inc_lane     receives S→N Straight arrivals
420–480     inc_lane     receives S→N Left arrivals
```

Lane center x values — spawn: 150, 210, 270 · incoming: 330, 390, 450

#### South arm (x = 300–480, vertical, y = 600–900)

```
x range     Class        Vehicles        Routes (left→right in blueprint)
300–360     inc_lane     receives N→S Left arrivals
360–420     inc_lane     receives N→S Straight arrivals
420–480     inc_lane     receives N→S Right arrivals
480–540     spawn_lane   S→N, route l    l  (turns left → exits East arm)
540–600     spawn_lane   S→N, route s    s  (goes straight → exits North arm)
600–660     spawn_lane   S→N, route r    r  (turns right → exits West arm)
```

Lane center x values — incoming: 330, 390, 450 · spawn: 510, 570, 630

#### East arm (y = 120–300, horizontal, x = 600–900)

```
y range     Class        Vehicles        Routes (top→bottom in blueprint)
120–180     spawn_lane   E→W, route r    r  (turns right → exits South arm)
180–240     spawn_lane   E→W, route s    s  (goes straight → exits West arm)
240–300     spawn_lane   E→W, route l    l  (turns left → exits North arm)
300–360     inc_lane     receives W→E Right arrivals
360–420     inc_lane     receives W→E Straight arrivals
420–480     inc_lane     receives W→E Left arrivals
```

Lane center y values — spawn: 150, 210, 270 · incoming: 330, 390, 450

#### West arm (y = 300–480, horizontal, x = 0–300)

```
y range     Class        Vehicles        Routes (top→bottom in blueprint)
300–360     inc_lane     receives E→W Left arrivals
360–420     inc_lane     receives E→W Straight arrivals
420–480     inc_lane     receives E→W Right arrivals
480–540     spawn_lane   W→E, route l    l  (turns left → exits South arm)
540–600     spawn_lane   W→E, route s    s  (goes straight → exits East arm)
600–660     spawn_lane   W→E, route r    r  (turns right → exits North arm)
```

Lane center y values — incoming: 330, 390, 450 · spawn: 510, 570, 630

#### Summary table — spawn lane centers

| Direction | Route | Spawn lane center (x or y) | Spawns at (off-screen coord) |
|-----------|-------|-----------------------------|------------------------------|
| N→S       | Right    | x = 150 | (150, -60) — off top  |
| N→S       | Straight | x = 210 | (210, -60) — off top  |
| N→S       | Left     | x = 270 | (270, -60) — off top  |
| S→N       | Left     | x = 510 | (510, 960) — off bot  |
| S→N       | Straight | x = 570 | (570, 960) — off bot  |
| S→N       | Right    | x = 630 | (630, 960) — off bot  |
| E→W       | Right    | y = 150 | (960, 150) — off right|
| E→W       | Straight | y = 210 | (960, 210) — off right|
| E→W       | Left     | y = 270 | (960, 270) — off right|
| W→E       | Left     | y = 510 | (-60, 510) — off left |
| W→E       | Straight | y = 570 | (-60, 570) — off left |
| W→E       | Right    | y = 630 | (-60, 630) — off left |

#### Summary table — inc lane centers (exit targets)

| Arriving from | Route taken | Lands on arm | Inc lane center |
|---------------|-------------|--------------|-----------------|
| N→S Right     | → East      | East arm     | y = 330         |
| N→S Straight  | → South     | South arm    | x = 390         |
| N→S Left      | → West      | West arm     | y = 390         |
| S→N Left      | → East      | East arm     | y = 390         |
| S→N Straight  | → North     | North arm    | x = 390         |
| S→N Right     | → West      | West arm     | y = 330         |
| E→W Right     | → South     | South arm    | x = 330         |
| E→W Straight  | → West      | West arm     | y = 210         |
| E→W Left      | → North     | North arm    | x = 210         |
| W→E Left      | → South     | South arm    | x = 510         |
| W→E Straight  | → East      | East arm     | y = 570         |
| W→E Right     | → North     | North arm    | x = 570         |

### 4.2 Waypoint Paths

All 12 `(Direction, Route)` paths are **pre-computed at startup** as `Vec<Vec2>` and stored in a `HashMap<(Direction, Route), Vec<Vec2>>`. Never recalculated mid-simulation. Each path covers: **spawn (off-screen) → stop line → intersection traversal → inc_lane → off-screen**.

The stop line for each arm is at the edge of the intersection box (y=300 for north, y=600 for south, x=600 for east, x=300 for west). After exiting the box, paths travel along the correct `inc_lane` center coordinate to off-screen.

#### Full path table

| Direction | Route    | Waypoints (x, y)                                        | Exits onto     |
|-----------|----------|---------------------------------------------------------|----------------|
| N→S       | Right    | (150,-60)→(150,300)→(150,360)→(600,360)→(960,360)     | East inc y=360 |
| N→S       | Straight | (210,-60)→(210,300)→(210,600)→(210,960)               | South inc x=210 — **wait** |
| N→S       | Left     | (270,-60)→(270,300)→(270,600)→(390,600)→(390,960)     | West  inc y=390 |
| S→N       | Left     | (510,960)→(510,600)→(510,300)→(510,-60)               | North inc x=510 — **wait** |
| S→N       | Straight | (570,960)→(570,600)→(570,300)→(570,-60)               | North inc x=570 |
| S→N       | Right    | (630,960)→(630,600)→(630,360)→(0,360)                 | West  inc y=360 — **see note** |
| E→W       | Right    | (960,150)→(600,150)→(600,390)→(390,390)→(390,960)     | South inc x=390 |
| E→W       | Straight | (960,210)→(600,210)→(300,210)→(-60,210)               | West  inc y=210 |
| E→W       | Left     | (960,270)→(600,270)→(300,270)→(210,300)→(210,-60)     | North inc x=210 |
| W→E       | Left     | (-60,510)→(300,510)→(600,510)→(510,600)→(510,960)     | South inc x=510 |
| W→E       | Straight | (-60,570)→(300,570)→(600,570)→(960,570)               | East  inc y=570 |
| W→E       | Right    | (-60,630)→(300,630)→(570,600)→(570,-60)               | North inc x=570 |

> **Note on straight-through paths:** N→S Straight and S→N Left share x=510/x=210 — but these are different arms (south vs north). Straight-through vehicles travel the full length of the road arm and arrive at the matching `inc_lane` on the opposite side. There is no spatial conflict because opposing directions use opposite halves of each arm. The reservation system (§5) prevents time conflicts.

> **Turn geometry:** Turning paths use the intersection box interior as the curve. The waypoint at the box corner (e.g. `(600, 360)` for N→S Right) is the turn apex. The renderer interpolates linearly between waypoints; `angle_deg` is updated at each waypoint to produce the visual turn.

### 4.3 Road Rendering Geometry

Each arm renders 6 lane-width strips (60 px each). The divider between `spawn_lanes` and `inc_lanes` is drawn as a solid yellow center line:

| Road arm   | Divider position                             |
|------------|----------------------------------------------|
| North arm  | Vertical line at x = 300, y = 0–300         |
| South arm  | Vertical line at x = 480, y = 600–900       |
| East arm   | Horizontal line at y = 300, x = 600–900     |
| West arm   | Horizontal line at y = 480, x = 0–300       |

Dashed white lane markings run between individual lanes within each half.

---

## 5. Algorithm — Time-Window Scheduling

### 5.1 Concept

The intersection manages a set of **time-window slots**. Each slot records which vehicle holds it, which route it covers, and the exact millisecond window `[scheduled_entry_ms, scheduled_exit_ms]` during which that vehicle will occupy the intersection box. Before a vehicle enters, the scheduler computes the speed it must travel to arrive precisely at its assigned window — no earlier, no later. This guarantees collision-free traversal by construction: two conflicting paths can never have overlapping time windows.

### 5.2 Conflict Detection

Paths sharing grid cells are conflicting. Non-conflicting paths may proceed simultaneously. The conflict table is **pre-computed at startup** — never evaluated dynamically per frame.

| Route A      | Route B      | Conflict? |
|--------------|--------------|-----------|
| Straight N→S | Straight E→W | Yes       |
| Right  N→E   | Right  W→N   | No        |
| Left   N→W   | Straight E→W | Yes       |
| Right  N→E   | Straight W→E | No        |

### 5.3 `has_time_conflict(route, entry_ms, exit_ms) -> bool`

Lives in `intersection.rs`. Scans all active slots whose routes conflict with `route` and returns `true` if any of their `[scheduled_entry_ms, scheduled_exit_ms]` windows overlap with `[entry_ms, exit_ms]`.

```
overlap = NOT (exit_ms <= other.scheduled_entry_ms
            OR entry_ms >= other.scheduled_exit_ms)
```

### 5.4 `compute_approach_speed(id, dir, route, dist_to_stop, now_ms) -> f32`

**This is the main algorithm.** Called every frame for every `Approaching` vehicle by `vehicle.rs`. Lives in `intersection.rs`. Returns the `target_vel` the vehicle should pursue this frame. Also books or updates the vehicle's slot as a side effect.

The transit time for a given speed is `TRANSIT_LENGTH / speed` seconds, converted to ms.

```
transit_ms(speed) = (TRANSIT_LENGTH / speed * 1000.0) as u64
arrival_ms(speed) = now_ms + (dist_to_stop / speed * 1000.0) as u64
window(speed)     = [arrival_ms(speed), arrival_ms(speed) + transit_ms(speed)]
```

#### Step 1 — Vehicle already has a booked slot

```
remaining_ms = slot.scheduled_entry_ms - now_ms

if remaining_ms <= 0:
    // Missed the slot (held up by vehicle ahead). Re-book:
    // Speculatively find a new slot first (Steps 2–3 below),
    // book it, THEN release the old one. Never release before re-booking.
    → go to Step 2 with old slot still held

if remaining_ms > 0:
    exact_speed = dist_to_stop / (remaining_ms as f32 / 1000.0)
    exact_speed = clamp(exact_speed, SPEED_SLOW, SPEED_FAST)
    return exact_speed
    // Vehicle accelerates or decelerates smoothly toward this each frame.
    // Far away → fast. Close → slow. Speed is derived, not picked.
```

#### Step 2 — No slot (or re-booking after missed slot)

Try the three named speeds as probe candidates in order `[SPEED_FAST, SPEED_MEDIUM, SPEED_SLOW]`:

```
for speed in [SPEED_FAST, SPEED_MEDIUM, SPEED_SLOW]:
    w = window(speed)
    if NOT has_time_conflict(route, w.entry, w.exit):
        book slot: { reserved_by: id, route, scheduled_entry_ms: w.entry,
                     scheduled_exit_ms: w.exit }
        if re-booking: release old slot now
        return speed   // this IS one of the three named speeds
```

If a named speed fits, `target_vel` is exactly one of the three constants. No arbitrary float.

#### Step 3 — Even SPEED_SLOW conflicts

All three named speeds conflict. Find the earliest moment the intersection clears for this route:

```
earliest_entry_ms = min start time after which has_time_conflict returns false
                    (scan the sorted exit times of all conflicting active slots)

gap_seconds = (earliest_entry_ms - now_ms) as f32 / 1000.0
exact_speed = dist_to_stop / gap_seconds
exact_speed = clamp(exact_speed, SPEED_SLOW, SPEED_FAST)

book slot: { reserved_by: id, route,
             scheduled_entry_ms: earliest_entry_ms,
             scheduled_exit_ms:  earliest_entry_ms + transit_ms(exact_speed) }
if re-booking: release old slot now
return exact_speed
```

This is the only case where `target_vel` is a continuous float not equal to a named constant. It is still clamped to `[SPEED_SLOW, SPEED_FAST]`. It means the vehicle drives at precisely the speed needed to arrive when the conflict clears — **no stopping, no waiting, no magic**.

### 5.5 Reservation Lifecycle

```
1. Vehicle spawns off-screen → state = Approaching, target_vel = SPEED_FAST.

2. Each tick while Approaching:
   a. Layer 1 safe-following-distance check (§6.2) — may reduce target_vel.
   b. When within TRIGGER_DIST of stop line:
        entry_time_ms = now_ms   ← stats timer starts here
        scheduled_vel = compute_approach_speed(id, dir, route, dist, now_ms)
        target_vel = min(scheduled_vel, following_distance_vel)
        // Layer 1 always caps the scheduler's output; scheduler never overrides safety.
   c. Advance physics (§6.1).

3. Vehicle crosses stop-line waypoint → state = InIntersection,
   target_vel = SPEED_MEDIUM.

4. Last waypoint INSIDE intersection box cleared → release slot,
   state = Exiting, target_vel = SPEED_FAST.
   *** Release ONLY after full box clearance — never early. ***

5. Offscreen waypoint reached → state = Removed,
   exit_time_ms = now_ms, record stats.
```

`compute_approach_speed` is called **every frame** while `Approaching` and within `TRIGGER_DIST`, not just once. This lets the vehicle continuously re-derive its speed as real time elapses and its distance changes, keeping it locked onto the scheduled arrival time.

### 5.6 Why This Is Collision-Free

- **Intersection conflicts:** `has_time_conflict` guarantees no two conflicting-route slots overlap in time. Two vehicles on conflicting paths physically cannot be in the box simultaneously.
- **On-road rear-end:** Layer 1 safe-following-distance (§6.2) runs independently every frame. If the leader slows (e.g. gets re-booked to a later slot), the follower detects the gap closing and reduces `target_vel` accordingly.
- **No stopping:** Step 1 always produces a speed ≥ `SPEED_SLOW`. Step 3 clamps to `SPEED_SLOW` at minimum. The physics clamp enforces this as a hard floor.
- **Re-booking safety:** Old slot is never released before the new one is confirmed, so there is no window where another vehicle can steal the gap mid-rebooking.

---

## 6. Vehicle Physics

### 6.1 Position Update (per frame, `dt` in seconds)

```rust
// Step 1 — determine target_vel from reservation state and following distance (§5.3, §6.2).

// Step 2 — smooth velocity toward target_vel (no instantaneous speed changes).
if velocity < target_vel {
    velocity += ACCEL_RATE * dt;
}
if velocity > target_vel {
    velocity -= DECEL_RATE * dt;
}

// Step 3 — enforce floor and ceiling. Vehicles NEVER stop.
// Lower bound is SPEED_SLOW, not 0.0.
velocity = velocity.clamp(SPEED_SLOW, SPEED_FAST);

// Step 4 — advance along waypoint path.
let dir = (path[path_index] - pos).normalise();
pos += dir * velocity * dt;
distance_travelled += velocity * dt;

// Step 5 — waypoint advance.
if (pos - path[path_index]).length() < 2.0 {
    path_index += 1;
    angle_deg = -atan2(
        path[path_index].y - pos.y,
        path[path_index].x - pos.x
    ).to_degrees();  // negated: SDL2 is clockwise, atan2 is counter-clockwise
}
```

Velocity must **never** snap instantly. Any instant speed change in the codebase is a bug.

### 6.2 Collision Avoidance — Speed Control Only, No Stopping

Collision avoidance is handled entirely through **velocity reduction**. Vehicles never stop. There are two independent layers — keep them separate, do not merge.

#### Layer 1 — Safe following distance (on-road, same lane)

Runs every frame for every vehicle, regardless of reservation state. Each vehicle looks ahead along its own lane for the nearest vehicle travelling in the same direction. Filter strictly by `direction` and confirm the candidate is ahead along the travel axis — do not compare against vehicles on perpendicular lanes.

```rust
let gap = distance_to_vehicle_ahead - VEHICLE_LENGTH;
if gap < SAFE_DISTANCE {
    // Proportional reduction: full speed at gap == SAFE_DISTANCE,
    // floor speed (SPEED_SLOW) as gap → 0. Never reaches zero.
    let t = (gap / SAFE_DISTANCE).clamp(0.0, 1.0);
    target_vel = lerp(SPEED_SLOW, target_vel, t);
}
// target_vel is then clamped to >= SPEED_SLOW in §6.1 Step 3.
```

`SAFE_DISTANCE` is strictly positive, never zero, never overridden per-vehicle, and enforced on both approach roads and outgoing roads — not only inside the intersection.

#### Layer 2 — Intersection access control (reservation system)

Vehicles denied a reservation reduce `target_vel` to `SPEED_SLOW` (§5.3). This is the only intersection-specific speed control. The reservation system guarantees two vehicles with conflicting paths are never simultaneously granted access — vehicles inside the box never need to react to each other's position.

#### Scenario reference

| Scenario | Behaviour |
|---|---|
| Vehicle approaches busy intersection | Slows to `SPEED_SLOW`, keeps moving, receives grant before stop line |
| Two vehicles queued in same lane | Leader proceeds; follower slows via Layer 1, never stops |
| Two non-conflicting paths (both turning right) | Both granted simultaneously at `SPEED_MEDIUM` |
| Two conflicting paths | One granted, one at `SPEED_SLOW`; grant scheduled before slower vehicle reaches stop line |
| Vehicle inside intersection | `SPEED_MEDIUM`; conflict grid guarantees no opposing vehicle present |

#### Explicitly forbidden

- `velocity = 0` at any point for any reason.
- `target_vel` set below `SPEED_SLOW` for any reason.
- Stop-and-wait at stop line.
- Collision response after the fact — avoidance is entirely proactive (reservation) or proportional-velocity (safe distance).

### 6.3 Angle for Sprite Rotation

SDL2's `copy_ex` angle is **clockwise in screen space** (y-axis points down). `atan2` in standard math is counter-clockwise. Always negate:

```rust
angle_deg = -atan2(next_wp.y - pos.y, next_wp.x - pos.x).to_degrees();
```

Update `angle_deg` at every waypoint advance. If a vehicle slides sideways through a turn, this update is missing.

### 6.4 Velocity Level Summary

`target_vel` is a continuous `f32` always in `[SPEED_SLOW, SPEED_FAST]`. The scheduler produces it; Layer 1 may further reduce it; the physics clamp enforces the floor.

| Situation | target_vel source |
|---|---|
| No slot yet, SPEED_FAST window free | `SPEED_FAST` (named constant) |
| No slot yet, only SPEED_MEDIUM fits | `SPEED_MEDIUM` (named constant) |
| No slot yet, only SPEED_SLOW fits | `SPEED_SLOW` (named constant) |
| No named speed fits (Step 3) | `dist / gap_seconds`, clamped to `[SPEED_SLOW, SPEED_FAST]` |
| Slot already booked (Step 1) | `dist / remaining_seconds`, clamped to `[SPEED_SLOW, SPEED_FAST]` |
| Inside intersection box | `SPEED_MEDIUM` (fixed) |
| Exiting (offscreen run) | `SPEED_FAST` (fixed) |
| Layer 1 override (vehicle ahead) | `lerp(SPEED_SLOW, scheduled_vel, gap/SAFE_DISTANCE)` |
| Absolute floor (all states) | `SPEED_SLOW` — enforced by `clamp` in §6.1 Step 3 |

---

## 7. Renderer

### 7.1 Layers (drawn in order)

1. Road background (gray rectangles — all arms, both incoming and outgoing)
2. Lane markings (dashed white lines per lane)
3. Center divider lines (solid yellow, per §4.3)
4. Intersection box (slightly lighter gray)
5. Vehicles (sprites, rotated via `copy_ex`)
6. HUD (vehicle count, active reservations)

`renderer.rs` must only read vehicle and intersection state — it must never mutate it.

### 7.2 Sprite Animation

Frame index advances based on `distance_travelled`. Rotation is applied on top — the rendered image faces the direction of travel at every waypoint segment. Static facing direction or hardcoded frame indices are not acceptable.

```rust
let frame_x = (distance_travelled as u32 / FRAME_STRIDE % FRAME_COUNT) * SPRITE_W;
canvas.copy_ex(&texture, src_rect, dst_rect, angle_deg, center, false, false)?;
```

---

## 8. Input Handling

| Key | Action                                                        |
|-----|---------------------------------------------------------------|
| ↑   | Spawn vehicle: North → South (random route from N→S lanes)   |
| ↓   | Spawn vehicle: South → North (random route from S→N lanes)   |
| →   | Spawn vehicle: West  → East  (random route from W→E lanes)   |
| ←   | Spawn vehicle: East  → West  (random route from E→W lanes)   |
| R   | Spawn **exactly one** vehicle with a random direction AND random route |
| Esc | End simulation, display stats window                          |

**R key — single random vehicle per press.** Each press of `R` spawns exactly one vehicle. The direction is chosen randomly from `{N→S, S→N, E→W, W→E}` and the route is chosen randomly from `{Right, Straight, Left}` for that direction. `R` does **not** toggle a continuous mode — one press, one vehicle. The spawn guard still applies: if that direction's `last_spawn_time` is too recent, no vehicle is spawned on that press.

**Arrow keys — directional spawn.** Each press spawns one vehicle in the specified direction with a randomly chosen route (and therefore lane) for that direction.

**Spawn guard:** Each direction tracks `last_spawn_time`. A vehicle is only spawned if `now - last_spawn_time > SPAWN_INTERVAL_MS`. This prevents visual overlap at the spawn point when keys are pressed rapidly.

---

## 9. Statistics

Collected passively every frame; displayed only on Esc in a separate SDL2 overlay. Never displayed mid-simulation.

| Stat             | Description                                                                   |
|------------------|-------------------------------------------------------------------------------|
| Total passed     | Count of vehicles that reached `Removed` state                                |
| Max velocity     | Peak `velocity` recorded across all vehicles, all frames                      |
| Min velocity     | Lowest `velocity` recorded while in motion (floor is `SPEED_SLOW`)           |
| Max transit time | Max `(exit_time_ms - entry_time_ms)` across all vehicles                      |
| Min transit time | Min of the same                                                               |
| Close calls      | Frames where any two vehicles are within `CLOSE_CALL_DIST` (Euclidean distance between `pos` values) |

**Timing window:**
- `entry_time_ms` — set when the vehicle comes within `TRIGGER_DIST` of its stop line and the reservation system first evaluates it (enters `Approaching` detection zone). Not at spawn. Not at intersection box entry.
- `exit_time_ms` — set when vehicle transitions to `Removed` (fully off-canvas).

Close-call detection uses Euclidean distance between `pos` values of all vehicle pairs each frame. Count each frame of violation (or deduplicate per pair — be consistent and document the choice in `stats.rs`).

---

## 10. File Responsibilities Summary

| File             | Owns                                                                              | Must NOT                              |
|------------------|-----------------------------------------------------------------------------------|---------------------------------------|
| `types.rs`       | All pub consts, enums, structs                                                    | Contain logic or side-effecting methods|
| `main.rs`        | SDL2 init, game loop (`event → update → render`), FPS cap                         | Contain physics or rendering detail   |
| `intersection.rs`| Time-window slot manager, conflict table (pre-computed), `compute_approach_speed`, `has_time_conflict`, slot book/release | Spawn vehicles or draw anything       |
| `vehicle.rs`     | Physics update, waypoint traversal, safe-distance Layer 1, velocity floor          | Access SDL2 canvas                    |
| `renderer.rs`    | All SDL2 draw calls, lane markings, divider lines, sprite rotation, HUD, stats overlay | Mutate vehicle or intersection state |
| `input.rs`       | Event pump parsing, spawn guard, random-mode toggle                               | Contain physics or reservation logic  |
| `stats.rs`       | Stats struct update per frame, end-screen layout                                  | Drive simulation state                |

---

## 11. Bonus — Acceleration & Deceleration

Each `Vehicle` carries `target_vel` (desired) and `velocity` (current). The update in §6.1 already handles this. For the bonus `VehicleKind` variant feature:

```rust
pub enum VehicleKind { Standard, Sport, Heavy }
```

Different kinds may have different `ACCEL_RATE`/`DECEL_RATE` per-vehicle values (stored on the `Vehicle` struct as `accel_rate: f32` and `decel_rate: f32`), simulating better or worse performance. The floor `SPEED_SLOW` and ceiling `SPEED_FAST` remain the same for all kinds.

---

## 12. Submission Checklist

Before any change is submitted:

- [ ] All new types and constants are in `types.rs`, nowhere else.
- [ ] No module has gained a responsibility listed under another module in §10.
- [ ] Vehicles face their direction of travel through all waypoints (`angle_deg` updated at each waypoint advance, sign-corrected for SDL2).
- [ ] Slots are released only after the vehicle's last waypoint **inside** the intersection box — never early.
- [ ] When re-booking a slot, the new slot is booked **before** the old one is released.
- [ ] `compute_approach_speed` is called every frame while `Approaching` and within `TRIGGER_DIST`, not just once.
- [ ] Layer 1 (safe-following-distance) output is `min`'d with scheduler output — Layer 1 always wins when lower.
- [ ] `entry_time_ms` is set at first `TRIGGER_DIST` detection, not at spawn and not at box entry.
- [ ] Spawn guard is intact — no two vehicles can overlap at the spawn point.
- [ ] All vehicles spawn off-screen, never at the stop line or intersection edge.
- [ ] `SAFE_DISTANCE` is never zero, never per-vehicle override, enforced on approach and outgoing roads.
- [ ] `CLOSE_CALL_DIST < SAFE_DISTANCE` (it is a violation threshold, not a safe distance).
- [ ] Velocity changes are smooth — no instantaneous speed changes anywhere.
- [ ] `velocity.clamp(SPEED_SLOW, SPEED_FAST)` is the final step of the physics update every tick.
- [ ] Stats are collected passively and displayed only on Esc, not mid-simulation.
- [ ] Conflict table is pre-computed at startup, not evaluated dynamically per frame.
- [ ] Waypoints are pre-computed at startup, not regenerated per frame.

---

## 13. Changelog

### v1.5 (current)
- **§4 completely rewritten:** Lane terminology changed from "incoming/outgoing" to `spawn_lanes` / `inc_lanes` to match blueprint exactly. Full per-arm breakdown with x/y ranges, lane center coordinates, and route assignments derived directly from the blueprint image. Spawn lane centers and inc lane centers listed in dedicated summary tables. Waypoint paths rebuilt from scratch against the corrected coordinates — old paths were based on wrong lane positions. Divider line positions corrected to sit between `spawn_lanes` and `inc_lanes` on each arm.
- **§8 rewritten:** `R` key corrected — it spawns exactly **one** vehicle with random direction + random route per press. Previous spec incorrectly described it as a continuous-mode toggle. Arrow key directions corrected (↑ = N→S, ↓ = S→N).

### v1.4
- **§3.1:** `SPEED_SLOW/MEDIUM/FAST` annotated as probe candidates; `target_vel` clarified as continuous float; `TRANSIT_LENGTH` constant added.
- **§3.3 `IntersectionSlot`:** Added `scheduled_entry_ms` and `scheduled_exit_ms` fields — required by the time-window scheduler.
- **§5 completely rewritten:** Old grant/deny reservation model replaced with time-window scheduling algorithm. New sub-sections: §5.1 Concept, §5.2 Conflict Detection, §5.3 `has_time_conflict`, §5.4 `compute_approach_speed` (Step 1 existing slot, Step 2 probe named speeds, Step 3 exact speed for congestion), §5.5 Lifecycle, §5.6 Collision-free proof.
- **§6.4:** Velocity summary table rewritten to reflect continuous scheduler output and Layer 1 interaction.
- **§10:** `intersection.rs` responsibility updated to include `compute_approach_speed` and `has_time_conflict`.
- **§12:** Checklist updated with scheduler-specific items (re-booking order, per-frame call, Layer 1 min).

### v1.3
- `types.rs` single-source-of-truth rule stated explicitly. `SPEED_SLOW` as absolute floor. `angle_deg` SDL2 sign convention. `entry_time_ms` at TRIGGER_DIST detection. Full owns/must-not table. Submission checklist.

### v1.2
- Collision Avoidance section added. Stop-at-stop-line removed. `clamp(SPEED_SLOW, SPEED_FAST)`.

### v1.1
- 6 lanes per arm, outgoing lanes, full 12-path waypoint table, center divider lines.
- **§3 intro paragraph added:** `types.rs` single-source-of-truth rule restated explicitly.
- **§3.1:** Added rule against magic-number floats for speed values; `SPEED_SLOW` annotated as absolute minimum floor.
- **§3.3 `Vehicle`:** `route` annotated as never-changed mid-journey; `angle_deg` annotated as clockwise SDL2 degrees; `entry_time_ms` annotated as TRIGGER_DIST detection, not spawn.
- **§5.3 step 3:** `entry_time_ms` timing corrected — now set at TRIGGER_DIST detection, matching AGENTS.md §8. (v1.2 incorrectly set it at spawn.)
- **§5.3 step 4:** Explicit warning added: release reservation only after last waypoint *inside* box — never early.
- **§6.1 Step 3:** Clamp lower bound corrected from `0.0` to `SPEED_SLOW`. v1.1/v1.2 both had `clamp(0.0, SPEED_FAST)` in the bonus section — this was the original no-stop bug, now fully removed everywhere.
- **§6.1 Step 5:** `angle_deg` sign flip (`-atan2`) documented here with SDL2 clockwise convention explanation.
- **§6.3:** SDL2 clockwise vs atan2 counter-clockwise conflict explained explicitly; negation shown in code.
- **§8:** Clarified that `R` key randomises lane (not route independently); route always derived from lane.
- **§9:** `entry_time_ms` timing clarified: TRIGGER_DIST detection, not spawn, not box entry. Close-call detection method (Euclidean `pos` distance) made explicit.
- **§10:** Expanded to full "owns / must not" table matching AGENTS.md §2.
- **§12:** Submission checklist added (mirrors AGENTS.md §11 pre-submit checks).

### v1.2
- §6.2 added: Collision Avoidance — No Stopping, Speed Only.
- §5.3: removed stop-at-stop-line; SPEED_SLOW floor; reservation timing guarantee §5.4.
- §6.1: clamp added (incorrectly used 0.0 as lower bound — fixed in v1.3).
- §6.4: full velocity summary table.

### v1.1
- §4 rewritten: 6 lanes per arm, full 12-path waypoint table, outgoing lanes.
- §4.3 added: center divider line coordinates.
- §5.3: entry_time_ms at spawn (corrected to TRIGGER_DIST in v1.3).
- §6.3: Exiting → SPEED_FAST.
- TRIGGER_DIST promoted to named constant.
