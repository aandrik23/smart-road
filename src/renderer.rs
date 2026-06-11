use sdl2::render::Canvas;
use sdl2::video::Window;
use crate::types::{Vehicle, Stats};
use crate::intersection::IntersectionManager;

pub fn draw(
    _canvas: &mut Canvas<Window>,
    _vehicles: &[Vehicle],
    _manager: &IntersectionManager,
    _stats: &Stats,
) {
    todo!("C1: road background, lane markings, vehicles with rotation, HUD")
}
