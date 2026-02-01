//! Expanded trait for galactic exploration bots.

use crate::event::Event;
use crate::galaxy::GalaxyState;

/// Trait for bots participating in the galactic exploration simulation.
///
/// This is the expanded interface that supports expertise-weighted voting
/// on procedurally generated events.
pub trait GalacticCouncilMember: Send + Sync {
    /// Bot's display name.
    fn name(&self) -> &'static str;

    /// Expertise tags with proficiency levels (0.0 to 1.0).
    ///
    /// Higher proficiency means the bot's vote carries more weight
    /// when an event involves that expertise domain.
    ///
    /// Example: `[("diplomacy", 0.8), ("xenobiology", 0.6)]`
    fn expertise(&self) -> &[(&'static str, f32)];

    /// Vote on an event given current galaxy state.
    ///
    /// Returns the index of the chosen response option (0-indexed).
    fn vote(&self, event: &Event, galaxy: &GalaxyState) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Outcome, ResponseOption};

    struct TestExplorer;

    impl GalacticCouncilMember for TestExplorer {
        fn name(&self) -> &'static str {
            "test-explorer"
        }

        fn expertise(&self) -> &[(&'static str, f32)] {
            &[("science", 0.9), ("exploration", 0.7)]
        }

        fn vote(&self, _event: &Event, _galaxy: &GalaxyState) -> usize {
            0
        }
    }

    #[test]
    fn explorer_has_expertise() {
        let bot = TestExplorer;
        let expertise = bot.expertise();
        assert_eq!(expertise.len(), 2);
        assert_eq!(expertise[0], ("science", 0.9));
    }

    #[test]
    fn explorer_can_vote_on_event() {
        let bot = TestExplorer;
        let event = Event {
            description: "Test event".to_string(),
            relevant_expertise: vec![],
            options: vec![
                ResponseOption {
                    description: "Option A".to_string(),
                    outcome: Outcome {
                        description: "A".to_string(),
                        score_delta: 0,
                        state_changes: vec![],
                    },
                },
                ResponseOption {
                    description: "Option B".to_string(),
                    outcome: Outcome {
                        description: "B".to_string(),
                        score_delta: 0,
                        state_changes: vec![],
                    },
                },
            ],
        };
        let galaxy = GalaxyState::new();
        let choice = bot.vote(&event, &galaxy);
        assert!(choice < event.options.len());
    }
}
