#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Route {
    Right,
    Straight,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleState {
    Approaching,
    InIntersection,
    Exiting,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speed {
    Slow,
    Medium,
    Fast,
}

#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct Vehicle {
    pub id: u32,
    pub direction: Direction,
    pub route: Route,
    pub state: VehicleState,
    pub pos: Vec2,
    pub velocity: f32,
    pub target_vel: f32,
    pub angle_deg: f64,
    pub path: Vec<Vec2>,
    pub path_index: usize,
    pub entry_time_ms: u64,
    pub exit_time_ms: u64,
    pub distance_travelled: f32,
}

#[derive(Debug, Clone)]
pub struct IntersectionSlot {
    pub reserved_by: Option<u32>,
    pub route: Option<Route>,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub total_passed: u32,
    pub max_velocity: f32,
    pub min_velocity: f32,
    pub max_time_ms: u64,
    pub min_time_ms: u64,
    pub close_calls: u32,
}

#[derive(Debug, Clone)]
pub struct InputState {
    pub random_mode: bool,
    pub quit: bool,
    pub last_spawn: [u64; 4],
}


pub const WINDOW_WIDTH: u32 = 900;
pub const WINDOW_HEIGHT: u32 = 900;
pub const LANE_WIDTH: f32 = 60.0;
pub const TILE_SIZE: u32 = 60;
pub const INTER_X: f32 = 270.0;
pub const INTER_Y: f32 = 270.0;
pub const INTER_W: f32 = 360.0;
pub const INTER_H: f32 = 360.0;
pub const SPEED_SLOW: f32 = 40.0;
pub const SPEED_MEDIUM: f32 = 100.0;
pub const SPEED_FAST: f32 = 180.0;
pub const SAFE_DISTANCE: f32 = 80.0;
pub const CLOSE_CALL_DIST: f32 = 30.0;
pub const SPAWN_INTERVAL_MS: u64 = 800;
pub const ACCEL_RATE:         f32 = 150.0;
pub const DECEL_RATE:         f32 = 120.0;
pub const TRIGGER_DIST:       f32 = 200.0;
pub const SPRITE_W:           u32 = 20;
pub const SPRITE_H:           u32 = 12;
pub const FRAME_COUNT:        u32 = 1;
pub const FRAME_STRIDE:       u32 = 1;

// CLOSE_CALL_DIST (30.0) < SAFE_DISTANCE (80.0) — violation threshold is smaller than safe gap
const _: () = assert!(
    (CLOSE_CALL_DIST as u32) < (SAFE_DISTANCE as u32),
    "CLOSE_CALL_DIST must be less than SAFE_DISTANCE"
);
