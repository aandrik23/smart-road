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

pub fn record_close_calls(stats: &mut Stats, vehicles: &[Vehicle]) {
    for i in 0..vehicles.len() {
        for j in (i + 1)..vehicles.len() {
            let dx = vehicles[i].pos.x - vehicles[j].pos.x;
            let dy = vehicles[i].pos.y - vehicles[j].pos.y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > 0.0 && dist < CLOSE_CALL_DIST {
                stats.close_calls += 1;
            }
        }
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