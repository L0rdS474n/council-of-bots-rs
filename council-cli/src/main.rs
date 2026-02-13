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

const DEFAULT_ROUNDS: u32 = 25;

#[derive(Debug, Clone, Default)]
struct CliConfig {
    rounds: u32,
    enable_llm_bot: bool,
    ollama_host: String,
    ollama_model: String,
    spawn_ollama: bool,
    ollama_bin: String,
}

fn parse_args() -> CliConfig {
    // Minimal, dependency-free arg parsing.
    // Example:
    //   cargo run -p council-cli -- --enable-llm-bot --spawn-ollama --ollama-host 127.0.0.1:11434 --ollama-model llama3
    let mut cfg = CliConfig {
        rounds: DEFAULT_ROUNDS,
        enable_llm_bot: false,
        ollama_host: "127.0.0.1:11434".to_string(),
        ollama_model: "llama3".to_string(),
        spawn_ollama: false,
        ollama_bin: "ollama".to_string(),
    };

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--rounds" => {
                let Some(v) = it.next() else {
                    eprintln!("--rounds requires a number");
                    std::process::exit(2);
                };
                let rounds = v.parse::<u32>().unwrap_or(0);
                if rounds == 0 {
                    eprintln!("--rounds must be >= 1");
                    std::process::exit(2);
                }
                cfg.rounds = rounds;
            }
            "--enable-llm-bot" => cfg.enable_llm_bot = true,
            "--spawn-ollama" => cfg.spawn_ollama = true,
            "--ollama-bin" => {
                if let Some(v) = it.next() {
                    cfg.ollama_bin = v;
                }
            }
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
                println!(
                    "council-cli\n\nFlags:\n  --rounds <n> (default 25)\n  --enable-llm-bot\n  --spawn-ollama (start/stop ollama automatically for this run)\n  --ollama-bin <path> (default ollama)\n  --ollama-host <host:port> (default 127.0.0.1:11434)\n  --ollama-model <model> (default llama3)\n"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }

    cfg
}

fn parse_host(host: &str) -> (&str, u16) {
    let h = host.strip_prefix("http://").unwrap_or(host);
    let mut parts = h.split(':');
    let hostname = parts.next().unwrap_or("127.0.0.1");
    let port = parts
        .next()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(11434);
    (hostname, port)
}

fn can_connect(host: &str) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let (h, p) = parse_host(host);
    let addr = (h, p).to_socket_addrs().ok().and_then(|mut a| a.next());

    match addr {
        Some(a) => TcpStream::connect_timeout(&a, Duration::from_millis(300)).is_ok(),
        None => false,
    }
}

struct OllamaGuard {
    child: std::process::Child,
}

impl Drop for OllamaGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn maybe_spawn_ollama(cfg: &CliConfig) -> Option<OllamaGuard> {
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    if !cfg.spawn_ollama {
        return None;
    }

    if can_connect(&cfg.ollama_host) {
        return None;
    }

    let (h, p) = parse_host(&cfg.ollama_host);
    let ollama_host_env = format!("{}:{}", h, p);

    let mut child = Command::new(&cfg.ollama_bin)
        .arg("serve")
        .env("OLLAMA_HOST", ollama_host_env)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(8) {
        if can_connect(&cfg.ollama_host) {
            return Some(OllamaGuard { child });
        }
        thread::sleep(Duration::from_millis(200));
    }

    let _ = child.kill();
    let _ = child.wait();
    None
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

    let _ollama_guard = maybe_spawn_ollama(&cfg);

    if cfg.enable_llm_bot {
        if !can_connect(&cfg.ollama_host) {
            eprintln!(
                "llm-bot enabled but Ollama is not reachable at {}.\n\
                 - If you want council-cli to manage Ollama automatically: add --spawn-ollama\n\
                 - Otherwise start it yourself (e.g. `ollama serve`) and ensure the model exists.\n\
                 - You can change the path with --ollama-bin and endpoint with --ollama-host",
                cfg.ollama_host
            );
            std::process::exit(2);
        }

        bots.push(Box::new(LlmBot::new(cfg.ollama_host, cfg.ollama_model)));
    }

    let templates = default_templates();
    let mut galaxy = GalaxyState::new();
    let mut score = ScoreTracker::new();
    let mut rng = rand::rngs::StdRng::from_entropy();

    print_banner(cfg.rounds, bots.len() as u32);

    for round in 1..=cfg.rounds {
        galaxy.round = round;

        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!(
            "║  ROUND {:>2} / {}                                              ║",
            round, cfg.rounds
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

fn print_banner(rounds: u32, members: u32) {
    println!();
    println!(r"     ___       __         __  _         ___                  _ __");
    println!(r"    / _ \___ _/ /__ _____/ /_(_)___    / __/___  __ _____  / / /");
    println!(r"   / ___/ _ `/ / _ `/ __/ __/ / __/   / /  / _ \/ // / _ \/ / / ");
    println!(r"  /_/   \_,_/_/\_,_/\__/\__/_/\__/   /_/   \___/\_,_/_//_/_/_/  ");
    println!();
    println!("  === GALACTIC COUNCIL EXPLORATION SIMULATION ===");
    println!(
        "  {} rounds | {} council members | Infinite possibilities",
        rounds, members
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
