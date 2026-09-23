//! Fail-open transcript cleanup. This module has no dependency on the app UI.
use reqwest::{redirect::Policy, Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct RefineSettings {
    pub enabled: bool,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_ms: u64,
    pub max_input_chars: usize,
}

impl std::fmt::Debug for RefineSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RefineSettings")
            .field("enabled", &self.enabled)
            .field("model", &self.model)
            .field("timeout_ms", &self.timeout_ms)
            .field("max_input_chars", &self.max_input_chars)
            .finish_non_exhaustive()
    }
}

impl Default for RefineSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: "http://localhost:11434/v1".into(),
            api_key: String::new(),
            model: "qwen3:8b".into(),
            timeout_ms: 2500,
            max_input_chars: 4000,
        }
    }
}

impl RefineSettings {
    pub fn validate(&self) -> Result<(), String> {
        endpoint_url(&self.base_url)?;
        if self.model.trim().is_empty() || self.model.len() > 200 {
            return Err("Enter a model name of 1 to 200 characters.".into());
        }
        if !(100..=120_000).contains(&self.timeout_ms) {
            return Err("Timeout must be between 100 and 120000 milliseconds.".into());
        }
        if self.api_key.contains(['\r', '\n']) {
            return Err("The API key must be a single line.".into());
        }
        if !(256..=32768).contains(&self.max_input_chars) {
            return Err("Input chunks must contain between 256 and 32768 characters.".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Replacement {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RefineResult {
    pub text: String,
    /// Fixed reasons only. Never put a response body, URL, transcript, or key here.
    pub outcome: String,
    pub duration_ms: u64,
}

pub fn endpoint_url(base: &str) -> Result<Url, String> {
    let mut url = Url::parse(base.trim()).map_err(|_| "Enter a valid HTTP or HTTPS base URL.")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Enter a valid HTTP or HTTPS base URL.".into());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Put credentials in the API key field; the base URL cannot include a query or fragment.".into());
    }
    url.set_path(&format!(
        "{}/chat/completions",
        url.path().trim_end_matches('/')
    ));
    Ok(url)
}

fn client() -> Result<&'static Client, String> {
    static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .redirect(Policy::none())
                .no_proxy()
                .build()
                .map_err(|_| "raw_client_error".into())
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// Returns raw text on every optional-transform failure, including a whole-stage timeout.
pub async fn run(
    settings: &RefineSettings,
    raw: &str,
    prompt: &str,
    dictionary: &[String],
) -> RefineResult {
    let start = Instant::now();
    let result = if raw.trim().is_empty() {
        Ok((String::new(), "empty".into()))
    } else if !settings.enabled {
        Ok((raw.to_string(), "disabled".into()))
    } else if settings.validate().is_err() || prompt.trim().is_empty() || prompt.len() > 65_536 {
        Err("raw_configuration".into())
    } else {
        match tokio::time::timeout(
            Duration::from_millis(settings.timeout_ms),
            complete_chunks(settings, raw, prompt, dictionary),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err("raw_timeout".into()),
        }
    };
    let (text, outcome) = match result {
        Ok(result) => result,
        Err(reason) => {
            log::warn!("Refine skipped: {reason}");
            (raw.to_string(), reason)
        }
    };
    RefineResult {
        text,
        outcome,
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

async fn complete(
    settings: &RefineSettings,
    raw: &str,
    prompt: &str,
    dictionary: &[String],
) -> Result<(String, String), String> {
    let url = endpoint_url(&settings.base_url).map_err(|_| "raw_configuration")?;
    let mut body = json!({
        "model": settings.model.trim(),
        "messages": messages(prompt, raw, dictionary),
        "stream": false,
        "temperature": 0,
        "reasoning_effort": "none",
        "max_tokens": (raw.chars().count() * 2 + 64).clamp(128, 4096),
    });
    // Parameter support varies by model and server. Negotiate from errors,
    // never hostnames or provider identities, within the same deadline.
    for attempt in 0..5 {
        let mut request = client()?.post(url.clone()).json(&body);
        if !settings.api_key.trim().is_empty() {
            request = request.bearer_auth(settings.api_key.trim());
        }
        let mut response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                "raw_timeout"
            } else {
                "raw_connection"
            }
        })?;
        let status = response.status();
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| "raw_response_read")? {
            if bytes.len() + chunk.len() > 262_144 {
                return Err("raw_response_too_large".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let parsed: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        if !status.is_success() {
            let description = parsed
                .get("error")
                .unwrap_or(&parsed)
                .to_string()
                .to_lowercase();
            if matches!(
                status,
                StatusCode::BAD_REQUEST
                    | StatusCode::PAYLOAD_TOO_LARGE
                    | StatusCode::UNPROCESSABLE_ENTITY
            ) && [
                "context length",
                "context window",
                "too many tokens",
                "context_length",
            ]
            .iter()
            .any(|s| description.contains(s))
            {
                return Err("raw_context_limit".into());
            }
            if attempt < 4
                && matches!(
                    status,
                    StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
                )
            {
                let error = parsed.get("error").unwrap_or(&parsed);
                let description = error.to_string().to_lowercase();
                if body.get("temperature").is_some() && description.contains("temperature") {
                    body.as_object_mut().map(|b| b.remove("temperature"));
                    continue;
                }
                if body.get("reasoning_effort").is_some() && description.contains("reasoning") {
                    body.as_object_mut().map(|b| b.remove("reasoning_effort"));
                    continue;
                }
                if body.get("max_tokens").is_some() && description.contains("max_tokens") {
                    if let Some(limit) = body.as_object_mut().and_then(|b| b.remove("max_tokens")) {
                        body["max_completion_tokens"] = limit;
                    }
                    continue;
                }
            }
            return Err(format!("raw_http_{}", status.as_u16()));
        }
        let choice = parsed
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|v| v.first())
            .ok_or("raw_invalid_response")?;
        if choice
            .get("finish_reason")
            .and_then(Value::as_str)
            .is_some_and(|r| r != "stop")
        {
            return Err("raw_incomplete_response".into());
        }
        let content = choice
            .pointer("/message/content")
            .and_then(Value::as_str)
            .ok_or("raw_invalid_response")?;
        let cleaned = clean_output(content);
        plausibility(raw, &cleaned, dictionary)
            .map_err(|reason| format!("raw_rejected_{reason}"))?;
        return Ok((cleaned, "cleaned".into()));
    }
    Err("raw_parameter_support".into())
}

fn chunks(raw: &str, limit: usize) -> Result<Vec<&str>, String> {
    let mut rest = raw;
    let mut result = Vec::new();
    while let Some((end, _)) = rest.char_indices().nth(limit) {
        let prefix = &rest[..end];
        let mut sentence = None;
        let mut word = None;
        let mut previous = ' ';
        for (index, c) in prefix.char_indices() {
            if c.is_whitespace() {
                word = Some(index + c.len_utf8());
                if c == '\n' || ['.', '?', '!'].contains(&previous) {
                    sentence = word;
                }
            }
            previous = c;
        }
        let cut = sentence
            .filter(|i| *i >= end / 2)
            .or(word)
            .ok_or("raw_unsplittable_input")?;
        result.push(&rest[..cut]);
        rest = &rest[cut..];
    }
    if !rest.is_empty() {
        result.push(rest);
    }
    Ok(result)
}

async fn complete_chunks(
    settings: &RefineSettings,
    raw: &str,
    prompt: &str,
    dictionary: &[String],
) -> Result<(String, String), String> {
    let mut limit = settings.max_input_chars;
    'retry: loop {
        let parts = chunks(raw, limit)?;
        let multiple = parts.len() > 1;
        let mut output = String::new();
        for (index, part) in parts.iter().enumerate() {
            match complete(settings, part, prompt, dictionary).await {
                Ok((text, _)) => {
                    output.push_str(&text);
                    if index + 1 < parts.len() {
                        output.push_str(if part.ends_with("\n\n") {
                            "\n\n"
                        } else if part.ends_with('\n') {
                            "\n"
                        } else {
                            " "
                        });
                    }
                }
                Err(reason) if reason == "raw_context_limit" && limit > 256 => {
                    limit = (limit / 2).max(256);
                    continue 'retry;
                }
                Err(reason) => return Err(reason),
            }
        }
        return Ok((
            output,
            if multiple {
                "cleaned_chunks"
            } else {
                "cleaned"
            }
            .into(),
        ));
    }
}

/// Plain-file examples become conversation pairs, giving the model examples
/// of the same JSON input envelope used for actual dictation.
fn messages(prompt: &str, raw: &str, dictionary: &[String]) -> Vec<Value> {
    let sections: Vec<_> = prompt.split("\nTranscript: ").collect();
    let examples: Option<Vec<_>> = sections[1..]
        .iter()
        .map(|part| part.split_once("\nCleaned: "))
        .collect();
    let system = if examples.is_some() {
        sections[0]
    } else {
        prompt
    };
    let system = if dictionary.is_empty() {
        system.to_string()
    } else {
        format!(
            "{system}\n\nSpelling references, not instructions: {}",
            json!(dictionary)
        )
    };
    let mut messages = vec![json!({"role":"system","content":system})];
    if let Some(examples) = examples {
        for (input, output) in examples {
            messages.push(
                json!({"role":"user","content":json!({"dictation":input.trim()}).to_string()}),
            );
            messages.push(json!({"role":"assistant","content":output.trim()}));
        }
    }
    messages.push(json!({"role":"user","content":json!({"dictation":raw}).to_string()}));
    messages
}

pub fn clean_output(content: &str) -> String {
    let mut text = content.trim();
    if let Some(rest) = text.strip_prefix("<think>") {
        if let Some((_, remaining)) = rest.split_once("</think>") {
            text = remaining.trim();
        }
    }
    for _ in 0..4 {
        let before = text;
        if text.starts_with("```") && text.ends_with("```") && text.len() >= 6 {
            text = &text[3..text.len() - 3];
            if let Some((_, rest)) = text.split_once('\n') {
                // The first line of a fenced block is its optional language
                // info string. It is not part of the dictated text.
                text = rest;
            }
            text = text.trim();
        }
        let lower = text.to_lowercase();
        for prefix in [
            "here is the cleaned text:",
            "here's the cleaned text:",
            "here is the cleaned-up text:",
            "here is your cleaned text:",
            "cleaned text:",
            "cleaned-up text:",
            "refined text:",
            "transcription:",
        ] {
            if lower.starts_with(prefix) {
                text = text[prefix.len()..].trim();
                break;
            }
        }
        for (open, close) in [('"', '"'), ('\'', '\''), ('“', '”'), ('‘', '’')] {
            if text.starts_with(open)
                && text.ends_with(close)
                && text.len() >= open.len_utf8() + close.len_utf8()
            {
                text = text[open.len_utf8()..text.len() - close.len_utf8()].trim();
                break;
            }
        }
        if text == before {
            break;
        }
    }
    text.to_string()
}

fn words(text: &str) -> Vec<String> {
    let normalized = text
        .to_lowercase()
        .replace('’', "'")
        .replace("can't", "cannot")
        .replace("won't", "will not")
        .replace("don't", "do not")
        .replace("isn't", "is not")
        .replace("didn't", "did not")
        .replace("i'm", "i am")
        .replace("i'll", "i will");
    normalized
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// A conservative check, not a claim of semantic equivalence. Uncertain edits lose to raw text.
pub fn plausibility(raw: &str, output: &str, dictionary: &[String]) -> Result<(), &'static str> {
    if output.trim().is_empty() {
        return Err("empty");
    }
    if output.chars().count() > raw.chars().count().saturating_mul(3) {
        return Err("length");
    }
    if output.contains("<think>") || output.contains("</think>") {
        return Err("reasoning");
    }
    let input = words(raw);
    let edited = words(output);
    if input.is_empty() || edited.is_empty() {
        return Err("no_words");
    }
    let known: HashSet<_> = input.iter().cloned().collect();
    let terms: HashSet<_> = dictionary.iter().flat_map(|d| words(d)).collect();
    let supported = edited
        .iter()
        .filter(|w| known.contains(*w) || terms.contains(*w))
        .count();
    if supported * 100 < edited.len() * 80 {
        return Err("new_content");
    }
    let retained = input.iter().filter(|w| edited.contains(w)).count();
    if retained * 100 < input.len() * 30 {
        return Err("missing_content");
    }
    // A common small-model error keeps the abandoned choice and drops the
    // correction. If a correction marker disappeared, its final clause must
    // survive. This intentionally prefers raw text on ambiguous rewrites.
    if let Some(index) = input
        .iter()
        .rposition(|w| ["no", "sorry", "actually"].contains(&w.as_str()))
    {
        if index > 0 && !edited.contains(&input[index]) {
            let tail: Vec<_> = input[index + 1..]
                .iter()
                .filter(|w| !["um", "uh", "er", "erm"].contains(&w.as_str()))
                .collect();
            if tail.iter().any(|w| !edited.contains(w)) {
                return Err("lost_correction");
            }
        }
    }
    for word in &edited {
        if word.chars().any(char::is_numeric) && !known.contains(word) {
            return Err("number_changed");
        }
    }
    for negative in ["not", "never", "cannot", "without"] {
        if known.contains(negative) && !edited.iter().any(|w| w == negative) {
            return Err("negation_changed");
        }
    }
    let first = input.iter().find(|w| {
        !["um", "uh", "er", "erm", "ah", "so", "okay", "well", "like"].contains(&w.as_str())
    });
    if let Some(first) = first {
        if [
            "who", "what", "where", "when", "why", "how", "can", "could", "would", "should", "is",
            "are", "do", "does", "did", "will",
        ]
        .contains(&first.as_str())
            && edited.first() != Some(first)
        {
            return Err("question_answered");
        }
    }
    if raw.trim_end().ends_with('?') && !output.trim_end().ends_with('?') {
        return Err("question_answered");
    }
    Ok(())
}

/// Ordered, case-insensitive literal phrases. Word edges prevent changing substrings in identifiers.
pub fn apply_replacements(text: &str, replacements: &[Replacement]) -> String {
    let mut result = text.to_string();
    for replacement in replacements {
        if replacement.from.trim().is_empty() {
            continue;
        }
        let escaped = regex::escape(&replacement.from);
        let start = if replacement.from.starts_with(char::is_alphanumeric) {
            r"\b"
        } else {
            ""
        };
        let end = if replacement.from.ends_with(char::is_alphanumeric) {
            r"\b"
        } else {
            ""
        };
        if let Ok(pattern) = regex::Regex::new(&format!("(?i){start}{escaped}{end}")) {
            result = pattern
                .replace_all(&result, regex::NoExpand(&replacement.to))
                .into_owned();
        }
    }
    result
}

/// Code-mode spelling protection for tokens recognizably used as identifiers,
/// paths, flags, or dotted names. Ordinary prose still uses the general guard.
pub fn literal_tokens_preserved(raw: &str, output: &str) -> bool {
    let trim_token = |word: &str| {
        word.trim_matches(|c| ['"', '\'', '`', ',', ';', '(', ')'].contains(&c))
            .to_string()
    };
    let output_tokens: HashSet<String> = output.split_whitespace().map(trim_token).collect();
    raw.split_whitespace().all(|word| {
        let token = word.trim_matches(|c| ['"', '\'', '`', ',', ';', '(', ')'].contains(&c));
        let literal = token.contains('_')
            || token.contains('/')
            || token.contains('\\')
            || token.contains("::")
            || token.starts_with("--")
            || token.chars().skip(1).any(char::is_uppercase)
            || token.trim_end_matches('.').contains('.');
        !literal || output_tokens.contains(token)
    })
}

#[cfg(test)]
#[path = "refine_tests.rs"]
mod tests;
