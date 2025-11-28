/// Shared simulation context passed to all council members.
pub struct Context {
    pub round: u32,
}

/// A decision that a council member can make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Approve,
    Reject,
    Abstain,
    Custom(&'static str),
}

/// Core trait that all bots must implement.
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
        let ctx = Context { round: 1 };
        assert!(matches!(bot.vote(&ctx), Decision::Approve));
    }
}
