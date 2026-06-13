use std::collections::HashMap;
use crate::types::{Direction, Route, Vec2};

pub struct IntersectionManager;

impl IntersectionManager {
    pub fn new() -> Self {
        Self
    }

    pub fn request_reservation(&mut self, _id: u32, _dir: Direction, _route: Route) -> bool {
        todo!("B1: reservation grant/deny with conflict table")
    }

    pub fn release_reservation(&mut self, _id: u32) {
        todo!("B1: release after full intersection clearance")
    }

    pub fn is_in_trigger_zone(&self, _distance_to_intersection: f32) -> bool {
        todo!("B1: 200 px trigger zone check")
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

    // ── Direction::South  (spawn from south y=950, travel north) ─────────────

    // Right → exits West at y=540
    // Canonical shape (SDS §4.2): approach x=360, turn near SW corner, exit west
    map.insert((Direction::South, Route::Right), vec![
        Vec2 { x: 360.0, y: 950.0 },
        Vec2 { x: 360.0, y: 600.0 },
        Vec2 { x: 360.0, y: 570.0 }, // entering turn zone
        Vec2 { x: 330.0, y: 540.0 }, // 45° diagonal
        Vec2 { x: 300.0, y: 540.0 }, // west edge of intersection
        Vec2 { x: -50.0, y: 540.0 },
    ]);

    // Straight → exits North at x=420
    map.insert((Direction::South, Route::Straight), vec![
        Vec2 { x: 420.0, y: 950.0 },
        Vec2 { x: 420.0, y: 600.0 },
        Vec2 { x: 420.0, y: 300.0 },
        Vec2 { x: 420.0, y: -50.0 },
    ]);

    // Left → exits East at y=360
    // Travels north through intersection, turns near NE corner
    map.insert((Direction::South, Route::Left), vec![
        Vec2 { x: 480.0, y: 950.0 },
        Vec2 { x: 480.0, y: 600.0 },
        Vec2 { x: 480.0, y: 390.0 }, // entering turn zone
        Vec2 { x: 510.0, y: 360.0 }, // 45° diagonal
        Vec2 { x: 600.0, y: 360.0 }, // east edge of intersection
        Vec2 { x: 950.0, y: 360.0 },
    ]);

    // ── Direction::North  (spawn from north y=-50, travel south) ─────────────

    // Right → exits East at y=360
    map.insert((Direction::North, Route::Right), vec![
        Vec2 { x: 540.0, y: -50.0 },
        Vec2 { x: 540.0, y: 300.0 },
        Vec2 { x: 540.0, y: 330.0 },
        Vec2 { x: 570.0, y: 360.0 },
        Vec2 { x: 600.0, y: 360.0 },
        Vec2 { x: 950.0, y: 360.0 },
    ]);

    // Straight → exits South at x=480
    map.insert((Direction::North, Route::Straight), vec![
        Vec2 { x: 480.0, y: -50.0 },
        Vec2 { x: 480.0, y: 300.0 },
        Vec2 { x: 480.0, y: 600.0 },
        Vec2 { x: 480.0, y: 950.0 },
    ]);

    // Left → exits West at y=540
    // Travels south through intersection, turns near SW corner
    map.insert((Direction::North, Route::Left), vec![
        Vec2 { x: 420.0, y: -50.0 },
        Vec2 { x: 420.0, y: 300.0 },
        Vec2 { x: 420.0, y: 510.0 },
        Vec2 { x: 390.0, y: 540.0 },
        Vec2 { x: 300.0, y: 540.0 },
        Vec2 { x: -50.0, y: 540.0 },
    ]);

    // ── Direction::West  (spawn from west x=-50, travel east) ────────────────

    // Right → exits North at x=360
    map.insert((Direction::West, Route::Right), vec![
        Vec2 { x: -50.0, y: 360.0 },
        Vec2 { x: 300.0, y: 360.0 },
        Vec2 { x: 330.0, y: 360.0 },
        Vec2 { x: 360.0, y: 330.0 },
        Vec2 { x: 360.0, y: 300.0 },
        Vec2 { x: 360.0, y: -50.0 },
    ]);

    // Straight → exits East at y=420
    map.insert((Direction::West, Route::Straight), vec![
        Vec2 { x: -50.0, y: 420.0 },
        Vec2 { x: 300.0, y: 420.0 },
        Vec2 { x: 600.0, y: 420.0 },
        Vec2 { x: 950.0, y: 420.0 },
    ]);

    // Left → exits South at x=540
    // Travels east through intersection, turns near SE corner
    map.insert((Direction::West, Route::Left), vec![
        Vec2 { x: -50.0, y: 480.0 },
        Vec2 { x: 300.0, y: 480.0 },
        Vec2 { x: 510.0, y: 480.0 },
        Vec2 { x: 540.0, y: 510.0 },
        Vec2 { x: 540.0, y: 600.0 },
        Vec2 { x: 540.0, y: 950.0 },
    ]);

    // ── Direction::East  (spawn from east x=950, travel west) ────────────────

    // Right → exits South at x=540
    map.insert((Direction::East, Route::Right), vec![
        Vec2 { x: 950.0, y: 540.0 },
        Vec2 { x: 600.0, y: 540.0 },
        Vec2 { x: 570.0, y: 540.0 },
        Vec2 { x: 540.0, y: 570.0 },
        Vec2 { x: 540.0, y: 600.0 },
        Vec2 { x: 540.0, y: 950.0 },
    ]);

    // Straight → exits West at y=480
    map.insert((Direction::East, Route::Straight), vec![
        Vec2 { x: 950.0, y: 480.0 },
        Vec2 { x: 600.0, y: 480.0 },
        Vec2 { x: 300.0, y: 480.0 },
        Vec2 { x: -50.0, y: 480.0 },
    ]);

    // Left → exits North at x=360
    // Travels west through intersection, turns near NW corner
    map.insert((Direction::East, Route::Left), vec![
        Vec2 { x: 950.0, y: 420.0 },
        Vec2 { x: 600.0, y: 420.0 },
        Vec2 { x: 390.0, y: 420.0 },
        Vec2 { x: 360.0, y: 390.0 },
        Vec2 { x: 360.0, y: 300.0 },
        Vec2 { x: 360.0, y: -50.0 },
    ]);

    map
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
}
