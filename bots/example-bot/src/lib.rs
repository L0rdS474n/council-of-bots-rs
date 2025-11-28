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

#[cfg(test)]
mod tests {
    use super::*;
    use council_core::Context;

    #[test]
    fn example_bot_votes_deterministically() {
        let bot = ExampleBot;
        let ctx1 = Context { round: 1 };
        let ctx2 = Context { round: 2 };

        assert!(matches!(bot.vote(&ctx1), Decision::Reject));
        assert!(matches!(bot.vote(&ctx2), Decision::Approve));
    }
}
