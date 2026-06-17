use std::collections::HashSet;
use crate::types::{Stats, Vehicle, CLOSE_CALL_DIST};

pub fn record_passed(stats: &mut Stats) {
    stats.total_passed += 1;
}

pub fn record_velocity(stats: &mut Stats, v: f32) {
    if v <= 0.0 {
        return;
    }

    if v > stats.max_velocity {
        stats.max_velocity = v;
    }

    if v < stats.min_velocity {
        stats.min_velocity = v;
    }
}

pub fn record_transit(stats: &mut Stats, entry_ms: u64, exit_ms: u64) {
    if entry_ms == 0 || exit_ms <= entry_ms {
        return;
    }

    let duration = exit_ms - entry_ms;

    if duration > stats.max_time_ms {
        stats.max_time_ms = duration;
    }

    if duration < stats.min_time_ms {
        stats.min_time_ms = duration;
    }
}

pub fn record_close_calls(
    stats: &mut Stats,
    vehicles: &[Vehicle],
    active_pairs: &mut HashSet<(u32, u32)>,
) {
    let mut current: HashSet<(u32, u32)> = HashSet::new();

    for i in 0..vehicles.len() {
        for j in (i + 1)..vehicles.len() {
            let dx = vehicles[i].pos.x - vehicles[j].pos.x;
            let dy = vehicles[i].pos.y - vehicles[j].pos.y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > 0.0 && dist < CLOSE_CALL_DIST {
                let a = vehicles[i].id.min(vehicles[j].id);
                let b = vehicles[i].id.max(vehicles[j].id);
                current.insert((a, b));
            }
        }
    }

    for pair in &current {
        if !active_pairs.contains(pair) {
            stats.close_calls += 1;
        }
    }

    *active_pairs = current;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Direction, Route, Vec2, VehicleState};

    fn make_vehicle(id: u32, x: f32, y: f32) -> Vehicle {
        Vehicle {
            id,
            direction: Direction::South,
            route: Route::Straight,
            state: VehicleState::Approaching,
            pos: Vec2 { x, y },
            velocity: 0.0,
            target_vel: 0.0,
            angle_deg: 0.0,
            path: vec![],
            path_index: 0,
            entry_time_ms: 0,
            exit_time_ms: 0,
            distance_travelled: 0.0,
        }
    }

    fn empty_stats() -> Stats {
        Stats {
            total_passed: 0,
            max_velocity: 0.0,
            min_velocity: f32::MAX,
            max_time_ms: 0,
            min_time_ms: u64::MAX,
            close_calls: 0,
        }
    }

    #[test]
    fn close_call_counted_once_not_per_frame() {
        let mut stats = empty_stats();
        let mut pairs: HashSet<(u32, u32)> = HashSet::new();
        let vehicles = vec![
            make_vehicle(1, 0.0, 0.0),
            make_vehicle(2, CLOSE_CALL_DIST * 0.5, 0.0),
        ];
        for _ in 0..5 {
            record_close_calls(&mut stats, &vehicles, &mut pairs);
        }
        assert_eq!(stats.close_calls, 1, "same pair in range for 5 frames = 1 event");
    }

    #[test]
    fn close_call_resets_when_vehicles_separate() {
        let mut stats = empty_stats();
        let mut pairs: HashSet<(u32, u32)> = HashSet::new();
        let close = vec![
            make_vehicle(1, 0.0, 0.0),
            make_vehicle(2, CLOSE_CALL_DIST * 0.5, 0.0),
        ];
        let far = vec![
            make_vehicle(1, 0.0, 0.0),
            make_vehicle(2, CLOSE_CALL_DIST * 2.0, 0.0),
        ];
        record_close_calls(&mut stats, &close, &mut pairs); // enter: +1
        record_close_calls(&mut stats, &far,   &mut pairs); // exit
        record_close_calls(&mut stats, &close, &mut pairs); // re-enter: +1
        assert_eq!(stats.close_calls, 2, "pair that separates and re-enters = 2 events");
    }
}

pub fn print_final(stats: &Stats) {
    println!();
    println!("========== SIMULATION STATS ==========");
    println!("Vehicles passed: {}", stats.total_passed);
    println!("Max velocity: {:.2}", stats.max_velocity);

    if stats.min_velocity == f32::MAX {
        println!("Min velocity: 0.00");
    } else {
        println!("Min velocity: {:.2}", stats.min_velocity);
    }

    println!("Max time: {} ms", stats.max_time_ms);

    if stats.min_time_ms == u64::MAX {
        println!("Min time: 0 ms");
    } else {
        println!("Min time: {} ms", stats.min_time_ms);
    }

    println!("Close calls: {}", stats.close_calls);
    println!("======================================");
}