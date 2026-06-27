use crate::db::connection::Database;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSidecarManifest {
    pub project_id: String,
    pub dir_name: String,
    pub files: Vec<String>,
}

pub fn export_audit_sidecar(
    db: &Database,
    project_id: &str,
    base_dir: &Path,
) -> Result<AuditSidecarManifest, String> {
    let project = crate::db::projects::get_project(db, project_id)?;
    let bible = crate::db::bible::get_bible(db, project_id)?;
    let chapters = crate::db::chapters::get_chapters(db, project_id)?;
    let hard_facts = crate::db::hard_facts::list_hard_facts(db, project_id, false)?;
    let style_assets = crate::db::style_assets::list_style_assets(db, project_id, false)?;

    let dir_name = format!("audit-{}", project_id);
    let root = base_dir.join(&dir_name);
    create_dir(&root)?;
    create_dir(&root.join("bible"))?;
    create_dir(&root.join("state"))?;
    create_dir(&root.join("timeline"))?;
    create_dir(&root.join("memory"))?;

    let mut files = Vec::new();
    write_markdown(
        &root,
        "bible/project.md",
        &format!(
            "# {}\n\n- ID: {}\n- Genre: {}\n- Status: {}\n\n{}\n",
            project.name,
            project.id,
            project.genre.as_deref().unwrap_or(""),
            project.status,
            project.style_profile
        ),
        &mut files,
    )?;
    write_markdown(
        &root,
        "bible/characters.md",
        &bible
            .characters
            .iter()
            .map(|character| {
                format!(
                    "## {}\n\n- Role: {}\n- Status: {}\n\n{}\n",
                    character.name,
                    character.role.as_deref().unwrap_or(""),
                    character.status,
                    character.description_text()
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        &mut files,
    )?;
    write_markdown(
        &root,
        "bible/world.md",
        &bible
            .world_lore
            .iter()
            .map(|lore| {
                format!(
                    "## {}\n\n{}\n",
                    lore.title.as_deref().unwrap_or("Untitled lore"),
                    lore.content.as_deref().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        &mut files,
    )?;

    for chapter in chapters {
        let chapter_dir = format!("state/chapter-{:02}", chapter.sequence);
        create_dir(&root.join(&chapter_dir))?;
        if let Some(version) = crate::db::chapters::get_latest_version(db, &chapter.id)? {
            write_markdown(
                &root,
                &format!("{}/final.md", chapter_dir),
                &format!(
                    "# {}\n\n{}\n",
                    version.title.as_deref().unwrap_or("Untitled"),
                    version.body_markdown.unwrap_or_default()
                ),
                &mut files,
            )?;
            write_markdown(
                &root,
                &format!("{}/summary.md", chapter_dir),
                &format!(
                    "# {} Summary\n\n{}\n",
                    chapter.title.as_deref().unwrap_or("Untitled"),
                    version
                        .summary
                        .unwrap_or_else(|| chapter.summary.unwrap_or_default())
                ),
                &mut files,
            )?;
        }
    }

    write_markdown(
        &root,
        "timeline/history.md",
        &bible
            .timeline_events
            .iter()
            .map(|event| {
                format!(
                    "- {}: {}\n",
                    event.event_time_label.as_deref().unwrap_or("unknown"),
                    event.event_summary.as_deref().unwrap_or("")
                )
            })
            .collect::<String>(),
        &mut files,
    )?;
    write_markdown(
        &root,
        "memory/hard-facts.md",
        &hard_facts
            .iter()
            .map(|fact| {
                format!(
                    "- [{}] {} {} {} ({})\n",
                    fact.status, fact.subject, fact.predicate, fact.object, fact.value_text
                )
            })
            .collect::<String>(),
        &mut files,
    )?;
    write_markdown(
        &root,
        "memory/style-assets.md",
        &style_assets
            .iter()
            .map(|asset| {
                format!(
                    "## {}\n\n- Type: {}\n- Enabled: {}\n- Priority: {}\n\nFeatures: {}\n\nPositive examples:\n{}\n\nNegative examples:\n{}\n",
                    asset.name,
                    asset.asset_type,
                    asset.enabled,
                    asset.priority,
                    asset.features,
                    bullet_lines(&asset.positive_examples),
                    bullet_lines(&asset.negative_examples)
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        &mut files,
    )?;

    Ok(AuditSidecarManifest {
        project_id: project_id.to_string(),
        dir_name,
        files,
    })
}

trait CharacterAuditText {
    fn description_text(&self) -> String;
}

impl CharacterAuditText for crate::models::Character {
    fn description_text(&self) -> String {
        [
            self.personality.as_deref(),
            self.motivation.as_deref(),
            self.appearance.as_deref(),
            self.backstory.as_deref(),
        ]
        .into_iter()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
    }
}

fn bullet_lines(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("- {}", value))
        .collect::<Vec<_>>()
        .join("\n")
}

fn write_markdown(
    root: &Path,
    relative: &str,
    content: &str,
    files: &mut Vec<String>,
) -> Result<(), String> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        create_dir(parent)?;
    }
    std::fs::write(&path, content).map_err(|e| format!("Write audit '{}': {}", relative, e))?;
    files.push(relative.to_string());
    Ok(())
}

fn create_dir(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path)
        .map_err(|e| format!("Create audit dir '{}': {}", display(path), e))
}

fn display(path: &Path) -> String {
    PathBuf::from(path).to_string_lossy().to_string()
}
