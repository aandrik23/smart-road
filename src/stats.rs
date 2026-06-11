use crate::types::Stats;

pub fn record_passed(_stats: &mut Stats) {
    todo!("X1: increment total_passed")
}

pub fn record_velocity(_stats: &mut Stats, _v: f32) {
    todo!("X1: update max/min velocity (only while vehicle is in motion)")
}

pub fn record_transit(_stats: &mut Stats, _entry_ms: u64, _exit_ms: u64) {
    todo!("X1: compute duration, update max/min transit time")
}
