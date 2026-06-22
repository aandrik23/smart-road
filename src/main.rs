mod types;
mod intersection;
mod vehicle;
mod renderer;
mod input;
mod stats;

use std::time::{Duration, Instant};

use sdl2::event::Event;

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
    let mut active_close_pairs: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();

    let start_time = Instant::now();
    let mut prev_instant = Instant::now();
    let mut last_vehicle_count = 0;

    'running: loop {
        let frame_now = Instant::now();
        let dt = frame_now.duration_since(prev_instant).as_secs_f32().min(0.05);
        prev_instant = frame_now;
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
            renderer::draw_stats_overlay(&mut canvas, &stats);
            'overlay: loop {
                for event in event_pump.poll_iter() {
                    match event {
                        Event::Quit { .. } | Event::KeyDown { .. } | Event::MouseButtonDown { .. } => {
                            break 'overlay;
                        }
                        _ => {}
                    }
                }
                std::thread::sleep(Duration::from_millis(16));
            }
            break 'running;
        }

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

        stats::record_close_calls(&mut stats, &vehicles, &mut active_close_pairs);

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

}