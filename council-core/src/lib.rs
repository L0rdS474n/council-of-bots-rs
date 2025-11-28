use std::fmt;

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

    #[test]
    fn decision_displays_human_readable_text() {
        assert_eq!(Decision::Approve.to_string(), "approve");
        assert_eq!(Decision::Reject.to_string(), "reject");
        assert_eq!(Decision::Abstain.to_string(), "abstain");
        assert_eq!(Decision::Custom("chaos").to_string(), "chaos");
    }
}
