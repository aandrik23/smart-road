pub const WINDOW_WIDTH:      u32 = 900;
pub const WINDOW_HEIGHT:     u32 = 900;
pub const LANE_WIDTH:        f32 = 60.0;
pub const TILE_SIZE:         u32 = 60;

pub const INTER_X: f32 = 300.0;
pub const INTER_Y: f32 = 300.0;
pub const INTER_W: f32 = 300.0;
pub const INTER_H: f32 = 300.0;

pub const SPEED_SLOW:   f32 = 40.0;
pub const SPEED_MEDIUM: f32 = 100.0;
pub const SPEED_FAST:   f32 = 180.0;

pub const SAFE_DISTANCE:     f32 = 80.0;
pub const CLOSE_CALL_DIST:   f32 = 30.0;
pub const SPAWN_INTERVAL_MS: u64 = 800;
pub const TRIGGER_DIST:      f32 = 200.0;
pub const TRANSIT_LENGTH:    f32 = 300.0;
pub const ACCEL_RATE:        f32 = 60.0;
pub const DECEL_RATE:        f32 = 120.0;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VehicleState {
    Approaching,
    InIntersection,
    Exiting,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Speed {
    Slow,
    Medium,
    Fast,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct Vehicle {
    pub id:                 u32,
    pub direction:          Direction,
    pub route:              Route,
    pub state:              VehicleState,
    pub pos:                Vec2,
    pub velocity:           f32,
    pub target_vel:         f32,
    pub angle_deg:          f32,
    pub path:               Vec<Vec2>,
    pub path_index:         usize,
    pub entry_time_ms:      u64,
    pub exit_time_ms:       u64,
    pub distance_travelled: f32,
}

#[derive(Debug, Clone, Default)]
pub struct IntersectionSlot {
    pub reserved_by:        Option<u32>,
    pub route:              Option<Route>,
    pub scheduled_entry_ms: u64,
    pub scheduled_exit_ms:  u64,
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub total_passed: u32,
    pub max_velocity: f32,
    pub min_velocity: f32,
    pub max_time_ms:  u64,
    pub min_time_ms:  u64,
    pub close_calls:  u32,
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            total_passed: 0,
            max_velocity: 0.0,
            min_velocity: f32::MAX,
            max_time_ms:  0,
            min_time_ms:  u64::MAX,
            close_calls:  0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_call_less_than_safe_distance() {
        assert!(CLOSE_CALL_DIST < SAFE_DISTANCE);
    }

    #[test]
    fn speed_slow_is_positive() {
        assert!(SPEED_SLOW > 0.0);
    }

    #[test]
    fn safe_distance_is_positive() {
        assert!(SAFE_DISTANCE > 0.0);
    }
}
