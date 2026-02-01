use std::fmt;

// ============================================================================
// Galactic Exploration Modules (new simulation system)
// ============================================================================

pub mod event;
pub mod explorer;
pub mod galaxy;
pub mod scoring;
pub mod templates;
pub mod voting;

// Re-export commonly used types for convenience
pub use event::{Event, EventTemplate, Outcome, ResponseOption};
pub use explorer::GalacticCouncilMember;
pub use galaxy::{
    Discovery, GalaxyState, Relation, Sector, SectorType, Species, StateChange, Threat,
};
pub use scoring::{ScoreEvent, ScoreTracker};
pub use templates::{default_templates, generate_event};
pub use voting::{calculate_vote_weight, resolve_votes, Vote, BASE_WEIGHT};

// ============================================================================
// Legacy Simple Voting System (for backward compatibility)
// ============================================================================

/// Shared simulation context passed to all council members.
pub struct Context {
    pub round: u32,
    pub previous_tally: Option<RoundTally>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RoundTally {
    pub approvals: u32,
    pub rejections: u32,
    pub abstentions: u32,
    pub customs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DominantOutcome {
    Approve,
    Reject,
    Abstain,
    Custom,
    Tie,
}

impl RoundTally {
    pub fn record(&mut self, decision: &Decision) {
        match decision {
            Decision::Approve => self.approvals += 1,
            Decision::Reject => self.rejections += 1,
            Decision::Abstain => self.abstentions += 1,
            Decision::Custom(_) => self.customs += 1,
        }
    }

    pub fn describe(&self) -> String {
        format!(
            "approve: {}, reject: {}, abstain: {}, custom: {}",
            self.approvals, self.rejections, self.abstentions, self.customs
        )
    }

    pub fn dominant(&self) -> DominantOutcome {
        let values = [
            (self.approvals, DominantOutcome::Approve),
            (self.rejections, DominantOutcome::Reject),
            (self.abstentions, DominantOutcome::Abstain),
            (self.customs, DominantOutcome::Custom),
        ];
        let max_value = values.iter().map(|(count, _)| *count).max().unwrap_or(0);
        if max_value == 0 {
            return DominantOutcome::Tie;
        }
        let mut winner = DominantOutcome::Tie;
        let mut winner_count = 0;
        for (count, outcome) in values {
            if count == max_value {
                winner = outcome;
                winner_count += 1;
            }
        }
        if winner_count == 1 {
            winner
        } else {
            DominantOutcome::Tie
        }
    }
}

/// A decision that a council member can make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Approve,
    Reject,
    Abstain,
    Custom(&'static str),
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Decision::Approve => write!(f, "approve"),
            Decision::Reject => write!(f, "reject"),
            Decision::Abstain => write!(f, "abstain"),
            Decision::Custom(label) => write!(f, "{}", label),
        }
    }
}

/// Core trait that all bots must implement (legacy simple voting).
pub trait CouncilMember {
    fn name(&self) -> &'static str;
    fn vote(&self, ctx: &Context) -> Decision;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestBot;

    impl CouncilMember for TestBot {
        fn name(&self) -> &'static str {
            "test-bot"
        }

        fn vote(&self, _ctx: &Context) -> Decision {
            Decision::Approve
        }
    }

    #[test]
    fn test_bot_votes_approve() {
        let bot = TestBot;
        let ctx = Context {
            round: 1,
            previous_tally: None,
        };
        assert!(matches!(bot.vote(&ctx), Decision::Approve));
    }

    #[test]
    fn decision_displays_human_readable_text() {
        assert_eq!(Decision::Approve.to_string(), "approve");
        assert_eq!(Decision::Reject.to_string(), "reject");
        assert_eq!(Decision::Abstain.to_string(), "abstain");
        assert_eq!(Decision::Custom("chaos").to_string(), "chaos");
    }

    #[test]
    fn dominant_outcome_resolves_ties() {
        let tally = RoundTally {
            approvals: 2,
            rejections: 2,
            ..RoundTally::default()
        };
        assert_eq!(tally.dominant(), DominantOutcome::Tie);
    }

    #[test]
    fn dominant_outcome_picks_single_winner() {
        let tally = RoundTally {
            customs: 3,
            rejections: 1,
            ..RoundTally::default()
        };
        assert_eq!(tally.dominant(), DominantOutcome::Custom);
    }
}
