#![allow(dead_code)]

mod types;
mod intersection;
mod vehicle;
mod renderer;
mod input;
mod stats;

use std::time::Instant;

use types::{Stats, Vehicle, WINDOW_HEIGHT, WINDOW_WIDTH};
use input::InputState;
use intersection::{build_path_map, IntersectionManager};

fn main() {
    let sdl = sdl2::init().expect("SDL2 init failed");
    let video = sdl.video().expect("SDL2 video init failed");

    let window = video
        .window("Smart Road", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .expect("window creation failed");

    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .expect("canvas creation failed");

    let texture_creator = canvas.texture_creator();
    let textures = renderer::create_vehicle_textures(&texture_creator);

    let mut event_pump = sdl.event_pump().expect("event pump failed");

    let path_map = build_path_map();
    let mut manager = IntersectionManager::new();

    let mut vehicles: Vec<Vehicle> = Vec::new();
    let mut input_state = InputState::new();
    let mut next_id = 1;

    let mut stats = Stats {
        total_passed: 0,
        max_velocity: 0.0,
        min_velocity: f32::MAX,
        max_time_ms: 0,
        min_time_ms: u64::MAX,
        close_calls: 0,
    };

    let start_time = Instant::now();
    let mut last_vehicle_count = 0;

    'running: loop {
        let now_ms = start_time.elapsed().as_millis() as u64;

        input::handle_events(
            &mut event_pump,
            &mut input_state,
            &mut vehicles,
            &path_map,
            &mut next_id,
            now_ms,
        );

        if input_state.quit {
            break 'running;
        }

        let dt = 1.0 / 60.0;
        let snapshot = vehicles.clone();

        vehicles.retain_mut(|vehicle| {
            stats::record_velocity(&mut stats, vehicle.velocity);

            let alive = vehicle::update(
                vehicle,
                dt,
                &path_map,
                &mut manager,
                &snapshot,
                now_ms,
            );

            if !alive {
                stats::record_passed(&mut stats);
                stats::record_transit(
                    &mut stats,
                    vehicle.entry_time_ms,
                    vehicle.exit_time_ms,
                );
            }

            alive
        });

        stats::record_close_calls(&mut stats, &vehicles);

        if vehicles.len() != last_vehicle_count {
            println!("active vehicles: {}", vehicles.len());

            if let Some(vehicle) = vehicles.last() {
                println!(
                    "vehicle id={} from {:?}, route {:?}, pos=({}, {})",
                    vehicle.id,
                    vehicle.direction,
                    vehicle.route,
                    vehicle.pos.x,
                    vehicle.pos.y
                );
            }

            last_vehicle_count = vehicles.len();
        }

        renderer::draw(
            &mut canvas,
            &vehicles,
            &manager,
            &textures,
        );
    }

    stats::print_final(&stats);
}