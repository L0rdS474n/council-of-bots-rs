use std::fmt;

/// A simple counter of how many times each decision was taken in a round or
/// across the entire simulation.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct DecisionTally {
    pub approvals: u32,
    pub rejections: u32,
    pub abstentions: u32,
    pub customs: u32,
}

impl DecisionTally {
    pub fn record(&mut self, decision: &Decision) {
        match decision {
            Decision::Approve => self.approvals += 1,
            Decision::Reject => self.rejections += 1,
            Decision::Abstain => self.abstentions += 1,
            Decision::Custom(_) => self.customs += 1,
        }
    }

    pub fn merge(&mut self, other: &DecisionTally) {
        self.approvals += other.approvals;
        self.rejections += other.rejections;
        self.abstentions += other.abstentions;
        self.customs += other.customs;
    }

    pub fn describe(&self) -> String {
        format!(
            "approve: {}, reject: {}, abstain: {}, custom: {}",
            self.approvals, self.rejections, self.abstentions, self.customs
        )
    }
}

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

/// A single round's outcome, including every bot's vote and a tally of the
/// decisions made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundSummary {
    pub round: u32,
    pub votes: Vec<(&'static str, Decision)>,
    pub tally: DecisionTally,
}

/// Aggregated results for an entire simulation run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationReport {
    pub rounds: Vec<RoundSummary>,
    pub cumulative_tally: DecisionTally,
    pub bot_summaries: Vec<BotSummary>,
}

/// Tracks how a single bot behaved across a simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotSummary {
    pub name: &'static str,
    pub tally: DecisionTally,
}

/// Run a simulation for a set number of rounds and return rich reporting data.
pub fn simulate_rounds(bots: &[&dyn CouncilMember], rounds: u32) -> SimulationReport {
    let mut bot_summaries: Vec<BotSummary> = bots
        .iter()
        .map(|bot| BotSummary {
            name: bot.name(),
            tally: DecisionTally::default(),
        })
        .collect();

    let mut round_results = Vec::with_capacity(rounds as usize);
    let mut cumulative_tally = DecisionTally::default();

    for round in 1..=rounds {
        let ctx = Context { round };
        let mut tally = DecisionTally::default();
        let mut votes = Vec::with_capacity(bots.len());

        for (bot, bot_summary) in bots.iter().zip(bot_summaries.iter_mut()) {
            let decision = bot.vote(&ctx);
            tally.record(&decision);
            bot_summary.tally.record(&decision);
            votes.push((bot.name(), decision));
        }

        cumulative_tally.merge(&tally);
        round_results.push(RoundSummary {
            round,
            votes,
            tally,
        });
    }

    SimulationReport {
        rounds: round_results,
        cumulative_tally,
        bot_summaries,
    }
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

    #[test]
    fn tally_merges_counts() {
        let mut first = DecisionTally {
            approvals: 1,
            rejections: 2,
            abstentions: 0,
            customs: 1,
        };
        let second = DecisionTally {
            approvals: 2,
            rejections: 1,
            abstentions: 3,
            customs: 0,
        };

        first.merge(&second);

        assert_eq!(
            first,
            DecisionTally {
                approvals: 3,
                rejections: 3,
                abstentions: 3,
                customs: 1,
            }
        );
        assert!(first.describe().contains("approve: 3"));
    }

    #[test]
    fn simulator_collects_round_and_bot_summaries() {
        struct ApproveBot;
        struct RejectBot;

        impl CouncilMember for ApproveBot {
            fn name(&self) -> &'static str {
                "approve-bot"
            }

            fn vote(&self, _ctx: &Context) -> Decision {
                Decision::Approve
            }
        }

        impl CouncilMember for RejectBot {
            fn name(&self) -> &'static str {
                "reject-bot"
            }

            fn vote(&self, ctx: &Context) -> Decision {
                if ctx.round % 2 == 0 {
                    Decision::Reject
                } else {
                    Decision::Custom("alt")
                }
            }
        }

        let approve = ApproveBot;
        let reject = RejectBot;
        let report = simulate_rounds(&[&approve, &reject], 3);

        assert_eq!(report.rounds.len(), 3);
        assert_eq!(report.cumulative_tally.approvals, 3);
        assert_eq!(report.cumulative_tally.rejections, 1);
        assert_eq!(report.cumulative_tally.customs, 2);

        let round_two = &report.rounds[1];
        assert_eq!(round_two.round, 2);
        assert!(round_two.votes.contains(&("reject-bot", Decision::Reject)));

        let bot_names: Vec<_> = report
            .bot_summaries
            .iter()
            .map(|summary| summary.name)
            .collect();
        assert!(bot_names.contains(&"approve-bot"));
        assert!(bot_names.contains(&"reject-bot"));

        let reject_summary = report
            .bot_summaries
            .iter()
            .find(|summary| summary.name == "reject-bot")
            .unwrap();

        assert_eq!(reject_summary.tally.rejections, 1);
        assert_eq!(reject_summary.tally.customs, 2);
    }
}
