use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::intersection::IntersectionManager;
use crate::types::{
    Vehicle, Route,
    INTER_X, INTER_Y, INTER_W, INTER_H,
    SPRITE_W, SPRITE_H, FRAME_COUNT, FRAME_STRIDE,
    WINDOW_WIDTH, WINDOW_HEIGHT,
};

// 3×5 bitmapped font for digits 0–9; each row is a 3-bit mask, MSB = left column.
const DIGIT_FONT: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b010, 0b010, 0b010, 0b010], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b110, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b001, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
];

pub struct VehicleTextures<'a> {
    right:    Texture<'a>,
    straight: Texture<'a>,
    left:     Texture<'a>,
}

fn make_colored_texture<'a>(tc: &'a TextureCreator<WindowContext>, color: Color) -> Texture<'a> {
    let mut surface = Surface::new(SPRITE_W, SPRITE_H, PixelFormatEnum::RGB24)
        .expect("surface creation failed");
    surface.fill_rect(None, color).expect("surface fill failed");
    tc.create_texture_from_surface(surface).expect("texture from surface failed")
}

pub fn create_vehicle_textures<'a>(tc: &'a TextureCreator<WindowContext>) -> VehicleTextures<'a> {
    VehicleTextures {
        right:    make_colored_texture(tc, Color::RGB(0, 220, 0)),
        straight: make_colored_texture(tc, Color::RGB(220, 220, 0)),
        left:     make_colored_texture(tc, Color::RGB(220, 40, 40)),
    }
}

fn draw_dashed_vertical(canvas: &mut Canvas<Window>, x: i32, y1: i32, y2: i32) {
    let mut y = y1;
    while y < y2 {
        canvas.fill_rect(Rect::new(x - 1, y, 3, 22)).ok();
        y += 38;
    }
}

fn draw_dashed_horizontal(canvas: &mut Canvas<Window>, y: i32, x1: i32, x2: i32) {
    let mut x = x1;
    while x < x2 {
        canvas.fill_rect(Rect::new(x, y - 1, 22, 3)).ok();
        x += 38;
    }
}

fn draw_digit(canvas: &mut Canvas<Window>, digit: u8, x: i32, y: i32, scale: i32) {
    let rows = DIGIT_FONT[digit.min(9) as usize];
    for (row, &bits) in rows.iter().enumerate() {
        for col in 0i32..3 {
            if bits & (0b100 >> col) != 0 {
                canvas
                    .fill_rect(Rect::new(
                        x + col * scale,
                        y + row as i32 * scale,
                        scale as u32,
                        scale as u32,
                    ))
                    .ok();
            }
        }
    }
}

fn draw_number(canvas: &mut Canvas<Window>, mut n: u32, x: i32, y: i32, scale: i32) {
    let mut digits: Vec<u8> = Vec::new();
    if n == 0 {
        digits.push(0);
    }
    while n > 0 {
        digits.push((n % 10) as u8);
        n /= 10;
    }
    digits.reverse();
    let char_w = 3 * scale + scale; // glyph width + 1-cell gap
    for (i, &d) in digits.iter().enumerate() {
        draw_digit(canvas, d, x + i as i32 * char_w, y, scale);
    }
}

pub fn draw(
    canvas:   &mut Canvas<Window>,
    vehicles: &[Vehicle],
    manager:  &IntersectionManager,
    textures: &VehicleTextures<'_>,
) {
    // 1. Background
    canvas.set_draw_color(Color::RGB(20, 20, 20));
    canvas.clear();

    // 2. Road strips (horizontal + vertical)
    canvas.set_draw_color(Color::RGB(70, 70, 70));
    canvas.fill_rect(Rect::new(INTER_X as i32, 0, INTER_W as u32, WINDOW_HEIGHT)).ok();
    canvas.fill_rect(Rect::new(0, INTER_Y as i32, WINDOW_WIDTH, INTER_H as u32)).ok();

    // 3. Intersection box — slightly lighter gray
    canvas.set_draw_color(Color::RGB(90, 90, 90));
    canvas
        .fill_rect(Rect::new(
            INTER_X as i32,
            INTER_Y as i32,
            INTER_W as u32,
            INTER_H as u32,
        ))
        .ok();

    // 4. Lane markings
    canvas.set_draw_color(Color::RGB(230, 230, 230));

    for x in [330, 390, 450, 510, 570] {
        draw_dashed_vertical(canvas, x, 0, 300);
        draw_dashed_vertical(canvas, x, 600, 900);
    }
    for y in [330, 390, 450, 510, 570] {
        draw_dashed_horizontal(canvas, y, 0, 300);
        draw_dashed_horizontal(canvas, y, 600, 900);
    }
    // Center guides inside intersection
    draw_dashed_vertical(canvas, 450, 300, 600);
    draw_dashed_horizontal(canvas, 450, 300, 600);

    // Stop lines
    canvas.fill_rect(Rect::new(300, 295, 300, 5)).ok();
    canvas.fill_rect(Rect::new(300, 600, 300, 5)).ok();
    canvas.fill_rect(Rect::new(295, 300, 5, 300)).ok();
    canvas.fill_rect(Rect::new(600, 300, 5, 300)).ok();

    // 5. Vehicles — copy_ex with angle_deg rotation and frame animation
    for vehicle in vehicles {
        let frame_x =
            (vehicle.distance_travelled as u32 / FRAME_STRIDE % FRAME_COUNT) * SPRITE_W;
        let src_rect = Rect::new(frame_x as i32, 0, SPRITE_W, SPRITE_H);
        let dst_rect = Rect::new(
            vehicle.pos.x as i32 - SPRITE_W as i32 / 2,
            vehicle.pos.y as i32 - SPRITE_H as i32 / 2,
            SPRITE_W,
            SPRITE_H,
        );
        let texture = match vehicle.route {
            Route::Right    => &textures.right,
            Route::Straight => &textures.straight,
            Route::Left     => &textures.left,
        };
        canvas
            .copy_ex(texture, src_rect, dst_rect, vehicle.angle_deg, None, false, false)
            .ok();
    }

    // 6. HUD — top-left corner
    //    Green  row: vehicle count
    //    Cyan   row: active reservation count
    let scale = 2i32;
    let panel_w = 64u32;
    let panel_h = 34u32;

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.fill_rect(Rect::new(4, 4, panel_w, panel_h)).ok();

    // Vehicle count indicator (green square + digits)
    canvas.set_draw_color(Color::RGB(0, 220, 0));
    canvas.fill_rect(Rect::new(6, 8, 5, 5 * scale as u32)).ok();
    draw_number(canvas, vehicles.len() as u32, 16, 7, scale);

    // Reservation count indicator (cyan square + digits)
    canvas.set_draw_color(Color::RGB(100, 200, 255));
    canvas.fill_rect(Rect::new(6, 22, 5, 5 * scale as u32)).ok();
    draw_number(canvas, manager.active_count() as u32, 16, 21, scale);

    canvas.present();
}
