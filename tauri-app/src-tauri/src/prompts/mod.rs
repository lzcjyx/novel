use std::collections::HashMap;

/// Compile-time prompt registry. Add a .md file to prompts/ and it's available.
pub struct PromptRegistry {
    prompts: HashMap<&'static str, &'static str>,
}

impl PromptRegistry {
    fn new() -> Self {
        let mut prompts = HashMap::new();
        prompts.insert(
            "draft_writer",
            include_str!("../../prompts/draft_writer.md"),
        );
        prompts.insert(
            "revision_writer",
            include_str!("../../prompts/revision_writer.md"),
        );
        prompts.insert(
            "bible_generation",
            include_str!("../../prompts/bible_generation.md"),
        );
        prompts.insert(
            "canon_extractor",
            include_str!("../../prompts/canon_extractor.md"),
        );
        prompts.insert(
            "review_agents",
            include_str!("../../prompts/review_agents.md"),
        );
        prompts.insert(
            "weekly_planner",
            include_str!("../../prompts/weekly_planner.md"),
        );
        prompts.insert(
            "blog_metadata",
            include_str!("../../prompts/blog_publisher_metadata_agent.md"),
        );
        prompts.insert(
            "continuity_reviewer",
            include_str!("../../prompts/continuity_reviewer.md"),
        );
        prompts.insert(
            "character_reviewer",
            include_str!("../../prompts/character_reviewer.md"),
        );
        prompts.insert(
            "plot_logic_reviewer",
            include_str!("../../prompts/plot_logic_reviewer.md"),
        );
        prompts.insert(
            "pacing_reviewer",
            include_str!("../../prompts/pacing_reviewer.md"),
        );
        prompts.insert(
            "style_reviewer",
            include_str!("../../prompts/style_reviewer.md"),
        );
        prompts.insert(
            "safety_reviewer",
            include_str!("../../prompts/safety_reviewer.md"),
        );
        prompts.insert(
            "publication_reviewer",
            include_str!("../../prompts/publication_reviewer.md"),
        );
        prompts.insert(
            "review_arbiter",
            include_str!("../../prompts/review_arbiter.md"),
        );
        Self { prompts }
    }

    pub fn get(name: &str) -> Option<&'static str> {
        static REGISTRY: std::sync::LazyLock<PromptRegistry> =
            std::sync::LazyLock::new(PromptRegistry::new);
        REGISTRY.prompts.get(name).copied()
    }
}

/// Load a prompt template by name
pub fn load_prompt(name: &str) -> Result<String, String> {
    PromptRegistry::get(name)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Unknown prompt: {}", name))
}

/// Render a prompt template by replacing {{PLACEHOLDER}} variables
pub fn render_prompt(template: &str, vars: &HashMap<&str, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}
