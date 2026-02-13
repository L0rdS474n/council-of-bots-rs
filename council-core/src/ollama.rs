//! Shared Ollama integration for galactic council bots.
//!
//! Provides HTTP-based communication with a local Ollama instance,
//! JSON parsing utilities, and prompt building for galactic events.

use crate::event::Event;
use crate::galaxy::GalaxyState;
use serde::Deserialize;

/// LLM backend API type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmApi {
    /// Ollama native API (`POST /api/generate`).
    Ollama,
    /// OpenAI-compatible Chat Completions (`POST /v1/chat/completions`).
    /// Used by LM Studio local server.
    OpenAiChatCompletions,
}

/// Configuration for connecting to an LLM endpoint.
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// Endpoint host/url.
    ///
    /// - `LlmApi::Ollama`: accepts `127.0.0.1:11434` or `http://127.0.0.1:11434`.
    /// - `LlmApi::OpenAiChatCompletions`: accepts `http://127.0.0.1:1234/v1`.
    pub host: String,
    pub model: String,
    pub api: LlmApi,
    /// Optional API key (LM Studio often accepts any value).
    pub api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChoiceJson {
    choice: usize,
    #[allow(dead_code)]
    reason: Option<String>,
}

/// Parse a host string like "http://127.0.0.1:11434", "https://example.com:8080",
/// or "127.0.0.1:11434" into (hostname, port). Defaults to port 11434.
/// Returns Err on empty hostname or invalid port.
///
/// Note: This helper is intended for host:port style endpoints (Ollama).
/// For URLs with paths (LM Studio OpenAI-compatible), use `parse_http_url`.
pub fn parse_host(host: &str) -> Result<(String, u16), String> {
    let h = host
        .strip_prefix("https://")
        .or_else(|| host.strip_prefix("http://"))
        .unwrap_or(host);
    let mut parts = h.split(':');
    let hostname = parts.next().ok_or("missing host")?.trim().to_string();
    if hostname.is_empty() {
        return Err("empty hostname".to_string());
    }
    let port = parts
        .next()
        .unwrap_or("11434")
        .trim()
        .parse::<u16>()
        .map_err(|_| "invalid port".to_string())?;
    Ok((hostname, port))
}

/// Parse an HTTP url (http only) into (hostname, port, path_prefix).
///
/// Accepts:
/// - `http://127.0.0.1:1234/v1`
/// - `127.0.0.1:1234/v1`
/// - `127.0.0.1:1234`
///
/// Notes:
/// - `https://` is rejected (no TLS in this minimal client).
/// - `path_prefix` is empty or starts with `/`.
pub fn parse_http_url(url: &str) -> Result<(String, u16, String), String> {
    if url.trim().is_empty() {
        return Err("empty url".to_string());
    }

    if url.starts_with("https://") {
        return Err("https not supported".to_string());
    }

    let u = url.strip_prefix("http://").unwrap_or(url);
    let (hostport, path) = match u.split_once('/') {
        Some((a, b)) => (a, format!("/{}", b)),
        None => (u, "".to_string()),
    };

    let mut parts = hostport.split(':');
    let hostname = parts.next().unwrap_or("").trim().to_string();
    if hostname.is_empty() {
        return Err("missing host".to_string());
    }
    let port = parts
        .next()
        .unwrap_or("80")
        .trim()
        .parse::<u16>()
        .map_err(|_| "invalid port".to_string())?;

    Ok((hostname, port, path))
}

/// Parse an HTTP status line like "HTTP/1.1 200 OK" and return the status code
/// for 2xx responses, or an error for non-2xx or malformed lines.
pub fn parse_http_status(status_line: &str) -> Result<u16, String> {
    let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err("invalid HTTP status line".to_string());
    }
    let code: u16 = parts[1]
        .parse()
        .map_err(|_| "invalid HTTP status code".to_string())?;
    if (200..300).contains(&code) {
        Ok(code)
    } else {
        let reason = if parts.len() >= 3 {
            parts[2]
        } else {
            "Unknown"
        };
        Err(format!("HTTP error: {} {}", code, reason))
    }
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

/// Extract a choice index from an LLM response using multiple strategies:
/// 1. JSON with integer choice field
/// 2. JSON with string choice field
/// 3. Bare integer scan
///
/// The result is clamped to valid bounds for the given number of options.
pub fn extract_choice(response: &str, options_len: usize) -> Result<usize, String> {
    // Strategy 1: Try JSON with integer choice: {"choice": 2}
    if let Some(json_str) = extract_first_json_object(response) {
        if let Ok(parsed) = serde_json::from_str::<ChoiceJson>(json_str) {
            return Ok(clamp_choice(parsed.choice, options_len));
        }
        // Strategy 2: Try JSON with string choice: {"choice": "2"}
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(choice_str) = v.get("choice").and_then(|c| c.as_str()) {
                if let Ok(n) = choice_str.parse::<usize>() {
                    return Ok(clamp_choice(n, options_len));
                }
            }
        }
    }
    // Strategy 3: Bare integer scan - find first digit
    for word in response.split_whitespace() {
        if let Ok(n) = word
            .trim_matches(|c: char| !c.is_ascii_digit())
            .parse::<usize>()
        {
            return Ok(clamp_choice(n, options_len));
        }
    }
    Err("no valid choice found in response".to_string())
}

/// Check if an Ollama instance is reachable at the given host (host:port style).
pub fn can_connect(host: &str) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let parsed = match parse_host(host) {
        Ok((h, p)) => (h, p),
        Err(_) => return false,
    };
    let addr = match (parsed.0.as_str(), parsed.1).to_socket_addrs() {
        Ok(mut a) => match a.next() {
            Some(a) => a,
            None => return false,
        },
        Err(_) => return false,
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

/// Check if an HTTP endpoint (http only) is reachable.
pub fn can_connect_http(url: &str) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let parsed = match parse_http_url(url) {
        Ok((h, p, _)) => (h, p),
        Err(_) => return false,
    };
    let addr = match (parsed.0.as_str(), parsed.1).to_socket_addrs() {
        Ok(mut a) => match a.next() {
            Some(a) => a,
            None => return false,
        },
        Err(_) => return false,
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

/// Check if an LLM endpoint is reachable based on its configured API.
pub fn can_connect_llm(cfg: &OllamaConfig) -> bool {
    match cfg.api {
        LlmApi::Ollama => can_connect(&cfg.host),
        LlmApi::OpenAiChatCompletions => can_connect_http(&cfg.host),
    }
}

/// Send a generate request to Ollama and return the response text.
///
/// Applies connection timeout (5s), read/write timeouts (30s), buffer limit (1MB),
/// and validates HTTP status code.
pub fn ollama_generate(host: &str, model: &str, prompt: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let (hostname, port) = parse_host(host)?;

    let addr = (hostname.as_str(), port)
        .to_socket_addrs()
        .map_err(|_| "failed to resolve host".to_string())?
        .next()
        .ok_or_else(|| "failed to resolve host".to_string())?;

    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false
    })
    .to_string();

    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))
        .map_err(|_| "connection failed".to_string())?;

    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|_| "failed to set read timeout".to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|_| "failed to set write timeout".to_string())?;

    let req = format!(
        "POST /api/generate HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        hostname,
        body.len(),
        body
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|_| "write failed".to_string())?;

    let mut raw = String::new();
    stream
        .take(1_048_576)
        .read_to_string(&mut raw)
        .map_err(|_| "read failed".to_string())?;

    // Validate HTTP status from the first line
    let first_line = raw
        .lines()
        .next()
        .ok_or_else(|| "empty response".to_string())?;
    parse_http_status(first_line)?;

    let (_, body_str) = raw.split_once("\r\n\r\n").ok_or("invalid http response")?;

    let v: serde_json::Value = serde_json::from_str(body_str).map_err(|e| e.to_string())?;
    let resp = v
        .get("response")
        .and_then(|x| x.as_str())
        .ok_or("missing response field")?;

    Ok(resp.to_string())
}

fn normalize_openai_path_prefix(prefix: &str) -> String {
    let p = prefix.trim();
    if p.is_empty() {
        return "/v1".to_string();
    }
    if p == "/" {
        return "/v1".to_string();
    }
    p.trim_end_matches('/').to_string()
}

fn build_openai_chat_path(prefix: &str) -> String {
    let base = normalize_openai_path_prefix(prefix);
    if base.ends_with("/chat/completions") {
        base
    } else {
        format!("{}/chat/completions", base)
    }
}

fn decode_chunked(body: &str) -> Result<String, String> {
    // Very small, non-streaming chunked decoder for JSON bodies.
    let mut out = String::new();
    let mut i = 0;
    let bytes = body.as_bytes();

    while i < bytes.len() {
        // Read chunk size line
        let mut j = i;
        while j + 1 < bytes.len() && !(bytes[j] == b'\r' && bytes[j + 1] == b'\n') {
            j += 1;
        }
        if j + 1 >= bytes.len() {
            return Err("invalid chunked encoding".to_string());
        }
        let size_line = &body[i..j];
        let size_hex = size_line.split(';').next().unwrap_or("").trim();
        let size =
            usize::from_str_radix(size_hex, 16).map_err(|_| "invalid chunk size".to_string())?;
        i = j + 2; // skip \r\n

        if size == 0 {
            break;
        }
        if i + size > bytes.len() {
            return Err("chunk exceeds body length".to_string());
        }
        out.push_str(&body[i..i + size]);
        i += size;

        // skip trailing \r\n
        if i + 1 < bytes.len() && bytes[i] == b'\r' && bytes[i + 1] == b'\n' {
            i += 2;
        }
    }

    Ok(out)
}

fn openai_extract_content(body_str: &str) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(body_str).map_err(|e| e.to_string())?;
    // Chat completions: choices[0].message.content
    if let Some(s) = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|x| x.as_str())
    {
        return Ok(s.to_string());
    }
    // Fallback: choices[0].text
    if let Some(s) = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("text"))
        .and_then(|x| x.as_str())
    {
        return Ok(s.to_string());
    }
    Err("missing choices[0] content".to_string())
}

/// Send a Chat Completions request to an OpenAI-compatible endpoint (LM Studio).
///
/// `base_url` should normally include `/v1` (for example: `http://127.0.0.1:1234/v1`).
pub fn openai_chat_generate(
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    prompt: &str,
) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let (hostname, port, prefix) = parse_http_url(base_url)?;
    let path = build_openai_chat_path(&prefix);

    let addr = (hostname.as_str(), port)
        .to_socket_addrs()
        .map_err(|_| "failed to resolve host".to_string())?
        .next()
        .ok_or_else(|| "failed to resolve host".to_string())?;

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "stream": false,
        "temperature": 0
    })
    .to_string();

    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))
        .map_err(|_| "connection failed".to_string())?;

    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|_| "failed to set read timeout".to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|_| "failed to set write timeout".to_string())?;

    let mut auth_header = String::new();
    if let Some(k) = api_key {
        let k = k.trim();
        if !k.is_empty() {
            auth_header = format!("Authorization: Bearer {}\r\n", k);
        }
    }

    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        path,
        hostname,
        auth_header,
        body.len(),
        body
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|_| "write failed".to_string())?;

    let mut raw = String::new();
    stream
        .take(2_097_152)
        .read_to_string(&mut raw)
        .map_err(|_| "read failed".to_string())?;

    let (hdrs, body_str) = raw.split_once("\r\n\r\n").ok_or("invalid http response")?;

    let first_line = hdrs
        .lines()
        .next()
        .ok_or_else(|| "empty response".to_string())?;
    parse_http_status(first_line)?;

    let is_chunked = hdrs.lines().any(|l| {
        l.to_ascii_lowercase().starts_with("transfer-encoding:")
            && l.to_ascii_lowercase().contains("chunked")
    });

    let final_body = if is_chunked {
        decode_chunked(body_str)?
    } else {
        body_str.to_string()
    };

    openai_extract_content(&final_body)
}

/// Generate a response using either Ollama or an OpenAI-compatible endpoint.
pub fn llm_generate(cfg: &OllamaConfig, prompt: &str) -> Result<String, String> {
    match cfg.api {
        LlmApi::Ollama => ollama_generate(&cfg.host, &cfg.model, prompt),
        LlmApi::OpenAiChatCompletions => {
            openai_chat_generate(&cfg.host, cfg.api_key.as_deref(), &cfg.model, prompt)
        }
    }
}

/// Choose among options using either Ollama or an OpenAI-compatible endpoint.
pub fn llm_choose(cfg: &OllamaConfig, prompt: &str, options_len: usize) -> Result<usize, String> {
    let response = llm_generate(cfg, prompt)?;
    extract_choice(&response, options_len)
}

/// Deliberate (comment + preferred choice) using either backend.
pub fn llm_deliberate(
    cfg: &OllamaConfig,
    personality: &str,
    event: &Event,
    galaxy: &GalaxyState,
) -> Result<(usize, String), String> {
    let prompt = build_deliberation_prompt(personality, event, galaxy);
    let response = llm_generate(cfg, &prompt)?;
    let choice = extract_choice(&response, event.options.len())?;
    let comment = extract_comment(&response).unwrap_or_else(|| "(no comment)".to_string());
    Ok((choice, comment))
}

/// Ask Ollama to choose among options. Returns a clamped index.
///
/// Uses multi-strategy parsing (JSON integer, JSON string, bare integer scan)
/// to extract the choice from the LLM response.
pub fn ollama_choose(
    host: &str,
    model: &str,
    prompt: &str,
    options_len: usize,
) -> Result<usize, String> {
    let response = ollama_generate(host, model, prompt)?;
    extract_choice(&response, options_len)
}

/// Ask Ollama to produce a short deliberation comment AND a preferred choice.
///
/// Returns `(choice, comment)`.
pub fn ollama_deliberate(
    host: &str,
    model: &str,
    personality: &str,
    event: &Event,
    galaxy: &GalaxyState,
) -> Result<(usize, String), String> {
    let prompt = build_deliberation_prompt(personality, event, galaxy);
    let response = ollama_generate(host, model, &prompt)?;
    let choice = extract_choice(&response, event.options.len())?;
    let comment = extract_comment(&response).unwrap_or_else(|| "(no comment)".to_string());
    Ok((choice, comment))
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

/// Build a deliberation prompt used to generate a short council statement.
///
/// The model should return ONLY JSON: {"choice": <int>, "comment": <short string>}.
pub fn build_deliberation_prompt(personality: &str, event: &Event, galaxy: &GalaxyState) -> String {
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
    s.push_str("Your task: publish a short deliberation statement for the council AND include your preferred option index.\n");
    s.push_str("Return ONLY a JSON object: {\"choice\": <integer>, \"comment\": <short string>}\n");
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

    s.push_str("\nConstraints for comment:\n");
    s.push_str("- Be concise (<= 200 characters).\n");
    s.push_str("- Reference risks/tradeoffs.\n");

    s
}

/// Extract a deliberation comment from an LLM response.
///
/// Looks for JSON {comment: "..."} or falls back to {reason: "..."}.
pub fn extract_comment(response: &str) -> Option<String> {
    let json_str = extract_first_json_object(response)?;
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    v.get("comment")
        .and_then(|c| c.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            v.get("reason")
                .and_then(|c| c.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
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

    // AC-1: parse_host() handles https:// prefix, empty string returns Err, port 0 valid
    #[test]
    fn test_parse_host_strips_https_prefix() {
        let (h, p) = parse_host("https://example.com:8080").unwrap();
        assert_eq!(h, "example.com");
        assert_eq!(p, 8080);
    }

    #[test]
    fn test_parse_host_empty_string_is_err() {
        assert!(parse_host("").is_err());
    }

    #[test]
    fn test_parse_host_port_zero() {
        let (h, p) = parse_host("localhost:0").unwrap();
        assert_eq!(h, "localhost");
        assert_eq!(p, 0);
    }

    // AC-4: HTTP status code validation
    #[test]
    fn test_parse_http_status_200_ok() {
        assert_eq!(parse_http_status("HTTP/1.1 200 OK").unwrap(), 200);
    }

    #[test]
    fn test_parse_http_status_404_err() {
        let err = parse_http_status("HTTP/1.1 404 Not Found").unwrap_err();
        assert!(err.contains("404"));
    }

    #[test]
    fn test_parse_http_status_500_err() {
        let err = parse_http_status("HTTP/1.1 500 Internal Server Error").unwrap_err();
        assert!(err.contains("500"));
    }

    #[test]
    fn test_parse_http_status_malformed() {
        assert!(parse_http_status("garbage").is_err());
    }

    // AC-5: Multi-strategy choice extraction
    #[test]
    fn test_extract_choice_integer() {
        assert_eq!(
            extract_choice("{\"choice\": 2, \"reason\": \"ok\"}", 4).unwrap(),
            2
        );
    }

    #[test]
    fn test_extract_choice_string() {
        assert_eq!(
            extract_choice("{\"choice\": \"2\", \"reason\": \"ok\"}", 4).unwrap(),
            2
        );
    }

    #[test]
    fn test_extract_choice_bare_integer() {
        assert_eq!(extract_choice("I pick option 2 because", 4).unwrap(), 2);
    }

    #[test]
    fn test_extract_choice_clamped() {
        assert_eq!(extract_choice("{\"choice\": 99}", 3).unwrap(), 2);
    }

    #[test]
    fn test_extract_choice_empty_err() {
        assert!(extract_choice("", 3).is_err());
    }

    // AC-6: can_connect() moved to council-core
    #[test]
    fn test_can_connect_unreachable() {
        assert!(!can_connect("192.0.2.1:1"));
    }
}
