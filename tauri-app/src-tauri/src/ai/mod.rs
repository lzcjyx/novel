pub mod anthropic;
pub mod client;
pub mod deepseek;
pub mod gemini;
pub mod openai;
pub mod openai_compat;
pub mod types;

pub mod factory;

pub(crate) fn preview_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn preview_chars_handles_multibyte_boundaries() {
        let text = format!("x{}", "级".repeat(140));
        let preview = super::preview_chars(&text, 100);

        assert_eq!(preview.chars().count(), 100);
        assert!(preview.starts_with('x'));
        assert!(preview.ends_with('级'));
    }
}
