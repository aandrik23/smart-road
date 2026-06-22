use std::collections::HashMap;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use crate::types::{
    Direction, Route, Vehicle, VehicleState, VehicleType, Vec2,
    SPEED_FAST, SPAWN_INTERVAL_MS, SAFE_DISTANCE,
};

#[derive(Debug, Clone)]
pub struct InputState {
    pub quit: bool,
    pub last_spawn: [u64; 4],
}

impl InputState {
    pub fn new() -> Self {
        Self {
            quit: false,
            last_spawn: [0; 4],
        }
    }
}

fn direction_index(direction: Direction) -> usize {
    match direction {
        Direction::North => 0,
        Direction::South => 1,
        Direction::West => 2,
        Direction::East => 3,
    }
}

fn route_from_lane_index(lane_index: usize) -> Route {
    match lane_index % 3 {
        0 => Route::Right,
        1 => Route::Straight,
        _ => Route::Left,
    }
}

fn pseudo_random_index(seed: u64, limit: usize) -> usize {
    if limit == 0 {
        return 0;
    }

    let mixed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);

    (mixed as usize) % limit
}

fn initial_angle(path: &[Vec2]) -> f64 {
    if path.len() < 2 {
        return 0.0;
    }

    let dx = path[1].x - path[0].x;
    let dy = path[1].y - path[0].y;

    f64::atan2(dy as f64, dx as f64).to_degrees()
}

fn spawn_vehicle(
    direction: Direction,
    vehicles: &mut Vec<Vehicle>,
    path_map: &HashMap<(Direction, Route), Vec<Vec2>>,
    next_id: &mut u32,
    now_ms: u64,
) {
    let lane_index = pseudo_random_index(
        now_ms ^ ((*next_id as u64) << 32) ^ direction_index(direction) as u64,
        3,
    );

    let route = route_from_lane_index(lane_index);

    let Some(path) = path_map.get(&(direction, route)) else {
        return;
    };

    // Hard cap: at most 4 vehicles queued on the approach road.
    // Vehicles already in the intersection or exiting don't count — they're
    // no longer occupying the lane and shouldn't block new spawns.
    let lane_count = vehicles
        .iter()
        .filter(|v| {
            v.direction == direction
                && v.route == route
                && v.state == VehicleState::Approaching
        })
        .count();
    if lane_count >= 3 {
        return;
    }

    // Don't spawn if the lane is backed up to the spawn point.
    let spawn = path[0];
    let blocked = vehicles.iter().any(|v| {
        if v.direction != direction || v.route != route {
            return false;
        }
        let dx = v.pos.x - spawn.x;
        let dy = v.pos.y - spawn.y;
        (dx * dx + dy * dy).sqrt() < SAFE_DISTANCE
    });
    if blocked {
        return;
    }

    let type_index = pseudo_random_index(
        now_ms ^ ((*next_id as u64) << 8) ^ 0xBEEF,
        4,
    );
    let vehicle_type = match type_index {
        0 => VehicleType::Civic,
        1 => VehicleType::Jeep,
        2 => VehicleType::Sedan,
        _ => VehicleType::Taxi,
    };

    vehicles.push(Vehicle {
        id: *next_id,
        vehicle_type,
        direction,
        route,
        state: VehicleState::Approaching,
        pos: path[0],
        velocity: SPEED_FAST,
        target_vel: SPEED_FAST,
        angle_deg: initial_angle(path),
        path: path.clone(),
        path_index: 1,
        entry_time_ms: now_ms,
        exit_time_ms: 0,
        distance_travelled: 0.0,
    });

    *next_id += 1;
}

fn try_spawn_vehicle(
    direction: Direction,
    input_state: &mut InputState,
    vehicles: &mut Vec<Vehicle>,
    path_map: &HashMap<(Direction, Route), Vec<Vec2>>,
    next_id: &mut u32,
    now_ms: u64,
) {
    let index = direction_index(direction);
    let last_spawn = input_state.last_spawn[index];

    if last_spawn != 0 && now_ms.saturating_sub(last_spawn) <= SPAWN_INTERVAL_MS {
        return;
    }

    spawn_vehicle(direction, vehicles, path_map, next_id, now_ms);
    input_state.last_spawn[index] = now_ms.max(1);
}

pub fn handle_events(
    event_pump: &mut sdl2::EventPump,
    input_state: &mut InputState,
    vehicles: &mut Vec<Vehicle>,
    path_map: &HashMap<(Direction, Route), Vec<Vec2>>,
    next_id: &mut u32,
    now_ms: u64,
) {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. } => input_state.quit = true,

            Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => input_state.quit = true,

            Event::KeyDown {
                keycode: Some(Keycode::R),
                repeat: false,
                ..
            } => {
                let directions = [
                    Direction::North,
                    Direction::South,
                    Direction::West,
                    Direction::East,
                ];
                let index = pseudo_random_index(now_ms ^ ((*next_id as u64) << 16), directions.len());
                try_spawn_vehicle(directions[index], input_state, vehicles, path_map, next_id, now_ms);
            }

            Event::KeyDown {
                keycode: Some(Keycode::Up),
                ..
            } => try_spawn_vehicle(
                Direction::South,
                input_state,
                vehicles,
                path_map,
                next_id,
                now_ms,
            ),

            Event::KeyDown {
                keycode: Some(Keycode::Down),
                ..
            } => try_spawn_vehicle(
                Direction::North,
                input_state,
                vehicles,
                path_map,
                next_id,
                now_ms,
            ),

            Event::KeyDown {
                keycode: Some(Keycode::Right),
                ..
            } => try_spawn_vehicle(
                Direction::West,
                input_state,
                vehicles,
                path_map,
                next_id,
                now_ms,
            ),

            Event::KeyDown {
                keycode: Some(Keycode::Left),
                ..
            } => try_spawn_vehicle(
                Direction::East,
                input_state,
                vehicles,
                path_map,
                next_id,
                now_ms,
            ),

            _ => {}
        }
    }
}