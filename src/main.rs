mod types;
mod intersection;
mod vehicle;
mod renderer;
mod input;
mod stats;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use types::{WINDOW_WIDTH, WINDOW_HEIGHT};

fn main() {
    let sdl_context = sdl2::init().expect("SDL2 init failed");
    let video = sdl_context.video().expect("SDL2 video init failed");

    let window = video
        .window("Smart Road", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .expect("Window creation failed");

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .expect("Canvas creation failed");

    let mut event_pump = sdl_context.event_pump().expect("Event pump init failed");

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'running,
                _ => {}
            }
        }

        canvas.set_draw_color(sdl2::pixels::Color::RGB(34, 85, 34));
        canvas.clear();
        canvas.present();
    }
}
