use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tauri_app_lib::ai::client::ModelClient;
use tauri_app_lib::workflow::{learning, learning_intake};

#[derive(Default)]
struct PromptCaptureProvider {
    users: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ModelClient for PromptCaptureProvider {
    async fn generate_json(
        &self,
        _system_prompt: &str,
        user_prompt: &str,
        _json_schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        self.users.lock().unwrap().push(user_prompt.to_string());
        Ok(json!([{
            "category": "style",
            "pattern_name": "dense sensory image",
            "pattern_description": "Uses concrete sensory images to carry mood.",
            "application_notes": "Keep one strong object in each paragraph."
        }]))
    }

    async fn generate_text(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        Ok(String::new())
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts.iter().map(|_| vec![0.1; 8]).collect())
    }
}

struct AliasKnowledgeProvider;

#[async_trait]
impl ModelClient for AliasKnowledgeProvider {
    async fn generate_json(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _json_schema: &Value,
        _max_tokens: u32,
    ) -> Result<Value, String> {
        Ok(json!([
            {
                "type": "style",
                "name": "object-carried tension",
                "description": "Uses a concrete object instead of direct emotion labels.",
                "notes": "Apply when a scene needs pressure without exposition."
            },
            {
                "category": "dialogue",
                "title": "unfinished answer",
                "summary": "Lets a character stop before the key noun, keeping the exchange tense.",
                "application": "Use in interrogations and reveals."
            },
            {
                "category": "plot",
                "name": "   ",
                "description": ""
            }
        ]))
    }

    async fn generate_text(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: u32,
    ) -> Result<String, String> {
        Ok(String::new())
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts.iter().map(|_| vec![0.1; 8]).collect())
    }
}

#[test]
fn rejects_private_or_unsafe_learning_urls() {
    assert!(learning_intake::normalize_learning_url("file:///C:/Users/me/.env").is_err());
    assert!(learning_intake::normalize_learning_url("javascript:alert(1)").is_err());
    assert!(learning_intake::normalize_learning_url("http://127.0.0.1:5173").is_err());
    assert!(learning_intake::normalize_learning_url("localhost:5173").is_err());
    assert!(learning_intake::normalize_learning_url("https://192.168.1.10/page").is_err());

    assert_eq!(
        learning_intake::normalize_learning_url("example.com/novel").unwrap(),
        "https://example.com/novel"
    );
}

#[test]
fn extracts_article_text_without_page_boilerplate() {
    let html = r#"
        <html>
          <head>
            <title>Lantern Chapter - Example Site</title>
            <style>.hidden { display: none }</style>
            <script>function trackEverything() { return "noise"; }</script>
          </head>
          <body>
            <nav>Home Subscribe Archive Login</nav>
            <aside>Cookie settings and privacy preferences</aside>
            <article>
              <h1>Lantern Chapter</h1>
              <p>The lantern city woke under rain, and every roof held a different echo of the trial.</p>
              <p>Mara counted the bells, then hid the silver key before the inspector crossed the bridge.</p>
              <p>The scene keeps attention on action, weather, and consequence instead of menu text.</p>
            </article>
            <footer>Copyright 2026 Example Site</footer>
          </body>
        </html>
    "#;

    let text = learning_intake::extract_meaningful_text_from_html(html).unwrap();

    assert!(text.contains("The lantern city woke under rain"));
    assert!(text.contains("Mara counted the bells"));
    assert!(!text.to_lowercase().contains("subscribe"));
    assert!(!text.to_lowercase().contains("cookie"));
    assert!(!text.contains("trackEverything"));
}

#[test]
fn validates_file_learning_sources() {
    let accepted = learning_intake::validate_user_file_text(
        "sample.md",
        256,
        "The rain narrowed the alley while the witness folded the receipt into a crane.",
    )
    .unwrap();

    assert_eq!(accepted.source_title, "sample.md");
    assert!(accepted.text.contains("witness folded"));

    assert!(learning_intake::validate_user_file_text("sample.pdf", 256, "text").is_err());
    assert!(learning_intake::validate_user_file_text("empty.txt", 12, "   ").is_err());
    assert!(learning_intake::validate_user_file_text(
        "large.txt",
        learning_intake::MAX_SOURCE_BYTES + 1,
        "text"
    )
    .is_err());
}

#[tokio::test]
async fn extract_knowledge_truncates_multibyte_input_safely() {
    let provider = PromptCaptureProvider::default();
    let chinese_text = "界".repeat(1_400);

    let entries =
        learning::extract_knowledge(&provider, &chinese_text, "中文样章", "manual_file", None)
            .await
            .unwrap();

    assert_eq!(entries.len(), 1);
    let prompts = provider.users.lock().unwrap();
    assert_eq!(prompts.len(), 1);
    assert!(prompts[0].contains("分析以下文本"));
    assert!(prompts[0].chars().count() < chinese_text.chars().count() + 30);
}

#[tokio::test]
async fn learning_extraction_accepts_alias_fields_and_skips_empty_items() {
    let provider = AliasKnowledgeProvider;

    let entries = learning::extract_knowledge(
        &provider,
        "The witness turns the key twice and says only half the answer.",
        "Alias sample",
        "manual_file",
        None,
    )
    .await
    .unwrap();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].category, "style_pattern");
    assert_eq!(entries[0].pattern_name, "object-carried tension");
    assert!(entries[0].pattern_description.contains("concrete object"));
    assert_eq!(
        entries[0].application_notes.as_deref(),
        Some("Apply when a scene needs pressure without exposition.")
    );
    assert_eq!(entries[1].category, "dialogue_style");
    assert_eq!(entries[1].pattern_name, "unfinished answer");
    assert!(entries[1].pattern_description.contains("key noun"));
    assert!(entries
        .iter()
        .all(|entry| entry.pattern_name != "Unknown" && !entry.pattern_description.is_empty()));
}
