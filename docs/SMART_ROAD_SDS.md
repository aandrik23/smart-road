# Software Design Specification
## smart road
**Language:** Rust | **Graphics:** SDL2 | **Version:** 1.0

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

pub const SPEED_SLOW:   f32 = 40.0;   // px/s
pub const SPEED_MEDIUM: f32 = 100.0;  // px/s
pub const SPEED_FAST:   f32 = 180.0;  // px/s

pub const SAFE_DISTANCE:    f32 = 80.0;   // px
pub const CLOSE_CALL_DIST:  f32 = 30.0;   // px — violation threshold
pub const SPAWN_INTERVAL_MS: u64 = 800;   // min ms between spawns per lane
pub const ACCEL_RATE:  f32 = 60.0;   // px/s² (bonus)
pub const DECEL_RATE:  f32 = 120.0;  // px/s²
```

### 3.2 Enums

```rust
pub enum Direction { North, South, East, West }

pub enum Route { Right, Straight, Left }

pub enum VehicleState {
    Approaching,   // outside intersection, may be slowed by algorithm
    InIntersection,
    Exiting,
    Removed,
}

pub enum Speed { Slow, Medium, Fast }
```

### 3.3 Core Structs

```rust
pub struct Vec2 { pub x: f32, pub y: f32 }

pub struct Vehicle {
    pub id:            u32,
    pub direction:     Direction,
    pub route:         Route,
    pub state:         VehicleState,
    pub pos:           Vec2,
    pub velocity:      f32,
    pub target_vel:    f32,       // for smooth accel/decel (bonus)
    pub angle_deg:     f32,       // for sprite rotation
    pub path:          Vec<Vec2>, // waypoints through intersection
    pub path_index:    usize,
    pub entry_time_ms: u64,       // stats: when detection started
    pub exit_time_ms:  u64,
    pub distance_travelled: f32,
}

pub struct IntersectionSlot {
    pub reserved_by: Option<u32>,  // vehicle id
    pub route:       Option<Route>,
}

pub struct Stats {
    pub total_passed:   u32,
    pub max_velocity:   f32,
    pub min_velocity:   f32,
    pub max_time_ms:    u64,
    pub min_time_ms:    u64,
    pub close_calls:    u32,
}
```

---

## 4. Intersection Layout

The cross-intersection occupies pixels (300, 300) to (600, 600). Each cardinal approach has **3 lanes** (right, straight, left), each 60 px wide.

### 4.1 Lane Entry Points & Directions

| Key      | Origin   | Lanes (px x, fixed y or x) | Routes (L→R or T→B) |
|----------|----------|-----------------------------|----------------------|
| Arrow Up    | South → North | x: 360, 420, 480  | Right, Straight, Left |
| Arrow Down  | North → South | x: 540, 480, 420  | Right, Straight, Left |
| Arrow Right | West  → East  | y: 360, 420, 480  | Right, Straight, Left |
| Arrow Left  | East  → West  | y: 540, 480, 420  | Right, Straight, Left |

### 4.2 Waypoint Paths

Each `(Direction, Route)` pair maps to a fixed list of `Vec2` waypoints. Example for `(South, Right)`:

```
Spawn (360, 900) → approach stop line (360, 600) → turn entry (360, 540) → exit (300, 540) → offscreen (0, 540)
```

All 12 paths are pre-computed at startup and stored in a `HashMap<(Direction, Route), Vec<Vec2>>`.

---

## 5. Algorithm — Reservation-Based Intersection Control

### 5.1 Concept

The intersection is divided into a logical **conflict grid**. Before a vehicle enters, it requests a **reservation** for the set of grid cells its path will occupy. The manager grants or denies the reservation based on current occupancy.

### 5.2 Conflict Detection

Paths that share grid cells are **conflicting**. Non-conflicting paths (e.g. two vehicles turning right from perpendicular directions) may proceed simultaneously.

Conflict table (abbreviated):

| Route A      | Route B       | Conflict? |
|--------------|---------------|-----------|
| Straight N→S | Straight E→W  | Yes       |
| Right  N→E   | Right  W→N    | No        |
| Left   N→W   | Straight E→W  | Yes       |
| Right  N→E   | Straight W→E  | No        |

### 5.3 Reservation Flow

```
1. Vehicle enters VehicleState::Approaching.
2. Each tick: if vehicle is within TRIGGER_DIST (200 px) of stop line:
   a. Request reservation from IntersectionManager.
   b. If GRANTED  → set target_vel = SPEED_MEDIUM, advance toward intersection.
   c. If DENIED   → set target_vel = SPEED_SLOW (or 0 at stop line).
3. Vehicle crosses waypoints → state = InIntersection.
4. On exit waypoint reached → release reservation, state = Exiting.
5. When off-screen → state = Removed, record stats.
```

### 5.4 Safe Following Distance

Each frame, a vehicle checks the vehicle ahead (same lane, same direction). If distance < `SAFE_DISTANCE`, it reduces `target_vel` proportionally. This is independent of the reservation system.

---

## 6. Vehicle Physics

### 6.1 Position Update (per frame, `dt` in seconds)

```
velocity = lerp(velocity, target_vel, decel_rate * dt)   // smooth speed change
pos += direction_unit_vec * velocity * dt
distance_travelled += velocity * dt
```

For waypoint-based paths, the vehicle moves toward `path[path_index]`. When within 2 px, it advances to the next waypoint, updating `angle_deg` to face the next segment.

### 6.2 Angle (for sprite rotation)

```
angle_deg = atan2(next_wp.y - pos.y, next_wp.x - pos.x).to_degrees()
```

SDL2's `copy_ex` is used to render the sprite at the computed angle.

### 6.3 Velocity Levels

| State         | target_vel        |
|---------------|-------------------|
| Free approach | SPEED_FAST        |
| Reservation pending | SPEED_SLOW  |
| In intersection | SPEED_MEDIUM    |
| Following vehicle | clamped by gap  |

---

## 7. Renderer

### 7.1 Layers (drawn in order)

1. Road background (gray rectangles)
2. Lane markings (dashed white lines)
3. Intersection box (slightly lighter gray)
4. Vehicles (sprites, rotated via `copy_ex`)
5. HUD (vehicle count, active reservations)

### 7.2 Sprite Animation

Each vehicle sprite sheet has frames for idle and turning. The frame index advances based on `distance_travelled` modulo frame stride. Rotation is applied on top for turning routes — the rendered image faces the direction of travel at every waypoint segment.

```rust
let frame_x = (distance_travelled as u32 / FRAME_STRIDE % FRAME_COUNT) * SPRITE_W;
canvas.copy_ex(&texture, src_rect, dst_rect, angle_deg, center, false, false)?;
```

---

## 8. Input Handling

| Key        | Action                                              |
|------------|-----------------------------------------------------|
| ↑          | Spawn vehicle: South → North (random route)         |
| ↓          | Spawn vehicle: North → South (random route)         |
| →          | Spawn vehicle: West → East   (random route)         |
| ←          | Spawn vehicle: East → West   (random route)         |
| R          | Toggle continuous random spawn mode                 |
| Esc        | End simulation, display stats window                |

**Spawn guard:** Each direction tracks `last_spawn_time`. A new vehicle is only created if `now - last_spawn_time > SPAWN_INTERVAL_MS`. This prevents stacking.

---

## 9. Statistics

Collected passively during simulation; displayed on Esc in an SDL2 overlay.

| Stat               | Description                                              |
|--------------------|----------------------------------------------------------|
| Total passed       | Count of vehicles that reached Removed state             |
| Max velocity       | Peak `velocity` recorded across all vehicles             |
| Min velocity       | Lowest `velocity` recorded (while in motion)             |
| Max transit time   | Max `(exit_time_ms - entry_time_ms)` across all vehicles |
| Min transit time   | Min of the same                                          |
| Close calls        | Count of frames where two vehicles violated CLOSE_CALL_DIST |

Time window: `entry_time_ms` = when vehicle enters `Approaching` state; `exit_time_ms` = when it transitions to `Removed`.

---

## 10. File Responsibilities Summary

| File               | Responsibility                                                  |
|--------------------|-----------------------------------------------------------------|
| `types.rs`         | All pub consts, enums, structs — no logic                       |
| `main.rs`          | SDL2 init, game loop (`event → update → render`), FPS cap       |
| `intersection.rs`  | Reservation manager, conflict table, slot grant/release         |
| `vehicle.rs`       | Physics update, waypoint traversal, safe-distance check         |
| `renderer.rs`      | All SDL2 draw calls, sprite rotation, HUD, stats overlay        |
| `input.rs`         | Event pump parsing, spawn guard, random-mode toggle             |
| `stats.rs`         | Stats struct update, end-screen layout                          |

---

## 11. Bonus — Acceleration & Deceleration

Each `Vehicle` carries `target_vel` (desired) and `velocity` (current). Every tick:

```
if velocity < target_vel:
    velocity += ACCEL_RATE * dt
if velocity > target_vel:
    velocity -= DECEL_RATE * dt
velocity = clamp(velocity, 0.0, SPEED_FAST)
```

Different vehicle types (configurable via a `VehicleKind` enum) can have different `ACCEL_RATE`/`DECEL_RATE` values, simulating better or worse brakes.
