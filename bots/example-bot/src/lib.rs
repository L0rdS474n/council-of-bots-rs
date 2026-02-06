use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use council_core::{Context, CouncilMember, Decision};

/// A simple example bot that flips decision based on round parity.
pub struct ExampleBot;

impl CouncilMember for ExampleBot {
    fn name(&self) -> &'static str {
        "example-bot"
    }

    fn vote(&self, ctx: &Context) -> Decision {
        if ctx.round.is_multiple_of(2) {
            Decision::Approve
        } else {
            Decision::Reject
        }
    }
}

impl GalacticCouncilMember for ExampleBot {
    fn name(&self) -> &'static str {
        "example-bot"
    }

    fn expertise(&self) -> &[(&'static str, f32)] {
        &[("engineering", 0.6), ("science", 0.4)]
    }

    /// Alternates between first and second option each round.
    fn vote(&self, event: &Event, galaxy: &GalaxyState) -> usize {
        let pick = if galaxy.round.is_multiple_of(2) { 0 } else { 1 };
        pick.min(event.options.len().saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use council_core::Context;

    #[test]
    fn example_bot_votes_deterministically() {
        let bot = ExampleBot;
        let ctx1 = Context {
            round: 1,
            previous_tally: None,
        };
        let ctx2 = Context {
            round: 2,
            previous_tally: None,
        };

        assert!(matches!(CouncilMember::vote(&bot, &ctx1), Decision::Reject));
        assert!(matches!(
            CouncilMember::vote(&bot, &ctx2),
            Decision::Approve
        ));
    }
}
