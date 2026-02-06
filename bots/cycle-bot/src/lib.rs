use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use council_core::{Context, CouncilMember, Decision};

/// CycleBot rotates its stance every round to encourage variety in the council.
/// The pattern is approve ➜ reject ➜ abstain.
pub struct CycleBot;

impl CouncilMember for CycleBot {
    fn name(&self) -> &'static str {
        "cycle-bot"
    }

    fn vote(&self, ctx: &Context) -> Decision {
        match ctx.round % 3 {
            1 => Decision::Approve,
            2 => Decision::Reject,
            _ => Decision::Abstain,
        }
    }
}

impl GalacticCouncilMember for CycleBot {
    fn name(&self) -> &'static str {
        "cycle-bot"
    }

    fn expertise(&self) -> &[(&'static str, f32)] {
        &[("culture", 0.7), ("linguistics", 0.5), ("archaeology", 0.3)]
    }

    /// Cycles through available options based on round number.
    fn vote(&self, event: &Event, galaxy: &GalaxyState) -> usize {
        let num = event.options.len();
        if num == 0 {
            return 0;
        }
        (galaxy.round as usize) % num
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use council_core::Context;

    #[test]
    fn cycles_through_three_decisions() {
        let bot = CycleBot;
        let rounds = [
            (1, Decision::Approve),
            (2, Decision::Reject),
            (3, Decision::Abstain),
            (4, Decision::Approve),
        ];

        for (round, expected) in rounds {
            let ctx = Context {
                round,
                previous_tally: None,
            };
            assert_eq!(CouncilMember::vote(&bot, &ctx), expected);
        }
    }
}
