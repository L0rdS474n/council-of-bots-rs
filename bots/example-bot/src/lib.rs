use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use council_core::ollama::{build_galactic_prompt, ollama_choose, ollama_deliberate, OllamaConfig};
use council_core::{Context, CouncilMember, Decision};

const PERSONALITY: &str = "You are a methodical engineer who values data-driven decisions and systematic approaches. You prefer reliable, well-tested solutions over risky gambles.";

/// A simple example bot that flips decision based on round parity.
pub struct ExampleBot {
    ollama: Option<OllamaConfig>,
}

impl ExampleBot {
    pub fn new() -> Self {
        Self { ollama: None }
    }

    pub fn with_ollama(config: OllamaConfig) -> Self {
        Self {
            ollama: Some(config),
        }
    }
}

impl Default for ExampleBot {
    fn default() -> Self {
        Self::new()
    }
}

impl CouncilMember for ExampleBot {
    fn name(&self) -> &'static str {
        "example-bot"
    }

    fn vote(&self, ctx: &Context) -> Decision {
        if ctx.round.is_multiple_of(2) {
            Decision::Approve
        } else {
            Decision::Reject
        }
    }
}

impl GalacticCouncilMember for ExampleBot {
    fn name(&self) -> &'static str {
        "example-bot"
    }

    fn expertise(&self) -> &[(&'static str, f32)] {
        &[("engineering", 0.6), ("science", 0.4)]
    }

    /// Alternates between first and second option each round.
    /// Falls back to deterministic logic if Ollama is unavailable.
    fn vote(&self, event: &Event, galaxy: &GalaxyState) -> usize {
        if let Some(cfg) = &self.ollama {
            let prompt = build_galactic_prompt(PERSONALITY, event, galaxy);
            if let Ok(choice) = ollama_choose(&cfg.host, &cfg.model, &prompt, event.options.len()) {
                return choice;
            }
        }
        // Deterministic fallback
        let pick = if galaxy.round.is_multiple_of(2) { 0 } else { 1 };
        pick.min(event.options.len().saturating_sub(1))
    }

    fn comment(&self, event: &Event, galaxy: &GalaxyState) -> Option<String> {
        let cfg = self.ollama.as_ref()?;
        let (choice, comment) =
            ollama_deliberate(&cfg.host, &cfg.model, PERSONALITY, event, galaxy).ok()?;
        Some(format!("prefers [{}] — {}", choice, comment))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use council_core::Context;

    #[test]
    fn example_bot_votes_deterministically() {
        let bot = ExampleBot::new();
        let ctx1 = Context {
            round: 1,
            previous_tally: None,
        };
        let ctx2 = Context {
            round: 2,
            previous_tally: None,
        };

        assert!(matches!(CouncilMember::vote(&bot, &ctx1), Decision::Reject));
        assert!(matches!(
            CouncilMember::vote(&bot, &ctx2),
            Decision::Approve
        ));
    }

    #[test]
    fn test_new_has_no_ollama() {
        let bot = ExampleBot::new();
        assert!(bot.ollama.is_none());
    }

    #[test]
    fn test_with_ollama_stores_config() {
        let cfg = OllamaConfig {
            host: "127.0.0.1:11434".to_string(),
            model: "llama3".to_string(),
        };
        let bot = ExampleBot::with_ollama(cfg);
        assert!(bot.ollama.is_some());
    }

    #[test]
    fn test_personality_constant() {
        assert!(PERSONALITY.contains("methodical"));
        assert!(PERSONALITY.contains("engineer"));
    }
}
