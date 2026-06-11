use std::collections::HashMap;
use crate::types::{Direction, Route, Vehicle, Vec2};
use crate::intersection::IntersectionManager;

pub fn update(
    _vehicle: &mut Vehicle,
    _dt: f32,
    _path_map: &HashMap<(Direction, Route), Vec<Vec2>>,
    _manager: &mut IntersectionManager,
    _all_vehicles: &[Vehicle],
    _now_ms: u64,
) {
    todo!("B2: waypoint traversal, accel/decel, reservation lifecycle, safe-distance check")
}
