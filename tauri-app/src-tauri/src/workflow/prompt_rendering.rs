use std::collections::HashMap;

use regex::Regex;

pub fn find_unresolved_placeholders(text: &str) -> Vec<String> {
    let re = Regex::new(r"\{\{\s*([A-Za-z0-9_]+)\s*\}\}").expect("valid placeholder regex");
    let mut names = re
        .captures_iter(text)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

pub fn render_prompt_strict(
    prompt_name: &str,
    template: &str,
    vars: &HashMap<&str, String>,
) -> Result<String, String> {
    let mut rendered = template.to_string();
    for (key, value) in vars {
        rendered = rendered.replace(&format!("{{{{{}}}}}", key), value);
    }

    let unresolved = find_unresolved_placeholders(&rendered);
    if unresolved.is_empty() {
        Ok(rendered)
    } else {
        Err(format!(
            "Prompt '{}' has unresolved placeholders: {}",
            prompt_name,
            unresolved.join(", ")
        ))
    }
}
