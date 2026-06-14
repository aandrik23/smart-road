use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::intersection::IntersectionManager;
use crate::types::{
    Vehicle,
    Stats,
    Route,
    INTER_X,
    INTER_Y,
    INTER_W,
    INTER_H,
};

fn draw_dashed_vertical(
    canvas: &mut Canvas<Window>,
    x: i32,
    y1: i32,
    y2: i32,
) {
    let mut y = y1;

    while y < y2 {
        canvas
            .fill_rect(Rect::new(x - 1, y, 3, 22))
            .ok();

        y += 38;
    }
}

fn draw_dashed_horizontal(
    canvas: &mut Canvas<Window>,
    y: i32,
    x1: i32,
    x2: i32,
) {
    let mut x = x1;

    while x < x2 {
        canvas
            .fill_rect(Rect::new(x, y - 1, 22, 3))
            .ok();

        x += 38;
    }
}

pub fn draw(
    canvas: &mut Canvas<Window>,
    vehicles: &[Vehicle],
    _manager: &IntersectionManager,
    _stats: &Stats,
) {
    // Background
    canvas.set_draw_color(Color::RGB(20, 20, 20));
    canvas.clear();

    // Roads
    canvas.set_draw_color(Color::RGB(70, 70, 70));

    canvas.fill_rect(Rect::new(
        INTER_X as i32,
        0,
        INTER_W as u32,
        900,
    )).ok();

    canvas.fill_rect(Rect::new(
        0,
        INTER_Y as i32,
        900,
        INTER_H as u32,
    )).ok();

    // Intersection
    canvas.set_draw_color(Color::RGB(90, 90, 90));

    canvas.fill_rect(Rect::new(
        INTER_X as i32,
        INTER_Y as i32,
        INTER_W as u32,
        INTER_H as u32,
    )).ok();

    // ==========================
    // LANE MARKINGS
    // ==========================

    canvas.set_draw_color(Color::RGB(230, 230, 230));

    // Vertical road lane borders
    for x in [330, 390, 450, 510, 570] {
        draw_dashed_vertical(canvas, x, 0, 300);
        draw_dashed_vertical(canvas, x, 600, 900);
    }

    // Horizontal road lane borders
    for y in [330, 390, 450, 510, 570] {
        draw_dashed_horizontal(canvas, y, 0, 300);
        draw_dashed_horizontal(canvas, y, 600, 900);
    }

    // Center guides
    draw_dashed_vertical(canvas, 450, 300, 600);
    draw_dashed_horizontal(canvas, 450, 300, 600);

    // Stop lines
    canvas.fill_rect(Rect::new(300, 295, 300, 5)).ok();
    canvas.fill_rect(Rect::new(300, 600, 300, 5)).ok();
    canvas.fill_rect(Rect::new(295, 300, 5, 300)).ok();
    canvas.fill_rect(Rect::new(600, 300, 5, 300)).ok();

    // ==========================
    // VEHICLES
    // ==========================

    for vehicle in vehicles {
        match vehicle.route {
            Route::Right => {
                canvas.set_draw_color(Color::RGB(0, 220, 0));
            }
            Route::Straight => {
                canvas.set_draw_color(Color::RGB(220, 220, 0));
            }
            Route::Left => {
                canvas.set_draw_color(Color::RGB(220, 40, 40));
            }
        }

        let rect = Rect::new(
            vehicle.pos.x as i32 - 8,
            vehicle.pos.y as i32 - 8,
            16,
            16,
        );

        canvas.fill_rect(rect).ok();
    }

    canvas.present();
}