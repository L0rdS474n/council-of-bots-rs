use contrarian_bot::ContrarianBot;
use council_core::{Context, CouncilMember, RoundTally};
use cycle_bot::CycleBot;
use example_bot::ExampleBot;
use first_bot::FirstBot;

fn main() {
    let council: Vec<Box<dyn CouncilMember>> = vec![
        Box::new(ExampleBot),
        Box::new(FirstBot),
        Box::new(CycleBot),
        Box::new(ContrarianBot),
    ];

    let mut previous_tally: Option<RoundTally> = None;
    for round in 1..=5 {
        let ctx = Context {
            round,
            previous_tally,
        };
        let mut tally = RoundTally::default();

        println!("\n-- Round {} --", round);
        for bot in &council {
            let decision = bot.vote(&ctx);
            tally.record(&decision);
            println!("{} voted: {}", bot.name(), decision);
        }

        println!("Round {} tally: {}", round, tally.describe());
        previous_tally = Some(tally);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use council_core::Decision;

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
