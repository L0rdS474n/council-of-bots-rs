use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use council_core::ollama::{build_galactic_prompt, ollama_choose, OllamaConfig};

const PERSONALITY: &str = "You are an AI agent with broad knowledge across all domains. You analyze situations rationally and make balanced decisions.";

#[derive(Debug, Clone)]
pub struct LlmBot {
    config: OllamaConfig,
}

impl LlmBot {
    pub fn new(host: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            config: OllamaConfig {
                host: host.into(),
                model: model.into(),
            },
        }
    }
}

impl GalacticCouncilMember for LlmBot {
    fn name(&self) -> &'static str {
        "llm-bot"
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
        ollama_choose(
            &self.config.host,
            &self.config.model,
            &prompt,
            event.options.len(),
        )
        .unwrap_or_default()
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
}
