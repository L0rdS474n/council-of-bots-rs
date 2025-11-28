use council_core::{simulate_rounds, CouncilMember, DecisionTally};
use cycle_bot::CycleBot;
use example_bot::ExampleBot;
use first_bot::FirstBot;

fn main() {
    let council: Vec<Box<dyn CouncilMember>> =
        vec![Box::new(ExampleBot), Box::new(FirstBot), Box::new(CycleBot)];
    let borrowed_bots: Vec<&dyn CouncilMember> = council.iter().map(|bot| bot.as_ref()).collect();

    let report = simulate_rounds(&borrowed_bots, 5);

    for round in &report.rounds {
        println!("\n-- Round {} --", round.round);
        for (name, decision) in &round.votes {
            println!("{} voted: {}", name, decision);
        }
        println!("Round {} tally: {}", round.round, round.tally.describe());
    }

    println!("\n== Summary after {} rounds ==", report.rounds.len());
    println!("Cumulative: {}", report.cumulative_tally.describe());
    for bot in &report.bot_summaries {
        println!("{} totals -> {}", bot.name, format_bot_totals(&bot.tally));
    }
}

fn format_bot_totals(tally: &DecisionTally) -> String {
    format!(
        "approve: {}, reject: {}, abstain: {}, custom: {}",
        tally.approvals, tally.rejections, tally.abstentions, tally.customs
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use council_core::Decision;

    #[test]
    fn tally_counts_all_decisions() {
        let mut tally = DecisionTally::default();
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
            DecisionTally {
                approvals: 1,
                rejections: 1,
                abstentions: 1,
                customs: 1,
            }
        );
        assert!(tally.describe().contains("approve: 1"));
        assert!(tally.describe().contains("custom: 1"));
    }

    #[test]
    fn simulates_rounds_and_builds_summary() {
        let council: Vec<Box<dyn CouncilMember>> =
            vec![Box::new(ExampleBot), Box::new(FirstBot), Box::new(CycleBot)];
        let borrowed_bots: Vec<&dyn CouncilMember> =
            council.iter().map(|bot| bot.as_ref()).collect();

        let report = simulate_rounds(&borrowed_bots, 2);

        assert_eq!(report.rounds.len(), 2);
        assert_eq!(report.cumulative_tally.approvals, 4);

        let example_bot_summary = report
            .bot_summaries
            .iter()
            .find(|summary| summary.name == "example-bot")
            .unwrap();
        assert_eq!(example_bot_summary.tally.approvals, 1);
        assert_eq!(example_bot_summary.tally.rejections, 1);
    }
}
