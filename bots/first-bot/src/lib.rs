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
            assert_eq!(bot.vote(&ctx), Decision::Approve);
        }
    }

    #[test]
    fn abstains_after_initial_push() {
        let bot = FirstBot;
        let ctx = Context {
            round: 4,
            previous_tally: None,
        };
        assert_eq!(bot.vote(&ctx), Decision::Abstain);
    }
}
