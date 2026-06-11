use std::collections::HashMap;
use sdl2::event::Event;
use crate::types::{Direction, Route, Vehicle, Vec2};

pub fn handle_events(
    _events: &[Event],
    _vehicles: &mut Vec<Vehicle>,
    _path_map: &HashMap<(Direction, Route), Vec<Vec2>>,
    _last_spawn_times: &mut HashMap<Direction, u64>,
    _random_mode: &mut bool,
    _now_ms: u64,
) -> bool {
    todo!("C2: arrow-key spawn, R toggle, Esc quit with spawn guard")
}
