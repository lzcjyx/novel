use crate::db::connection::Database;
use crate::db::{chapters, projects};

pub fn export_chapter_markdown(
    db: &Database,
    chapter_id: &str,
    data_dir: &str,
) -> Result<String, String> {
    let chapter = chapters::get_chapter(db, chapter_id)?;
    let version = chapters::get_latest_version(db, chapter_id)?
        .ok_or_else(|| "No version found for chapter".to_string())?;

    let body = version.body_markdown.unwrap_or_default();
    let title = version
        .title
        .unwrap_or_else(|| format!("Chapter {}", chapter.sequence));

    // Write to paper directory
    let dir = projects::get_project_paper_dir(db, &chapter.project_id, data_dir)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Mkdir: {}", e))?;

    let filename = format!("ch{:03}.md", chapter.sequence);
    let path = format!("{}/{}", dir, filename);
    let content = format!("# {}\n\n{}\n", title, body);
    std::fs::write(&path, &content).map_err(|e| format!("Write: {}", e))?;

    Ok(path)
}

pub fn export_novel_markdown(
    db: &Database,
    project_id: &str,
    data_dir: &str,
) -> Result<String, String> {
    let chapters = chapters::get_chapters(db, project_id)?;
    let project = projects::get_project(db, project_id)?;
    let dir = projects::get_project_paper_dir(db, project_id, data_dir)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Mkdir: {}", e))?;

    let mut full_text = format!("# {}\n\n", project.name);
    for ch in chapters {
        if let Some(version) = chapters::get_latest_version(db, &ch.id)? {
            let title = version
                .title
                .unwrap_or_else(|| format!("Chapter {}", ch.sequence));
            let body = version.body_markdown.unwrap_or_default();
            full_text.push_str(&format!(
                "\n---\n\n## Chapter {}: {}\n\n{}\n",
                ch.sequence, title, body
            ));
        }
    }

    let path = format!("{}/full_novel.md", dir);
    std::fs::write(&path, &full_text).map_err(|e| format!("Write: {}", e))?;
    Ok(path)
}
