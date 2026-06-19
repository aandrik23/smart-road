use std::collections::HashMap;
#[allow(unused_imports)]
use crate::types::{
    Direction, Route, Vehicle, Vec2, VehicleState,
    INTER_X, INTER_Y, INTER_W, INTER_H,
    SPEED_FAST, SPEED_MEDIUM, SPEED_SLOW, SAFE_DISTANCE, TRIGGER_DIST,
    ACCEL_RATE, DECEL_RATE,
};
use crate::intersection::IntersectionManager;

fn is_inside_intersection(pos: Vec2) -> bool {
    (INTER_X..INTER_X + INTER_W).contains(&pos.x)
        && (INTER_Y..INTER_Y + INTER_H).contains(&pos.y)
}


fn nearest_ahead(vehicle: &Vehicle, all_vehicles: &[Vehicle]) -> f32 {
    all_vehicles
        .iter()
        .filter(|o| o.id != vehicle.id
                 && o.direction == vehicle.direction
                 && o.route == vehicle.route)
        .filter(|o| match vehicle.direction {
            Direction::South => o.pos.y < vehicle.pos.y,
            Direction::North => o.pos.y > vehicle.pos.y,
            Direction::West  => o.pos.x > vehicle.pos.x,
            Direction::East  => o.pos.x < vehicle.pos.x,
        })
        .map(|o| {
            let dx = o.pos.x - vehicle.pos.x;
            let dy = o.pos.y - vehicle.pos.y;
            (dx * dx + dy * dy).sqrt()
        })
        .fold(f32::INFINITY, f32::min)
}

pub fn update(
    vehicle:      &mut Vehicle,
    dt:           f32,
    _path_map:    &HashMap<(Direction, Route), Vec<Vec2>>, // path pre-loaded onto vehicle at spawn
    manager:      &mut IntersectionManager,
    all_vehicles: &[Vehicle],
    now_ms:       u64,
) -> bool {
    // Early-out for already-removed vehicles.
    if vehicle.state == VehicleState::Removed {
        return false;
    }

    // Velocity physics — uses previous frame's target_vel to advance velocity.
    if vehicle.velocity < vehicle.target_vel {
        vehicle.velocity += ACCEL_RATE * dt;
        vehicle.velocity = vehicle.velocity.min(vehicle.target_vel);
    } else if vehicle.velocity > vehicle.target_vel {
        vehicle.velocity -= DECEL_RATE * dt;
        vehicle.velocity = vehicle.velocity.max(vehicle.target_vel);
    }
    vehicle.velocity = vehicle.velocity.clamp(0.0, SPEED_FAST);

    // Position update
    let angle_rad = (vehicle.angle_deg as f32).to_radians();
    vehicle.pos.x += angle_rad.cos() * vehicle.velocity * dt;
    vehicle.pos.y += angle_rad.sin() * vehicle.velocity * dt;
    vehicle.distance_travelled += vehicle.velocity * dt;

    // Waypoint advance
    if vehicle.path_index >= vehicle.path.len() {
        manager.release_reservation(vehicle.id);
        vehicle.state = VehicleState::Removed;
        return false;
    }
    let target = vehicle.path[vehicle.path_index];
    let dx = target.x - vehicle.pos.x;
    let dy = target.y - vehicle.pos.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= 2.0 {
        vehicle.path_index += 1;
        if vehicle.path_index >= vehicle.path.len() {
            manager.release_reservation(vehicle.id);
            vehicle.state = VehicleState::Removed;
            return false;
        }
        let next = vehicle.path[vehicle.path_index];
        let ndx = next.x - target.x;
        let ndy = next.y - target.y;
        vehicle.angle_deg = f64::atan2(ndy as f64, ndx as f64).to_degrees();
    }

    // State machine — reads the updated position from this frame.
    match vehicle.state {
        VehicleState::Approaching => {
            let in_trigger = manager.is_in_trigger_zone(vehicle);
            let in_box     = is_inside_intersection(vehicle.pos);
            if in_box {
                // Crossed the stop line — commit the slot and transition regardless.
                if vehicle.entry_time_ms == 0 {
                    vehicle.entry_time_ms = now_ms;
                }
                let _ = manager.request_reservation(
                    vehicle.id, vehicle.direction, vehicle.route,
                );
                vehicle.target_vel = SPEED_MEDIUM;
                vehicle.state = VehicleState::InIntersection;
            } else if in_trigger {
                if vehicle.entry_time_ms == 0 {
                    vehicle.entry_time_ms = now_ms;
                }
                let granted = manager.request_reservation(
                    vehicle.id, vehicle.direction, vehicle.route,
                );
                if granted {
                    vehicle.target_vel = SPEED_MEDIUM;
                } else {
                    // Hold at stop line: set target to zero AND kinematically cap
                    // velocity so the vehicle can always halt before the box edge,
                    // even if it was already approaching at speed.
                    vehicle.target_vel = 0.0;
                    let dist_to_box = match vehicle.direction {
                        Direction::South => (vehicle.pos.y - (INTER_Y + INTER_H)).max(0.0),
                        Direction::North => (INTER_Y - vehicle.pos.y).max(0.0),
                        Direction::West  => (INTER_X - vehicle.pos.x).max(0.0),
                        Direction::East  => (vehicle.pos.x - (INTER_X + INTER_W)).max(0.0),
                    };
                    let v_limit = (2.0 * DECEL_RATE * dist_to_box.max(1.0)).sqrt();
                    if vehicle.velocity > v_limit {
                        vehicle.velocity = v_limit;
                    }
                }
            } else {
                vehicle.target_vel = SPEED_FAST;
            }
        }
        VehicleState::InIntersection => {
            vehicle.target_vel = SPEED_MEDIUM;
            if !is_inside_intersection(vehicle.pos) {
                if vehicle.exit_time_ms == 0 {
                    vehicle.exit_time_ms = now_ms;
                }
                manager.release_reservation(vehicle.id);
                vehicle.state = VehicleState::Exiting;
            }
        }
        VehicleState::Exiting => {
            vehicle.target_vel = SPEED_FAST;
        }
        VehicleState::Removed => {
            return false;
        }
    }

    // Safe-distance following: never close faster than SPEED_SLOW into the vehicle ahead.
    let gap = nearest_ahead(vehicle, all_vehicles);
    let excess = (vehicle.velocity - SPEED_SLOW).max(0.0);
    let brake_window = SAFE_DISTANCE + excess * excess / (2.0 * DECEL_RATE);
    if gap < brake_window && vehicle.target_vel > SPEED_SLOW {
        vehicle.target_vel = SPEED_SLOW;
    }

    vehicle.state != VehicleState::Removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intersection::{IntersectionManager, build_path_map};

    fn make_vehicle(id: u32, dir: Direction, route: Route) -> Vehicle {
        let path_map = build_path_map();
        let path = path_map[&(dir, route)].clone();
        let p0 = path[0];
        let p1 = path[1];
        let ndx = p1.x - p0.x;
        let ndy = p1.y - p0.y;
        Vehicle {
            id,
            direction: dir,
            route,
            state: VehicleState::Approaching,
            pos: p0,
            velocity: SPEED_FAST,
            target_vel: SPEED_FAST,
            angle_deg: f64::atan2(ndy as f64, ndx as f64).to_degrees(),
            path,
            path_index: 1,
            entry_time_ms: 0,
            exit_time_ms: 0,
            distance_travelled: 0.0,
        }
    }

    #[test]
    fn vehicle_traverses_south_straight_to_completion() {
        let path_map = build_path_map();
        let mut mgr = IntersectionManager::new();
        let mut v = make_vehicle(1, Direction::South, Route::Straight);
        let dt = 1.0_f32 / 60.0;
        let mut alive = true;
        for frame in 0..10_000u64 {
            alive = update(&mut v, dt, &path_map, &mut mgr, &[], frame);
            if !alive { break; }
        }
        assert!(!alive, "vehicle should reach Removed state within 10 000 frames");
    }

    #[test]
    fn velocity_does_not_snap() {
        let path_map = build_path_map();
        let mut mgr = IntersectionManager::new();
        let mut v = make_vehicle(1, Direction::South, Route::Straight);
        v.velocity = 0.0;
        v.target_vel = SPEED_FAST;
        update(&mut v, 1.0 / 60.0, &path_map, &mut mgr, &[], 0);
        assert!(v.velocity > 0.0,          "velocity must increase from 0");
        assert!(v.velocity < SPEED_FAST,   "velocity must not snap to target_vel in one frame");
    }

    #[test]
    fn entry_time_ms_set_at_trigger_zone_not_spawn() {
        let path_map = build_path_map();
        let mut mgr = IntersectionManager::new();
        let mut v = make_vehicle(1, Direction::South, Route::Straight);
        assert_eq!(v.entry_time_ms, 0, "entry_time_ms must be 0 at spawn");
        let dt = 1.0_f32 / 60.0;
        for frame in 1..=5_000u64 {
            update(&mut v, dt, &path_map, &mut mgr, &[], frame);
            // South trigger zone: pos.y ∈ (INTER_Y + INTER_H, INTER_Y + INTER_H + TRIGGER_DIST]
            if v.pos.y <= INTER_Y + INTER_H + TRIGGER_DIST && v.pos.y > INTER_Y + INTER_H {
                assert!(v.entry_time_ms > 0,
                    "entry_time_ms must be set on first trigger zone frame");
                let captured = v.entry_time_ms;
                update(&mut v, dt, &path_map, &mut mgr, &[], frame + 1);
                assert_eq!(v.entry_time_ms, captured,
                    "entry_time_ms must not change once set");
                return;
            }
        }
        panic!("vehicle never entered trigger zone in 5 000 frames");
    }

    #[test]
    fn reservation_released_after_intersection_exit() {
        let path_map = build_path_map();
        let mut mgr = IntersectionManager::new();
        let mut v = make_vehicle(1, Direction::South, Route::Straight);
        let dt = 1.0_f32 / 60.0;
        for frame in 0..10_000u64 {
            update(&mut v, dt, &path_map, &mut mgr, &[], frame);
            if v.state == VehicleState::Exiting {
                assert_eq!(mgr.active_count(), 0,
                    "reservation must be released when state transitions to Exiting");
                return;
            }
            if v.state == VehicleState::Removed { break; }
        }
        panic!("vehicle never reached Exiting state");
    }

    #[test]
    fn safe_distance_slows_follower() {
        let path_map = build_path_map();
        let mut mgr = IntersectionManager::new();
        // Both South vehicles on the approach road (y > 600), leader further north.
        // South travels north (y decreasing), so smaller y = further ahead.
        let mut leader   = make_vehicle(1, Direction::South, Route::Straight);
        let mut follower = make_vehicle(2, Direction::South, Route::Straight);
        leader.pos   = Vec2 { x: 360.0, y: 820.0 };
        follower.pos = Vec2 { x: 360.0, y: 820.0 + SAFE_DISTANCE * 0.5 };
        follower.target_vel = SPEED_FAST;

        let all = vec![leader.clone()];
        update(&mut follower, 1.0 / 60.0, &path_map, &mut mgr, &all, 0);

        assert_eq!(follower.target_vel, SPEED_SLOW,
            "follower must reduce to SPEED_SLOW when gap to leader is inside SAFE_DISTANCE");
    }
}
