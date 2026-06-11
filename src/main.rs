// Stub modules are intentionally unused until later tickets wire them up.
#![allow(dead_code)]

mod types;
mod intersection;
mod vehicle;
mod renderer;
mod input;
mod stats;

use std::time::{Duration, Instant};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;

use types::WINDOW_WIDTH;
use types::WINDOW_HEIGHT;

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

    let mut event_pump = sdl.event_pump().expect("event pump failed");

    let target_frame = Duration::from_micros(16_667); // ~60 FPS

    'running: loop {
        let frame_start = Instant::now();

        // --- events ---
        let events: Vec<Event> = event_pump.poll_iter().collect();
        for event in &events {
            if let Event::KeyDown { keycode: Some(Keycode::Escape), .. } = event {
                break 'running;
            }
        }
        // Placeholder: handle_events(...) will be wired here in C2/C3

        // --- update ---
        // Placeholder: vehicle::update(...) called per vehicle here in C3

        // --- draw ---
        canvas.set_draw_color(Color::RGB(0x3a, 0x3a, 0x3a));
        canvas.clear();
        // Placeholder: renderer::draw(...) called here in C3
        canvas.present();

        // --- FPS cap ---
        let elapsed = frame_start.elapsed();
        if elapsed < target_frame {
            std::thread::sleep(target_frame - elapsed);
        }
    }
}
