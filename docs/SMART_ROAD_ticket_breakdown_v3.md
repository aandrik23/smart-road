# Ticket Progress Tracker

This file tracks delivery progress for the **Smart Road** project (Rust + SDL2 simulation).
Aligned with **SDS v1.5** and **AGENTS v3**.

## Team Assignment

| Track | Dev | Focus Area |
|-------|-----|------------|
| **A** | Dev 1 | Foundation & Types (`types.rs` contract, module scaffold, waypoint paths, `main.rs` window/loop skeleton, project hygiene) |
| **B** | Dev 2 | Vehicles & Motion (`vehicle.rs`: physics update, waypoint traversal, accel/decel, safe-distance check, state machine) |
| **C** | Dev 3 | Rendering & Control (`renderer.rs`: static + dynamic scene, sprite rotation, HUD; `input.rs`: spawn guard, key bindings, random mode; stats overlay) |

---

Detailed ticket definitions live in:

- `docs/track-a.md`
- `docs/track-b.md`
- `docs/track-c.md`

The canonical product and technical requirements are in:

- `docs/SDS.md` — software design specification (source of truth)
- `AGENTS.md` — LLM-context digest of the SDS; read before touching any module

## Update Rules

1. Keep each ticket in the line format: status + ticket ID + short description + dependency fields.
2. Use `[x]` only when the verification gate in the owning track file is satisfied.
3. Use `[-]` when a meaningful subset of that ticket already exists in code.
4. Keep `Depends on` and `Blocks` synchronised with the owning track file when ticket definitions change.
5. Do not remove completed tickets from the tracker.
6. A ticket may start early, even if some lower-priority earlier-phase work is still open, only when all of its direct dependencies are already complete.

## Status Legend

- `[ ]` = Not Started
- `[-]` = Partially Implemented / In Progress
- `[x]` = Done

## Summary Snapshot

- Total tickets: `11`
- Done: `0`
- Partially Implemented: `0`
- Not Started: `11`

---

## Implementation Order — Foundation-First Strategy

Organised into **4 waves**. Wave 1 lands the shared `types.rs` contract and path map that
unblock everyone. Waves 2–3 run in parallel across the three tracks. Wave 4 is stats,
hygiene, and audit sign-off. Maps to SDS v1.4 §§3–12 milestones.

---

### Wave 1 — Foundation (P0)

> **Goal:** the shared data contract — all enums, constants, structs (including the updated
> `IntersectionSlot` with time fields) — plus compiling stub modules, pre-computed waypoint
> paths, and a window that opens and exits on Esc. Unblocks every other ticket.
> (SDS §3, §4)

| # | Status | Ticket | Track | Description | Depends on | Blocks |
|---|--------|--------|-------|-------------|------------|--------|
| 1 | [ ] | **A1** | A | **`types.rs` contract + module scaffold** — all `pub const` (`WINDOW_WIDTH/HEIGHT`, `LANE_WIDTH`, `TILE_SIZE`, `INTER_X/Y/W/H`, `SPEED_SLOW/MEDIUM/FAST`, `SAFE_DISTANCE`, `CLOSE_CALL_DIST`, `SPAWN_INTERVAL_MS`, `TRIGGER_DIST`, `TRANSIT_LENGTH`, `ACCEL_RATE`, `DECEL_RATE`); all enums (`Direction`, `Route`, `VehicleState`, `Speed`); all structs (`Vec2`, `Vehicle`, `IntersectionSlot` with `scheduled_entry_ms`/`scheduled_exit_ms`, `Stats`); stub `mod` declarations for all 6 modules; `main.rs` window + event-pump + loop that clears to background color and exits cleanly on Esc. No logic in `types.rs`. (SDS §3, §2) | None | A2, B1, B2, C1, C2 |
| 2 | [ ] | **A2** | A | **Waypoint path pre-computation** — `build_path_map() -> HashMap<(Direction, Route), Vec<Vec2>>` covering all 12 `(Direction, Route)` pairs using `spawn_lane` center coordinates and `inc_lane` exit coordinates from SDS §4.1; all paths start off-screen at the spawn point for that direction+route (e.g. N→S Right spawns at (150,-60)), pass through the stop line at the intersection box edge, traverse the box to the turn apex, then travel along the correct `inc_lane` center to off-screen; paths must never touch another `spawn_lane` after the box exit — exit is always onto an `inc_lane`; `#[cfg(test)]` assertions: first waypoint is off-screen, last waypoint is off-screen, exit coordinate matches the `inc_lane` center in SDS §4.1 for all 12 paths. (SDS §4) | A1 | B1, B2, C2 |

---

### Wave 2 — Core Modules (P1)

> **Goal:** the time-window scheduler, vehicle physics core, and static renderer — three tickets
> in parallel after Wave 1. This is where the new algorithm lives. (SDS §5, §6, §7)

| # | Status | Ticket | Track | Description | Depends on | Blocks |
|---|--------|--------|-------|-------------|------------|--------|
| 3 | [ ] | **B1** | B | **`IntersectionManager` + `has_time_conflict`** — pre-computed conflict table at startup (SDS §5.2, all 12×12 pairs); `has_time_conflict(route, entry_ms, exit_ms) -> bool` scanning all active `IntersectionSlot`s for temporal overlap with conflicting routes (`NOT (exit_ms <= other.scheduled_entry_ms OR entry_ms >= other.scheduled_exit_ms)`); `release_slot(id)` gated on caller confirming full box clearance (manager does not enforce geometry, caller does); `is_in_trigger_zone(dist_to_stop) -> bool` (`dist_to_stop <= TRIGGER_DIST`); unit tests covering: all conflict pairs from SDS §5.2, non-conflicting simultaneous windows allowed, overlap boundary conditions (adjacent windows must NOT conflict, overlapping by 1 ms must conflict). (SDS §5.2, §5.3) | A1 | B2 |
| 4 | [ ] | **B2** | B | **`compute_approach_speed` scheduler** — `compute_approach_speed(id, dir, route, dist_to_stop, now_ms, slots) -> f32` implementing all three steps from SDS §5.4: **Step 1** (vehicle has existing slot: compute `dist / remaining_s`, clamp to `[SPEED_SLOW, SPEED_FAST]`; if `remaining_ms <= 0` go to Step 2 while holding old slot); **Step 2** (probe `[SPEED_FAST, SPEED_MEDIUM, SPEED_SLOW]` in order, call `has_time_conflict` for each projected window, book first that fits, release old slot if re-booking, return the named-constant speed); **Step 3** (all named speeds conflict: find `earliest_entry_ms` by scanning exit times of all conflicting active slots, compute `exact_speed = clamp(dist / gap_s, SPEED_SLOW, SPEED_FAST)`, book window, release old slot if re-booking, return `exact_speed`); re-booking rule: new slot booked before old slot released; unit tests: Step 1 far-away returns fast, close returns slow; Step 2 books fastest available; Step 3 returns clamped exact speed; re-booking does not create a gap. (SDS §5.4) | A1, A2, B1 | B3, C3 |
| 5 | [ ] | **C1** | C | **Renderer static + dynamic scene** — road background: four arm rectangles (north 120–480 wide × 0–300 tall, south 300–660 wide × 600–900 tall, east 600–900 wide × 120–480 tall, west 0–300 wide × 300–660 tall) plus lighter intersection box; solid yellow divider line between `spawn_lanes` and `inc_lanes` on each arm per SDS §4.3; dashed white lane markings between individual lanes within each half; dynamic layer: vehicles rendered via `copy_ex` with `angle_deg` rotation and frame index from `distance_travelled % frame_stride`; HUD (vehicle count + count of active slots); layer order: road → markings → dividers → box → vehicles → HUD; `renderer.rs` reads state only — zero mutations to vehicle or intersection data. (SDS §7, §4.3) | A1 | C3 |

---

### Wave 3 — Physics, Wiring & Input (P2)

> **Goal:** full vehicle physics integrated with the scheduler, keyboard input, spawn guard,
> and an end-to-end interactive build. (SDS §6, §8)

| # | Status | Ticket | Track | Description | Depends on | Blocks |
|---|--------|--------|-------|-------------|------------|--------|
| 6 | [ ] | **B3** | B | **Vehicle physics & state machine** — `update(vehicle, dt, path_map, manager, all_vehicles, now_ms)`: waypoint traversal (advance `path_index` when within 2 px of current waypoint; recompute `angle_deg = -atan2(dy, dx).to_degrees()` at each advance for SDL2 clockwise convention); smooth accel/decel (`velocity += ACCEL_RATE * dt` / `velocity -= DECEL_RATE * dt`, never instant-snap); velocity clamp `clamp(SPEED_SLOW, SPEED_FAST)` as final step every tick; state machine: `Approaching` → call `compute_approach_speed` **every frame** when within `TRIGGER_DIST`, set `entry_time_ms` at first detection, `target_vel = min(scheduler_vel, layer1_vel)`; `InIntersection` → `target_vel = SPEED_MEDIUM`; `Exiting` → `target_vel = SPEED_FAST`; `Removed` → set `exit_time_ms`, remove from active list; **Layer 1 safe-distance check**: filter `all_vehicles` by same `direction` and ahead along travel axis only, compute gap, apply `lerp(SPEED_SLOW, target_vel, gap/SAFE_DISTANCE)` when gap < `SAFE_DISTANCE`; `distance_travelled` accumulation each tick. (SDS §6) | A1, A2, B1, B2 | C3, X1 |
| 7 | [ ] | **C2** | C | **Input handling & spawner** — SDL2 event pump: arrow keys spawn one vehicle in the specified direction (↑=N→S, ↓=S→N, →=W→E, ←=E→W) with route chosen randomly from that direction's three `spawn_lanes`; **`R` key spawns exactly one vehicle** with both direction and route chosen at random — it does NOT toggle a continuous mode, one press = one vehicle; `Esc` signals stats display + quit; per-direction spawn guard (`last_spawn_time`, only spawn if `now - last_spawn_time > SPAWN_INTERVAL_MS`, applies to `R` key too on whichever direction it picks); vehicle spawned at `path[0]` (off-screen spawn waypoint for that direction+route from `build_path_map`); initial `velocity = SPEED_SLOW`, `target_vel = SPEED_FAST`, `state = Approaching`. (SDS §8) | A1, A2 | C3 |
| 8 | [ ] | **C3** | C | **`main.rs` wiring** — `dt`-clamped game loop (cap dt to avoid spiral-of-death on lag spikes); each tick: process input (`input.rs`), update all vehicles (`vehicle.rs`), collect stats (`stats.rs`), draw frame (`renderer.rs`); Esc path: collect final stats, render overlay, wait for any key, clean SDL2 exit; end-to-end interactive build verified manually: vehicles spawn, approach, slow/speed via scheduler, traverse intersection, exit, stats appear on Esc. (SDS §2) | B3, C1, C2 | X1, X2 |

---

### Wave 4 — Statistics, Hygiene & Polish (P3)

> **Goal:** passive stats collection, end-screen overlay, bonus `VehicleKind`, and
> codebase hygiene sweep. (SDS §9, §11, §12)

| # | Status | Ticket | Track | Description | Depends on | Blocks |
|---|--------|--------|-------|-------------|------------|--------|
| 9 | [ ] | **X1** | A/C | **Stats collection & end-screen** — passive per-frame updates in `stats.rs`: `record_velocity(v)` only when vehicle is in motion (`velocity > 0`); `record_transit(entry_ms, exit_ms)` on `Removed`; Euclidean close-call detection every frame over all vehicle pairs (`pos` distance < `CLOSE_CALL_DIST`), document chosen policy (per-frame count vs deduplicated per pair) in `stats.rs` header; SDL2 Esc overlay window showing: total vehicles passed, max/min velocity, max/min transit time (`entry_time_ms` at `TRIGGER_DIST` detection → `exit_time_ms` at `Removed`), close-call count; bonus: `VehicleKind` enum (`Standard`, `Sport`, `Heavy`) in `types.rs`, per-vehicle `accel_rate`/`decel_rate` fields on `Vehicle` struct, `VehicleKind` assigned randomly at spawn, floor/ceiling (`SPEED_SLOW`/`SPEED_FAST`) identical for all kinds. (SDS §9, §11) | B3, C3 | X2 |
| 10 | [ ] | **X2** | A | **Project-wide hygiene sweep** — zero `cargo build` warnings; `cargo clippy -- -D warnings` clean; no magic number floats outside `types.rs`; no SDL2 imports in `vehicle.rs`, `intersection.rs`, `stats.rs`; no physics or reservation logic in `renderer.rs` or `input.rs`; `CLOSE_CALL_DIST < SAFE_DISTANCE` verified by `#[cfg(test)]` assertion in `types.rs`; `entry_time_ms` confirmed not set at spawn (grep audit); `compute_approach_speed` confirmed called every frame (not cached); re-booking confirmed books new before releasing old; README with prerequisites (`sdl2`, `sdl2_image`, `Rust stable`), build command, controls; `.gitignore`; minimal `Cargo.toml`. (AGENTS.md §2, §11) | X1, C3 | None |

---

> **Ticket counts by track:** A = A1, A2, X2 (3) · B = B1, B2, B3 (3) · C = C1, C2, C3 (3) · Shared = X1 (1) = **10 tickets + 1 shared = 11 total**.
> Track B carries the heaviest single tickets (B2 is the new scheduler, B3 is full physics integration); work per ticket is correspondingly larger than Track C.

---

## Key Changes From Previous Tracker (v2 → v3)

| Area | Old (ticket breakdown v2 / SDS v1.4) | New (ticket breakdown v3 / SDS v1.5) |
|---|---|---|
| Lane naming | "incoming" / "outgoing" | `spawn_lanes` (origin) / `inc_lanes` (destination) — matches blueprint |
| Lane coordinates | Based on old SDS (wrong halves of each arm) | Rebuilt from blueprint image: spawn lanes on far half, inc lanes on near half |
| Waypoint paths (A2) | Old x/y from incorrect lane positions | New spawn centers and inc_lane exit centers from §4.1 summary tables |
| Road rendering (C1) | Two gray rects for cross; divider at x=510/y=390 | Four per-arm rects with correct widths; divider between spawn and inc halves |
| `R` key (C2) | Toggled continuous random-spawn mode | Spawns exactly **one** vehicle (random direction + random route) per press |
| Arrow key directions | ↑=S→N, ↓=N→S | Corrected: ↑=N→S, ↓=S→N (arrow shows vehicle travel direction) |
| SDS reference | v1.4 | v1.5 |

---

## Pre-Submission Checklist (per AGENTS.md v3 §11)

Before any ticket is marked Done:

- [ ] All new types and constants are in `types.rs`, nowhere else
- [ ] No module has gained a responsibility listed under another module (AGENTS.md §2)
- [ ] Vehicles face their direction of travel through all waypoints (`angle_deg` updated at every waypoint advance, sign-corrected for SDL2 clockwise convention)
- [ ] Slots are released only after full intersection clearance — last waypoint **inside** the box, never the first exit waypoint
- [ ] When re-booking a slot, the new slot is booked **before** the old one is released
- [ ] `compute_approach_speed` is called **every frame** while `Approaching` and within `TRIGGER_DIST` — never cached
- [ ] `target_vel = min(compute_approach_speed(...), layer1_following_vel)` — Layer 1 always wins when lower
- [ ] `entry_time_ms` is set at first `TRIGGER_DIST` detection — not at spawn, not at box entry
- [ ] Spawn guard is intact — no two vehicles can visually overlap at the spawn point
- [ ] All vehicles spawn off-screen at the correct `spawn_lane` center for their direction+route (`path[0]`)
- [ ] Exit waypoints always land on an `inc_lane` center — never on a `spawn_lane` coordinate
- [ ] `SAFE_DISTANCE` is never zero and is enforced on approach roads and inc_lane roads
- [ ] `CLOSE_CALL_DIST < SAFE_DISTANCE` (violation threshold, not a safe distance)
- [ ] Velocity changes are smooth (accel/decel) — instant snaps are bugs
- [ ] `velocity = clamp(velocity, SPEED_SLOW, SPEED_FAST)` — lower bound is `SPEED_SLOW`, not `0.0`
- [ ] No vehicle has `velocity` or `target_vel` set below `SPEED_SLOW` at any point
- [ ] `R` key spawns exactly one vehicle per press — no continuous mode
- [ ] Conflict table is pre-computed at startup, never evaluated dynamically per frame
- [ ] Waypoint paths are pre-computed at startup, never regenerated mid-simulation
- [ ] Stats are collected passively every frame and displayed only on Esc, never mid-simulation
- [ ] Close-call detection uses Euclidean distance between `pos` values, not cell overlap
