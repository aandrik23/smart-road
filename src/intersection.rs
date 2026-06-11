use crate::types::{Direction, Route};

pub struct IntersectionManager;

impl IntersectionManager {
    pub fn new() -> Self {
        Self
    }

    pub fn request_reservation(&mut self, _id: u32, _dir: Direction, _route: Route) -> bool {
        todo!("B1: reservation grant/deny with conflict table")
    }

    pub fn release_reservation(&mut self, _id: u32) {
        todo!("B1: release after full intersection clearance")
    }

    pub fn is_in_trigger_zone(&self, _distance_to_intersection: f32) -> bool {
        todo!("B1: 200 px trigger zone check")
    }
}
