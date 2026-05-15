/// Parse a video filename to extract a searchable title and possible ID/number.
/// Supports common naming patterns for movies and adult videos.
pub fn parse_filename(filename: &str) -> ParsedFilename {
    let name = filename.trim();
    // Remove file extension
    let name = name.rsplit('.').next().map(|_| {
        let last_dot = name.rfind('.').unwrap_or(name.len());
        &name[..last_dot]
    }).unwrap_or(name);

    // Remove common patterns like resolution, codec info
    let cleaned = remove_technical_suffixes(name);

    // Try to detect common patterns
    let id_pattern = detect_id_pattern(&cleaned);

    ParsedFilename {
        original: filename.to_string(),
        cleaned: cleaned.trim().to_string(),
        id_number: id_pattern.id_number,
        id_prefix: id_pattern.id_prefix,
    }
}

pub struct ParsedFilename {
    pub original: String,
    pub cleaned: String,
    pub id_number: Option<String>,
    pub id_prefix: Option<String>,
}

struct IdPattern {
    id_number: Option<String>,
    id_prefix: Option<String>,
}

fn remove_technical_suffixes(name: &str) -> String {
    let suffixes = [
        "1080p", "720p", "4K", "2160p", "HDR", "DV", "HDR10",
        "x264", "x265", "HEVC", "AVC", "AV1",
        "BluRay", "WEB-DL", "WEBRip", "BDRip", "HDTV",
        "DDP5.1", "DDP7.1", "TrueHD", "Atmos", "DTS-HD", "DTS",
        "AAC", "FLAC",
        "CHS", "CHT", "中文字幕", "中文",
        "완전", "Uncensored", "Leak", "无码",
    ];

    let mut result = name.to_string();
    for suffix in &suffixes {
        result = result.replace(suffix, " ");
    }
    // Collapse multiple spaces
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }
    // Remove text in brackets/parentheses that contains resolution info
    result = remove_bracket_content(&result);
    result
}

fn remove_bracket_content(s: &str) -> String {
    let mut result = s.to_string();
    // Remove [...] and (...) containing common patterns
    for (open, close) in &[('[', ']'), ('(', ')')] {
        while let Some(start) = result.find(*open) {
            if let Some(end) = result[start..].find(*close) {
                let content = &result[start + 1..start + end];
                let should_remove = content.contains("1080p") || content.contains("720p")
                    || content.contains("HEVC") || content.contains("x265")
                    || content.contains("x264");
                if should_remove {
                    result.replace_range(start..=start + end, " ");
                } else {
                    break; // Don't remove non-technical brackets
                }
            } else {
                break;
            }
        }
    }
    // Remove trailing content after certain patterns
    for sep in &[" - ", " – "] {
        if let Some(pos) = result.find(sep) {
            let after = &result[pos + sep.len()..];
            if after.contains("1080p") || after.contains("720p") || after.contains("x265") || after.contains("x264") {
                result = result[..pos].to_string();
            }
        }
    }
    result
}

fn detect_id_pattern(cleaned: &str) -> IdPattern {
    // Pattern: ABC-123 or ABC123
    let patterns = [
        regex_lite::Regex::new(r"(?i)\b([A-Z]{2,6})[-_]?(\d{2,5})\b"),
        regex_lite::Regex::new(r"(?i)\b(\d{2,5})[-_]?([A-Z]{2,6})\b"),
    ];

    for re in patterns.iter().flatten() {
        if let Some(caps) = re.captures(cleaned) {
            let g1 = caps.get(1).map(|m| m.as_str().to_uppercase());
            let g2 = caps.get(2).map(|m| m.as_str().to_string());
            if let (Some(prefix), Some(number)) = (g1, g2) {
                // Only match if prefix looks like letters and number looks like digits
                if prefix.chars().all(|c| c.is_ascii_uppercase()) && number.chars().all(|c| c.is_ascii_digit()) {
                    return IdPattern {
                        id_number: Some(format!("{}-{}", prefix, number)),
                        id_prefix: Some(prefix),
                    };
                }
            }
        }
    }

    IdPattern { id_number: None, id_prefix: None }
}

// Minimal regex-like pattern matching without external regex crate
mod regex_lite {
    pub struct Regex {
        pattern: String,
    }

    impl Regex {
        pub fn new(pattern: &str) -> Option<Self> {
            Some(Self { pattern: pattern.to_string() })
        }

        pub fn captures(&self, text: &str) -> Option<Captures> {
            // Simple pattern matching for known formats like ABC-123 or ABC123
            let text_upper = text.to_uppercase();
            let chars: Vec<char> = text_upper.chars().collect();
            let len = chars.len();

            // Look for pattern: 2-6 uppercase letters followed by optional separator and 2-5 digits
            for i in 0..len {
                let mut letter_count = 0;
                while i + letter_count < len && chars[i + letter_count].is_ascii_uppercase() {
                    letter_count += 1;
                }
                if letter_count >= 2 && letter_count <= 6 {
                    let mut pos = i + letter_count;
                    // Skip separator
                    if pos < len && (chars[pos] == '-' || chars[pos] == '_') {
                        pos += 1;
                    }
                    let mut digit_count = 0;
                    while pos + digit_count < len && chars[pos + digit_count].is_ascii_digit() {
                        digit_count += 1;
                    }
                    if digit_count >= 2 && digit_count <= 5 {
                        return Some(Captures {
                            group1: text[i..i + letter_count].to_string(),
                            group2: text[pos..pos + digit_count].to_string(),
                        });
                    }
                }
            }
            None
        }
    }

    pub struct Captures {
        group1: String,
        group2: String,
    }

    impl Captures {
        pub fn get(&self, idx: usize) -> Option<Match<'_>> {
            match idx {
                1 => Some(Match { value: &self.group1 }),
                2 => Some(Match { value: &self.group2 }),
                _ => None,
            }
        }
    }

    pub struct Match<'a> {
        value: &'a str,
    }

    impl<'a> Match<'a> {
        pub fn as_str(&self) -> &str {
            self.value
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_movie() {
        let r = parse_filename("Inception.2010.1080p.BluRay.x264.mkv");
        assert!(r.cleaned.contains("Inception"));

        let r = parse_filename("SSNI-888.mp4");
        assert_eq!(r.id_number, Some("SSNI-888".into()));
    }
}
