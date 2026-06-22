use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::intersection::IntersectionManager;
use crate::types::{
    Stats, Vehicle,
    INTER_X, INTER_Y, INTER_W, INTER_H,
    SPRITE_W, SPRITE_H,
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
    sheets: [Texture<'a>; 4],
}

fn load_sheet<'a>(tc: &'a TextureCreator<WindowContext>, path: &str) -> Texture<'a> {
    let img = image::open(path)
        .unwrap_or_else(|_| panic!("failed to load {path}"))
        .into_rgba8();
    let (w, h) = img.dimensions();
    let mut pixels = img.into_raw();
    // ABGR8888 interprets memory as R,G,B,A on little-endian x86, matching image's RGBA output.
    let surface = Surface::from_data(&mut pixels, w, h, w * 4, PixelFormatEnum::ABGR8888)
        .expect("surface from data");
    tc.create_texture_from_surface(&surface).expect("texture from surface")
}

pub fn create_vehicle_textures<'a>(tc: &'a TextureCreator<WindowContext>) -> VehicleTextures<'a> {
    VehicleTextures {
        sheets: [
            load_sheet(tc, "assets/CIVIC_CLEAN_8D_000-sheet.png"),
            load_sheet(tc, "assets/JEEP_CLEAN_8D_000-sheet.png"),
            load_sheet(tc, "assets/SEDAN_CLEAN_8D_000-sheet.png"),
            load_sheet(tc, "assets/TAXI_CLEAN_8D0000-sheet.png"),
        ],
    }
}

// Each sheet is 300×300 with 8 sprites in a 3×3 grid (100×100 each, last cell empty).
// Layout: row0 = E, SE, S  |  row1 = SW, W, NW  |  row2 = N, NE, (empty)
// angle_deg follows atan2(dy, dx): 0=East, 90=South, ±180=West, -90=North.
fn angle_to_src_rect(angle_deg: f64) -> Rect {
    let norm = ((angle_deg % 360.0) + 360.0) % 360.0;
    let index = ((norm + 22.5) / 45.0) as u32 % 8;
    let (col, row): (u32, u32) = match index {
        0 => (0, 0), // East
        1 => (1, 0), // Southeast
        2 => (2, 0), // South
        3 => (0, 1), // Southwest
        4 => (1, 1), // West
        5 => (2, 1), // Northwest
        6 => (0, 2), // North
        7 => (1, 2), // Northeast
        _ => (0, 0),
    };
    Rect::new((col * 100) as i32, (row * 100) as i32, 100, 100)
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

fn glyph_for(ch: char) -> Option<[u8; 5]> {
    match ch {
        '0'..='9' => Some(DIGIT_FONT[(ch as u8 - b'0') as usize]),
        'A' => Some([0b010, 0b101, 0b111, 0b101, 0b101]),
        'C' => Some([0b111, 0b100, 0b100, 0b100, 0b111]),
        'E' => Some([0b111, 0b100, 0b110, 0b100, 0b111]),
        'H' => Some([0b101, 0b101, 0b111, 0b101, 0b101]),
        'I' => Some([0b111, 0b010, 0b010, 0b010, 0b111]),
        'K' => Some([0b101, 0b101, 0b110, 0b101, 0b101]),
        'L' => Some([0b100, 0b100, 0b100, 0b100, 0b111]),
        'M' => Some([0b101, 0b111, 0b101, 0b101, 0b101]),
        'N' => Some([0b101, 0b110, 0b101, 0b101, 0b101]),
        'O' => Some([0b111, 0b101, 0b101, 0b101, 0b111]),
        'P' => Some([0b111, 0b101, 0b111, 0b100, 0b100]),
        'S' => Some([0b111, 0b100, 0b111, 0b001, 0b111]),
        'T' => Some([0b111, 0b010, 0b010, 0b010, 0b010]),
        'V' => Some([0b101, 0b101, 0b101, 0b101, 0b010]),
        'X' => Some([0b101, 0b101, 0b010, 0b101, 0b101]),
        'Y' => Some([0b101, 0b101, 0b010, 0b010, 0b010]),
        ' ' => Some([0b000, 0b000, 0b000, 0b000, 0b000]),
        ':' => Some([0b000, 0b010, 0b000, 0b010, 0b000]),
        _   => None,
    }
}

fn draw_char(canvas: &mut Canvas<Window>, ch: char, x: i32, y: i32, scale: i32) {
    let Some(rows) = glyph_for(ch) else { return };
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

fn draw_str(canvas: &mut Canvas<Window>, text: &str, x: i32, y: i32, scale: i32) {
    let step = 3 * scale + scale;
    for (i, ch) in text.chars().enumerate() {
        draw_char(canvas, ch.to_ascii_uppercase(), x + i as i32 * step, y, scale);
    }
}

pub fn draw_stats_overlay(canvas: &mut Canvas<Window>, stats: &Stats) {
    const SC: i32 = 4;
    const CW: i32 = 3 * SC + SC; // 16 px per glyph cell
    const LH: i32 = 30;          // line height

    canvas.set_draw_color(Color::RGB(8, 8, 20));
    canvas.clear();

    let px = 110i32;
    let py = 170i32;
    let pw = 680u32;
    let ph = 540u32;

    canvas.set_draw_color(Color::RGB(20, 20, 50));
    canvas.fill_rect(Rect::new(px, py, pw, ph)).ok();
    canvas.set_draw_color(Color::RGB(70, 70, 200));
    canvas.draw_rect(Rect::new(px, py, pw, ph)).ok();

    let lx = px + 24;
    let mut ly = py + 24;

    // Title
    canvas.set_draw_color(Color::RGB(255, 215, 0));
    draw_str(canvas, "VEHICLE STATS", lx, ly, SC);
    ly += LH + LH / 2;

    // Divider
    canvas.set_draw_color(Color::RGB(70, 70, 200));
    canvas.fill_rect(Rect::new(lx, ly, pw - 48, 2)).ok();
    ly += LH;

    // All labels are 11 chars wide; values start at this x offset
    let vx = lx + 11 * CW;

    canvas.set_draw_color(Color::RGB(80, 220, 80));
    draw_str(canvas, "VEHICLES  :", lx, ly, SC);
    draw_number(canvas, stats.total_passed, vx, ly, SC);
    ly += LH;

    canvas.set_draw_color(Color::RGB(100, 180, 255));
    draw_str(canvas, "MAX VEL   :", lx, ly, SC);
    draw_number(canvas, stats.max_velocity as u32, vx, ly, SC);
    ly += LH;

    canvas.set_draw_color(Color::RGB(100, 180, 255));
    draw_str(canvas, "MIN VEL   :", lx, ly, SC);
    let min_v = if stats.min_velocity >= f32::MAX / 2.0 { 0 } else { stats.min_velocity as u32 };
    draw_number(canvas, min_v, vx, ly, SC);
    ly += LH;

    canvas.set_draw_color(Color::RGB(255, 160, 60));
    draw_str(canvas, "MAX TIME  :", lx, ly, SC);
    draw_number(canvas, stats.max_time_ms as u32, vx, ly, SC);
    ly += LH;

    canvas.set_draw_color(Color::RGB(255, 160, 60));
    draw_str(canvas, "MIN TIME  :", lx, ly, SC);
    let min_t = if stats.min_time_ms == u64::MAX { 0 } else { stats.min_time_ms as u32 };
    draw_number(canvas, min_t, vx, ly, SC);
    ly += LH;

    canvas.set_draw_color(Color::RGB(255, 80, 80));
    draw_str(canvas, "CLOSE CALL:", lx, ly, SC);
    draw_number(canvas, stats.close_calls, vx, ly, SC);
    ly += LH * 2;

    // Divider
    canvas.set_draw_color(Color::RGB(70, 70, 200));
    canvas.fill_rect(Rect::new(lx, ly - LH / 2, pw - 48, 2)).ok();

    // Dismiss prompt
    canvas.set_draw_color(Color::RGB(150, 150, 150));
    draw_str(canvas, "ANY KEY TO CLOSE", lx + CW * 3, ly, SC);

    canvas.present();
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
        draw_dashed_vertical(canvas, x, 0, INTER_Y as i32);
        draw_dashed_vertical(canvas, x, (INTER_Y + INTER_H) as i32, WINDOW_HEIGHT as i32);
    }
    for y in [330, 390, 450, 510, 570] {
        draw_dashed_horizontal(canvas, y, 0, INTER_X as i32);
        draw_dashed_horizontal(canvas, y, (INTER_X + INTER_W) as i32, WINDOW_WIDTH as i32);
    }
    // Center guide inside intersection
    draw_dashed_vertical(canvas, 450, INTER_Y as i32, (INTER_Y + INTER_H) as i32);
    draw_dashed_horizontal(canvas, 450, INTER_X as i32, (INTER_X + INTER_W) as i32);

    // Stop lines
    canvas.fill_rect(Rect::new(INTER_X as i32, INTER_Y as i32 - 5, INTER_W as u32, 5)).ok();
    canvas.fill_rect(Rect::new(INTER_X as i32, (INTER_Y + INTER_H) as i32, INTER_W as u32, 5)).ok();
    canvas.fill_rect(Rect::new(INTER_X as i32 - 5, INTER_Y as i32, 5, INTER_H as u32)).ok();
    canvas.fill_rect(Rect::new((INTER_X + INTER_W) as i32, INTER_Y as i32, 5, INTER_H as u32)).ok();

    // 5. Vehicles — directional sprite selected by angle_deg, no rotation needed
    for vehicle in vehicles {
        let src_rect = angle_to_src_rect(vehicle.angle_deg);
        let dst_rect = Rect::new(
            vehicle.pos.x as i32 - SPRITE_W as i32 / 2,
            vehicle.pos.y as i32 - SPRITE_H as i32 / 2,
            SPRITE_W,
            SPRITE_H,
        );
        let sheet = &textures.sheets[vehicle.vehicle_type as usize];
        canvas.copy(sheet, src_rect, dst_rect).ok();
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
