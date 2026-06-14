# B1 — IntersectionManager Design

**Date:** 2026-06-14  
**Ticket:** B1 (Wave 2, P1)  
**Module:** `intersection.rs`  
**Depends on:** A1, A2 (complete)  
**Blocks:** B2

---

## Overview

B1 delivers the reservation-based intersection controller. It is pure logic — no
SDL2, no spawning, no rendering. Every other module (B2, C3) calls into this one;
it calls nothing except `build_path_map()` (already in `intersection.rs`) during
construction.

---

## Types.rs Change Required

Add one constant to `types.rs`:

```rust
pub const TRIGGER_DIST: f32 = 200.0;
```

This is the distance (px) from the stop line at which a vehicle enters the trigger
zone and the reservation system first evaluates it. All other types are already
present.

---

## Struct & Data Layout

```rust
pub struct IntersectionManager {
    conflicts: [[bool; 12]; 12],
    active:    HashMap<u32, (Direction, Route)>,
}
```

### Path index

All 12 `(Direction, Route)` pairs map to indices 0–11 via a private helper:

```
path_index(dir, route) = dir_ord * 3 + route_ord
```

| Direction | ord | Route    | ord |
|-----------|-----|----------|-----|
| North     | 0   | Right    | 0   |
| South     | 1   | Straight | 1   |
| West      | 2   | Left     | 2   |
| East      | 3   |          |     |

So `(North, Right) = 0`, `(North, Straight) = 1`, ..., `(East, Left) = 11`.

### `conflicts`

A symmetric 12×12 bool array. `conflicts[i][j] == true` means paths i and j share
at least one grid cell inside the intersection box. Computed once in `new()`, never
mutated. Diagonal is always `false` (a path does not conflict with itself for grant
purposes — a vehicle can't hold two reservations).

### `active`

Maps vehicle id (`u32`) to the `(Direction, Route)` of its granted reservation.
A vehicle holds at most one reservation at a time.

---

## Conflict Table — Grid-Cell Rasterization

Built inside `new()` by calling `build_path_map()`.

### Grid definition

The intersection box `x∈[INTER_X, INTER_X+INTER_W)`, `y∈[INTER_Y, INTER_Y+INTER_H)`
is divided into a **5×5 grid** of 60×60 px cells (matching `LANE_WIDTH`):

```
col = ((x - INTER_X) / LANE_WIDTH) as u8   // 0..4
row = ((y - INTER_Y) / LANE_WIDTH) as u8   // 0..4
```

### Rasterization

For each path:
1. Walk every segment (`windows(2)`) in 5 px steps along the segment length.
2. For each sampled point, check if it falls strictly inside the intersection box.
3. If yes, compute `(col, row)` and insert into a `HashSet<(u8, u8)>` for that path.

### Conflict detection

After all 12 cell sets are built:

```
conflicts[i][j] = !cell_sets[i].is_disjoint(&cell_sets[j])   for i ≠ j
conflicts[i][i] = false
```

The table is symmetric, so the inner loop only needs to compute the upper triangle
and mirror, but both directions must be set so lookups work without ordering.

---

## Public API

```rust
impl IntersectionManager {
    pub fn new() -> Self
    pub fn request_reservation(&mut self, id: u32, dir: Direction, route: Route) -> bool
    pub fn release_reservation(&mut self, id: u32)
    pub fn is_in_trigger_zone(&self, vehicle: &Vehicle) -> bool
    pub fn active_count(&self) -> usize
}
```

### `new()`

1. Call `build_path_map()` to get all 12 paths.
2. Rasterize each path into its cell set.
3. Build `conflicts` from the cell sets.
4. Return `IntersectionManager { conflicts, active: HashMap::new() }`.

### `request_reservation(id, dir, route) -> bool`

```
if active.contains_key(&id) → return true   // idempotent re-entry
req_idx = path_index(dir, route)
for each (_, (a_dir, a_route)) in active:
    if conflicts[req_idx][path_index(a_dir, a_route)] → return false
active.insert(id, (dir, route))
return true
```

The idempotent check handles vehicles that re-enter the trigger zone after a
denied request — `entry_time_ms` is set at first detection regardless of grant/deny
(B2's responsibility, not B1's).

### `release_reservation(id)`

```
active.remove(&id)
```

Caller (B2) is responsible for timing — must only be called after the vehicle has
fully cleared the intersection box, not at the first exit waypoint.

### `is_in_trigger_zone(vehicle) -> bool`

Match on `vehicle.direction`, compare `vehicle.pos` to the stop line ± `TRIGGER_DIST`:

| Direction | Stop line          | Trigger condition                                            |
|-----------|--------------------|--------------------------------------------------------------|
| South (→N) | y = INTER_Y+INTER_H (600) | `pos.y > 600.0 && pos.y <= 600.0 + TRIGGER_DIST` |
| North (→S) | y = INTER_Y (300)        | `pos.y < 300.0 && pos.y >= 300.0 - TRIGGER_DIST` |
| West  (→E) | x = INTER_X (300)        | `pos.x < 300.0 && pos.x >= 300.0 - TRIGGER_DIST` |
| East  (→W) | x = INTER_X+INTER_W (600) | `pos.x > 600.0 && pos.x <= 600.0 + TRIGGER_DIST` |

Uses only `types.rs` constants — no magic numbers.

### `active_count() -> usize`

```
active.len()
```

---

## Unit Tests (`#[cfg(test)]`)

All tests use `IntersectionManager::new()` directly — no mocking needed since
`new()` is pure (builds from `build_path_map()`).

| # | Name | What it proves |
|---|------|----------------|
| 1 | `all_four_right_turns_non_conflicting` | Grant (N,R), (S,R), (W,R), (E,R) simultaneously — all return `true`, `active_count() == 4` |
| 2 | `conflicting_request_denied` | Grant `(N,St)`, then request `(S,L)` → `false` (both cross x=480 inside the box) |
| 3 | `release_frees_slot` | Grant `(N,St)`, release it, then grant `(S,L)` → `true` |
| 4 | `active_count_tracks_grants_and_releases` | Grant 3, release 1, `active_count() == 2` |
| 5 | `spec_confirmed_non_conflict` | Grant `(N,R)` and `(W,St)` simultaneously — both `true` (spec §5.2 example) |
| 6 | `idempotent_re_request` | Grant `(N,St)`, call `request_reservation` again with same id → `true`, `active_count() == 1` |

---

## Module Boundary Checklist

- [ ] No SDL2 import in `intersection.rs`
- [ ] No struct or constant defined in `intersection.rs` — all types come from `types.rs`
- [ ] `TRIGGER_DIST` added to `types.rs`, not inline in `intersection.rs`
- [ ] `intersection.rs` does not spawn vehicles or draw anything
- [ ] `build_path_map()` remains in `intersection.rs` (already there — no move needed)

---

## Verification Gate (from track-b.md)

- [ ] Unit tests pass: all conflict/non-conflict cases covered
- [ ] Two non-conflicting vehicles can hold reservations simultaneously
- [ ] A conflicting request is denied while the first reservation is active
- [ ] `release_reservation` actually frees the slot (subsequent grant succeeds)
- [ ] No SDL2 import in `intersection.rs`
