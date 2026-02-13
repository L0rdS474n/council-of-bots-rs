use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct LlmBot {
    host: String,
    model: String,
}

impl LlmBot {
    pub fn new(host: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            model: model.into(),
        }
    }

    fn build_prompt(&self, event: &Event, galaxy: &GalaxyState) -> String {
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
        s.push_str("You are an AI agent participating as a council member in a galactic exploration simulation.\n");
        s.push_str("Your task: pick the best option index for the council, given the event and galaxy state.\n");
        s.push_str(
            "Return ONLY a JSON object: {\"choice\": <integer>, \"reason\": <short string>}\n",
        );
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
}

impl GalacticCouncilMember for LlmBot {
    fn name(&self) -> &'static str {
        "llm-bot"
    }

    fn expertise(&self) -> &[(&'static str, f32)] {
        // Broad, but not maximal. It should still be meaningfully weighted.
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
        let prompt = self.build_prompt(event, galaxy);

        ollama_choose(&self.host, &self.model, &prompt, event.options.len()).unwrap_or_default()
    }
}

#[derive(Debug, Deserialize)]
struct ChoiceJson {
    choice: usize,
    #[allow(dead_code)]
    reason: Option<String>,
}

fn clamp_choice(choice: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        choice.min(len - 1)
    }
}

fn extract_first_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end < start {
        return None;
    }
    Some(&s[start..=end])
}

fn ollama_choose(
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

fn parse_host(host: &str) -> Result<(String, u16), String> {
    // Accept: http://127.0.0.1:11434 or 127.0.0.1:11434
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

fn ollama_generate(host: &str, model: &str, prompt: &str) -> Result<String, String> {
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

    // Split headers/body.
    let (_, body) = raw.split_once("\r\n\r\n").ok_or("invalid http response")?;

    // Parse JSON body.
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let resp = v
        .get("response")
        .and_then(|x| x.as_str())
        .ok_or("missing response field")?;

    Ok(resp.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
