//! Event system for the galactic exploration simulation.

use crate::galaxy::{GalaxyState, StateChange};

/// An event the council must respond to.
#[derive(Debug, Clone)]
pub struct Event {
    /// Description of what is happening.
    pub description: String,
    /// Which expertise domains are relevant, with weights (0.0-1.0).
    pub relevant_expertise: Vec<(String, f32)>,
    /// Available response options.
    pub options: Vec<ResponseOption>,
}

/// A possible response to an event.
#[derive(Debug, Clone)]
pub struct ResponseOption {
    /// Description of this choice.
    pub description: String,
    /// What happens if this option wins.
    pub outcome: Outcome,
}

/// The result of choosing a response option.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// Narrative description of what happened.
    pub description: String,
    /// Points gained or lost.
    pub score_delta: i32,
    /// Changes to galaxy state.
    pub state_changes: Vec<StateChange>,
}

/// Trait for event templates that generate concrete events.
pub trait EventTemplate: Send + Sync {
    /// Name of this template for debugging.
    fn name(&self) -> &'static str;

    /// Can this template generate an event given current state?
    fn is_applicable(&self, galaxy: &GalaxyState) -> bool;

    /// Relative weight for selection (higher = more likely when applicable).
    fn weight(&self) -> u32 {
        10
    }

    /// Generate a concrete event from this template.
    fn generate(&self, galaxy: &GalaxyState, rng: &mut dyn RngCore) -> Event;
}

/// Re-export for templates to use.
pub use rand::RngCore;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_can_have_multiple_expertise_tags() {
        let event = Event {
            description: "Test event".to_string(),
            relevant_expertise: vec![("science".to_string(), 0.5), ("diplomacy".to_string(), 0.3)],
            options: vec![],
        };
        assert_eq!(event.relevant_expertise.len(), 2);
    }

    #[test]
    fn outcome_can_have_state_changes() {
        use crate::galaxy::{Sector, SectorType};

        let outcome = Outcome {
            description: "Discovered new sector".to_string(),
            score_delta: 10,
            state_changes: vec![StateChange::AddSector(Sector {
                name: "New Sector".to_string(),
                sector_type: SectorType::Nebula,
            })],
        };
        assert_eq!(outcome.score_delta, 10);
        assert_eq!(outcome.state_changes.len(), 1);
    }
}
