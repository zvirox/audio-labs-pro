//! Text normalization and sentence/paragraph chunking for speech synthesis.

/// Normalizes raw English text into spoken form (numbers, currencies, abbreviations).
pub struct TextNormalizer;

impl TextNormalizer {
    /// Normalize an input text string into pronounceable spoken English.
    #[must_use]
    pub fn normalize(text: &str) -> String {
        let mut words = Vec::new();
        for raw_token in text.split_whitespace() {
            words.push(Self::normalize_token(raw_token));
        }
        words.join(" ")
    }

    /// Split long text into paragraphs, separated by double newlines.
    #[must_use]
    pub fn split_paragraphs(text: &str) -> Vec<String> {
        text.split("\n\n")
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect()
    }

    /// Split a paragraph or text into individual sentences.
    #[must_use]
    pub fn split_sentences(text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();

        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let c = chars[i];
            current.push(c);

            if (c == '.' || c == '!' || c == '?') && (i + 1 == len || chars[i + 1].is_whitespace())
            {
                // Check if this period is part of a common abbreviation like "Dr." or "Mr."
                let trimmed = current.trim();
                let is_abbrev = trimmed.ends_with("Dr.")
                    || trimmed.ends_with("Mr.")
                    || trimmed.ends_with("Mrs.")
                    || trimmed.ends_with("Ms.")
                    || trimmed.ends_with("St.")
                    || trimmed.ends_with("etc.")
                    || trimmed.ends_with("vs.");

                if !is_abbrev {
                    sentences.push(current.trim().to_string());
                    current.clear();
                }
            }
            i += 1;
        }

        if !current.trim().is_empty() {
            sentences.push(current.trim().to_string());
        }

        sentences
    }

    fn normalize_token(token: &str) -> String {
        let expanded = match token {
            "Dr." | "Dr" => "Doctor",
            "Mr." | "Mr" => "Mister",
            "Mrs." | "Mrs" => "Missus",
            "Ms." | "Ms" => "Miz",
            "St." | "St" => "Saint",
            "Ave." | "Ave" => "Avenue",
            "Rd." | "Rd" => "Road",
            "etc." | "etc" => "et cetera",
            "vs." | "vs" => "versus",
            "e.g." | "eg" => "for example",
            "i.e." | "ie" => "that is",
            _ => "",
        };
        if !expanded.is_empty() {
            return expanded.to_string();
        }

        // Strip trailing punctuation if present
        let (prefix, core, suffix) = split_affixes(token);
        if core.is_empty() {
            return token.to_string();
        }

        // 1. Abbreviations within punctuation
        let expanded_abbrev = match core {
            "Dr." | "Dr" => "Doctor",
            "Mr." | "Mr" => "Mister",
            "Mrs." | "Mrs" => "Missus",
            "Ms." | "Ms" => "Miz",
            "St." | "St" => "Saint",
            "Ave." | "Ave" => "Avenue",
            "Rd." | "Rd" => "Road",
            "etc." | "etc" => "et cetera",
            "vs." | "vs" => "versus",
            "e.g." | "eg" => "for example",
            "i.e." | "ie" => "that is",
            _ => "",
        };
        if !expanded_abbrev.is_empty() {
            return format!("{prefix}{expanded_abbrev}{suffix}");
        }

        // 2. Currency
        if let Some(curr) = normalize_currency(core) {
            return format!("{prefix}{curr}{suffix}");
        }

        // 3. Percentages
        if let Some(num_part) = core.strip_suffix('%')
            && let Ok(n) = num_part.parse::<i64>()
        {
            return format!("{prefix}{} percent{suffix}", number_to_words(n));
        }

        // 4. Ordinals (1st, 2nd, 3rd, 4th, etc.)
        if let Some(ord) = normalize_ordinal(core) {
            return format!("{prefix}{ord}{suffix}");
        }

        // 5. Integers and years
        if let Ok(num) = core.parse::<i64>() {
            if (1000..=2099).contains(&num) {
                // Year format e.g. 2026 -> twenty twenty-six
                return format!("{prefix}{}{suffix}", year_to_words(num));
            }
            return format!("{prefix}{}{suffix}", number_to_words(num));
        }

        // 6. Decimals
        if let Ok(_flt) = core.parse::<f64>() {
            let parts: Vec<&str> = core.split('.').collect();
            if parts.len() == 2
                && let (Ok(int_p), Ok(_)) = (parts[0].parse::<i64>(), parts[1].parse::<u64>())
            {
                let mut dec_words = Vec::new();
                for c in parts[1].chars() {
                    if let Some(digit) = c.to_digit(10) {
                        dec_words.push(digit_to_word(digit));
                    }
                }
                return format!(
                    "{prefix}{} point {}{suffix}",
                    number_to_words(int_p),
                    dec_words.join(" ")
                );
            }
        }

        token.to_string()
    }
}

fn split_affixes(token: &str) -> (&str, &str, &str) {
    let bytes = token.as_bytes();
    let mut start = 0;
    while start < bytes.len() && matches!(bytes[start], b'(' | b'[' | b'{' | b'"' | b'\'') {
        start += 1;
    }
    let mut end = bytes.len();
    while end > start
        && matches!(
            bytes[end - 1],
            b'.' | b',' | b';' | b':' | b'!' | b'?' | b')' | b']' | b'}' | b'"' | b'\''
        )
    {
        end -= 1;
    }
    (&token[..start], &token[start..end], &token[end..])
}

fn normalize_currency(core: &str) -> Option<String> {
    let (symbol, rest) = if let Some(stripped) = core.strip_prefix('$') {
        ("$", stripped)
    } else if let Some(stripped) = core.strip_prefix('€') {
        ("€", stripped)
    } else {
        let stripped = core.strip_prefix('£')?;
        ("£", stripped)
    };

    let currency_name = match symbol {
        "$" => ("dollar", "dollars", "cent", "cents"),
        "€" => ("euro", "euros", "cent", "cents"),
        "£" => ("pound", "pounds", "pence", "pence"),
        _ => return None,
    };

    if let Ok(val) = rest.parse::<i64>() {
        let name = if val == 1 {
            currency_name.0
        } else {
            currency_name.1
        };
        return Some(format!("{} {name}", number_to_words(val)));
    }

    if let Some((major_s, minor_s)) = rest.split_once('.')
        && let (Ok(major), Ok(minor)) = (major_s.parse::<i64>(), minor_s.parse::<i64>())
    {
        let maj_name = if major == 1 {
            currency_name.0
        } else {
            currency_name.1
        };
        let min_name = if minor == 1 {
            currency_name.2
        } else {
            currency_name.3
        };
        if minor == 0 {
            return Some(format!("{} {maj_name}", number_to_words(major)));
        }
        return Some(format!(
            "{} {maj_name} and {} {min_name}",
            number_to_words(major),
            number_to_words(minor)
        ));
    }

    None
}

fn normalize_ordinal(core: &str) -> Option<String> {
    let (num_part, suffix) = if let Some(n) = core.strip_suffix("st") {
        (n, "st")
    } else if let Some(n) = core.strip_suffix("nd") {
        (n, "nd")
    } else if let Some(n) = core.strip_suffix("rd") {
        (n, "rd")
    } else {
        let n = core.strip_suffix("th")?;
        (n, "th")
    };

    let n: i64 = num_part.parse().ok()?;
    let _ = suffix;
    Some(match n {
        1 => "first".into(),
        2 => "second".into(),
        3 => "third".into(),
        4 => "fourth".into(),
        5 => "fifth".into(),
        6 => "sixth".into(),
        7 => "seventh".into(),
        8 => "eighth".into(),
        9 => "ninth".into(),
        10 => "tenth".into(),
        11 => "eleventh".into(),
        12 => "twelfth".into(),
        13 => "thirteenth".into(),
        14 => "fourteenth".into(),
        15 => "fifteenth".into(),
        16 => "sixteenth".into(),
        17 => "seventeenth".into(),
        18 => "eighteenth".into(),
        19 => "nineteenth".into(),
        20 => "twentieth".into(),
        21 => "twenty-first".into(),
        22 => "twenty-second".into(),
        23 => "twenty-third".into(),
        30 => "thirtieth".into(),
        40 => "fortieth".into(),
        50 => "fiftieth".into(),
        100 => "one hundredth".into(),
        _ => format!("{}th", number_to_words(n)),
    })
}

fn digit_to_word(digit: u32) -> &'static str {
    match digit {
        0 => "zero",
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        7 => "seven",
        8 => "eight",
        9 => "nine",
        _ => "",
    }
}

fn year_to_words(year: i64) -> String {
    let century = year / 100;
    let remainder = year % 100;
    if remainder == 0 {
        format!("{} hundred", number_to_words(century))
    } else if (1..=9).contains(&remainder) {
        format!(
            "{} oh {}",
            number_to_words(century),
            digit_to_word(remainder as u32)
        )
    } else {
        format!(
            "{} {}",
            number_to_words(century),
            number_to_words(remainder)
        )
    }
}

/// Convert an integer to English words.
#[must_use]
pub fn number_to_words(n: i64) -> String {
    if n == 0 {
        return "zero".into();
    }
    if n < 0 {
        return format!("minus {}", number_to_words(-n));
    }

    let mut parts = Vec::new();

    let billions = n / 1_000_000_000;
    let millions = (n % 1_000_000_000) / 1_000_000;
    let thousands = (n % 1_000_000) / 1_000;
    let remainder = n % 1_000;

    if billions > 0 {
        parts.push(format!("{} billion", chunk_to_words(billions)));
    }
    if millions > 0 {
        parts.push(format!("{} million", chunk_to_words(millions)));
    }
    if thousands > 0 {
        parts.push(format!("{} thousand", chunk_to_words(thousands)));
    }
    if remainder > 0 {
        parts.push(chunk_to_words(remainder));
    }

    parts.join(" ")
}

fn chunk_to_words(chunk: i64) -> String {
    let mut parts = Vec::new();
    let hundreds = chunk / 100;
    let remainder = chunk % 100;

    if hundreds > 0 {
        parts.push(format!("{} hundred", digit_to_word(hundreds as u32)));
    }

    if remainder > 0 {
        match remainder {
            1..=9 => parts.push(digit_to_word(remainder as u32).into()),
            10 => parts.push("ten".into()),
            11 => parts.push("eleven".into()),
            12 => parts.push("twelve".into()),
            13 => parts.push("thirteen".into()),
            14 => parts.push("fourteen".into()),
            15 => parts.push("fifteen".into()),
            16 => parts.push("sixteen".into()),
            17 => parts.push("seventeen".into()),
            18 => parts.push("eighteen".into()),
            19 => parts.push("nineteen".into()),
            20..=99 => {
                let tens_word = match remainder / 10 {
                    2 => "twenty",
                    3 => "thirty",
                    4 => "forty",
                    5 => "fifty",
                    6 => "sixty",
                    7 => "seventy",
                    8 => "eighty",
                    9 => "ninety",
                    _ => "",
                };
                let ones = remainder % 10;
                if ones > 0 {
                    parts.push(format!("{tens_word}-{}", digit_to_word(ones as u32)));
                } else {
                    parts.push(tens_word.into());
                }
            }
            _ => {}
        }
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_abbreviations_and_numbers() {
        let input = "Dr. Smith paid $25.50 for 3 books in 2026.";
        let normalized = TextNormalizer::normalize(input);
        assert_eq!(
            normalized,
            "Doctor Smith paid twenty-five dollars and fifty cents for three books in twenty twenty-six."
        );
    }

    #[test]
    fn splits_sentences_without_breaking_abbreviations() {
        let text = "Hello Dr. Smith! How are you today? We have 50% discount.";
        let sentences = TextNormalizer::split_sentences(text);
        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0], "Hello Dr. Smith!");
        assert_eq!(sentences[1], "How are you today?");
        assert_eq!(sentences[2], "We have 50% discount.");
    }
}
