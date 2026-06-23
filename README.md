# smart-road

A real-time simulation of a smart cross-intersection managing autonomous vehicles (AVs), written in Rust with SDL2.

Vehicles spawn from all four cardinal directions, negotiate the intersection without traffic lights, and exit on the other side. The intersection manager grants or denies entry using a reservation-based algorithm that prevents collisions while maximising throughput. A stats overlay is shown when you quit.

![900×900 window with a 6-lane cross-intersection, vehicle sprites, and a live HUD]

---

## Features

- **4-way, 6-lane intersection** — 3 inbound lanes per direction (right, straight, left turns)
- **12 distinct paths** — pre-computed waypoint sequences; vehicles follow them exactly
- **Reservation-based access control** — no traffic lights; entry is granted/denied per-frame based on conflict geometry
- **Smooth kinematics** — acceleration, braking, and safe-distance following between vehicles
- **4 vehicle types** — Civic, Jeep, Sedan, Taxi; each with a directional sprite sheet (8 angles)
- **Live HUD** — vehicle count + active reservation count drawn each frame
- **End-of-session stats** — total vehicles passed, max/min velocity, max/min transit time, close-call count

---

## Controls

| Key | Action |
|---|---|
| `↑` | Spawn a vehicle from the South (traveling north) |
| `↓` | Spawn a vehicle from the North (traveling south) |
| `→` | Spawn a vehicle from the West (traveling east) |
| `←` | Spawn a vehicle from the East (traveling west) |
| `R` | Spawn a vehicle from a random direction |
| `Esc` / close window | Show stats overlay, then quit |

Holding a direction key spawns at most one vehicle per `SPAWN_INTERVAL_MS` (800 ms) from that side. A lane is also capped at 3 queued vehicles.

---

## Build & Run

**Prerequisites:** Rust (stable), SDL2 development libraries.

```bash
# Ubuntu/Debian
sudo apt install libsdl2-dev

# macOS (Homebrew)
brew install sdl2
```

```bash
git clone <repo>
cd smart-road
cargo run --release
```

The `assets/` directory must be present next to the binary (the default `cargo run` working directory is the repo root, so this works out of the box).

---

## Algorithm

### Path representation

All 12 routes (4 directions × 3 turns) are hard-coded as ordered lists of 2D waypoints, built once at startup in `build_path_map()`. Vehicles advance through waypoints at runtime; direction angle is derived from consecutive waypoint pairs so the correct sprite frame is always selected.

### Intersection reservation

The core of the simulation lives in `IntersectionManager` ([src/intersection.rs](src/intersection.rs)).

**Conflict table (built once at startup)**

Each of the 12 paths is rasterized into a set of 60×60 px grid cells that fall inside the intersection box. Two paths *conflict* if their cell sets overlap. This produces a static 12×12 boolean conflict matrix.

**Per-frame reservation request**

When a vehicle enters the *trigger zone* (200 px before the stop line) it calls `request_reservation`. The manager:

1. Checks all currently *active* vehicles (already inside) for conflict. A conflicting active vehicle is only a blocker if it has not yet *cleared* the shared cells — checked by comparing its current position against the conflict cell coordinates for its direction of travel.
2. If no active conflict, the vehicle is granted immediately (fast path for merging traffic).
3. If blocked, the vehicle joins a *waiting queue* (arrival order preserved). When the intersection drains, a new *phase* is computed.

**Phase scheduling (greedy MIS)**

`begin_phase()` iterates the waiting queue in arrival order and builds the largest set of mutually non-conflicting paths — a greedy maximal independent set over the conflict graph. All vehicles whose path is in the current phase may enter concurrently.

`expand_phase()` is called whenever an active vehicle exits: it tries to admit additional waiting paths that are now free of conflicts with both the remaining active set *and* existing phase members. This enables pipelining — later vehicles can enter before the phase fully clears.

**Kinematic stop guarantee**

When a reservation is denied, the vehicle's target velocity is set to zero and an additional kinematic cap (`v ≤ √(2·a·d)`) is applied so it can always halt before the stop line even if already approaching at full speed.

### Vehicle kinematics

Each frame (`vehicle::update`):

1. Velocity ramps toward `target_vel` at `ACCEL_RATE` / `DECEL_RATE` (150 / 120 px·s⁻²).
2. Position is integrated: `pos += cos(angle)·v·dt`, `pos += sin(angle)·v·dt`.
3. Waypoint proximity (≤ 2 px) triggers advance to the next waypoint and angle recalculation.
4. State machine transitions: `Approaching → InIntersection → Exiting → Removed`.
5. Safe-distance following: if the nearest same-lane vehicle ahead is within `SAFE_DISTANCE` (90 px), velocity is zeroed immediately; a lookahead braking window slows the vehicle earlier to avoid hard stops.

---

## Project structure

```
src/
├── main.rs          — SDL2 init, game loop, event dispatch, stat collection
├── types.rs         — All constants, enums (Direction, Route, VehicleState), structs
├── intersection.rs  — IntersectionManager, conflict table, phase logic, path map
├── vehicle.rs       — Per-frame update, kinematics, state machine
├── renderer.rs      — SDL2 drawing: road, lane markings, sprites, HUD, stats overlay
├── input.rs         — Keyboard events, vehicle spawning, spawn-rate limiting
└── stats.rs         — Velocity / transit-time / close-call tracking

assets/
├── CIVIC_CLEAN_8D_000-sheet.png
├── JEEP_CLEAN_8D_000-sheet.png
├── SEDAN_CLEAN_8D_000-sheet.png
└── TAXI_CLEAN_8D0000-sheet.png   — 300×300 sprite sheets, 8 directional frames each
```

---

## Known challenges

| Challenge | How it was handled |
|---|---|
| Guaranteeing vehicles stop before the box even when approaching at speed | Per-frame kinematic cap: `v ≤ √(2·a·d)` applied alongside `target_vel = 0` |
| Two vehicles arriving simultaneously grab the same slot | Sequential frame processing: the first to be updated wins insertion into `active`; the second sees it and is denied in the same frame |
| Vehicles blocking the intersection after the phase changes | Position-aware clearance check — a conflicting active vehicle only blocks if it hasn't passed all shared cells yet |
| Close-call counting without double-counting the same near-miss across many frames | Pair tracked in a `HashSet`; the counter increments only on the frame a pair *enters* the threshold, not every frame it stays within it |
| Sprite rotation without SDL2 `copy_ex` | 8 pre-rendered directional frames per sheet; `angle_deg` snapped to the nearest 45° to select the correct source rect |
