//! SSML (Speech Synthesis Markup Language) parser for TTS.
//!
//! Supports a practical subset of SSML:
//! - `<speak>` - Root element (required wrapper)
//! - `<break time="500ms"/>` or `<break strength="strong"/>` - Silence insertion
//! - `<prosody rate="fast">` - Speed control per segment
//! - `<s>` - Sentence boundary
//! - `<p>` - Paragraph boundary (adds 750ms pause)
//!
//! Unsupported tags are logged and their text content is extracted.

/// A segment produced by SSML parsing, ready for synthesis.
#[derive(Debug, Clone, PartialEq)]
pub enum SsmlSegment {
    /// Plain text to synthesize with given speed.
    Text {
        text: String,
        /// Speed multiplier (1.0 = normal).
        speed: f32,
    },
    /// Insert silence of specified duration.
    Silence { duration_ms: u32 },
}

/// Returns true if the input appears to be SSML (starts with `<speak>`).
pub fn is_ssml(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with("<speak>") || trimmed.starts_with("<speak ")
}

/// Parse SSML input into a flat list of segments.
///
/// Returns an error string if the SSML is fundamentally malformed.
/// Unknown tags are silently skipped (text content preserved).
pub fn parse_ssml(input: &str) -> Result<Vec<SsmlSegment>, String> {
    let trimmed = input.trim();

    // Strip <speak> wrapper
    let inner = strip_speak_wrapper(trimmed)?;

    let mut segments = Vec::new();
    let mut speed_stack: Vec<f32> = vec![1.0];
    let mut current_text = String::new();
    let mut chars = inner.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch == '<' {
            // Flush accumulated text
            flush_text(&mut current_text, &speed_stack, &mut segments);

            // Parse the tag
            let tag = parse_tag(&mut chars)?;
            handle_tag(&tag, &mut speed_stack, &mut segments);
        } else {
            current_text.push(ch);
            chars.next();
        }
    }

    // Flush remaining text
    flush_text(&mut current_text, &speed_stack, &mut segments);

    Ok(merge_adjacent_text(segments))
}

/// Strip XML tags as a fallback when parsing fails.
pub fn strip_xml_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result.trim().to_string()
}

// --- Internal helpers ---

fn strip_speak_wrapper(text: &str) -> Result<&str, String> {
    let start = text.find('>').ok_or("Missing <speak> opening tag")?;
    let inner = &text[start + 1..];

    // Find </speak> from the end
    if let Some(end_pos) = inner.rfind("</speak>") {
        Ok(inner[..end_pos].trim())
    } else {
        // Tolerate missing </speak>
        Ok(inner.trim())
    }
}

/// Parsed tag representation.
#[derive(Debug)]
struct Tag {
    name: String,
    is_closing: bool,
    is_self_closing: bool,
    attrs: Vec<(String, String)>,
}

fn parse_tag(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<Tag, String> {
    // Consume '<'
    chars.next();

    let mut raw = String::new();
    let mut depth = 1;
    for ch in chars.by_ref() {
        if ch == '>' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        } else if ch == '<' {
            depth += 1;
        }
        raw.push(ch);
    }

    let is_closing = raw.starts_with('/');
    let is_self_closing = raw.ends_with('/');

    let content = raw
        .trim_start_matches('/')
        .trim_end_matches('/')
        .trim();

    let mut parts = content.splitn(2, |c: char| c.is_whitespace());
    let name = parts.next().unwrap_or("").to_lowercase();
    let attr_str = parts.next().unwrap_or("");

    let attrs = parse_attrs(attr_str);

    Ok(Tag {
        name,
        is_closing,
        is_self_closing,
        attrs,
    })
}

fn parse_attrs(s: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let mut remaining = s.trim();

    while !remaining.is_empty() {
        // Find key=
        let eq_pos = match remaining.find('=') {
            Some(p) => p,
            None => break,
        };
        let key = remaining[..eq_pos].trim().to_lowercase();
        remaining = remaining[eq_pos + 1..].trim();

        // Parse value (quoted)
        let value = if remaining.starts_with('"') {
            remaining = &remaining[1..];
            let end = remaining.find('"').unwrap_or(remaining.len());
            let val = remaining[..end].to_string();
            remaining = if end < remaining.len() {
                remaining[end + 1..].trim()
            } else {
                ""
            };
            val
        } else if remaining.starts_with('\'') {
            remaining = &remaining[1..];
            let end = remaining.find('\'').unwrap_or(remaining.len());
            let val = remaining[..end].to_string();
            remaining = if end < remaining.len() {
                remaining[end + 1..].trim()
            } else {
                ""
            };
            val
        } else {
            // Unquoted value
            let end = remaining
                .find(|c: char| c.is_whitespace())
                .unwrap_or(remaining.len());
            let val = remaining[..end].to_string();
            remaining = remaining[end..].trim();
            val
        };

        attrs.push((key, value));
    }

    attrs
}

fn get_attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn handle_tag(tag: &Tag, speed_stack: &mut Vec<f32>, segments: &mut Vec<SsmlSegment>) {
    match tag.name.as_str() {
        "break" => {
            let duration_ms = if let Some(time_str) = get_attr(&tag.attrs, "time") {
                parse_time_ms(time_str).unwrap_or(400)
            } else if let Some(strength) = get_attr(&tag.attrs, "strength") {
                strength_to_ms(strength)
            } else {
                400 // default medium pause
            };

            if duration_ms > 0 {
                // Cap at 10 seconds
                let capped = duration_ms.min(10000);
                segments.push(SsmlSegment::Silence { duration_ms: capped });
            }
        }
        "prosody" => {
            if tag.is_closing {
                // Pop speed
                if speed_stack.len() > 1 {
                    speed_stack.pop();
                }
            } else {
                // Push new speed
                let rate = get_attr(&tag.attrs, "rate")
                    .map(parse_rate)
                    .unwrap_or_else(|| *speed_stack.last().unwrap_or(&1.0));
                speed_stack.push(rate);

                if tag.is_self_closing {
                    speed_stack.pop();
                }
            }
        }
        "s" => {
            // Sentence boundary - just a text segment split point
            // (flush_text already happened before this tag)
        }
        "p" => {
            if tag.is_closing {
                // End of paragraph - add pause
                segments.push(SsmlSegment::Silence { duration_ms: 750 });
            }
        }
        "speak" => {
            // Nested <speak> - ignore
        }
        _ => {
            // Unknown tags: log and skip
            if !tag.is_closing {
                log::debug!("Ignoring unsupported SSML tag: <{}>", tag.name);
            }
        }
    }
}

fn flush_text(current: &mut String, speed_stack: &[f32], segments: &mut Vec<SsmlSegment>) {
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        let speed = *speed_stack.last().unwrap_or(&1.0);
        segments.push(SsmlSegment::Text {
            text: trimmed,
            speed,
        });
    }
    current.clear();
}

/// Merge adjacent Text segments with the same speed.
fn merge_adjacent_text(segments: Vec<SsmlSegment>) -> Vec<SsmlSegment> {
    let mut merged: Vec<SsmlSegment> = Vec::new();

    for seg in segments {
        match (&seg, merged.last_mut()) {
            (
                SsmlSegment::Text { text, speed },
                Some(SsmlSegment::Text {
                    text: prev_text,
                    speed: prev_speed,
                }),
            ) if (*speed - *prev_speed).abs() < f32::EPSILON => {
                prev_text.push(' ');
                prev_text.push_str(text);
            }
            _ => merged.push(seg),
        }
    }

    merged
}

/// Convert SSML break strength to milliseconds.
fn strength_to_ms(strength: &str) -> u32 {
    match strength {
        "none" => 0,
        "x-weak" => 100,
        "weak" => 200,
        "medium" => 400,
        "strong" => 750,
        "x-strong" => 1200,
        _ => 400, // default to medium
    }
}

/// Parse SSML prosody rate attribute.
fn parse_rate(rate: &str) -> f32 {
    match rate {
        "x-slow" => 0.5,
        "slow" => 0.75,
        "medium" => 1.0,
        "fast" => 1.25,
        "x-fast" => 1.75,
        _ => {
            // Try percentage: "80%" -> 0.8, "120%" -> 1.2
            if let Some(pct) = rate.strip_suffix('%') {
                if let Ok(val) = pct.parse::<f32>() {
                    return (val / 100.0).clamp(0.25, 4.0);
                }
            }
            // Try raw float
            if let Ok(val) = rate.parse::<f32>() {
                return val.clamp(0.25, 4.0);
            }
            1.0
        }
    }
}

/// Parse SSML time string (e.g., "500ms", "1.5s", "2s").
fn parse_time_ms(time_str: &str) -> Option<u32> {
    let s = time_str.trim();
    if let Some(ms_str) = s.strip_suffix("ms") {
        ms_str.trim().parse::<u32>().ok()
    } else if let Some(s_str) = s.strip_suffix('s') {
        s_str.trim().parse::<f32>().ok().map(|v| (v * 1000.0) as u32)
    } else {
        // Try plain number as milliseconds
        s.parse::<u32>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ssml() {
        assert!(is_ssml("<speak>hello</speak>"));
        assert!(is_ssml("  <speak>hello</speak>  "));
        assert!(is_ssml("<speak xml:lang=\"zh\">hello</speak>"));
        assert!(!is_ssml("hello"));
        assert!(!is_ssml("hello <speak>"));
        assert!(!is_ssml(""));
    }

    #[test]
    fn test_plain_text() {
        let result = parse_ssml("<speak>hello world</speak>")
            .expect("parse_ssml should parse simple <speak> content");
        assert_eq!(result, vec![SsmlSegment::Text {
            text: "hello world".to_string(),
            speed: 1.0,
        }]);
    }

    #[test]
    fn test_break_time_ms() {
        let result = parse_ssml("<speak>hello<break time=\"500ms\"/>world</speak>").unwrap();
        assert_eq!(result, vec![
            SsmlSegment::Text { text: "hello".to_string(), speed: 1.0 },
            SsmlSegment::Silence { duration_ms: 500 },
            SsmlSegment::Text { text: "world".to_string(), speed: 1.0 },
        ]);
    }

    #[test]
    fn test_break_time_seconds() {
        let result = parse_ssml("<speak>hello<break time=\"1.5s\"/>world</speak>").unwrap();
        assert_eq!(result, vec![
            SsmlSegment::Text { text: "hello".to_string(), speed: 1.0 },
            SsmlSegment::Silence { duration_ms: 1500 },
            SsmlSegment::Text { text: "world".to_string(), speed: 1.0 },
        ]);
    }

    #[test]
    fn test_break_strength() {
        let result = parse_ssml("<speak>hello<break strength=\"strong\"/>world</speak>").unwrap();
        assert_eq!(result, vec![
            SsmlSegment::Text { text: "hello".to_string(), speed: 1.0 },
            SsmlSegment::Silence { duration_ms: 750 },
            SsmlSegment::Text { text: "world".to_string(), speed: 1.0 },
        ]);
    }

    #[test]
    fn test_prosody_rate_named() {
        let result = parse_ssml("<speak><prosody rate=\"fast\">hello</prosody></speak>").unwrap();
        assert_eq!(result, vec![SsmlSegment::Text {
            text: "hello".to_string(),
            speed: 1.25,
        }]);
    }

    #[test]
    fn test_prosody_rate_percentage() {
        let result = parse_ssml("<speak><prosody rate=\"80%\">slow</prosody></speak>").unwrap();
        assert_eq!(result, vec![SsmlSegment::Text {
            text: "slow".to_string(),
            speed: 0.8,
        }]);
    }

    #[test]
    fn test_prosody_restores_speed() {
        let result = parse_ssml(
            "<speak>normal<prosody rate=\"fast\">fast</prosody>normal again</speak>"
        ).unwrap();
        assert_eq!(result, vec![
            SsmlSegment::Text { text: "normal".to_string(), speed: 1.0 },
            SsmlSegment::Text { text: "fast".to_string(), speed: 1.25 },
            SsmlSegment::Text { text: "normal again".to_string(), speed: 1.0 },
        ]);
    }

    #[test]
    fn test_paragraph_boundary() {
        let result = parse_ssml(
            "<speak><p>First paragraph.</p><p>Second paragraph.</p></speak>"
        ).unwrap();
        assert_eq!(result, vec![
            SsmlSegment::Text { text: "First paragraph.".to_string(), speed: 1.0 },
            SsmlSegment::Silence { duration_ms: 750 },
            SsmlSegment::Text { text: "Second paragraph.".to_string(), speed: 1.0 },
            SsmlSegment::Silence { duration_ms: 750 },
        ]);
    }

    #[test]
    fn test_sentence_boundary_merges() {
        // Adjacent <s> segments at same speed get merged (fewer synthesis calls)
        let result = parse_ssml(
            "<speak><s>First.</s><s>Second.</s></speak>"
        ).unwrap();
        assert_eq!(result, vec![
            SsmlSegment::Text { text: "First. Second.".to_string(), speed: 1.0 },
        ]);
    }

    #[test]
    fn test_sentence_boundary_with_break() {
        // <s> segments separated by <break> remain distinct
        let result = parse_ssml(
            "<speak><s>First.</s><break time=\"300ms\"/><s>Second.</s></speak>"
        ).unwrap();
        assert_eq!(result, vec![
            SsmlSegment::Text { text: "First.".to_string(), speed: 1.0 },
            SsmlSegment::Silence { duration_ms: 300 },
            SsmlSegment::Text { text: "Second.".to_string(), speed: 1.0 },
        ]);
    }

    #[test]
    fn test_complex_ssml() {
        let ssml = r#"<speak>
            今天天气真不错。
            <break time="500ms"/>
            <prosody rate="fast">我们快点走吧！</prosody>
            <break strength="strong"/>
            再见。
        </speak>"#;

        let result = parse_ssml(ssml).unwrap();
        assert_eq!(result, vec![
            SsmlSegment::Text { text: "今天天气真不错。".to_string(), speed: 1.0 },
            SsmlSegment::Silence { duration_ms: 500 },
            SsmlSegment::Text { text: "我们快点走吧！".to_string(), speed: 1.25 },
            SsmlSegment::Silence { duration_ms: 750 },
            SsmlSegment::Text { text: "再见。".to_string(), speed: 1.0 },
        ]);
    }

    #[test]
    fn test_unknown_tags_preserved_text() {
        let result = parse_ssml(
            "<speak><emphasis>important</emphasis> text</speak>"
        ).unwrap();
        assert_eq!(result, vec![
            SsmlSegment::Text { text: "important text".to_string(), speed: 1.0 },
        ]);
    }

    #[test]
    fn test_time_parsing() {
        assert_eq!(parse_time_ms("500ms"), Some(500));
        assert_eq!(parse_time_ms("1.5s"), Some(1500));
        assert_eq!(parse_time_ms("2s"), Some(2000));
        assert_eq!(parse_time_ms("300"), Some(300));
        assert_eq!(parse_time_ms("abc"), None);
    }

    #[test]
    fn test_rate_parsing() {
        assert_eq!(parse_rate("x-slow"), 0.5);
        assert_eq!(parse_rate("slow"), 0.75);
        assert_eq!(parse_rate("medium"), 1.0);
        assert_eq!(parse_rate("fast"), 1.25);
        assert_eq!(parse_rate("x-fast"), 1.75);
        assert!((parse_rate("120%") - 1.2).abs() < 0.01);
        assert!((parse_rate("80%") - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_strength_mapping() {
        assert_eq!(strength_to_ms("none"), 0);
        assert_eq!(strength_to_ms("x-weak"), 100);
        assert_eq!(strength_to_ms("weak"), 200);
        assert_eq!(strength_to_ms("medium"), 400);
        assert_eq!(strength_to_ms("strong"), 750);
        assert_eq!(strength_to_ms("x-strong"), 1200);
    }

    #[test]
    fn test_strip_xml_tags() {
        assert_eq!(strip_xml_tags("<speak>hello <b>world</b></speak>"), "hello world");
        assert_eq!(strip_xml_tags("no tags here"), "no tags here");
    }

    #[test]
    fn test_silence_capped() {
        // Silence > 10s should be capped
        let result = parse_ssml("<speak>a<break time=\"30000ms\"/>b</speak>").unwrap();
        assert_eq!(result[1], SsmlSegment::Silence { duration_ms: 10000 });
    }

    #[test]
    fn test_missing_closing_speak() {
        // Should tolerate missing </speak>
        let result = parse_ssml("<speak>hello world");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![
            SsmlSegment::Text { text: "hello world".to_string(), speed: 1.0 },
        ]);
    }
}
