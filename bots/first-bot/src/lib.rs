use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use council_core::ollama::{build_galactic_prompt, ollama_choose, OllamaConfig};
use council_core::{Context, CouncilMember, Decision};

const PERSONALITY: &str = "You are a bold frontier explorer who believes fortune favors the brave. You take decisive action and lead from the front, especially in the early stages of any mission.";

/// FirstBot takes a simple optimistic stance: it approves early rounds
/// to build momentum, but abstains once the council has had a few turns
/// to speak.
pub struct FirstBot {
    ollama: Option<OllamaConfig>,
}

impl FirstBot {
    pub fn new() -> Self {
        Self { ollama: None }
    }

    pub fn with_ollama(config: OllamaConfig) -> Self {
        Self {
            ollama: Some(config),
        }
    }
}

impl Default for FirstBot {
    fn default() -> Self {
        Self::new()
    }
}

impl CouncilMember for FirstBot {
    fn name(&self) -> &'static str {
        "first-bot"
    }

    fn vote(&self, ctx: &Context) -> Decision {
        if ctx.round <= 3 {
            Decision::Approve
        } else {
            Decision::Abstain
        }
    }
}

impl GalacticCouncilMember for FirstBot {
    fn name(&self) -> &'static str {
        "first-bot"
    }

    fn expertise(&self) -> &[(&'static str, f32)] {
        &[("exploration", 0.8), ("science", 0.5)]
    }

    /// Optimistic explorer: always picks the boldest option (index 0) in the
    /// first 10 rounds, then switches to cautious (last option) later.
    /// Falls back to deterministic logic if Ollama is unavailable.
    fn vote(&self, event: &Event, galaxy: &GalaxyState) -> usize {
        if let Some(cfg) = &self.ollama {
            let prompt = build_galactic_prompt(PERSONALITY, event, galaxy);
            if let Ok(choice) = ollama_choose(&cfg.host, &cfg.model, &prompt, event.options.len()) {
                return choice;
            }
        }
        // Deterministic fallback
        if galaxy.round <= 10 {
            0
        } else {
            event.options.len().saturating_sub(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use council_core::Context;

    #[test]
    fn approves_initial_rounds() {
        let bot = FirstBot::new();
        for round in 1..=3 {
            let ctx = Context {
                round,
                previous_tally: None,
            };
            assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Approve);
        }
    }

    #[test]
    fn abstains_after_initial_push() {
        let bot = FirstBot::new();
        let ctx = Context {
            round: 4,
            previous_tally: None,
        };
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Abstain);
    }

    #[test]
    fn test_new_has_no_ollama() {
        let bot = FirstBot::new();
        assert!(bot.ollama.is_none());
    }

    #[test]
    fn test_with_ollama_stores_config() {
        let cfg = OllamaConfig {
            host: "127.0.0.1:11434".to_string(),
            model: "llama3".to_string(),
        };
        let bot = FirstBot::with_ollama(cfg);
        assert!(bot.ollama.is_some());
    }

    #[test]
    fn test_personality_constant() {
        assert!(PERSONALITY.contains("bold"));
        assert!(PERSONALITY.contains("explorer"));
    }
}
