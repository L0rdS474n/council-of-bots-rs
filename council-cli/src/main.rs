use council_core::{Context, CouncilMember};
use example_bot::ExampleBot;
use first_bot::FirstBot;

fn main() {
    let ctx = Context { round: 1 };

    let example_bot = ExampleBot;
    let first_bot = FirstBot;
    let council: Vec<&dyn CouncilMember> = vec![&example_bot, &first_bot];

    for bot in council {
        let decision = bot.vote(&ctx);
        println!("{} voted: {}", bot.name(), decision);
    }
}
