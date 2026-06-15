# Track C — Rendering & Control
**Developer:** Dev 3
**Modules owned:** `renderer.rs`, `input.rs`, `main.rs` (wiring only, C3), `stats.rs` (SDL2 overlay, shared with A on X1)

---

## Overview

Track C makes the simulation visible and interactive. C1 delivers the static and
dynamic scene; C2 delivers keyboard input and the spawn guard; C3 wires everything
together into a running interactive build. X1 (stats end-screen) is jointly owned
with Dev 1 (Track A) — coordinate on the `Stats` API before starting.

`renderer.rs` is the only module allowed to call SDL2 draw functions. Nothing
outside `renderer.rs` (and `main.rs` SDL2 init) may mutate the canvas.

Dependency order: **A1 → C1, C2 → C3 (also needs B2) → X1**

---

## Tickets

---

### C1 — Renderer: static + dynamic scene
**Wave:** 2 (P1) | **Depends on:** A1 | **Blocks:** C3

#### What to build

All SDL2 draw calls live here. `renderer.rs` reads vehicle and manager state but
must **not** mutate it.

**Public API:**
```rust
pub fn draw(
    canvas:   &mut sdl2::render::Canvas<sdl2::video::Window>,
    vehicles: &[Vehicle],
    manager:  &IntersectionManager,
    textures: &TextureMap,          // or however you pass sprite textures
)
```

**Layer order (SDS §7.1) — draw in this exact sequence:**
1. **Road background** — two gray rectangles (horizontal road strip + vertical road strip). Use the intersection constants from `types.rs` (`INTER_X`, `INTER_Y`, `INTER_W`, `INTER_H`, `LANE_WIDTH`).
2. **Lane markings** — dashed white lines separating lanes. Each approach road has two dividing lines between its three lanes. Draw as short white rectangles or line segments with gaps.
3. **Intersection box** — a slightly lighter gray rectangle over `(INTER_X, INTER_Y, INTER_W, INTER_H)`.
4. **Vehicles** — per vehicle, call `canvas.copy_ex` with rotation (see below).
5. **HUD** — vehicle count (top-left or similar) + active reservation count from `manager.active_count()`.

**Vehicle sprite rendering (SDS §7.2, AGENTS.md §6):**
```rust
let frame_x = (vehicle.distance_travelled as u32 / FRAME_STRIDE % FRAME_COUNT) * SPRITE_W;
let src_rect = Rect::new(frame_x as i32, 0, SPRITE_W, SPRITE_H);
let dst_rect = Rect::new(vehicle.pos.x as i32 - SPRITE_W as i32 / 2,
                          vehicle.pos.y as i32 - SPRITE_H as i32 / 2,
                          SPRITE_W, SPRITE_H);
canvas.copy_ex(&texture, src_rect, dst_rect, vehicle.angle_deg as f64, None, false, false)?;
```

- `angle_deg` is already the SDL2-convention angle (clockwise, Track B computes it)
- A vehicle rendered as a static image facing one fixed direction is **not acceptable**
- If a vehicle appears to slide sideways through a turn, the bug is in `angle_deg` (Track B), but verify by printing the angle per waypoint

**Constants** — define `FRAME_STRIDE`, `FRAME_COUNT`, `SPRITE_W`, `SPRITE_H` in `types.rs`.
Do not hardcode them in `renderer.rs`.

**HUD text:** SDL2's `ttf` feature or a simple bitmapped approach. Display at
minimum: `Vehicles: N` and `Reservations: M`. Keep it unobtrusive (small font, corner).

#### Verification gate
- [x] Road, lane markings, intersection box, vehicles, and HUD are all visible and correctly layered
- [x] Vehicles rotate to face direction of travel at every waypoint
- [x] Frame animation advances with `distance_travelled`
- [x] HUD updates each frame (vehicle count changes as vehicles spawn/despawn)
- [x] No canvas mutation outside `renderer.rs`
- [x] No vehicle or intersection state is modified inside `renderer.rs`

---

### C2 — Input handling & spawner
**Wave:** 3 (P2) | **Depends on:** A1, A2 | **Blocks:** C3

#### What to build

All keyboard input parsing and vehicle spawning live here. No physics, no
reservation logic.

**Public API:**
```rust
pub struct InputState {
    pub random_mode: bool,
    pub quit:        bool,
    pub last_spawn:  [u64; 4],  // indexed by Direction as usize, or a HashMap
}

pub fn handle_events(
    event_pump:   &mut sdl2::EventPump,
    input_state:  &mut InputState,
    vehicles:     &mut Vec<Vehicle>,
    path_map:     &HashMap<(Direction, Route), Vec<Vec2>>,
    next_id:      &mut u32,
    now_ms:       u64,
)
```

**Key bindings (fixed by spec — do not reassign, AGENTS.md §7):**

| Key        | Spawns from | Travels toward | Lane selection |
|------------|-------------|----------------|----------------|
| Arrow Up   | South       | North          | random of 3 lanes |
| Arrow Down | North       | South          | random of 3 lanes |
| Arrow Right| West        | East           | random of 3 lanes |
| Arrow Left | East        | West           | random of 3 lanes |
| R          | —           | —              | toggle `random_mode` |
| Esc        | —           | —              | set `quit = true` |

**Spawn guard (mandatory — AGENTS.md §7):**
```rust
if now_ms - input_state.last_spawn[dir_index] > SPAWN_INTERVAL_MS {
    // spawn vehicle
    input_state.last_spawn[dir_index] = now_ms;
}
```
Never skip this check. If a key is held, only one vehicle spawns per
`SPAWN_INTERVAL_MS` window per direction. Visual overlap at the spawn point is a bug.

**Lane selection:** when a key is pressed (or in random mode), pick one of the
three lanes for that direction randomly. The route (`Right`, `Straight`, `Left`)
is determined by the lane — it is not chosen independently (AGENTS.md §9, Pitfall 7).
Lane definitions from SDS §4.1:

| Direction | Lanes (x or y) | Routes |
|-----------|----------------|--------|
| South→North | x = 360, 420, 480 | Right, Straight, Left |
| North→South | x = 540, 480, 420 | Right, Straight, Left |
| West→East   | y = 360, 420, 480 | Right, Straight, Left |
| East→West   | y = 540, 480, 420 | Right, Straight, Left |

**Spawning a vehicle:**
- Look up `path_map[(direction, route)]` to get the waypoint path
- Create a `Vehicle` with:
  - `pos` = `path[0]` (first waypoint = off-screen spawn point)
  - `path_index = 0`
  - `velocity = SPEED_FAST`, `target_vel = SPEED_FAST`
  - `angle_deg` pointing toward `path[1]`
  - `state = VehicleState::Approaching`
  - `entry_time_ms = 0` (set later by `vehicle::update`, not here)
  - `distance_travelled = 0.0`
- Push to `vehicles` vec; increment `next_id`

**Random mode (`R` key):**
- Toggles `input_state.random_mode`
- While active, each game loop tick attempts to spawn a vehicle for a randomly
  chosen direction, still subject to the spawn guard per direction

#### Verification gate
- [x] Each arrow key spawns a vehicle off-screen that drives into view
- [x] Spawn guard prevents overlap: holding a key for 2 seconds produces at most `2000 / SPAWN_INTERVAL_MS` vehicles per direction
- [x] `R` key toggles random mode; vehicles spawn automatically while active
- [x] Esc sets `quit = true` (C3 uses this to trigger stats + exit)
- [x] Route is always derived from lane, never set independently
- [x] Vehicles spawn at `path[0]` (off-screen), not at the stop line or intersection edge
- [x] No physics or reservation logic in `input.rs`

---

### C3 — `main.rs` wiring
**Wave:** 3 (P2) | **Depends on:** B2, C1, C2 | **Blocks:** X1, X2

#### What to build

Wire the modules together into an interactive, end-to-end build. The skeleton
from A1 already has placeholder slots — this ticket fills them in.

**Game loop structure:**
```rust
loop {
    let now_ms = timer.ticks64();   // or SDL2 equivalent
    let dt     = (now_ms - prev_ms) as f32 / 1000.0;
    let dt     = dt.min(0.05);      // clamp dt to prevent spiral of death
    prev_ms    = now_ms;

    handle_events(&mut event_pump, &mut input_state, &mut vehicles,
                  &path_map, &mut next_id, now_ms);

    if input_state.quit {
        show_stats_overlay(&stats, &mut canvas, ...);
        break;
    }

    // Random-mode spawning
    if input_state.random_mode { /* spawn per tick via same guard */ }

    // Update all vehicles
    vehicles.retain_mut(|v| {
        let alive = vehicle::update(v, dt, &path_map, &mut manager, &vehicles, now_ms);
        if !alive {
            stats.record_passed();
            stats.record_transit(v.entry_time_ms, v.exit_time_ms);
        }
        alive
    });

    // Passive stats (velocity, close calls) — every frame
    for v in &vehicles { stats.record_velocity(v.velocity); }
    // close-call detection (see X1)

    draw(&mut canvas, &vehicles, &manager, &textures);

    // FPS cap (~60)
    ::std::thread::sleep(Duration::from_millis(16));
}
```

Notes:
- `dt.min(0.05)` clamps the timestep if the window is moved/frozen to avoid
  vehicles teleporting.
- `retain_mut` (or equivalent) removes vehicles that returned `false` from
  `update` and records their stats before dropping.
- Canvas is cleared inside `draw`; `main.rs` does not call `canvas.clear()` or
  `canvas.present()` directly.

**Esc → stats overlay:**
When `input_state.quit` is true, call the stats overlay renderer (Track A / X1)
before breaking out of the loop. The overlay should block until dismissed or
for a fixed duration — do not just exit instantly.

**FPS cap:** target 60 FPS. A simple `sleep(16ms)` is acceptable; a proper
frame-time budget is better but not required.

**End-to-end manual test:**
- Launch the sim → window opens with road visible
- Press arrow keys → vehicles appear from off-screen and drive through intersection
- Press R → vehicles spawn continuously
- Press Esc → stats overlay appears, then process exits cleanly
- No panics, no SDL2 errors in stderr

#### Verification gate
- [ ] End-to-end interactive build works as described above
- [ ] `dt` is clamped (no spiral-of-death on window drag)
- [ ] Stats are updated correctly per frame and per vehicle removal
- [ ] Esc shows stats overlay before exit
- [ ] `cargo build` succeeds, zero panics during a 30-second run with R-mode active

---

### X1 — Stats end-screen overlay *(joint A / C)*
**Wave:** 4 (P3) | **Depends on:** B2, C3 | **Blocks:** X2

> **Ownership split:** Dev 1 (Track A) owns `stats.rs` update methods and data
> logic. Dev 3 (Track C) owns the SDL2 overlay rendering in `renderer.rs`.
> Agree on the `Stats` API (the `record_*` signatures) before writing either side.

#### What to build (Dev 3 portion — SDL2 overlay)

`pub fn draw_stats_overlay(canvas: &mut Canvas<Window>, stats: &Stats)`

Rendered as an opaque or semi-transparent SDL2 surface / texture drawn over the
simulation. Must display (SDS §9):

| Label              | Value |
|--------------------|-------|
| Total passed       | `stats.total_passed` |
| Max velocity       | `stats.max_velocity` px/s |
| Min velocity       | `stats.min_velocity` px/s |
| Max transit time   | `stats.max_time_ms` ms (or formatted as seconds) |
| Min transit time   | `stats.min_time_ms` ms |
| Close calls        | `stats.close_calls` |

Layout: centered or top-third of canvas, readable font size, dark background
panel. A plain `fill_rect` + text render is sufficient — no animations needed.

**Stats must appear only after Esc.** Never render stats data mid-simulation.
Stats overlay does not need to be dismissible — exiting after a short delay or
on any keypress is fine, but document the behaviour.

#### Verification gate
- [ ] Stats overlay appears on Esc and shows all six stat fields
- [ ] Stats overlay does not appear or flash during live simulation
- [ ] All values match what was recorded during the session
- [ ] Overlay is readable (dark background, sufficient contrast)

---

## Pre-submission checklist for Track C

Before marking any ticket `[x]`:

- [ ] No canvas mutation outside `renderer.rs` (including `main.rs` beyond SDL2 init)
- [ ] No vehicle or intersection state is mutated inside `renderer.rs`
- [ ] Spawn guard is intact — no two vehicles can visually overlap at spawn
- [ ] Route is always derived from lane at spawn — never randomly overridden
- [ ] Vehicles spawn off-screen at `path[0]`, not at the stop line
- [ ] Stats displayed only on Esc, never mid-simulation
- [ ] `cargo clippy -- -D warnings` passes for changed files
