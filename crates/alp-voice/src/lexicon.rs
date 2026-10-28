//! Pronunciation lexicon and pause/emphasis markup processing.

use std::collections::HashMap;

/// An element of parsed speech script containing text or control markers.
#[derive(Debug, Clone, PartialEq)]
pub enum SpeechToken {
    /// Normal spoken word or phrase.
    Words(String),
    /// Explicit pause in milliseconds.
    PauseMs(u32),
    /// Emphasized spoken section.
    Emphasis(String),
}

/// Pronunciation dictionary providing word-level phonetic respelling.
#[derive(Debug, Clone, Default)]
pub struct PronunciationDictionary {
    overrides: HashMap<String, String>,
}

impl PronunciationDictionary {
    /// Create an empty pronunciation dictionary.
    #[must_use]
    pub fn new() -> Self {
        Self {
            overrides: HashMap::new(),
        }
    }

    /// Add a custom pronunciation replacement.
    pub fn add(&mut self, word: impl Into<String>, pronunciation: impl Into<String>) {
        self.overrides
            .insert(word.into().to_lowercase(), pronunciation.into());
    }

    /// Look up a word replacement.
    #[must_use]
    pub fn lookup(&self, word: &str) -> Option<&str> {
        self.overrides.get(&word.to_lowercase()).map(String::as_str)
    }

    /// Remove a word from the dictionary.
    pub fn remove(&mut self, word: &str) -> bool {
        self.overrides.remove(&word.to_lowercase()).is_some()
    }

    /// Apply dictionary substitutions to a text string.
    #[must_use]
    pub fn apply(&self, text: &str) -> String {
        let mut words = Vec::new();
        for word in text.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
            if let Some(replacement) = self.lookup(clean) {
                words.push(word.replace(clean, replacement));
            } else {
                words.push(word.to_string());
            }
        }
        words.join(" ")
    }
}

/// Parser for inline speech markup (e.g. `[pause=500ms]`, `[emphasis]word[/emphasis]`).
pub struct MarkupParser;

impl MarkupParser {
    /// Parse marked-up text into structured speech tokens.
    #[must_use]
    pub fn parse(input: &str) -> Vec<SpeechToken> {
        let mut tokens = Vec::new();
        let mut remaining = input;

        while !remaining.is_empty() {
            if let Some(start_idx) = remaining.find('[') {
                if start_idx > 0 {
                    let text = remaining[..start_idx].trim();
                    if !text.is_empty() {
                        tokens.push(SpeechToken::Words(text.to_string()));
                    }
                }

                let tag_rest = &remaining[start_idx..];
                if let Some(end_idx) = tag_rest.find(']') {
                    let tag = &tag_rest[1..end_idx];
                    remaining = &tag_rest[end_idx + 1..];

                    if let Some(ms_str) = tag.strip_prefix("pause=") {
                        let ms = parse_duration_ms(ms_str);
                        tokens.push(SpeechToken::PauseMs(ms));
                    } else if tag == "emphasis"
                        && let Some(close_idx) = remaining.find("[/emphasis]")
                    {
                        let content = &remaining[..close_idx];
                        tokens.push(SpeechToken::Emphasis(content.trim().to_string()));
                        remaining = &remaining[close_idx + 11..];
                    }
                } else {
                    tokens.push(SpeechToken::Words(remaining.to_string()));
                    break;
                }
            } else {
                let text = remaining.trim();
                if !text.is_empty() {
                    tokens.push(SpeechToken::Words(text.to_string()));
                }
                break;
            }
        }

        tokens
    }
}

fn parse_duration_ms(s: &str) -> u32 {
    let s = s.trim();
    if let Some(val) = s.strip_suffix("ms") {
        val.parse().unwrap_or(250)
    } else if let Some(val) = s.strip_suffix('s') {
        let secs: f32 = val.parse().unwrap_or(1.0);
        (secs * 1000.0) as u32
    } else {
        s.parse().unwrap_or(250)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_replaces_words_case_insensitively() {
        let mut dict = PronunciationDictionary::new();
        dict.add("ALP", "Audio Labs Pro");
        dict.add("SQL", "sequel");

        let input = "Welcome to alp with sql database.";
        let output = dict.apply(input);
        assert_eq!(output, "Welcome to Audio Labs Pro with sequel database.");
    }

    #[test]
    fn markup_parser_extracts_pauses_and_emphasis() {
        let input = "Intro [pause=500ms] middle [emphasis]important point[/emphasis] conclusion.";
        let tokens = MarkupParser::parse(input);

        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0], SpeechToken::Words("Intro".into()));
        assert_eq!(tokens[1], SpeechToken::PauseMs(500));
        assert_eq!(tokens[2], SpeechToken::Words("middle".into()));
        assert_eq!(tokens[3], SpeechToken::Emphasis("important point".into()));
        assert_eq!(tokens[4], SpeechToken::Words("conclusion.".into()));
    }
}
