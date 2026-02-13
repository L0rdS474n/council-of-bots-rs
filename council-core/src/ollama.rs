//! Shared Ollama integration for galactic council bots.
//!
//! Provides HTTP-based communication with a local Ollama instance,
//! JSON parsing utilities, and prompt building for galactic events.

use crate::event::Event;
use crate::galaxy::GalaxyState;
use serde::Deserialize;

/// Configuration for connecting to an Ollama instance.
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub host: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
struct ChoiceJson {
    choice: usize,
    #[allow(dead_code)]
    reason: Option<String>,
}

/// Parse a host string like "http://127.0.0.1:11434" or "127.0.0.1:11434"
/// into (hostname, port). Defaults to port 11434.
pub fn parse_host(host: &str) -> Result<(String, u16), String> {
    let h = host.strip_prefix("http://").unwrap_or(host);
    let mut parts = h.split(':');
    let hostname = parts.next().ok_or("missing host")?.trim().to_string();
    let port = parts
        .next()
        .unwrap_or("11434")
        .trim()
        .parse::<u16>()
        .map_err(|_| "invalid port".to_string())?;
    Ok((hostname, port))
}

/// Extract the first JSON object `{...}` from a string that may contain
/// surrounding text.
pub fn extract_first_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end < start {
        return None;
    }
    Some(&s[start..=end])
}

/// Clamp a choice index to valid bounds for the given number of options.
pub fn clamp_choice(choice: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        choice.min(len - 1)
    }
}

/// Send a generate request to Ollama and return the response text.
pub fn ollama_generate(host: &str, model: &str, prompt: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let (hostname, port) = parse_host(host)?;
    let addr = format!("{}:{}", hostname, port);

    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false
    })
    .to_string();

    let mut stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
    let req = format!(
        "POST /api/generate HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        hostname,
        body.len(),
        body
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut raw = String::new();
    stream.read_to_string(&mut raw).map_err(|e| e.to_string())?;

    let (_, body) = raw.split_once("\r\n\r\n").ok_or("invalid http response")?;

    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let resp = v
        .get("response")
        .and_then(|x| x.as_str())
        .ok_or("missing response field")?;

    Ok(resp.to_string())
}

/// Ask Ollama to choose among options. Returns a clamped index.
pub fn ollama_choose(
    host: &str,
    model: &str,
    prompt: &str,
    options_len: usize,
) -> Result<usize, String> {
    let response = ollama_generate(host, model, prompt)?;

    let json_str = extract_first_json_object(&response).ok_or("no json in response")?;
    let parsed: ChoiceJson = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
    Ok(clamp_choice(parsed.choice, options_len))
}

/// Build a galactic event prompt with a personality prefix.
pub fn build_galactic_prompt(personality: &str, event: &Event, galaxy: &GalaxyState) -> String {
    let threats = galaxy
        .threats
        .iter()
        .map(|t| format!("{}(sev={}, rounds={})", t.name, t.severity, t.rounds_active))
        .collect::<Vec<_>>()
        .join(", ");

    let species = galaxy
        .relations
        .iter()
        .map(|(n, r)| format!("{}={:?}", n, r))
        .collect::<Vec<_>>()
        .join(", ");

    let mut s = String::new();
    s.push_str(personality);
    s.push_str("\n\n");
    s.push_str("You are participating as a council member in a galactic exploration simulation.\n");
    s.push_str("Your task: pick the best option index for the council, given the event and galaxy state.\n");
    s.push_str("Return ONLY a JSON object: {\"choice\": <integer>, \"reason\": <short string>}\n");
    s.push_str("Do not include any other text.\n\n");

    s.push_str(&format!("ROUND: {}\n", galaxy.round));
    s.push_str(&format!("SECTORS: {}\n", galaxy.explored_sectors.len()));
    s.push_str(&format!("SPECIES: {}\n", galaxy.known_species.len()));
    s.push_str(&format!(
        "RELATIONS: {}\n",
        if species.is_empty() {
            "(none)"
        } else {
            &species
        }
    ));
    s.push_str(&format!(
        "THREATS: {}\n\n",
        if threats.is_empty() {
            "(none)"
        } else {
            &threats
        }
    ));

    s.push_str("EVENT:\n");
    s.push_str(&event.description);
    s.push_str("\n\nOPTIONS:\n");
    for (i, opt) in event.options.iter().enumerate() {
        s.push_str(&format!("{}: {}\n", i, opt.description));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Outcome, ResponseOption};
    use crate::galaxy::{GalaxyState, Threat};

    #[test]
    fn test_parse_host_with_http_prefix() {
        let (h, p) = parse_host("http://127.0.0.1:11434").unwrap();
        assert_eq!(h, "127.0.0.1");
        assert_eq!(p, 11434);
    }

    #[test]
    fn test_parse_host_default_port() {
        let (h, p) = parse_host("127.0.0.1").unwrap();
        assert_eq!(h, "127.0.0.1");
        assert_eq!(p, 11434);
    }

    #[test]
    fn test_extract_json_object() {
        let s = "noise {\"choice\": 2, \"reason\": \"ok\"} tail";
        let j = extract_first_json_object(s).unwrap();
        assert_eq!(j, "{\"choice\": 2, \"reason\": \"ok\"}");
    }

    #[test]
    fn test_extract_json_no_braces() {
        assert!(extract_first_json_object("no json here").is_none());
    }

    #[test]
    fn test_clamp_choice_bounds() {
        assert_eq!(clamp_choice(0, 0), 0);
        assert_eq!(clamp_choice(0, 3), 0);
        assert_eq!(clamp_choice(2, 3), 2);
        assert_eq!(clamp_choice(3, 3), 2);
        assert_eq!(clamp_choice(999, 1), 0);
    }

    fn make_test_event(num_options: usize) -> Event {
        let options = (0..num_options)
            .map(|i| ResponseOption {
                description: format!("Option {}", i),
                outcome: Outcome {
                    description: format!("Outcome {}", i),
                    score_delta: 0,
                    state_changes: vec![],
                },
            })
            .collect();
        Event {
            description: "A strange signal detected".to_string(),
            relevant_expertise: vec![("science".to_string(), 0.5)],
            options,
        }
    }

    #[test]
    fn test_build_galactic_prompt_includes_personality() {
        let event = make_test_event(2);
        let galaxy = GalaxyState::new();
        let prompt = build_galactic_prompt("You are a bold explorer.", &event, &galaxy);
        assert!(prompt.starts_with("You are a bold explorer."));
    }

    #[test]
    fn test_build_galactic_prompt_includes_event_and_options() {
        let event = make_test_event(3);
        let galaxy = GalaxyState::new();
        let prompt = build_galactic_prompt("Test personality", &event, &galaxy);
        assert!(prompt.contains("A strange signal detected"));
        assert!(prompt.contains("Option 0"));
        assert!(prompt.contains("Option 1"));
        assert!(prompt.contains("Option 2"));
    }

    #[test]
    fn test_build_galactic_prompt_includes_galaxy_state() {
        let event = make_test_event(2);
        let mut galaxy = GalaxyState::new();
        galaxy.threats.push(Threat {
            name: "Void Reapers".to_string(),
            severity: 5,
            rounds_active: 2,
        });
        let prompt = build_galactic_prompt("Test", &event, &galaxy);
        assert!(prompt.contains("Void Reapers"));
        assert!(prompt.contains("sev=5"));
        assert!(prompt.contains("ROUND:"));
        assert!(prompt.contains("SECTORS:"));
    }
}
