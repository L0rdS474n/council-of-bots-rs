use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use council_core::ollama::{
    build_deliberation_prompt, build_galactic_prompt, extract_choice, extract_comment,
    ollama_choose, ollama_generate, OllamaConfig,
};

const PERSONALITY: &str = "You are an AI agent with broad knowledge across all domains. You analyze situations rationally and make balanced decisions.";

#[derive(Debug, Clone)]
pub struct LlmBot {
    name: &'static str,
    config: OllamaConfig,
}

impl LlmBot {
    pub fn new(host: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new_named("llm-bot", host, model)
    }

    pub fn new_named(
        name: &'static str,
        host: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            name,
            config: OllamaConfig {
                host: host.into(),
                model: model.into(),
            },
        }
    }
}

fn fallback_choice(round: u32, num_options: usize) -> usize {
    if num_options == 0 {
        return 0;
    }
    (round as usize) % num_options
}

impl GalacticCouncilMember for LlmBot {
    fn name(&self) -> &'static str {
        self.name
    }

    fn expertise(&self) -> &[(&'static str, f32)] {
        &[
            ("strategy", 0.8),
            ("science", 0.7),
            ("diplomacy", 0.6),
            ("engineering", 0.6),
            ("exploration", 0.6),
            ("culture", 0.4),
            ("military", 0.4),
            ("security", 0.4),
        ]
    }

    fn vote(&self, event: &Event, galaxy: &GalaxyState) -> usize {
        let prompt = build_galactic_prompt(PERSONALITY, event, galaxy);
        match ollama_choose(
            &self.config.host,
            &self.config.model,
            &prompt,
            event.options.len(),
        ) {
            Ok(choice) => choice,
            Err(e) => {
                eprintln!("[{}] LLM failed ({}), using fallback", self.name, e);
                fallback_choice(galaxy.round, event.options.len())
            }
        }
    }

    fn comment(&self, event: &Event, galaxy: &GalaxyState) -> Option<String> {
        let prompt = build_deliberation_prompt(PERSONALITY, event, galaxy);
        let response = ollama_generate(&self.config.host, &self.config.model, &prompt).ok()?;
        let choice = extract_choice(&response, event.options.len()).ok()?;
        let comment = extract_comment(&response).unwrap_or_else(|| "(no comment)".to_string());
        Some(format!("prefers [{}] — {}", choice, comment))
    }
}

#[cfg(test)]
mod tests {
    use council_core::ollama::{clamp_choice, extract_first_json_object, parse_host};

    #[test]
    fn extract_json_object_works() {
        let s = "noise {\"choice\": 2, \"reason\": \"ok\"} tail";
        let j = extract_first_json_object(s).unwrap();
        assert_eq!(j, "{\"choice\": 2, \"reason\": \"ok\"}");
    }

    #[test]
    fn clamp_choice_bounds() {
        assert_eq!(clamp_choice(0, 0), 0);
        assert_eq!(clamp_choice(0, 3), 0);
        assert_eq!(clamp_choice(2, 3), 2);
        assert_eq!(clamp_choice(3, 3), 2);
        assert_eq!(clamp_choice(999, 1), 0);
    }

    #[test]
    fn parse_host_accepts_http_prefix() {
        let (h, p) = parse_host("http://127.0.0.1:11434").unwrap();
        assert_eq!(h, "127.0.0.1");
        assert_eq!(p, 11434);
    }

    #[test]
    fn parse_host_default_port() {
        let (h, p) = parse_host("127.0.0.1").unwrap();
        assert_eq!(h, "127.0.0.1");
        assert_eq!(p, 11434);
    }

    // AC-7: llm-bot deterministic fallback
    #[test]
    fn test_fallback_cycles_by_round() {
        use super::fallback_choice;
        assert_eq!(fallback_choice(1, 3), 1);
        assert_eq!(fallback_choice(2, 3), 2);
        assert_eq!(fallback_choice(3, 3), 0);
    }

    #[test]
    fn test_fallback_zero_options() {
        use super::fallback_choice;
        assert_eq!(fallback_choice(5, 0), 0);
    }
}
