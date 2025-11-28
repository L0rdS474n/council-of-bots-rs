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
            let ctx = Context { round };
            assert_eq!(bot.vote(&ctx), expected);
        }
    }
}
