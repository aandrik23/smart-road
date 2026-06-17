use std::collections::{HashMap, HashSet};
use crate::types::{
    Direction, Route, Vec2, Vehicle,
    INTER_X, INTER_Y, INTER_W, INTER_H, LANE_WIDTH, TRIGGER_DIST,
};

pub struct IntersectionManager {
    conflicts: [[bool; 12]; 12],
    active:    HashMap<u32, (Direction, Route)>,
}

fn path_index(dir: Direction, route: Route) -> usize {
    let d = match dir {
        Direction::North => 0,
        Direction::South => 1,
        Direction::West  => 2,
        Direction::East  => 3,
    };
    let r = match route {
        Route::Right    => 0,
        Route::Straight => 1,
        Route::Left     => 2,
    };
    d * 3 + r
}

fn rasterize_segment(a: Vec2, b: Vec2, cells: &mut HashSet<(u8, u8)>) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let dist = (dx * dx + dy * dy).sqrt();
    let steps = (dist / 5.0).ceil() as u32;
    for i in 0..=steps {
        let t = if steps == 0 { 0.0 } else { i as f32 / steps as f32 };
        let x = a.x + t * dx;
        let y = a.y + t * dy;
        if (INTER_X..INTER_X + INTER_W).contains(&x)
            && (INTER_Y..INTER_Y + INTER_H).contains(&y)
        {
            let col = ((x - INTER_X) / LANE_WIDTH) as u8;
            let row = ((y - INTER_Y) / LANE_WIDTH) as u8;
            cells.insert((col, row));
        }
    }
}

fn build_conflict_table() -> [[bool; 12]; 12] {
    let path_map = build_path_map();
    let mut cell_sets: Vec<HashSet<(u8, u8)>> = (0..12).map(|_| HashSet::new()).collect();

    for ((dir, route), path) in &path_map {
        let idx = path_index(*dir, *route);
        for segment in path.windows(2) {
            rasterize_segment(segment[0], segment[1], &mut cell_sets[idx]);
        }
    }

    let mut conflicts = [[false; 12]; 12];
    for i in 0..12 {
        for j in (i + 1)..12 {
            if !cell_sets[i].is_disjoint(&cell_sets[j]) {
                conflicts[i][j] = true;
                conflicts[j][i] = true;
            }
        }
    }
    conflicts
}

impl IntersectionManager {
    pub fn new() -> Self {
        IntersectionManager {
            conflicts: build_conflict_table(),
            active:    HashMap::new(),
        }
    }

    pub fn request_reservation(
        &mut self,
        id:    u32,
        dir:   Direction,
        route: Route,
    ) -> bool {
        if self.active.contains_key(&id) {
            return true;
        }
        let req_idx = path_index(dir, route);
        for (a_dir, a_route) in self.active.values() {
            if self.conflicts[req_idx][path_index(*a_dir, *a_route)] {
                return false;
            }
        }
        self.active.insert(id, (dir, route));
        true
    }

    pub fn release_reservation(&mut self, id: u32) {
        self.active.remove(&id);
    }

    pub fn is_in_trigger_zone(&self, vehicle: &Vehicle) -> bool {
        match vehicle.direction {
            Direction::South => {
                vehicle.pos.y > INTER_Y + INTER_H
                    && vehicle.pos.y <= INTER_Y + INTER_H + TRIGGER_DIST
            }
            Direction::North => {
                vehicle.pos.y < INTER_Y
                    && vehicle.pos.y >= INTER_Y - TRIGGER_DIST
            }
            Direction::West => {
                vehicle.pos.x < INTER_X
                    && vehicle.pos.x >= INTER_X - TRIGGER_DIST
            }
            Direction::East => {
                vehicle.pos.x > INTER_X + INTER_W
                    && vehicle.pos.x <= INTER_X + INTER_W + TRIGGER_DIST
            }
        }
    }

    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Path map — pre-computed once at startup, never mutated during the game loop.
//
// Intersection box: x∈[300,600], y∈[300,600]  (INTER_X/Y/W/H from types.rs)
//
// Approach lane coordinates (outside the intersection):
//   South (→ North, y decreasing): x = 360 (Right), 420 (Straight), 480 (Left)
//   North (→ South, y increasing): x = 540 (Right), 480 (Straight), 420 (Left)
//   West  (→ East,  x increasing): y = 360 (Right), 420 (Straight), 480 (Left)
//   East  (→ West,  x decreasing): y = 540 (Right), 480 (Straight), 420 (Left)
//
// Turn geometry:
//   Right turn — corner one LANE_WIDTH (60 px) from the nearest intersection edge.
//   Left  turn — corner one LANE_WIDTH (60 px) from the far intersection edge.
//   Each turn uses a 45° diagonal waypoint so angle_deg changes gradually.
//
// All paths start and end 50 px off-screen.
// ─────────────────────────────────────────────────────────────────────────────
pub fn build_path_map() -> HashMap<(Direction, Route), Vec<Vec2>> {
    let mut map = HashMap::with_capacity(12);

    // ── South (spawn y=950, travel north) ───────────────────────────────────
    // Northbound lanes: x=300 (Right), x=360 (Straight), x=420 (Left)

    // Right turn → exits West at y=480 (innermost westbound lane)
    map.insert((Direction::South, Route::Right), vec![
        Vec2 { x: 300.0, y: 950.0 },
        Vec2 { x: 300.0, y: 630.0 },
        Vec2 { x: 300.0, y: 510.0 }, // turn entry
        Vec2 { x: 270.0, y: 480.0 }, // 45° diagonal → west edge
        Vec2 { x: -50.0, y: 480.0 },
    ]);

    // Straight → exits North at x=360
    map.insert((Direction::South, Route::Straight), vec![
        Vec2 { x: 360.0, y: 950.0 },
        Vec2 { x: 360.0, y: 630.0 },
        Vec2 { x: 360.0, y: 270.0 },
        Vec2 { x: 360.0, y: -50.0 },
    ]);

    // Left turn → exits East at y=300 (outermost eastbound lane)
    map.insert((Direction::South, Route::Left), vec![
        Vec2 { x: 420.0, y: 950.0 },
        Vec2 { x: 420.0, y: 630.0 },
        Vec2 { x: 420.0, y: 330.0 }, // turn entry
        Vec2 { x: 450.0, y: 300.0 }, // 45° diagonal
        Vec2 { x: 630.0, y: 300.0 }, // east edge
        Vec2 { x: 950.0, y: 300.0 },
    ]);

    // ── North (spawn y=-50, travel south) ───────────────────────────────────
    // Southbound lanes: x=600 (Right), x=540 (Straight), x=480 (Left)

    // Right turn → exits East at y=420 (innermost eastbound lane)
    map.insert((Direction::North, Route::Right), vec![
        Vec2 { x: 600.0, y: -50.0 },
        Vec2 { x: 600.0, y: 270.0 },
        Vec2 { x: 600.0, y: 390.0 }, // turn entry
        Vec2 { x: 630.0, y: 420.0 }, // 45° diagonal → east edge
        Vec2 { x: 950.0, y: 420.0 },
    ]);

    // Straight → exits South at x=540
    map.insert((Direction::North, Route::Straight), vec![
        Vec2 { x: 540.0, y: -50.0 },
        Vec2 { x: 540.0, y: 270.0 },
        Vec2 { x: 540.0, y: 630.0 },
        Vec2 { x: 540.0, y: 950.0 },
    ]);

    // Left turn → exits West at y=600 (outermost westbound lane)
    map.insert((Direction::North, Route::Left), vec![
        Vec2 { x: 480.0, y: -50.0 },
        Vec2 { x: 480.0, y: 270.0 },
        Vec2 { x: 480.0, y: 570.0 }, // turn entry
        Vec2 { x: 450.0, y: 600.0 }, // 45° diagonal
        Vec2 { x: 270.0, y: 600.0 }, // west edge
        Vec2 { x: -50.0, y: 600.0 },
    ]);

    // ── West (spawn x=-50, travel east) ─────────────────────────────────────
    // Eastbound lanes: y=300 (Right), y=360 (Straight), y=420 (Left)

    // Right turn → exits North at x=420 (innermost northbound lane)
    map.insert((Direction::West, Route::Right), vec![
        Vec2 { x: -50.0, y: 300.0 },
        Vec2 { x: 270.0, y: 300.0 },
        Vec2 { x: 390.0, y: 300.0 }, // turn entry
        Vec2 { x: 420.0, y: 270.0 }, // 45° diagonal → north edge
        Vec2 { x: 420.0, y: -50.0 },
    ]);

    // Straight → exits East at y=360
    map.insert((Direction::West, Route::Straight), vec![
        Vec2 { x: -50.0, y: 360.0 },
        Vec2 { x: 270.0, y: 360.0 },
        Vec2 { x: 630.0, y: 360.0 },
        Vec2 { x: 950.0, y: 360.0 },
    ]);

    // Left turn → exits South at x=600 (outermost southbound lane)
    map.insert((Direction::West, Route::Left), vec![
        Vec2 { x: -50.0, y: 420.0 },
        Vec2 { x: 270.0, y: 420.0 },
        Vec2 { x: 570.0, y: 420.0 }, // turn entry
        Vec2 { x: 600.0, y: 450.0 }, // 45° diagonal
        Vec2 { x: 600.0, y: 630.0 }, // south edge
        Vec2 { x: 600.0, y: 950.0 },
    ]);

    // ── East (spawn x=950, travel west) ─────────────────────────────────────
    // Westbound lanes: y=600 (Right), y=540 (Straight), y=480 (Left)

    // Right turn → exits South at x=480 (innermost southbound lane)
    map.insert((Direction::East, Route::Right), vec![
        Vec2 { x: 950.0, y: 600.0 },
        Vec2 { x: 630.0, y: 600.0 },
        Vec2 { x: 510.0, y: 600.0 }, // turn entry
        Vec2 { x: 480.0, y: 630.0 }, // 45° diagonal → south edge
        Vec2 { x: 480.0, y: 950.0 },
    ]);

    // Straight → exits West at y=540
    map.insert((Direction::East, Route::Straight), vec![
        Vec2 { x: 950.0, y: 540.0 },
        Vec2 { x: 630.0, y: 540.0 },
        Vec2 { x: 270.0, y: 540.0 },
        Vec2 { x: -50.0, y: 540.0 },
    ]);

    // Left turn → exits North at x=300 (outermost northbound lane)
    map.insert((Direction::East, Route::Left), vec![
        Vec2 { x: 950.0, y: 480.0 },
        Vec2 { x: 630.0, y: 480.0 },
        Vec2 { x: 330.0, y: 480.0 }, // turn entry
        Vec2 { x: 300.0, y: 450.0 }, // 45° diagonal
        Vec2 { x: 300.0, y: 270.0 }, // north edge
        Vec2 { x: 300.0, y: -50.0 },
    ]);

    map
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::VehicleState;

    fn make_vehicle(dir: Direction, x: f32, y: f32) -> Vehicle {
        Vehicle {
            id: 99,
            direction: dir,
            route: Route::Straight,
            state: VehicleState::Approaching,
            pos: Vec2 { x, y },
            velocity: 0.0,
            target_vel: 0.0,
            angle_deg: 0.0,
            path: vec![],
            path_index: 0,
            entry_time_ms: 0,
            exit_time_ms: 0,
            distance_travelled: 0.0,
        }
    }

    fn is_off_screen(p: &Vec2) -> bool {
        p.x < 0.0 || p.x > 900.0 || p.y < 0.0 || p.y > 900.0
    }

    #[test]
    fn path_map_has_all_12_entries() {
        let map = build_path_map();
        assert_eq!(map.len(), 12);

        for dir in &[Direction::North, Direction::South, Direction::East, Direction::West] {
            for route in &[Route::Right, Route::Straight, Route::Left] {
                assert!(
                    map.contains_key(&(*dir, *route)),
                    "missing path for {:?} {:?}",
                    dir,
                    route
                );
            }
        }
    }

    #[test]
    fn no_path_is_empty_and_no_consecutive_duplicates() {
        let map = build_path_map();
        for ((dir, route), path) in &map {
            assert!(!path.is_empty(), "{:?} {:?} path is empty", dir, route);
            for window in path.windows(2) {
                let a = &window[0];
                let b = &window[1];
                assert!(
                    !(a.x == b.x && a.y == b.y),
                    "{:?} {:?}: consecutive duplicate waypoints at ({}, {})",
                    dir,
                    route,
                    a.x,
                    a.y
                );
            }
        }
    }

    #[test]
    fn first_waypoint_is_off_screen() {
        let map = build_path_map();
        for ((dir, route), path) in &map {
            assert!(
                is_off_screen(&path[0]),
                "{:?} {:?}: first waypoint ({}, {}) is not off-screen",
                dir,
                route,
                path[0].x,
                path[0].y
            );
        }
    }

    #[test]
    fn last_waypoint_is_off_screen() {
        let map = build_path_map();
        for ((dir, route), path) in &map {
            let last = path.last().unwrap();
            assert!(
                is_off_screen(last),
                "{:?} {:?}: last waypoint ({}, {}) is not off-screen",
                dir,
                route,
                last.x,
                last.y
            );
        }
    }

    // Spot-check four specific paths for first/last waypoints
    #[test]
    fn spot_check_four_paths() {
        let map = build_path_map();

        // (South, Right) — canonical shape from SDS §4.2
        let p = &map[&(Direction::South, Route::Right)];
        assert!(p[0].y > 900.0, "South-Right must start below screen");
        assert!(p.last().unwrap().x < 0.0, "South-Right must exit off west edge");

        // (North, Straight) — straight through, exits south
        let p = &map[&(Direction::North, Route::Straight)];
        assert!(p[0].y < 0.0, "North-Straight must start above screen");
        assert!(p.last().unwrap().y > 900.0, "North-Straight must exit below screen");

        // (West, Left) — turns south, exits bottom
        let p = &map[&(Direction::West, Route::Left)];
        assert!(p[0].x < 0.0, "West-Left must start left of screen");
        assert!(p.last().unwrap().y > 900.0, "West-Left must exit below screen");

        // (East, Right) — turns south, exits bottom
        let p = &map[&(Direction::East, Route::Right)];
        assert!(p[0].x > 900.0, "East-Right must start right of screen");
        assert!(p.last().unwrap().y > 900.0, "East-Right must exit below screen");
    }

    #[test]
    fn path_index_all_12_unique_and_in_range() {
        let dirs   = [Direction::North, Direction::South, Direction::West, Direction::East];
        let routes = [Route::Right, Route::Straight, Route::Left];
        let mut seen = std::collections::HashSet::new();
        for dir in dirs {
            for route in routes {
                let idx = path_index(dir, route);
                assert!(idx < 12, "index {} out of range", idx);
                assert!(seen.insert(idx), "duplicate index {} for {:?} {:?}", idx, dir, route);
            }
        }
    }

    #[test]
    fn all_four_right_turns_non_conflicting() {
        let mut mgr = IntersectionManager::new();
        assert!(mgr.request_reservation(1, Direction::North, Route::Right));
        assert!(mgr.request_reservation(2, Direction::South, Route::Right));
        assert!(mgr.request_reservation(3, Direction::West,  Route::Right));
        assert!(mgr.request_reservation(4, Direction::East,  Route::Right));
        assert_eq!(mgr.active_count(), 4);
    }

    #[test]
    fn conflicting_request_denied() {
        let mut mgr = IntersectionManager::new();
        assert!(mgr.request_reservation(1, Direction::North, Route::Straight));
        assert!(!mgr.request_reservation(2, Direction::South, Route::Left));
        assert_eq!(mgr.active_count(), 1);
    }

    #[test]
    fn release_frees_slot() {
        let mut mgr = IntersectionManager::new();
        assert!(mgr.request_reservation(1, Direction::North, Route::Straight));
        mgr.release_reservation(1);
        assert!(mgr.request_reservation(2, Direction::South, Route::Left));
        assert_eq!(mgr.active_count(), 1);
    }

    #[test]
    fn active_count_tracks_grants_and_releases() {
        let mut mgr = IntersectionManager::new();
        mgr.request_reservation(1, Direction::North, Route::Right);
        mgr.request_reservation(2, Direction::South, Route::Right);
        mgr.request_reservation(3, Direction::West,  Route::Right);
        assert_eq!(mgr.active_count(), 3);
        mgr.release_reservation(2);
        assert_eq!(mgr.active_count(), 2);
    }

    #[test]
    fn spec_south_straight_north_right_no_conflict() {
        // South-Straight occupies column 1 (x=360); North-Right occupies column 5
        // (x=600 then exits east). Their intersection cells do not overlap.
        let mut mgr = IntersectionManager::new();
        assert!(mgr.request_reservation(1, Direction::South, Route::Straight));
        assert!(mgr.request_reservation(2, Direction::North, Route::Right));
        assert_eq!(mgr.active_count(), 2);
    }

    #[test]
    fn idempotent_re_request() {
        let mut mgr = IntersectionManager::new();
        assert!(mgr.request_reservation(1, Direction::North, Route::Straight));
        assert!(mgr.request_reservation(1, Direction::North, Route::Straight));
        assert_eq!(mgr.active_count(), 1);
    }

    #[test]
    fn trigger_zone_south_inside() {
        // South vehicle (traveling north) — stop line y=600, zone y∈(600, 800]
        let mgr = IntersectionManager::new();
        assert!(mgr.is_in_trigger_zone(&make_vehicle(Direction::South, 420.0, 750.0)));
    }

    #[test]
    fn trigger_zone_south_too_far() {
        let mgr = IntersectionManager::new();
        assert!(!mgr.is_in_trigger_zone(&make_vehicle(Direction::South, 420.0, 850.0)));
    }

    #[test]
    fn trigger_zone_south_past_stop_line() {
        let mgr = IntersectionManager::new();
        // y=590 is inside the intersection box — no longer in approach trigger zone
        assert!(!mgr.is_in_trigger_zone(&make_vehicle(Direction::South, 420.0, 590.0)));
    }

    #[test]
    fn trigger_zone_north_inside() {
        // North vehicle (traveling south) — stop line y=300, zone y∈[100, 300)
        let mgr = IntersectionManager::new();
        assert!(mgr.is_in_trigger_zone(&make_vehicle(Direction::North, 480.0, 150.0)));
    }

    #[test]
    fn trigger_zone_west_inside() {
        // West vehicle (traveling east) — stop line x=300, zone x∈[100, 300)
        let mgr = IntersectionManager::new();
        assert!(mgr.is_in_trigger_zone(&make_vehicle(Direction::West, 200.0, 420.0)));
    }

    #[test]
    fn trigger_zone_east_inside() {
        // East vehicle (traveling west) — stop line x=600, zone x∈(600, 800]
        let mgr = IntersectionManager::new();
        assert!(mgr.is_in_trigger_zone(&make_vehicle(Direction::East, 650.0, 480.0)));
    }
}
