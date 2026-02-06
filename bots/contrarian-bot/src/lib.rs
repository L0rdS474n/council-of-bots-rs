use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use council_core::{Context, CouncilMember, Decision, DominantOutcome};

/// ContrarianBot reacts to the council's previous round by opposing the majority.
pub struct ContrarianBot;

impl CouncilMember for ContrarianBot {
    fn name(&self) -> &'static str {
        "contrarian-bot"
    }

    fn vote(&self, ctx: &Context) -> Decision {
        match ctx.previous_tally {
            None => Decision::Abstain,
            Some(tally) => match tally.dominant() {
                DominantOutcome::Approve => Decision::Reject,
                DominantOutcome::Reject => Decision::Approve,
                DominantOutcome::Abstain => Decision::Custom("wildcard"),
                DominantOutcome::Custom => Decision::Reject,
                DominantOutcome::Tie => Decision::Abstain,
            },
        }
    }
}

impl GalacticCouncilMember for ContrarianBot {
    fn name(&self) -> &'static str {
        "contrarian-bot"
    }

    fn expertise(&self) -> &[(&'static str, f32)] {
        &[("military", 0.8), ("strategy", 0.6)]
    }

    /// Always picks the last available option — the contrarian choice.
    /// Most event templates put the cautious/avoidant option last, so
    /// contrarian-bot creates tension by consistently going against the grain.
    fn vote(&self, event: &Event, _galaxy: &GalaxyState) -> usize {
        event.options.len().saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use council_core::RoundTally;

    fn context_with_tally(tally: RoundTally) -> Context {
        Context {
            round: 2,
            previous_tally: Some(tally),
        }
    }

    #[test]
    fn abstains_on_first_round() {
        let bot = ContrarianBot;
        let ctx = Context {
            round: 1,
            previous_tally: None,
        };
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Abstain);
    }

    #[test]
    fn opposes_approval_majority() {
        let bot = ContrarianBot;
        let ctx = context_with_tally(RoundTally {
            approvals: 3,
            rejections: 1,
            abstentions: 0,
            customs: 0,
        });
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Reject);
    }

    #[test]
    fn opposes_rejection_majority() {
        let bot = ContrarianBot;
        let ctx = context_with_tally(RoundTally {
            approvals: 0,
            rejections: 4,
            abstentions: 1,
            customs: 0,
        });
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Approve);
    }

    #[test]
    fn disrupts_abstention_majority() {
        let bot = ContrarianBot;
        let ctx = context_with_tally(RoundTally {
            approvals: 0,
            rejections: 1,
            abstentions: 5,
            customs: 0,
        });
        assert_eq!(
            CouncilMember::vote(&bot, &ctx),
            Decision::Custom("wildcard")
        );
    }

    #[test]
    fn counters_custom_majority() {
        let bot = ContrarianBot;
        let ctx = context_with_tally(RoundTally {
            approvals: 1,
            rejections: 1,
            abstentions: 0,
            customs: 4,
        });
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Reject);
    }

    #[test]
    fn abstains_on_ties() {
        let bot = ContrarianBot;
        let ctx = context_with_tally(RoundTally {
            approvals: 2,
            rejections: 2,
            abstentions: 0,
            customs: 0,
        });
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Abstain);
    }
}
