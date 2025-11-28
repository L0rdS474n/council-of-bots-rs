use council_core::{Context, CouncilMember, Decision};
use cycle_bot::CycleBot;
use example_bot::ExampleBot;
use first_bot::FirstBot;

#[derive(Default, Debug, PartialEq, Eq)]
struct RoundTally {
    approvals: u32,
    rejections: u32,
    abstentions: u32,
    customs: u32,
}

impl RoundTally {
    fn record(&mut self, decision: &Decision) {
        match decision {
            Decision::Approve => self.approvals += 1,
            Decision::Reject => self.rejections += 1,
            Decision::Abstain => self.abstentions += 1,
            Decision::Custom(_) => self.customs += 1,
        }
    }

    fn describe(&self) -> String {
        format!(
            "approve: {}, reject: {}, abstain: {}, custom: {}",
            self.approvals, self.rejections, self.abstentions, self.customs
        )
    }
}

fn main() {
    let council: Vec<Box<dyn CouncilMember>> =
        vec![Box::new(ExampleBot), Box::new(FirstBot), Box::new(CycleBot)];

    for round in 1..=5 {
        let ctx = Context { round };
        let mut tally = RoundTally::default();

        println!("\n-- Round {} --", round);
        for bot in &council {
            let decision = bot.vote(&ctx);
            tally.record(&decision);
            println!("{} voted: {}", bot.name(), decision);
        }

        println!("Round {} tally: {}", round, tally.describe());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tally_counts_all_decisions() {
        let mut tally = RoundTally::default();
        let decisions = [
            Decision::Approve,
            Decision::Reject,
            Decision::Abstain,
            Decision::Custom("wildcard"),
        ];

        for decision in &decisions {
            tally.record(decision);
        }

        assert_eq!(
            tally,
            RoundTally {
                approvals: 1,
                rejections: 1,
                abstentions: 1,
                customs: 1,
            }
        );
        assert!(tally.describe().contains("approve: 1"));
        assert!(tally.describe().contains("custom: 1"));
    }
}
