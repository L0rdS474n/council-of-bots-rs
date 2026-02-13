use contrarian_bot::ContrarianBot;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use council_core::scoring::ScoreTracker;
use council_core::voting::{calculate_vote_weight, resolve_votes, Vote};
use council_core::{default_templates, generate_event};
use cycle_bot::CycleBot;
use example_bot::ExampleBot;
use first_bot::FirstBot;
use llm_bot::LlmBot;
use oracle_bot::OracleBot;
use rand::SeedableRng;

const TOTAL_ROUNDS: u32 = 25;

#[derive(Debug, Clone, Default)]
struct CliConfig {
    enable_llm_bot: bool,
    ollama_host: String,
    ollama_model: String,
}

fn parse_args() -> CliConfig {
    // Minimal, dependency-free arg parsing.
    // Example:
    //   cargo run -p council-cli -- --enable-llm-bot --ollama-host 127.0.0.1:11434 --ollama-model llama3
    let mut cfg = CliConfig {
        enable_llm_bot: false,
        ollama_host: "127.0.0.1:11434".to_string(),
        ollama_model: "llama3".to_string(),
    };

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--enable-llm-bot" => cfg.enable_llm_bot = true,
            "--ollama-host" => {
                if let Some(v) = it.next() {
                    cfg.ollama_host = v;
                }
            }
            "--ollama-model" => {
                if let Some(v) = it.next() {
                    cfg.ollama_model = v;
                }
            }
            "--help" | "-h" => {
                println!("council-cli\n\nFlags:\n  --enable-llm-bot\n  --ollama-host <host:port> (default 127.0.0.1:11434)\n  --ollama-model <model> (default llama3)\n");
                std::process::exit(0);
            }
            _ => {}
        }
    }

    cfg
}

fn main() {
    let cfg = parse_args();

    let mut bots: Vec<Box<dyn GalacticCouncilMember>> = vec![
        Box::new(ExampleBot),
        Box::new(FirstBot),
        Box::new(CycleBot),
        Box::new(ContrarianBot),
        Box::new(OracleBot),
    ];

    if cfg.enable_llm_bot {
        bots.push(Box::new(LlmBot::new(cfg.ollama_host, cfg.ollama_model)));
    }

    let templates = default_templates();
    let mut galaxy = GalaxyState::new();
    let mut score = ScoreTracker::new();
    let mut rng = rand::rngs::StdRng::from_entropy();

    print_banner();

    for round in 1..=TOTAL_ROUNDS {
        galaxy.round = round;

        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!(
            "║  ROUND {:>2} / {}                                              ║",
            round, TOTAL_ROUNDS
        );
        println!("╚══════════════════════════════════════════════════════════════╝");

        // Generate event
        let event = generate_event(&templates, &galaxy, &mut rng);
        println!();
        println!("  [EVENT] {}", event.description);
        println!();

        for (i, option) in event.options.iter().enumerate() {
            println!("    [{}] {}", i, option.description);
        }
        println!();

        // Collect votes
        let mut votes = Vec::new();
        for bot in &bots {
            let weight = calculate_vote_weight(bot.as_ref(), &event);
            let chosen = bot.vote(&event, &galaxy);
            let chosen = chosen.min(event.options.len().saturating_sub(1));
            println!(
                "    {} votes [{}] (weight: {:.2})",
                bot.name(),
                chosen,
                weight
            );
            votes.push(Vote {
                bot_name: bot.name().to_string(),
                chosen_option: chosen,
                weight,
            });
        }

        // Resolve
        let winner = resolve_votes(&votes, event.options.len());
        let outcome = &event.options[winner].outcome;

        println!();
        println!("  >> COUNCIL CHOOSES: [{}]", winner);
        println!("  >> {}", outcome.description);

        score.add(round, outcome.score_delta, &outcome.description);
        galaxy.apply_changes(&outcome.state_changes);

        if outcome.score_delta > 0 {
            println!("     +{} points", outcome.score_delta);
        } else if outcome.score_delta < 0 {
            println!("     {} points", outcome.score_delta);
        }

        // Process threats
        let threat_penalty = galaxy.process_threats();
        if threat_penalty != 0 {
            println!(
                "  !! Active threats inflict {} point penalty",
                threat_penalty
            );
            score.add(round, threat_penalty, "Unresolved threats");
        }

        // Status line
        println!();
        println!(
            "  Score: {} | Sectors: {} | Species: {} | Threats: {} | Discoveries: {}",
            score.total,
            galaxy.explored_sectors.len(),
            galaxy.known_species.len(),
            galaxy.threats.len(),
            galaxy.discoveries.len()
        );
    }

    print_final_report(&galaxy, &score, &bots);
}

fn print_banner() {
    println!();
    println!(r"     ___       __         __  _         ___                  _ __");
    println!(r"    / _ \___ _/ /__ _____/ /_(_)___    / __/___  __ _____  / / /");
    println!(r"   / ___/ _ `/ / _ `/ __/ __/ / __/   / /  / _ \/ // / _ \/ / / ");
    println!(r"  /_/   \_,_/_/\_,_/\__/\__/_/\__/   /_/   \___/\_,_/_//_/_/_/  ");
    println!();
    println!("  === GALACTIC COUNCIL EXPLORATION SIMULATION ===");
    println!(
        "  {} rounds | 5 council members | Infinite possibilities",
        TOTAL_ROUNDS
    );
    println!();
}

fn print_final_report(
    galaxy: &GalaxyState,
    score: &ScoreTracker,
    bots: &[Box<dyn GalacticCouncilMember>],
) {
    // End-game bonuses
    let mut final_score = score.total;
    let ally_bonus = galaxy.allied_count() as i32 * 10;
    let hostile_penalty = galaxy.hostile_count() as i32 * -5;
    let discovery_bonus = galaxy.discoveries.len() as i32 * 5;
    final_score += ally_bonus + hostile_penalty + discovery_bonus;

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    FINAL COUNCIL REPORT                     ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║                                                              ║");
    println!(
        "║  Sectors explored: {:>3}                                      ║",
        galaxy.explored_sectors.len()
    );
    println!(
        "║  Species known:    {:>3}                                      ║",
        galaxy.known_species.len()
    );
    println!(
        "║  Discoveries:      {:>3}                                      ║",
        galaxy.discoveries.len()
    );
    println!(
        "║  Active threats:   {:>3}                                      ║",
        galaxy.threats.len()
    );
    println!(
        "║  Allied species:   {:>3}                                      ║",
        galaxy.allied_count()
    );
    println!(
        "║  Hostile species:  {:>3}                                      ║",
        galaxy.hostile_count()
    );
    println!("║                                                              ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║  Base score:        {:>+4}                                    ║",
        score.total
    );

    if ally_bonus != 0 {
        println!(
            "║  Allied bonus:      {:>+4}                                    ║",
            ally_bonus
        );
    }
    if hostile_penalty != 0 {
        println!(
            "║  Hostile penalty:   {:>+4}                                    ║",
            hostile_penalty
        );
    }
    if discovery_bonus != 0 {
        println!(
            "║  Discovery bonus:   {:>+4}                                    ║",
            discovery_bonus
        );
    }

    println!("║                    ────                                      ║");
    println!(
        "║  FINAL SCORE:       {:>+4}                                    ║",
        final_score
    );
    println!("║                                                              ║");

    // Determine rating based on adjusted score
    let rating = match final_score {
        200.. => "Legendary Council",
        150..=199 => "Distinguished",
        100..=149 => "Competent",
        50..=99 => "Struggling",
        _ => "Dysfunctional",
    };

    println!("║  Rating: {:<20}                             ║", rating);
    println!("║                                                              ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  COUNCIL MEMBERS                                            ║");
    println!("╠══════════════════════════════════════════════════════════════╣");

    for bot in bots {
        let tags: Vec<String> = bot
            .expertise()
            .iter()
            .map(|(tag, prof)| format!("{}({:.1})", tag, prof))
            .collect();
        println!("║  {:16} {}", bot.name(), tags.join(", "));
    }

    println!("║                                                              ║");

    if let Some(best) = score.best_moment() {
        println!(
            "║  Best moment (round {}): +{} — {}",
            best.round,
            best.delta,
            truncate(&best.reason, 30)
        );
    }
    if let Some(worst) = score.worst_moment() {
        println!(
            "║  Worst moment (round {}): {} — {}",
            worst.round,
            worst.delta,
            truncate(&worst.reason, 30)
        );
    }

    println!("║                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
}

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

#[cfg(test)]
mod tests {
    use council_core::explorer::GalacticCouncilMember;
    use council_core::galaxy::GalaxyState;
    use council_core::scoring::ScoreTracker;
    use council_core::voting::{calculate_vote_weight, resolve_votes, Vote};
    use council_core::{default_templates, generate_event};
    use rand::SeedableRng;

    use contrarian_bot::ContrarianBot;
    use cycle_bot::CycleBot;
    use example_bot::ExampleBot;
    use first_bot::FirstBot;
    use oracle_bot::OracleBot;

    #[test]
    fn full_simulation_runs_deterministically() {
        let bots: Vec<Box<dyn GalacticCouncilMember>> = vec![
            Box::new(ExampleBot),
            Box::new(FirstBot),
            Box::new(CycleBot),
            Box::new(ContrarianBot),
            Box::new(OracleBot),
        ];

        let templates = default_templates();
        let mut galaxy = GalaxyState::new();
        let mut score = ScoreTracker::new();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        for round in 1..=25 {
            galaxy.round = round;
            let event = generate_event(&templates, &galaxy, &mut rng);

            let mut votes = Vec::new();
            for bot in &bots {
                let weight = calculate_vote_weight(bot.as_ref(), &event);
                let chosen = bot
                    .vote(&event, &galaxy)
                    .min(event.options.len().saturating_sub(1));
                votes.push(Vote {
                    bot_name: bot.name().to_string(),
                    chosen_option: chosen,
                    weight,
                });
            }

            let winner = resolve_votes(&votes, event.options.len());
            let outcome = &event.options[winner].outcome;
            score.add(round, outcome.score_delta, &outcome.description);
            galaxy.apply_changes(&outcome.state_changes);

            let penalty = galaxy.process_threats();
            if penalty != 0 {
                score.add(round, penalty, "Unresolved threats");
            }
        }

        // With seed 42 we should get a deterministic result
        assert!(
            score.history.len() >= 25,
            "should have at least 25 score events"
        );
    }

    #[test]
    fn same_seed_same_outcome() {
        fn run_sim(seed: u64) -> i32 {
            let bots: Vec<Box<dyn GalacticCouncilMember>> = vec![
                Box::new(ExampleBot),
                Box::new(FirstBot),
                Box::new(CycleBot),
                Box::new(ContrarianBot),
                Box::new(OracleBot),
            ];

            let templates = default_templates();
            let mut galaxy = GalaxyState::new();
            let mut score = ScoreTracker::new();
            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

            for round in 1..=25 {
                galaxy.round = round;
                let event = generate_event(&templates, &galaxy, &mut rng);

                let mut votes = Vec::new();
                for bot in &bots {
                    let weight = calculate_vote_weight(bot.as_ref(), &event);
                    let chosen = bot
                        .vote(&event, &galaxy)
                        .min(event.options.len().saturating_sub(1));
                    votes.push(Vote {
                        bot_name: bot.name().to_string(),
                        chosen_option: chosen,
                        weight,
                    });
                }

                let winner = resolve_votes(&votes, event.options.len());
                let outcome = &event.options[winner].outcome;
                score.add(round, outcome.score_delta, &outcome.description);
                galaxy.apply_changes(&outcome.state_changes);

                let penalty = galaxy.process_threats();
                if penalty != 0 {
                    score.add(round, penalty, "Unresolved threats");
                }
            }

            score.total
        }

        let a = run_sim(123);
        let b = run_sim(123);
        assert_eq!(a, b, "same seed must produce same score");
    }
}
