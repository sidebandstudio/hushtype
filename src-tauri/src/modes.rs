use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use specta::Type;
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct ContextFields {
    pub app: bool,
    pub window_title: bool,
    pub url: bool,
    pub selected_text: bool,
    pub clipboard: bool,
}

impl Default for ContextFields {
    fn default() -> Self {
        Self {
            app: true,
            window_title: true,
            url: true,
            selected_text: false,
            clipboard: false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PromptBehavior {
    #[default]
    Append,
    Replace,
}

/// How a mode formats text, after cleanup. A style changes capitalization and
/// punctuation only, never the words.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    /// Capitals and full punctuation.
    #[default]
    Serious,
    /// Capitals, without the closing period.
    Casual,
    /// Lowercase sentence starts and "i", without the closing period.
    VeryCasual,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Mode {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub bundle_ids: Vec<String>,
    #[serde(default)]
    pub url_globs: Vec<String>,
    pub refine_enabled: bool,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub prompt_behavior: PromptBehavior,
    #[serde(default)]
    pub context: ContextFields,
    #[serde(default)]
    pub style: Style,
}

#[derive(Clone, Default)]
pub struct CapturedContext {
    pub app_name: Option<String>,
    pub bundle_id: Option<String>,
    pub window_title: Option<String>,
    pub url: Option<String>,
    pub selected_text: Option<String>,
    pub clipboard: Option<String>,
}

#[derive(Clone)]
pub struct ResolvedContext {
    pub mode: Mode,
    pub context: CapturedContext,
}

impl Default for ResolvedContext {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            context: CapturedContext::default(),
        }
    }
}

pub fn default_mode() -> Mode {
    Mode {
        id: "default".into(),
        name: "Default".into(),
        bundle_ids: vec![],
        url_globs: vec![],
        refine_enabled: true,
        prompt: String::new(),
        prompt_behavior: PromptBehavior::Append,
        context: ContextFields::default(),
        style: Style::Serious,
    }
}

pub fn default_modes() -> Vec<Mode> {
    let make =
        |id: &str, name: &str, bundles: &[&str], urls: &[&str], refine_enabled, prompt: &str| {
            Mode {
                id: id.into(),
                name: name.into(),
                bundle_ids: bundles.iter().map(|s| s.to_string()).collect(),
                url_globs: urls.iter().map(|s| s.to_string()).collect(),
                refine_enabled,
                prompt: prompt.into(),
                style: if id == "messaging" {
                    Style::Casual
                } else {
                    Style::Serious
                },
                ..default_mode()
            }
        };
    vec![
        make("messaging", "Messaging", &["com.tinyspeck.slackmacgap", "com.apple.MobileSMS", "com.hnc.Discord", "ru.keepcoder.Telegram", "org.telegram.desktop", "net.whatsapp.WhatsApp"],
            &["*://app.slack.com/*", "*://*.slack.com/*", "*://discord.com/*", "*://web.telegram.org/*", "*://web.whatsapp.com/*"], true,
            "This is a casual message. Use natural contractions: I am becomes I'm, I will becomes I'll, and do not becomes don't. Keep one short paragraph without dropping meaning. Do not add greetings or sign-offs."),
        make("email", "Email", &["com.apple.mail", "com.microsoft.Outlook", "com.superhuman.Superhuman"],
            &["*://mail.google.com/*", "*://outlook.live.com/*", "*://outlook.office.com/*", "*://outlook.office365.com/*", "*://mail.superhuman.com/*"], true,
            "This is email text. Use complete sentences and readable paragraphs. Remove filler. Preserve the speaker's degree of formality. Do not add a greeting or signature."),
        make("code", "Code", &["com.todesktop.230313mzl4w4u92", "com.microsoft.VSCode", "com.apple.dt.Xcode", "com.googlecode.iterm2", "com.apple.Terminal", "dev.warp.Warp-Stable", "com.mitchellh.ghostty"], &[], false,
            "This is text for a code editor or terminal. Preserve literal tokens, identifiers, paths, symbols, capitalization, and number notation exactly. Do not correct identifiers or translate words into code. Do not append punctuation to a command."),
        default_mode(),
    ]
}

fn glob_regex(pattern: &str) -> Result<regex::Regex, regex::Error> {
    let escaped = regex::escape(pattern)
        .replace(r"\*", ".*")
        .replace(r"\?", ".");
    regex::Regex::new(&format!("^{escaped}$"))
}

pub fn match_mode(modes: &[Mode], context: &CapturedContext) -> Mode {
    modes
        .iter()
        .find(|mode| {
            mode.id != "default"
                && (context
                    .bundle_id
                    .as_ref()
                    .is_some_and(|id| mode.bundle_ids.iter().any(|b| b.eq_ignore_ascii_case(id)))
                    || context.url.as_ref().is_some_and(|url| {
                        mode.url_globs
                            .iter()
                            .any(|g| glob_regex(g).is_ok_and(|r| r.is_match(url)))
                    }))
        })
        .cloned()
        .or_else(|| modes.iter().find(|m| m.id == "default").cloned())
        .unwrap_or_else(default_mode)
}

pub fn validate_modes(modes: &[Mode]) -> Result<(), String> {
    if modes.is_empty() || modes.len() > 32 || modes.last().is_none_or(|m| m.id != "default") {
        return Err("Keep between 1 and 32 modes, with Default last.".into());
    }
    let mut ids = HashSet::new();
    for mode in modes {
        if mode.id.is_empty()
            || mode.id.len() > 64
            || !ids.insert(&mode.id)
            || mode.name.trim().is_empty()
            || mode.name.len() > 80
        {
            return Err("Modes need unique IDs and names of 1 to 80 bytes.".into());
        }
        if mode.prompt.len() > 16_384
            || (mode.prompt_behavior == PromptBehavior::Replace && mode.prompt.trim().is_empty())
        {
            return Err("Prompts must fit in 16 KiB; a replacement prompt cannot be empty.".into());
        }
        if mode.id == "default" {
            if !mode.bundle_ids.is_empty() || !mode.url_globs.is_empty() || mode.name != "Default" {
                return Err("Default must keep its name and match everything.".into());
            }
        } else if mode.bundle_ids.is_empty() && mode.url_globs.is_empty() {
            return Err("Each non-default mode needs a bundle ID or URL glob.".into());
        }
        if mode.bundle_ids.len() + mode.url_globs.len() > 100
            || mode
                .bundle_ids
                .iter()
                .any(|b| b.is_empty() || b.len() > 255)
            || mode
                .url_globs
                .iter()
                .any(|g| g.is_empty() || g.len() > 2048 || glob_regex(g).is_err())
        {
            return Err("Use at most 100 valid matchers per mode.".into());
        }
    }
    Ok(())
}

impl ResolvedContext {
    /// Context is kept out of the instructions so the instructions and
    /// examples stay a stable prefix that servers can cache between dictations.
    pub fn prompt(&self, base: &str) -> String {
        let prompt = if self.mode.prompt_behavior == PromptBehavior::Replace {
            self.mode.prompt.as_str()
        } else {
            base
        };
        let (instructions, examples) = prompt.split_once("\nTranscript: ").unwrap_or((prompt, ""));
        let mut result = instructions.to_string();
        if self.mode.prompt_behavior == PromptBehavior::Append
            && !self.mode.prompt.trim().is_empty()
        {
            result.push_str(&format!("\n\nMode instructions: {}", self.mode.prompt));
        }
        if !examples.is_empty() {
            result.push_str(&format!("\nTranscript: {examples}"));
        }
        result
    }

    /// The captured fields this mode allows sending, as untrusted reference data.
    pub fn context_data(&self) -> Option<Value> {
        let mut data = Map::new();
        let fields = &self.mode.context;
        let context = &self.context;
        for (enabled, key, value) in [
            (fields.app, "app_name", &context.app_name),
            (fields.app, "bundle_id", &context.bundle_id),
            (fields.window_title, "window_title", &context.window_title),
            (fields.url, "url", &context.url),
            (
                fields.selected_text,
                "selected_text",
                &context.selected_text,
            ),
            (fields.clipboard, "clipboard", &context.clipboard),
        ] {
            if enabled {
                if let Some(value) = value {
                    data.insert(key.into(), json!(value));
                }
            }
        }
        (!data.is_empty()).then_some(Value::Object(data))
    }
}

/// Formats cleaned text for a style. Words are never added, removed, or
/// reordered, so this also applies when cleanup is off or falls back to raw.
pub fn apply_style(text: &str, style: Style, dictionary: &[String]) -> String {
    let mut text = match style {
        Style::Serious => return text.to_string(),
        Style::Casual => text.to_string(),
        Style::VeryCasual => lowercase_casual(text, dictionary),
    };
    // Drop only a single closing period, never an ellipsis or other marks.
    let end = text.trim_end().len();
    if text[..end].ends_with('.') && !text[..end].ends_with("..") {
        text.remove(end - 1);
    }
    text
}

/// Lowercases "I" and ordinary words that start a sentence. Words that look
/// like names keep their capital: dictionary terms, words also capitalized
/// mid-sentence, and a word followed by another capitalized word ("Mac Mini").
fn lowercase_casual(text: &str, dictionary: &[String]) -> String {
    let word_pattern = regex::Regex::new(r"\p{L}[\p{L}'’]*").expect("valid word pattern");
    let words: Vec<_> = word_pattern.find_iter(text).collect();
    let sentence_start = |start: usize| {
        text[..start]
            .trim_end_matches(|c: char| c.is_whitespace() || "\"'“‘([".contains(c))
            .chars()
            .last()
            .is_none_or(|c| ".!?".contains(c))
            || text[..start].trim_end_matches(' ').ends_with('\n')
    };
    let ordinary = |word: &str| {
        let mut chars = word.chars();
        chars.next().is_some_and(char::is_uppercase) && chars.all(|c| !c.is_uppercase())
    };
    let pronoun_i = |word: &str| {
        word == "I"
            || (["I'", "I’"].iter().any(|p| word.starts_with(p)) && word.chars().count() <= 4)
    };
    let names: HashSet<&str> = words
        .iter()
        .filter(|w| !sentence_start(w.start()) && ordinary(w.as_str()) && !pronoun_i(w.as_str()))
        .map(|w| w.as_str())
        .chain(dictionary.iter().flat_map(|d| d.split_whitespace()))
        .collect();
    let mut result = text.to_string();
    for (index, word) in words.iter().enumerate() {
        let value = word.as_str();
        let starts_name = words.get(index + 1).is_some_and(|next| {
            &text[word.end()..next.start()] == " "
                && ordinary(next.as_str())
                && !pronoun_i(next.as_str())
        });
        let lower = pronoun_i(value)
            || (sentence_start(word.start())
                && ordinary(value)
                && !names.contains(value)
                && !starts_name);
        if lower {
            // Byte offsets stay valid only for same-length lowercase forms;
            // the rare letters without one keep their capital.
            let first = value.chars().next().unwrap_or_default();
            let replacement: String = first.to_lowercase().collect();
            if replacement.len() == first.len_utf8() {
                result.replace_range(word.start()..word.start() + first.len_utf8(), &replacement);
            }
        }
    }
    result
}

/// The optional readers are only invoked after matching and explicit opt-in.
pub fn capture_optional(
    context: &mut CapturedContext,
    fields: &ContextFields,
    selected: impl FnOnce() -> Option<String>,
    clipboard: impl FnOnce() -> Option<String>,
) {
    if fields.selected_text {
        context.selected_text = selected().map(|s| s.chars().take(4000).collect());
    }
    if fields.clipboard {
        context.clipboard = clipboard().map(|s| s.chars().take(4000).collect());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_match_apps_and_browser_urls() {
        let modes = default_modes();
        validate_modes(&modes).unwrap();
        for (bundle, url, expected) in [
            ("com.tinyspeck.slackmacgap", None, "messaging"),
            ("com.apple.mail", None, "email"),
            ("com.todesktop.230313mzl4w4u92", None, "code"),
            (
                "com.google.Chrome",
                Some("https://mail.google.com/mail/u/0/"),
                "email",
            ),
            (
                "com.apple.Safari",
                Some("https://app.slack.com/client/team"),
                "messaging",
            ),
            ("unknown", None, "default"),
            (
                "com.apple.Safari",
                Some("https://mail.google.com.evil.test/"),
                "default",
            ),
        ] {
            assert_eq!(
                match_mode(
                    &modes,
                    &CapturedContext {
                        bundle_id: Some(bundle.into()),
                        url: url.map(str::to_string),
                        ..Default::default()
                    }
                )
                .id,
                expected
            );
        }
        assert!(!modes[2].refine_enabled);
    }
    #[test]
    fn first_match_wins_and_default_cannot_disappear() {
        let mut modes = default_modes();
        modes[0].bundle_ids.push("com.apple.mail".into());
        assert_eq!(
            match_mode(
                &modes,
                &CapturedContext {
                    bundle_id: Some("com.apple.mail".into()),
                    ..Default::default()
                }
            )
            .id,
            "messaging"
        );
        modes.pop();
        assert!(validate_modes(&modes).is_err());
    }
    #[test]
    fn privacy_defaults_do_not_even_call_optional_readers() {
        capture_optional(
            &mut CapturedContext::default(),
            &ContextFields::default(),
            || panic!("selected text read"),
            || panic!("clipboard read"),
        );
        let mut context = CapturedContext::default();
        capture_optional(
            &mut context,
            &ContextFields {
                selected_text: true,
                ..Default::default()
            },
            || Some("selected".into()),
            || panic!("clipboard read"),
        );
        assert_eq!(context.selected_text.as_deref(), Some("selected"));
        context.app_name = Some("Notes".into());
        let resolved = ResolvedContext {
            context,
            ..Default::default()
        };
        let data = resolved.context_data().unwrap();
        assert_eq!(data["app_name"], "Notes");
        assert!(data.get("selected_text").is_none());
        assert!(!resolved.prompt("Clean.").contains("Notes"));
    }
    #[test]
    fn addendum_precedes_examples_and_override_replaces_instructions() {
        let mut resolved = ResolvedContext::default();
        resolved.mode.prompt = "Casual.".into();
        let text = resolved.prompt("Clean.\nTranscript: hello\nCleaned: Hello.");
        assert!(text.find("Casual.").unwrap() < text.find("Transcript:").unwrap());
        resolved.mode.prompt_behavior = PromptBehavior::Replace;
        assert_eq!(resolved.prompt("Original."), "Casual.");
    }
    #[test]
    fn styles_change_only_capitals_and_the_closing_period() {
        let text = "I'm not gonna lie, it's pretty useful. So you don't need internet on my Mac. Mac Mini works too.";
        assert_eq!(apply_style(text, Style::Serious, &[]), text);
        assert_eq!(
            apply_style(text, Style::Casual, &[]),
            "I'm not gonna lie, it's pretty useful. So you don't need internet on my Mac. Mac Mini works too"
        );
        assert_eq!(
            apply_style(text, Style::VeryCasual, &[]),
            "i'm not gonna lie, it's pretty useful. so you don't need internet on my Mac. Mac Mini works too"
        );
        for (style, input, expected) in [
            (
                Style::Casual,
                "Are you free tomorrow? Let's do 12.",
                "Are you free tomorrow? Let's do 12",
            ),
            (Style::Casual, "I guess...", "I guess..."),
            (Style::Casual, "Ship it!", "Ship it!"),
            (
                Style::VeryCasual,
                "Are you free tomorrow? Let's do 12.",
                "are you free tomorrow? let's do 12",
            ),
            (
                Style::VeryCasual,
                "The API is ready. I’ll merge it.",
                "the API is ready. i’ll merge it",
            ),
            (
                Style::VeryCasual,
                "Okay.\nSounds good, I think.",
                "okay.\nsounds good, i think",
            ),
            (
                Style::VeryCasual,
                "Wispr Flow is nice. Maya agrees.",
                "Wispr Flow is nice. Maya agrees",
            ),
            (
                Style::VeryCasual,
                "\"Really?\" Yes, OK.",
                "\"really?\" yes, OK",
            ),
        ] {
            assert_eq!(apply_style(input, style, &["Maya".into()]), expected);
        }
        let modes = default_modes();
        assert_eq!(modes[0].style, Style::Casual);
        assert!(modes[1..].iter().all(|m| m.style == Style::Serious));
    }
}
