use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use council_core::{Context, CouncilMember, Decision};

/// FirstBot takes a simple optimistic stance: it approves early rounds
/// to build momentum, but abstains once the council has had a few turns
/// to speak.
pub struct FirstBot;

impl CouncilMember for FirstBot {
    fn name(&self) -> &'static str {
        "first-bot"
    }

    fn vote(&self, ctx: &Context) -> Decision {
        if ctx.round <= 3 {
            Decision::Approve
        } else {
            Decision::Abstain
        }
    }
}

impl GalacticCouncilMember for FirstBot {
    fn name(&self) -> &'static str {
        "first-bot"
    }

    fn expertise(&self) -> &[(&'static str, f32)] {
        &[("exploration", 0.8), ("science", 0.5)]
    }

    /// Optimistic explorer: always picks the boldest option (index 0) in the
    /// first 10 rounds, then switches to cautious (last option) later.
    fn vote(&self, event: &Event, galaxy: &GalaxyState) -> usize {
        if galaxy.round <= 10 {
            0
        } else {
            event.options.len().saturating_sub(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use council_core::Context;

    #[test]
    fn approves_initial_rounds() {
        let bot = FirstBot;
        for round in 1..=3 {
            let ctx = Context {
                round,
                previous_tally: None,
            };
            assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Approve);
        }
    }

    #[test]
    fn abstains_after_initial_push() {
        let bot = FirstBot;
        let ctx = Context {
            round: 4,
            previous_tally: None,
        };
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Abstain);
    }
}
