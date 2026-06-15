# Ticket Progress Tracker

This file tracks delivery progress for the **Smart Road** project (Rust + SDL2 simulation).

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
3. Use `[-]` only when a meaningful subset of that ticket already exists in code.
4. Keep `Depends on` and `Blocks` synchronised with the owning track file when ticket definitions change.
5. Do not remove completed tickets from the tracker.
6. A ticket may start early, even if some lower-priority earlier-phase work is still open, only when all of its direct dependencies are already complete.

## Status Legend

- `[ ]` = Not Started
- `[-]` = Partially Implemented / In Progress
- `[x]` = Done

## Summary Snapshot

- Total tickets: `9`
- Done: `8`
- Partially Implemented: `0`
- Not Started: `1`

---

## Implementation Order — Foundation-First Strategy

Organised into **4 waves**. Wave 1 lands the shared `types.rs` contract and path map that unblock
everyone. Waves 2–3 run in parallel across the three tracks. Wave 4 is hygiene, stats overlay, and
audit sign-off. Maps to SDS §§3–11 milestones.

### Wave 1 — Foundation (P0)

> **Goal:** the shared contract — all enums, constants, structs — plus compiling stub modules,
> pre-computed waypoint paths, and a window that opens and exits on Esc. Unblocks every other ticket. (SDS §3, §4)

| # | Status | Ticket | Track | Description | Depends on | Blocks |
|---|--------|--------|-------|-------------|------------|--------|
| 1 | [x] | **A1** | A | `types.rs` contract — all `pub const`, enums (`Direction`, `Route`, `VehicleState`, `Speed`), structs (`Vec2`, `Vehicle`, `IntersectionSlot`, `Stats`); module scaffold (stub `vehicle`/`intersection`/`renderer`/`input`/`stats`); `main.rs` window + event-pump + loop that clears to background and exits on Esc (SDS §3, §2) | None | A2, B1, C1, C2 |
| 2 | [x] | **A2** | A | Waypoint path pre-computation — `build_path_map() -> HashMap<(Direction, Route), Vec<Vec2>>` covering all 12 `(Direction, Route)` pairs using lane geometry from SDS §4.1/§4.2; all paths start and end off-screen; `#[cfg(test)]` assertions on first/last waypoints for ≥ 4 paths (SDS §4, §5) | A1 | B1, B2, C2 |

### Wave 2 — Core Modules (P1)

> **Goal:** the reservation manager, vehicle physics core, and static renderer — three tickets in parallel after Wave 1. (SDS §5, §6, §7)

| # | Status | Ticket | Track | Description | Depends on | Blocks |
|---|--------|--------|-------|-------------|------------|--------|
| 3 | [x] | **B1** | B | `IntersectionManager` — pre-computed conflict table (SDS §5.2), `request_reservation(id, dir, route) -> bool` (grant/deny with simultaneous non-conflicting support), `release_reservation(id)` gated on full intersection clearance, `is_in_trigger_zone` helper (200 px); unit tests for all conflict pairs (SDS §5) | A1, A2 | B2 |
| 4 | [x] | **B2** | B | Vehicle physics & state machine — `update(vehicle, dt, path_map, manager, all_vehicles, now_ms)`: waypoint traversal (advance at 2 px, `angle_deg` via `atan2` with SDL2 sign flip), smooth accel/decel (`ACCEL_RATE`/`DECEL_RATE`, never instant-snap), reservation lifecycle (`Approaching→InIntersection→Exiting→Removed`), same-lane safe-distance check (filter by `direction` + travel axis), `distance_travelled` accumulation, `entry_time_ms` set at first algorithm detection (SDS §6, §8) | A1, A2, B1 | C3, X1 |
| 5 | [x] | **C1** | C | Renderer static + dynamic scene — road background (two gray rects + lighter intersection box), dashed lane markings (SDS §7.1); dynamic layer: vehicles rendered via `copy_ex` with `angle_deg` rotation and frame index from `distance_travelled`, HUD (vehicle count + active reservations); layer order: road → markings → box → vehicles → HUD; no canvas mutations outside `renderer.rs` (SDS §7) | A1 | C3 |

### Wave 3 — Input & Wiring (P2)

> **Goal:** keyboard input, spawn guard, and end-to-end interactive build. (SDS §8)

| # | Status | Ticket | Track | Description | Depends on | Blocks |
|---|--------|--------|-------|-------------|------------|--------|
| 6 | [x] | **C2** | C | Input handling & spawner — SDL2 event pump: arrow keys spawn per-direction (route from lane definition, never independent), `R` toggles continuous random-spawn mode, `Esc` triggers stats + quit; per-direction spawn guard (`last_spawn_time`, gated on `SPAWN_INTERVAL_MS`); vehicles spawn off-screen at path waypoint 0 (SDS §8) | A1, A2 | C3 |
| 7 | [x] | **C3** | C | `main.rs` wiring — `dt`-clamped game loop calling vehicle `update`, `IntersectionManager`, and `draw` each tick; Esc triggers stats overlay then clean exit; interactive end-to-end verified manually (SDS §2, §8) | B2, C1, C2 | A3, X2 |

### Wave 4 — Statistics, Hygiene & Polish (P3)

> **Goal:** passive stats collection, end-screen overlay, bonus `VehicleKind`, and codebase hygiene. (SDS §9, §11)

| # | Status | Ticket | Track | Description | Depends on | Blocks |
|---|--------|--------|-------|-------------|------------|--------|
| 8 | [x] | **X1** | A/C | Stats collection & end-screen — `Stats` update methods (`record_passed`, `record_velocity` in-motion only, `record_transit`); Euclidean close-call detection per frame (`< CLOSE_CALL_DIST`, documented per-frame vs deduplicated policy); SDL2 Esc overlay showing total passed, max/min velocity, max/min transit time, close-call count; bonus `VehicleKind` enum in `types.rs` with per-kind `ACCEL_RATE`/`DECEL_RATE` (SDS §9, §11) | B2, C3 | X2 |
| 9 | [ ] | **X2** | A | Project-wide hygiene sweep — zero warnings, `clippy -D warnings`, README prerequisites, `.gitignore`, minimal `Cargo.toml`; module-boundary audit (no SDL2 in pure modules, no magic numbers outside `types.rs`, `CLOSE_CALL_DIST < SAFE_DISTANCE` enforced, `entry_time_ms` not set at spawn) (AGENTS.md §2, §11) | X1, C3 + all | None |

> Ticket counts by track: A = A1–A2 + X2 (3), B = B1–B2 (2), C = C1–C3 (3), shared X1 (1) = **9 tickets, 8 rows** (X1 is jointly owned by A/C). Track B carries fewer tickets because `vehicle.rs` is a single module; work per ticket is correspondingly larger.

---

## Pre-Submission Checklist (per AGENTS.md §11)

Before any ticket is marked Done:

- [ ] All new types and constants are in `types.rs`, nowhere else
- [ ] No module has gained a responsibility listed under another module (AGENTS.md §2)
- [ ] Vehicles face their direction of travel through all waypoints (`angle_deg` updated at every waypoint)
- [ ] Reservations are released only after full intersection clearance, not at first exit waypoint
- [ ] `entry_time_ms` is set at first algorithm detection — not at spawn, not at box entry
- [ ] Spawn guard is intact — no two vehicles can visually overlap at the spawn point
- [ ] `SAFE_DISTANCE` is never zero and is enforced on approach roads, not just inside the intersection
- [ ] `CLOSE_CALL_DIST < SAFE_DISTANCE` (violation threshold, not a safe distance)
- [ ] Velocity changes are smooth (accel/decel) — instant snaps are bugs
- [ ] Stats are collected passively and displayed only on Esc, never mid-simulation
