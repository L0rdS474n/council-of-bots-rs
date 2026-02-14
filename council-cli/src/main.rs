use contrarian_bot::ContrarianBot;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use council_core::ollama::{can_connect, can_connect_llm, parse_host, LlmApi, OllamaConfig};
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
    enable_llm: bool,
    enable_llm_bot: bool,
    deliberate: bool,
    galnet: bool,

    llm_provider: String,
    llm_base_url: String,
    llm_model: String,
    llm_api_key: String,

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
        enable_llm: false,
        enable_llm_bot: false,
        deliberate: false,
        galnet: false,

        llm_provider: "ollama".to_string(),
        llm_base_url: "http://127.0.0.1:1234/v1".to_string(),
        llm_model: "".to_string(),
        llm_api_key: "".to_string(),

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
            "--enable-llm" => cfg.enable_llm = true,
            "--enable-llm-bot" => cfg.enable_llm_bot = true,
            "--deliberate" => cfg.deliberate = true,
            "--galnet" => cfg.galnet = true,
            "--llm-provider" => {
                if let Some(v) = it.next() {
                    cfg.llm_provider = v;
                }
            }
            "--llm-base-url" => {
                if let Some(v) = it.next() {
                    cfg.llm_base_url = v;
                }
            }
            "--llm-model" => {
                if let Some(v) = it.next() {
                    cfg.llm_model = v;
                }
            }
            "--llm-api-key" => {
                if let Some(v) = it.next() {
                    cfg.llm_api_key = v;
                }
            }
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
                    "council-cli\n\nFlags:\n  --rounds <n>          Number of rounds (default: 25)\n  --enable-llm          Give all 5 bots unique LLM personalities via a local LLM\n  --enable-llm-bot      Add a 6th dedicated LLM bot to the council\n  --deliberate          Let bots publish short comments before the final vote\n  --galnet             Add small GalNet news blurbs each round (for fun)\n\n  --llm-provider <ollama|lmstudio>  Which local LLM API to use (default: ollama)\n  --llm-base-url <url>   LM Studio base URL (default: http://127.0.0.1:1234/v1)\n  --llm-model <model>    LM Studio model id (defaults to --ollama-model if unset)\n  --llm-api-key <key>    Optional API key (LM Studio often accepts any value)\n\n  --spawn-ollama        Start/stop Ollama automatically for this run (ollama only)\n  --ollama-bin <path>   Path to ollama binary (default: ollama)\n  --ollama-host <host:port>  Ollama endpoint (default: 127.0.0.1:11434)\n  --ollama-model <model>     Model name (default: llama3)\n"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }

    cfg
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

    let (h, p) = parse_host(&cfg.ollama_host).unwrap_or_else(|_| ("127.0.0.1".to_string(), 11434));
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

fn resolve_llm_config(cfg: &CliConfig) -> Result<OllamaConfig, String> {
    let provider = cfg.llm_provider.trim().to_ascii_lowercase();
    match provider.as_str() {
        "ollama" => Ok(OllamaConfig {
            host: cfg.ollama_host.clone(),
            model: cfg.ollama_model.clone(),
            api: LlmApi::Ollama,
            api_key: None,
        }),
        "lmstudio" | "lm-studio" | "lm_studio" => {
            let model = if cfg.llm_model.trim().is_empty() {
                cfg.ollama_model.clone()
            } else {
                cfg.llm_model.clone()
            };
            Ok(OllamaConfig {
                host: cfg.llm_base_url.clone(),
                model,
                api: LlmApi::OpenAiChatCompletions,
                api_key: if cfg.llm_api_key.trim().is_empty() {
                    None
                } else {
                    Some(cfg.llm_api_key.clone())
                },
            })
        }
        _ => Err(format!(
            "unknown --llm-provider '{}'. Use 'ollama' or 'lmstudio'",
            cfg.llm_provider
        )),
    }
}

fn main() {
    let cfg = parse_args();

    let needs_llm = cfg.enable_llm || cfg.enable_llm_bot;
    let llm_cfg = if needs_llm {
        match resolve_llm_config(&cfg) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(2);
            }
        }
    } else {
        // Not used.
        OllamaConfig {
            host: cfg.ollama_host.clone(),
            model: cfg.ollama_model.clone(),
            api: LlmApi::Ollama,
            api_key: None,
        }
    };

    let _ollama_guard = if needs_llm && llm_cfg.api == LlmApi::Ollama {
        maybe_spawn_ollama(&cfg)
    } else {
        None
    };

    if needs_llm && !can_connect_llm(&llm_cfg) {
        match llm_cfg.api {
            LlmApi::Ollama => {
                eprintln!(
                    "LLM mode enabled but Ollama is not reachable at {}.\n\
                     - If you want council-cli to manage Ollama automatically: add --spawn-ollama\n\
                     - Otherwise start it yourself (e.g. `ollama serve`) and ensure the model exists.\n\
                     - You can change the path with --ollama-bin and endpoint with --ollama-host",
                    cfg.ollama_host
                );
            }
            LlmApi::OpenAiChatCompletions => {
                eprintln!(
                    "LLM mode enabled but LM Studio (OpenAI-compatible) is not reachable at {}.\n\
                     - Start LM Studio Local Server and verify the base URL.\n\
                     - Default is --llm-base-url http://127.0.0.1:1234/v1",
                    llm_cfg.host
                );
            }
        }
        std::process::exit(2);
    }

    let mut bots: Vec<Box<dyn GalacticCouncilMember>> = if cfg.enable_llm {
        vec![
            Box::new(ExampleBot::with_ollama(llm_cfg.clone())),
            Box::new(FirstBot::with_ollama(llm_cfg.clone())),
            Box::new(CycleBot::with_ollama(llm_cfg.clone())),
            Box::new(ContrarianBot::with_ollama(llm_cfg.clone())),
            Box::new(OracleBot::with_ollama(llm_cfg.clone())),
        ]
    } else {
        vec![
            Box::new(ExampleBot::new()),
            Box::new(FirstBot::new()),
            Box::new(CycleBot::new()),
            Box::new(ContrarianBot::new()),
            Box::new(OracleBot::new()),
        ]
    };

    if cfg.enable_llm_bot {
        bots.push(Box::new(LlmBot::new_with_config(llm_cfg.clone())));
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

        // Optional deliberation phase
        let mut event_for_vote = event.clone();
        if cfg.deliberate {
            let mut lines = Vec::new();
            for bot in &bots {
                if let Some(comment) = bot.comment(&event, &galaxy) {
                    lines.push(format!("{}: {}", bot.name(), comment));
                }
            }

            if !lines.is_empty() {
                println!("  [DELIBERATION]");
                for line in &lines {
                    println!("    {}", line);
                }
                println!();

                event_for_vote.description = format!(
                    "{}\n\nCOUNCIL DELIBERATION:\n{}",
                    event_for_vote.description,
                    lines.join("\n")
                );
            }
        }

        // Collect votes
        let mut votes = Vec::new();
        for bot in &bots {
            let weight = calculate_vote_weight(bot.as_ref(), &event);
            let chosen = bot.vote(&event_for_vote, &galaxy);
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

        if cfg.galnet {
            println!();
            println!(
                "  [GALNET] {}",
                galnet_blurb(
                    round,
                    winner,
                    outcome.score_delta,
                    score.total,
                    galaxy.threats.len(),
                    galaxy.discoveries.len(),
                )
            );
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

fn galnet_blurb(
    round: u32,
    winner: usize,
    score_delta: i32,
    total_score: i32,
    threats: usize,
    discoveries: usize,
) -> String {
    const BLURBS: &[&str] = &[
        "Markets rally; analysts pretend this was always the plan.",
        "Eyewitnesses report the Council looked confident. This may be a hallucination.",
        "Diplomats applaud politely while checking the nearest exit.",
        "A spokesperson clarifies: 'No, this is not a coup. It's a feature update.'",
        "Galactic weather: 0% chance of peace, 100% chance of paperwork.",
        "Citizen morale rises sharply, then remembers the tax code.",
        "Historians mark this as 'a decision'. The bar is low.",
        "A rogue AI claims credit. The Council denies everything, repeatedly.",
        "Breaking: nobody understands the plan, but everyone is nodding.",
    ];

    let idx = ((round as usize)
        .wrapping_mul(31)
        .wrapping_add(winner.wrapping_mul(7))
        .wrapping_add(threats.wrapping_mul(13))
        .wrapping_add(discoveries.wrapping_mul(5)))
        % BLURBS.len();

    let mood = match score_delta {
        d if d > 0 => "WIN",
        d if d < 0 => "OUCH",
        _ => "MEH",
    };

    format!(
        "Round {} [{}] ({:+} pts, total {}): {} Threats={}, Discoveries={}",
        round, mood, score_delta, total_score, BLURBS[idx], threats, discoveries
    )
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
            Box::new(ExampleBot::new()),
            Box::new(FirstBot::new()),
            Box::new(CycleBot::new()),
            Box::new(ContrarianBot::new()),
            Box::new(OracleBot::new()),
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
                Box::new(ExampleBot::new()),
                Box::new(FirstBot::new()),
                Box::new(CycleBot::new()),
                Box::new(ContrarianBot::new()),
                Box::new(OracleBot::new()),
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
