use council_core::{Context, CouncilMember};
use example_bot::ExampleBot;

fn main() {
    let bot = ExampleBot;
    let ctx = Context { round: 1 };
    let decision = bot.vote(&ctx);

    println!("{} voted: {}", bot.name(), decision);
}
