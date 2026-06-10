use crate::ai::client::ModelClient;
use crate::db::connection::Database;
use crate::models::*;

pub async fn extract_knowledge(
    provider: &dyn ModelClient,
    text: &str,
    source_title: &str,
    source_type: &str, // "manual" | "web" | "self_reflection"
    source_url: Option<&str>,
) -> Result<Vec<LearningEntry>, String> {
    let system = format!(
        "从文本中提取3-5个最突出的写作技巧。每个技巧用50-100字简要说明即可。\n\
         类别：plot(情节), character(人物), dialogue(对话), style(文风), pacing(节奏)\n\
         输出JSON数组。来源：{}",
        source_title
    );

    let user = format!(
        "分析以下文本，提取写作技巧：\n\n{}",
        &text[..text.len().min(4000)]
    );

    let schema = serde_json::json!({
        "type": "array", "maxItems": 5,
        "items": {
            "type": "object",
            "properties": {
                "category": {"type": "string"},
                "pattern_name": {"type": "string"},
                "pattern_description": {"type": "string"},
                "application_notes": {"type": "string"}
            },
            "required": ["category", "pattern_name", "pattern_description"]
        }
    });

    let output = provider
        .generate_json(&system, &user, &schema, 4096)
        .await?;
    let arr = output
        .as_array()
        .ok_or("Expected JSON array from knowledge extraction")?;

    let entries: Vec<LearningEntry> = arr
        .iter()
        .map(|item| LearningEntry {
            id: String::new(),
            project_id: String::new(),
            source_type: source_type.into(),
            source_url: source_url.map(|s| s.into()),
            source_title: Some(source_title.into()),
            category: item["category"]
                .as_str()
                .unwrap_or("narrative_device")
                .into(),
            pattern_name: item["pattern_name"].as_str().unwrap_or("Unknown").into(),
            pattern_description: item["pattern_description"].as_str().unwrap_or("").into(),
            example_text: item["example_text"].as_str().map(|s| s.into()),
            application_notes: item["application_notes"].as_str().map(|s| s.into()),
            confidence: 0.7,
            usage_count: 0,
            last_used_at: None,
            metadata: "{}".into(),
            created_at: String::new(),
            updated_at: String::new(),
        })
        .collect();

    Ok(entries)
}

pub async fn reflect_on_chapter(
    provider: &dyn ModelClient,
    chapter_title: &str,
    chapter_body: &str,
    review_scores: &str,
    learning_entries: &[LearningEntry],
) -> Result<Vec<LearningEntry>, String> {
    let patterns_text = learning_entries
        .iter()
        .map(|e| format!("- {}: {}", e.pattern_name, e.pattern_description))
        .collect::<Vec<_>>()
        .join("\n");

    let system = format!(
        "你是一位严格的自我批评文学导师。对比本章和已学习的写作技巧，分析哪些技巧运用成功，哪些有待改进。\n\
         输出JSON数组，每项包含：category=improvement_note, pattern_name, pattern_description(具体改进建议), application_notes(如何在下一章应用)。\n\
         已学习的技巧：\n{}", patterns_text
    );
    let user = format!(
        "章节标题：{}\n\n章节内容：\n{}\n\n审稿评分：\n{}",
        chapter_title,
        &chapter_body[..chapter_body.len().min(5000)],
        review_scores
    );

    let schema = serde_json::json!({
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "category": {"type": "string"},
                "pattern_name": {"type": "string"},
                "pattern_description": {"type": "string"},
                "application_notes": {"type": "string"}
            },
            "required": ["category", "pattern_name", "pattern_description"]
        }
    });

    let output = provider
        .generate_json(&system, &user, &schema, 4096)
        .await?;
    let arr = output
        .as_array()
        .ok_or("Expected JSON array from reflection")?;

    let entries: Vec<LearningEntry> = arr
        .iter()
        .map(|item| LearningEntry {
            id: String::new(),
            project_id: String::new(),
            source_type: "self_reflection".into(),
            source_url: None,
            source_title: Some(format!("Reflection on {}", chapter_title)),
            category: "improvement_note".into(),
            pattern_name: item["pattern_name"].as_str().unwrap_or("改进建议").into(),
            pattern_description: item["pattern_description"].as_str().unwrap_or("").into(),
            example_text: None,
            application_notes: item["application_notes"].as_str().map(|s| s.into()),
            confidence: 0.8,
            usage_count: 0,
            last_used_at: None,
            metadata: "{}".into(),
            created_at: String::new(),
            updated_at: String::new(),
        })
        .collect();
    Ok(entries)
}

pub fn get_top_learning_entries(
    db: &Database,
    project_id: &str,
    limit: usize,
) -> Result<Vec<LearningEntry>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, source_type, source_url, source_title, category,
                pattern_name, pattern_description, example_text, application_notes,
                confidence, usage_count, last_used_at, metadata, created_at, updated_at
         FROM learning_entries
         WHERE project_id = ?1
         ORDER BY confidence DESC, usage_count ASC, created_at DESC
         LIMIT ?2",
        )
        .map_err(|e| format!("Prepare learning entries: {}", e))?;

    let entries = stmt
        .query_map(rusqlite::params![project_id, limit as i64], |row| {
            Ok(LearningEntry {
                id: row.get(0)?,
                project_id: row.get(1)?,
                source_type: row.get(2)?,
                source_url: row.get(3)?,
                source_title: row.get(4)?,
                category: row.get(5)?,
                pattern_name: row.get(6)?,
                pattern_description: row.get(7)?,
                example_text: row.get(8)?,
                application_notes: row.get(9)?,
                confidence: row.get(10)?,
                usage_count: row.get(11)?,
                last_used_at: row.get(12)?,
                metadata: row.get(13)?,
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        })
        .map_err(|e| format!("Query learning entries: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect learning entries: {}", e))?;

    Ok(entries)
}

pub fn mark_learning_entries_used(db: &Database, ids: &[String]) -> Result<(), String> {
    if ids.is_empty() {
        return Ok(());
    }

    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    for id in ids {
        conn.execute(
            "UPDATE learning_entries
             SET usage_count = usage_count + 1,
                 last_used_at = datetime('now'),
                 updated_at = datetime('now')
             WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| format!("Mark learning entry used: {}", e))?;
    }
    Ok(())
}

pub fn save_reflection_entries(
    db: &Database,
    project_id: &str,
    entries: &[LearningEntry],
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    for entry in entries {
        conn.execute(
            "INSERT INTO learning_entries
             (id, project_id, source_type, source_url, source_title, category,
              pattern_name, pattern_description, example_text, application_notes,
              confidence, metadata)
             VALUES (?1, ?2, 'self_reflection', NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                Database::new_uuid(),
                project_id,
                entry.source_title,
                entry.category,
                entry.pattern_name,
                entry.pattern_description,
                entry.example_text,
                entry.application_notes,
                entry.confidence,
                entry.metadata,
            ],
        )
        .map_err(|e| format!("Save reflection entry: {}", e))?;
    }
    Ok(())
}
