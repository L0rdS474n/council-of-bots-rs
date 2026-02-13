use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use council_core::ollama::{build_galactic_prompt, llm_choose, llm_deliberate, OllamaConfig};
use council_core::{Context, CouncilMember, Decision};

const PERSONALITY: &str = "You are a cultural diplomat who seeks balance and harmony. You believe in giving every approach a fair chance and rotating strategies to maintain equilibrium.";

/// CycleBot rotates its stance every round to encourage variety in the council.
/// The pattern is approve -> reject -> abstain.
pub struct CycleBot {
    ollama: Option<OllamaConfig>,
}

impl CycleBot {
    pub fn new() -> Self {
        Self { ollama: None }
    }

    pub fn with_ollama(config: OllamaConfig) -> Self {
        Self {
            ollama: Some(config),
        }
    }
}

impl Default for CycleBot {
    fn default() -> Self {
        Self::new()
    }
}

impl CouncilMember for CycleBot {
    fn name(&self) -> &'static str {
        "cycle-bot"
    }

    fn vote(&self, ctx: &Context) -> Decision {
        match ctx.round % 3 {
            1 => Decision::Approve,
            2 => Decision::Reject,
            _ => Decision::Abstain,
        }
    }
}

impl GalacticCouncilMember for CycleBot {
    fn name(&self) -> &'static str {
        "cycle-bot"
    }

    fn expertise(&self) -> &[(&'static str, f32)] {
        &[("culture", 0.7), ("linguistics", 0.5), ("archaeology", 0.3)]
    }

    /// Cycles through available options based on round number.
    /// Falls back to deterministic logic if Ollama is unavailable.
    fn vote(&self, event: &Event, galaxy: &GalaxyState) -> usize {
        if let Some(cfg) = &self.ollama {
            let prompt = build_galactic_prompt(PERSONALITY, event, galaxy);
            if let Ok(choice) = llm_choose(cfg, &prompt, event.options.len()) {
                return choice;
            }
        }
        // Deterministic fallback
        let num = event.options.len();
        if num == 0 {
            return 0;
        }
        (galaxy.round as usize) % num
    }

    fn comment(&self, event: &Event, galaxy: &GalaxyState) -> Option<String> {
        let cfg = self.ollama.as_ref()?;
        let (choice, comment) = llm_deliberate(cfg, PERSONALITY, event, galaxy).ok()?;
        Some(format!("prefers [{}] — {}", choice, comment))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use council_core::Context;

    #[test]
    fn cycles_through_three_decisions() {
        let bot = CycleBot::new();
        let rounds = [
            (1, Decision::Approve),
            (2, Decision::Reject),
            (3, Decision::Abstain),
            (4, Decision::Approve),
        ];

        for (round, expected) in rounds {
            let ctx = Context {
                round,
                previous_tally: None,
            };
            assert_eq!(CouncilMember::vote(&bot, &ctx), expected);
        }
    }

    #[test]
    fn test_new_has_no_ollama() {
        let bot = CycleBot::new();
        assert!(bot.ollama.is_none());
    }

    #[test]
    fn test_with_ollama_stores_config() {
        let cfg = OllamaConfig {
            host: "127.0.0.1:11434".to_string(),
            model: "llama3".to_string(),
            api: council_core::ollama::LlmApi::Ollama,
            api_key: None,
        };
        let bot = CycleBot::with_ollama(cfg);
        assert!(bot.ollama.is_some());
    }

    #[test]
    fn test_personality_constant() {
        assert!(PERSONALITY.contains("diplomat"));
        assert!(PERSONALITY.contains("balance"));
    }
}
