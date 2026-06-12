use regex::Regex;
use std::net::IpAddr;
use std::path::Path;

pub const MAX_SOURCE_BYTES: usize = 1_048_576;
pub const MAX_LEARNING_CHARS: usize = 15_000;
const MIN_WEB_TEXT_CHARS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedLearningText {
    pub source_title: String,
    pub text: String,
}

pub fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

pub fn normalize_learning_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("URL is empty.".into());
    }

    let lower = trimmed.to_ascii_lowercase();
    let explicit_scheme = trimmed.contains("://")
        || lower.starts_with("file:")
        || lower.starts_with("data:")
        || lower.starts_with("javascript:")
        || lower.starts_with("about:");
    let candidate = if explicit_scheme {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed)
    };

    let url = reqwest::Url::parse(&candidate).map_err(|e| format!("Invalid URL: {}", e))?;
    match url.scheme() {
        "http" | "https" => {}
        _ => return Err("Only http and https URLs can be learned from.".into()),
    }

    let host = url
        .host_str()
        .ok_or_else(|| "URL must include a host.".to_string())?;
    if is_private_or_local_host(host) {
        return Err("Local and private network URLs cannot be learned from.".into());
    }

    Ok(url.to_string())
}

pub fn extract_source_title(url: &str, html: &str) -> String {
    if let Ok(title_re) = Regex::new(r"(?is)<title[^>]*>(.*?)</title>") {
        if let Some(caps) = title_re.captures(html) {
            if let Some(raw) = caps.get(1) {
                let title = normalize_spaces(&strip_tags(&decode_html_entities(raw.as_str())));
                if !title.is_empty() {
                    return truncate_chars(&title, 120);
                }
            }
        }
    }

    reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| {
            parsed
                .path_segments()
                .and_then(|mut segments| segments.next_back().map(|s| s.to_string()))
        })
        .filter(|segment| !segment.trim().is_empty())
        .unwrap_or_else(|| "web source".into())
}

pub fn extract_meaningful_text_from_html(html: &str) -> Result<String, String> {
    if html.len() > MAX_SOURCE_BYTES {
        return Err("Page too large (>1 MiB).".into());
    }

    let mut cleaned = html.to_string();
    cleaned = replace_regex(&cleaned, r"(?is)<!--.*?-->", " ");

    for tag in [
        "script", "style", "noscript", "svg", "nav", "header", "footer", "aside", "form", "button",
        "select", "template", "iframe",
    ] {
        let pattern = format!(r"(?is)<{tag}[^>]*>.*?</{tag}>");
        cleaned = replace_regex(&cleaned, &pattern, "\n");
    }

    for pattern in [
        r"(?i)<br\s*/?>",
        r"(?i)</p\s*>",
        r"(?i)</div\s*>",
        r"(?i)</section\s*>",
        r"(?i)</article\s*>",
        r"(?i)</li\s*>",
        r"(?i)</h[1-6]\s*>",
    ] {
        cleaned = replace_regex(&cleaned, pattern, "\n");
    }

    cleaned = strip_tags(&cleaned);
    cleaned = decode_html_entities(&cleaned);

    let lines = cleaned
        .lines()
        .map(normalize_spaces)
        .filter(|line| is_meaningful_line(line))
        .collect::<Vec<_>>();

    let text = truncate_chars(&lines.join("\n"), MAX_LEARNING_CHARS);
    if text.chars().count() < MIN_WEB_TEXT_CHARS {
        return Err("No meaningful page content found.".into());
    }

    Ok(text)
}

pub fn validate_user_file_text(
    file_name: &str,
    byte_len: usize,
    text: &str,
) -> Result<ValidatedLearningText, String> {
    if byte_len > MAX_SOURCE_BYTES {
        return Err("File too large (>1 MiB).".into());
    }

    let source_title = Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(file_name)
        .trim()
        .to_string();
    if source_title.is_empty() {
        return Err("File name is empty.".into());
    }

    let ext = Path::new(&source_title)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    let allowed = [
        "txt", "md", "markdown", "text", "log", "csv", "json", "html",
    ];
    if !allowed.contains(&ext.as_str()) {
        return Err("Unsupported file type for Learn. Use a text-like file.".into());
    }

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("File has no readable text.".into());
    }

    let normalized_text = if ext == "html" {
        extract_meaningful_text_from_html(trimmed)?
    } else {
        truncate_chars(trimmed, MAX_LEARNING_CHARS)
    };

    Ok(ValidatedLearningText {
        source_title,
        text: normalized_text,
    })
}

fn is_private_or_local_host(host: &str) -> bool {
    let lower = host.trim_matches(['[', ']']).to_ascii_lowercase();
    if lower == "localhost"
        || lower.ends_with(".localhost")
        || lower.ends_with(".local")
        || lower.ends_with(".internal")
    {
        return true;
    }

    if lower.starts_with("127.") || lower.starts_with("0.") {
        return true;
    }

    if let Ok(ip) = lower.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(addr) => {
                addr.is_private()
                    || addr.is_loopback()
                    || addr.is_link_local()
                    || addr.is_broadcast()
                    || addr.is_unspecified()
            }
            IpAddr::V6(addr) => {
                let first = addr.segments()[0];
                addr.is_loopback()
                    || addr.is_unspecified()
                    || (first & 0xfe00) == 0xfc00
                    || (first & 0xffc0) == 0xfe80
            }
        };
    }

    false
}

fn is_meaningful_line(line: &str) -> bool {
    let char_count = line.chars().count();
    if char_count < 25 {
        return false;
    }

    let lower = line.to_lowercase();
    let noise_terms = [
        "cookie",
        "subscribe",
        "privacy",
        "login",
        "sign in",
        "newsletter",
        "advertisement",
        "copyright",
        "all rights reserved",
        "terms of service",
        "share this",
        "广告",
        "版权",
        "隐私",
        "登录",
        "注册",
        "订阅",
        "分享",
        "ICP备案",
    ];
    if noise_terms.iter().any(|term| lower.contains(term)) {
        return false;
    }

    line.chars().filter(|ch| ch.is_alphanumeric()).count() >= 12
}

fn replace_regex(input: &str, pattern: &str, replacement: &str) -> String {
    Regex::new(pattern)
        .map(|re| re.replace_all(input, replacement).to_string())
        .unwrap_or_else(|_| input.to_string())
}

fn strip_tags(input: &str) -> String {
    replace_regex(input, r"(?is)<[^>]+>", " ")
}

fn normalize_spaces(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_html_entities(input: &str) -> String {
    let mut decoded = input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&mdash;", "-")
        .replace("&ndash;", "-");

    if let Ok(numeric) = Regex::new(r"&#(x?[0-9A-Fa-f]+);") {
        decoded = numeric
            .replace_all(&decoded, |caps: &regex::Captures<'_>| {
                let raw = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
                let parsed =
                    if let Some(hex) = raw.strip_prefix('x').or_else(|| raw.strip_prefix('X')) {
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        raw.parse::<u32>().ok()
                    };
                parsed
                    .and_then(char::from_u32)
                    .map(|ch| ch.to_string())
                    .unwrap_or_else(|| caps[0].to_string())
            })
            .to_string();
    }

    decoded
}
