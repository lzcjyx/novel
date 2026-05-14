use regex::Regex;

pub fn redact_secrets(text: &str) -> String {
    let mut result = text.to_string();

    let patterns = [
        (Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(), "sk-***REDACTED***"),
        (Regex::new(r"Bearer [a-zA-Z0-9_\-.]{20,}").unwrap(), "Bearer ***REDACTED***"),
        (Regex::new(r"x-api-key[=:]\s*[a-zA-Z0-9_\-.]{10,}").unwrap(), "x-api-key=***REDACTED***"),
        (Regex::new(r"api[_-]key[=:]\s*[a-zA-Z0-9_\-.]{10,}").unwrap(), "api_key=***REDACTED***"),
        (Regex::new(r"AIza[0-9A-Za-z\-_]{35}").unwrap(), "AIza***REDACTED***"),
    ];

    for (re, replacement) in &patterns {
        result = re.replace_all(&result, *replacement).to_string();
    }

    result
}

pub fn redact_prompt_if_not_debug(text: &str, debug_mode: bool) -> String {
    if debug_mode { text.to_string() } else { "[prompt hidden — enable debug mode to view]".to_string() }
}
