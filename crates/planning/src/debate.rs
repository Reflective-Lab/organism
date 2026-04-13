//! Debate loop.
//!
//! Planner proposes → adversaries attack → planner revises → second critique
//! → final bundle. The debate loop is the seam where the `adversarial` crate
//! plugs in.

use crate::Plan;

/// A bundle of plans surviving the debate loop, ready for the simulation swarm.
#[derive(Debug, Clone)]
pub struct PlanBundle {
    pub candidates: Vec<Plan>,
    pub round: u32,
}

impl PlanBundle {
    pub fn new(candidates: Vec<Plan>) -> Self {
        Self {
            candidates,
            round: 0,
        }
    }
}
