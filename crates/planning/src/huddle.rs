//! Huddle — multiple reasoning capabilities collaborate on an intent.
//!
//! The huddle is *parallel and diverse* by design. We do not pick the
//! "best" reasoner — we let several attempt the same intent and feed every
//! proposal into the debate loop.

use organism_intent::IntentPacket;

use crate::{Plan, Reasoner};

pub struct Huddle {
    reasoners: Vec<Box<dyn Reasoner>>,
}

impl Huddle {
    pub fn new() -> Self {
        Self {
            reasoners: Vec::new(),
        }
    }

    pub fn add(mut self, r: Box<dyn Reasoner>) -> Self {
        self.reasoners.push(r);
        self
    }

    /// Run every reasoner against the intent. Failures from individual
    /// reasoners are dropped — a huddle with one survivor still proceeds.
    pub async fn run(&self, intent: &IntentPacket) -> Vec<Plan> {
        let mut out = Vec::with_capacity(self.reasoners.len());
        for r in &self.reasoners {
            if let Ok(p) = r.propose(intent).await {
                out.push(p);
            }
        }
        out
    }
}

impl Default for Huddle {
    fn default() -> Self {
        Self::new()
    }
}
