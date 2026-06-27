use crate::db::connection::Database;
use crate::db::{
    context_rules::ContextRule, draft_alternatives::DraftCandidate, hard_facts::HardFact,
    model_profiles::ModelProfile, prompt_presets::PromptPresetPackage, style_assets::StyleAsset,
};
use crate::extensions::host::ExtensionPackage;
use crate::models::{BibleData, Chapter, ChapterPlan, ChapterVersion, Project};
use crate::workflow::feedback_decisions::FeedbackRevisionDecision;
use crate::workflow::operator_recipes::{is_allowed_action_kind, UserOperatorRecipe};
use rusqlite::{params, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelBiblePackage {
    pub format: String,
    pub format_version: i32,
    pub source_project_id: String,
    pub exported_at: String,
    pub bible: BibleData,
    #[serde(default)]
    pub style_assets: Vec<StyleAsset>,
    #[serde(default)]
    pub hard_facts: Vec<HardFact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NovelBibleImportSummary {
    pub imported_characters: usize,
    pub imported_world_lore: usize,
    pub imported_style_assets: usize,
    pub imported_hard_facts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPackage {
    pub format: String,
    pub format_version: i32,
    pub source_project_id: String,
    pub exported_at: String,
    pub project: Project,
    pub chapter_plans: Vec<ChapterPlan>,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
    #[serde(default)]
    pub chapter_versions: Vec<ChapterVersion>,
    pub bible: NovelBiblePackage,
    #[serde(default)]
    pub context_rules: Vec<ContextRule>,
    #[serde(default)]
    pub prompt_presets: Vec<PromptPresetPackage>,
    #[serde(default)]
    pub model_profiles: Vec<ModelProfile>,
    #[serde(default)]
    pub draft_candidates: Vec<DraftCandidate>,
    #[serde(default)]
    pub extension_packages: Vec<ExtensionPackage>,
    #[serde(default)]
    pub user_recipes: Vec<UserOperatorRecipe>,
    #[serde(default)]
    pub reader_feedback: Vec<ReaderFeedbackPackage>,
    #[serde(default)]
    pub feedback_decisions: Vec<FeedbackRevisionDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderFeedbackPackage {
    pub id: String,
    pub project_id: String,
    pub chapter_id: Option<String>,
    pub source: Option<String>,
    pub external_id: Option<String>,
    pub rating: Option<f64>,
    pub comment_text: Option<String>,
    pub sentiment: Option<String>,
    pub metadata: String,
}

pub fn export_novel_bible_package(
    db: &Database,
    project_id: &str,
) -> Result<NovelBiblePackage, String> {
    crate::db::projects::get_project(db, project_id)?;
    let bible = crate::db::bible::get_bible(db, project_id)?;
    let style_assets = crate::db::style_assets::list_style_assets(db, project_id, false)?;
    let hard_facts = crate::db::hard_facts::list_hard_facts(db, project_id, false)?;
    Ok(NovelBiblePackage {
        format: "ai_novel_factory.novel_bible".to_string(),
        format_version: 1,
        source_project_id: project_id.to_string(),
        exported_at: chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
        bible,
        style_assets,
        hard_facts,
    })
}

pub fn export_project_package(db: &Database, project_id: &str) -> Result<ProjectPackage, String> {
    let project = crate::db::projects::get_project(db, project_id)?;
    let chapter_plans = crate::db::chapters::get_chapter_plans(db, project_id)?;
    let chapters = crate::db::chapters::get_chapters(db, project_id)?;
    let mut chapter_versions = Vec::new();
    for chapter in &chapters {
        chapter_versions.extend(crate::db::chapters::get_chapter_versions(db, &chapter.id)?);
    }
    let bible = export_novel_bible_package(db, project_id)?;
    let context_rules = crate::db::context_rules::list_context_rules(db, project_id)?;
    let prompt_presets = crate::db::prompt_presets::list_prompt_presets(db)?
        .iter()
        .map(|preset| crate::db::prompt_presets::export_prompt_preset_package(db, &preset.id))
        .collect::<Result<Vec<_>, _>>()?;
    let model_profiles = crate::db::model_profiles::list_model_profiles(db)?;
    let mut draft_candidates = Vec::new();
    for plan in &chapter_plans {
        draft_candidates.extend(crate::db::draft_alternatives::list_draft_candidates(
            db, &plan.id,
        )?);
    }
    let extension_packages = crate::extensions::host::list_extension_packages(db)?;
    let user_recipes = crate::workflow::operator_recipes::list_user_recipes(db, project_id, false)?;
    let reader_feedback = list_reader_feedback_package_rows(db, project_id)?;
    let feedback_decisions =
        crate::workflow::feedback_decisions::list_feedback_decisions(db, project_id)?;
    Ok(ProjectPackage {
        format: "ai_novel_factory.project".to_string(),
        format_version: 1,
        source_project_id: project_id.to_string(),
        exported_at: chrono::Local::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
        project,
        chapter_plans,
        chapters,
        chapter_versions,
        bible,
        context_rules,
        prompt_presets,
        model_profiles,
        draft_candidates,
        extension_packages,
        user_recipes,
        reader_feedback,
        feedback_decisions,
    })
}

fn list_reader_feedback_package_rows(
    db: &Database,
    project_id: &str,
) -> Result<Vec<ReaderFeedbackPackage>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, chapter_id, source, external_id, rating, comment_text, sentiment, metadata
             FROM reader_feedback
             WHERE project_id = ?1
             ORDER BY created_at ASC, id ASC",
        )
        .map_err(|e| format!("Prepare reader feedback package rows: {}", e))?;
    let rows = stmt
        .query_map(params![project_id], |row| {
            Ok(ReaderFeedbackPackage {
                id: row.get(0)?,
                project_id: row.get(1)?,
                chapter_id: row.get(2)?,
                source: row.get(3)?,
                external_id: row.get(4)?,
                rating: row.get(5)?,
                comment_text: row.get(6)?,
                sentiment: row.get(7)?,
                metadata: row.get(8)?,
            })
        })
        .map_err(|e| format!("Query reader feedback package rows: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect reader feedback package rows: {}", e))?;
    Ok(rows)
}

pub fn import_project_package(db: &Database, package: &ProjectPackage) -> Result<String, String> {
    validate_project_package(package)?;
    let mut conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Begin project package import: {}", e))?;
    let imported_project_id = Database::new_uuid();
    let project_metadata = provenance_metadata(
        &package.project.metadata,
        &package.source_project_id,
        "project",
        &package.project.id,
    )?;
    tx.execute(
        "INSERT INTO projects
            (id, name, genre, target_audience, style_profile, total_target_words,
             daily_target_words, auto_publish, quality_threshold, blog_provider, status, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            imported_project_id,
            package.project.name,
            package.project.genre,
            package.project.target_audience,
            package.project.style_profile,
            package.project.total_target_words,
            package.project.daily_target_words,
            package.project.auto_publish as i32,
            package.project.quality_threshold,
            package.project.blog_provider,
            package.project.status,
            project_metadata,
        ],
    )
    .map_err(|e| format!("Import project: {}", e))?;

    let mut plan_id_map = HashMap::new();
    for plan in &package.chapter_plans {
        let imported_plan_id = Database::new_uuid();
        let metadata = provenance_metadata(
            &plan.metadata,
            &package.source_project_id,
            "chapter_plan",
            &plan.id,
        )?;
        tx.execute(
            "INSERT INTO chapter_plans
                (id, project_id, volume_id, sequence, title, outline, pov_character_id,
                 target_word_count, required_characters, required_locations, plot_goals,
                 required_foreshadowing, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                imported_plan_id,
                imported_project_id,
                plan.volume_id,
                plan.sequence,
                plan.title,
                plan.outline,
                plan.pov_character_id,
                plan.target_word_count,
                plan.required_characters,
                plan.required_locations,
                plan.plot_goals,
                plan.required_foreshadowing,
                plan.status,
                metadata,
            ],
        )
        .map_err(|e| format!("Import chapter plan '{}': {}", plan.id, e))?;
        plan_id_map.insert(plan.id.clone(), imported_plan_id);
    }

    let mut chapter_id_map = HashMap::new();
    for chapter in &package.chapters {
        let imported_chapter_id = Database::new_uuid();
        let imported_plan_id = chapter
            .chapter_plan_id
            .as_ref()
            .and_then(|source_plan_id| plan_id_map.get(source_plan_id))
            .cloned();
        let metadata = provenance_metadata(
            &chapter.metadata,
            &package.source_project_id,
            "chapter",
            &chapter.id,
        )?;
        tx.execute(
            "INSERT INTO chapters
                (id, project_id, chapter_plan_id, sequence, title, final_version_id, status,
                 word_count, summary, published_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9, ?10)",
            params![
                imported_chapter_id,
                imported_project_id,
                imported_plan_id,
                chapter.sequence,
                chapter.title,
                chapter.status,
                chapter.word_count,
                chapter.summary,
                chapter.published_at,
                metadata,
            ],
        )
        .map_err(|e| format!("Import chapter '{}': {}", chapter.id, e))?;
        chapter_id_map.insert(chapter.id.clone(), imported_chapter_id);
    }

    let mut version_id_map = HashMap::new();
    for version in &package.chapter_versions {
        let Some(imported_chapter_id) = chapter_id_map.get(&version.chapter_id).cloned() else {
            continue;
        };
        let imported_version_id = Database::new_uuid();
        let metadata = provenance_metadata(
            &version.metadata,
            &package.source_project_id,
            "chapter_version",
            &version.id,
        )?;
        tx.execute(
            "INSERT INTO chapter_versions
                (id, chapter_id, project_id, version_number, version_type, title, body_markdown,
                 summary, word_count, model_provider, model_name, prompt_hash, context_hash,
                 created_by_agent, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                imported_version_id,
                imported_chapter_id,
                imported_project_id,
                version.version_number,
                version.version_type,
                version.title,
                version.body_markdown,
                version.summary,
                version.word_count,
                version.model_provider,
                version.model_name,
                version.prompt_hash,
                version.context_hash,
                version.created_by_agent,
                metadata,
            ],
        )
        .map_err(|e| format!("Import chapter version '{}': {}", version.id, e))?;
        version_id_map.insert(version.id.clone(), imported_version_id);
    }

    for chapter in &package.chapters {
        let Some(source_final_version_id) = chapter.final_version_id.as_ref() else {
            continue;
        };
        let Some(imported_chapter_id) = chapter_id_map.get(&chapter.id) else {
            continue;
        };
        let Some(imported_version_id) = version_id_map.get(source_final_version_id) else {
            continue;
        };
        tx.execute(
            "UPDATE chapters SET final_version_id = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![imported_version_id, imported_chapter_id],
        )
        .map_err(|e| format!("Link imported final version '{}': {}", chapter.id, e))?;
    }

    insert_project_runtime_assets(
        &tx,
        &imported_project_id,
        &plan_id_map,
        &chapter_id_map,
        &version_id_map,
        package,
    )?;
    insert_bible_rows(
        &tx,
        &imported_project_id,
        &package.bible,
        Some(&chapter_id_map),
        Some(&version_id_map),
    )?;
    tx.commit()
        .map_err(|e| format!("Commit project package import: {}", e))?;
    Ok(imported_project_id)
}

pub fn import_novel_bible_package(
    db: &Database,
    target_project_id: &str,
    package: &NovelBiblePackage,
) -> Result<NovelBibleImportSummary, String> {
    crate::db::projects::get_project(db, target_project_id)?;
    validate_novel_bible_package(package)?;

    let mut conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Begin novel bible import: {}", e))?;
    let summary = insert_bible_rows(&tx, target_project_id, package, None, None)?;

    tx.commit()
        .map_err(|e| format!("Commit novel bible import: {}", e))?;
    Ok(summary)
}

fn validate_project_package(package: &ProjectPackage) -> Result<(), String> {
    if package.format != "ai_novel_factory.project" {
        return Err(format!("Unsupported project package '{}'", package.format));
    }
    if package.format_version != 1 {
        return Err(format!(
            "Unsupported project package version {}",
            package.format_version
        ));
    }
    if package.source_project_id.trim().is_empty() {
        return Err("source_project_id is required".to_string());
    }
    if package.project.name.trim().is_empty() {
        return Err("project name is required".to_string());
    }
    for rule in &package.context_rules {
        if rule.name.trim().is_empty() {
            return Err("context rule name is required".to_string());
        }
        if rule.content.trim().is_empty() {
            return Err("context rule content is required".to_string());
        }
    }
    for preset in &package.prompt_presets {
        if preset.id.trim().is_empty() || preset.name.trim().is_empty() {
            return Err("prompt preset id and name are required".to_string());
        }
        for unit in &preset.units {
            if unit.identifier.trim().is_empty() {
                return Err("prompt unit identifier is required".to_string());
            }
        }
    }
    for profile in &package.model_profiles {
        if profile.id.trim().is_empty()
            || profile.name.trim().is_empty()
            || profile.provider.trim().is_empty()
            || profile.model.trim().is_empty()
        {
            return Err("model profile id, name, provider, and model are required".to_string());
        }
    }
    let plan_ids = package
        .chapter_plans
        .iter()
        .map(|plan| plan.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    for candidate in &package.draft_candidates {
        if candidate.title.trim().is_empty() || candidate.body_markdown.trim().is_empty() {
            return Err("draft candidate title and body are required".to_string());
        }
        if !plan_ids.contains(candidate.chapter_plan_id.as_str()) {
            return Err(format!(
                "draft candidate '{}' references missing chapter plan '{}'",
                candidate.id, candidate.chapter_plan_id
            ));
        }
    }
    for extension in &package.extension_packages {
        crate::extensions::manifest::validate_extension_manifest(&extension.manifest)?;
    }
    for recipe in &package.user_recipes {
        if recipe.name.trim().is_empty() || recipe.actions.is_empty() {
            return Err("user recipe name and actions are required".to_string());
        }
        for action in &recipe.actions {
            if !is_allowed_action_kind(&action.kind) {
                return Err(format!("Unsupported recipe action '{}'", action.kind));
            }
        }
    }
    for feedback in &package.reader_feedback {
        if feedback.id.trim().is_empty() {
            return Err("reader feedback id is required".to_string());
        }
    }
    for decision in &package.feedback_decisions {
        if decision.title.trim().is_empty() || decision.body_markdown.trim().is_empty() {
            return Err("feedback decision title and body are required".to_string());
        }
    }
    validate_novel_bible_package(&package.bible)?;
    Ok(())
}

fn insert_project_runtime_assets(
    tx: &Transaction<'_>,
    imported_project_id: &str,
    plan_id_map: &HashMap<String, String>,
    chapter_id_map: &HashMap<String, String>,
    version_id_map: &HashMap<String, String>,
    package: &ProjectPackage,
) -> Result<(), String> {
    for rule in &package.context_rules {
        let metadata = provenance_metadata_from_value(
            &rule.metadata,
            &package.source_project_id,
            "context_rule",
            &rule.id,
        )?;
        tx.execute(
            "INSERT INTO context_rules
                (id, project_id, name, primary_keywords, secondary_keywords, entity_refs,
                 chapter_ranges, priority, token_budget, sticky_chapters, cooldown_chapters,
                 content, source_type, source_id, enabled, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                Database::new_uuid(),
                imported_project_id,
                rule.name,
                serde_json::to_string(&rule.primary_keywords)
                    .map_err(|e| format!("Serialize context primary keywords: {}", e))?,
                serde_json::to_string(&rule.secondary_keywords)
                    .map_err(|e| format!("Serialize context secondary keywords: {}", e))?,
                serde_json::to_string(&rule.entity_refs)
                    .map_err(|e| format!("Serialize context entity refs: {}", e))?,
                serde_json::to_string(&rule.chapter_ranges)
                    .map_err(|e| format!("Serialize context chapter ranges: {}", e))?,
                rule.priority,
                rule.token_budget,
                rule.sticky_chapters,
                rule.cooldown_chapters,
                rule.content,
                rule.source_type,
                rule.source_id,
                rule.enabled as i32,
                metadata,
            ],
        )
        .map_err(|e| format!("Import context rule '{}': {}", rule.id, e))?;
    }

    for preset in &package.prompt_presets {
        let preset_metadata = serde_json::to_string(&preset.metadata)
            .map_err(|e| format!("Serialize prompt preset metadata: {}", e))?;
        tx.execute(
            "INSERT INTO prompt_presets
                (id, name, description, scope, is_builtin, status, metadata, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                scope = excluded.scope,
                is_builtin = excluded.is_builtin,
                status = 'active',
                metadata = excluded.metadata,
                updated_at = datetime('now')",
            params![
                preset.id,
                preset.name,
                preset.description,
                preset.scope,
                preset.is_builtin as i32,
                preset_metadata,
            ],
        )
        .map_err(|e| format!("Import prompt preset '{}': {}", preset.id, e))?;
        tx.execute(
            "DELETE FROM prompt_preset_units WHERE preset_id = ?1",
            params![preset.id],
        )
        .map_err(|e| format!("Clear prompt units '{}': {}", preset.id, e))?;
        for unit in &preset.units {
            let metadata = serde_json::to_string(&unit.metadata)
                .map_err(|e| format!("Serialize prompt unit metadata: {}", e))?;
            tx.execute(
                "INSERT INTO prompt_preset_units
                    (id, preset_id, identifier, role, unit_order, enabled, injection_position,
                     generation_phase, content, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    Database::new_uuid(),
                    preset.id,
                    unit.identifier,
                    unit.role,
                    unit.order,
                    unit.enabled as i32,
                    unit.injection_position,
                    unit.generation_phase,
                    unit.content,
                    metadata,
                ],
            )
            .map_err(|e| format!("Import prompt unit '{}': {}", unit.identifier, e))?;
        }
    }

    for profile in &package.model_profiles {
        let metadata = serde_json::to_string(&profile.metadata)
            .map_err(|e| format!("Serialize model profile metadata: {}", e))?;
        tx.execute(
            "INSERT INTO model_profiles
                (id, name, provider, base_url, model, context_window, supports_json,
                 supports_streaming, supports_embeddings, input_cost_per_million,
                 output_cost_per_million, intended_use, status, metadata, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'active', ?13, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                provider = excluded.provider,
                base_url = excluded.base_url,
                model = excluded.model,
                context_window = excluded.context_window,
                supports_json = excluded.supports_json,
                supports_streaming = excluded.supports_streaming,
                supports_embeddings = excluded.supports_embeddings,
                input_cost_per_million = excluded.input_cost_per_million,
                output_cost_per_million = excluded.output_cost_per_million,
                intended_use = excluded.intended_use,
                status = 'active',
                metadata = excluded.metadata,
                updated_at = datetime('now')",
            params![
                profile.id,
                profile.name,
                profile.provider,
                profile.base_url,
                profile.model,
                profile.context_window,
                profile.supports_json as i32,
                profile.supports_streaming as i32,
                profile.supports_embeddings as i32,
                profile.input_cost_per_million,
                profile.output_cost_per_million,
                profile.intended_use,
                metadata,
            ],
        )
        .map_err(|e| format!("Import model profile '{}': {}", profile.id, e))?;
    }

    for candidate in &package.draft_candidates {
        let Some(imported_plan_id) = plan_id_map.get(&candidate.chapter_plan_id) else {
            continue;
        };
        let metadata = provenance_metadata_from_value(
            &candidate.metadata,
            &package.source_project_id,
            "draft_candidate",
            &candidate.id,
        )?;
        tx.execute(
            "INSERT INTO draft_alternatives
                (id, project_id, chapter_plan_id, candidate_number, title, body_markdown,
                 summary, word_count, prompt_hash, context_hash, model_profile_id,
                 review_notes, estimated_cost_usd, status, selection_reason, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                Database::new_uuid(),
                imported_project_id,
                imported_plan_id,
                candidate.candidate_number,
                candidate.title,
                candidate.body_markdown,
                candidate.summary,
                candidate.word_count,
                candidate.prompt_hash,
                candidate.context_hash,
                candidate.model_profile_id,
                serde_json::to_string(&candidate.review_notes)
                    .map_err(|e| format!("Serialize draft review notes: {}", e))?,
                candidate.estimated_cost_usd,
                candidate.status,
                candidate.selection_reason,
                metadata,
            ],
        )
        .map_err(|e| format!("Import draft candidate '{}': {}", candidate.id, e))?;
    }

    for extension in &package.extension_packages {
        let manifest = serde_json::to_string(&extension.manifest)
            .map_err(|e| format!("Serialize extension manifest: {}", e))?;
        let contributions = serde_json::to_string(&extension.contributions)
            .map_err(|e| format!("Serialize extension contributions: {}", e))?;
        let metadata = serde_json::to_string(&extension.manifest.metadata)
            .map_err(|e| format!("Serialize extension metadata: {}", e))?;
        tx.execute(
            "INSERT INTO extension_packages
                (id, name, version, description, manifest, contributions, enabled, status, metadata, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 'installed', ?7, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                version = excluded.version,
                description = excluded.description,
                manifest = excluded.manifest,
                contributions = excluded.contributions,
                enabled = 0,
                status = 'installed',
                metadata = excluded.metadata,
                updated_at = datetime('now')",
            params![
                extension.manifest.id,
                extension.manifest.name,
                extension.manifest.version,
                extension.manifest.description,
                manifest,
                contributions,
                metadata,
            ],
        )
        .map_err(|e| format!("Import extension package '{}': {}", extension.manifest.id, e))?;
    }

    for recipe in &package.user_recipes {
        let metadata = provenance_metadata_from_value(
            &recipe.metadata,
            &package.source_project_id,
            "user_operator_recipe",
            &recipe.id,
        )?;
        tx.execute(
            "INSERT INTO user_operator_recipes
                (id, project_id, name, description, parameter_schema, actions, enabled, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Database::new_uuid(),
                imported_project_id,
                recipe.name,
                recipe.description,
                serde_json::to_string(&recipe.parameter_schema)
                    .map_err(|e| format!("Serialize user recipe parameter schema: {}", e))?,
                serde_json::to_string(&recipe.actions)
                    .map_err(|e| format!("Serialize user recipe actions: {}", e))?,
                recipe.enabled as i32,
                metadata,
            ],
        )
        .map_err(|e| format!("Import user recipe '{}': {}", recipe.id, e))?;
    }

    let mut feedback_id_map = HashMap::new();
    for feedback in &package.reader_feedback {
        let imported_feedback_id = Database::new_uuid();
        let imported_chapter_id = feedback
            .chapter_id
            .as_ref()
            .and_then(|source_chapter_id| chapter_id_map.get(source_chapter_id))
            .cloned();
        let metadata = provenance_metadata(
            &feedback.metadata,
            &package.source_project_id,
            "reader_feedback",
            &feedback.id,
        )?;
        tx.execute(
            "INSERT INTO reader_feedback
                (id, project_id, chapter_id, source, external_id, rating, comment_text, sentiment, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                imported_feedback_id,
                imported_project_id,
                imported_chapter_id,
                feedback.source,
                feedback.external_id,
                feedback.rating,
                feedback.comment_text,
                feedback.sentiment,
                metadata,
            ],
        )
        .map_err(|e| format!("Import reader feedback '{}': {}", feedback.id, e))?;
        feedback_id_map.insert(feedback.id.clone(), imported_feedback_id);
    }

    for decision in &package.feedback_decisions {
        let Some(imported_feedback_id) = feedback_id_map.get(&decision.feedback_id).cloned() else {
            continue;
        };
        let Some(imported_chapter_id) = chapter_id_map.get(&decision.chapter_id).cloned() else {
            continue;
        };
        let imported_version_id = decision
            .resulting_chapter_version_id
            .as_ref()
            .and_then(|source_version_id| version_id_map.get(source_version_id))
            .cloned();
        let metadata = provenance_metadata_from_value(
            &decision.metadata,
            &package.source_project_id,
            "feedback_revision_decision",
            &decision.id,
        )?;
        tx.execute(
            "INSERT INTO feedback_revision_decisions
                (id, project_id, feedback_id, chapter_id, title, body_markdown, summary,
                 status, decision_note, resulting_chapter_version_id, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                Database::new_uuid(),
                imported_project_id,
                imported_feedback_id,
                imported_chapter_id,
                decision.title,
                decision.body_markdown,
                decision.summary,
                decision.status,
                decision.decision_note,
                imported_version_id,
                metadata,
            ],
        )
        .map_err(|e| format!("Import feedback decision '{}': {}", decision.id, e))?;
    }

    Ok(())
}

fn insert_bible_rows(
    tx: &Transaction<'_>,
    target_project_id: &str,
    package: &NovelBiblePackage,
    chapter_id_map: Option<&HashMap<String, String>>,
    version_id_map: Option<&HashMap<String, String>>,
) -> Result<NovelBibleImportSummary, String> {
    let mut summary = NovelBibleImportSummary::default();
    let mut character_id_map = HashMap::new();
    let mut location_id_map = HashMap::new();

    for character in &package.bible.characters {
        let imported_id = Database::new_uuid();
        let metadata = provenance_metadata(
            &character.metadata,
            &package.source_project_id,
            "character",
            &character.id,
        )?;
        tx.execute(
            "INSERT INTO characters
                (id, project_id, name, aliases, role, personality, motivation, speech_style,
                 appearance, backstory, relationship_map, locked_fields, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                imported_id,
                target_project_id,
                character.name,
                character.aliases,
                character.role,
                character.personality,
                character.motivation,
                character.speech_style,
                character.appearance,
                character.backstory,
                character.relationship_map,
                character.locked_fields,
                character.status,
                metadata,
            ],
        )
        .map_err(|e| format!("Import character '{}': {}", character.name, e))?;
        character_id_map.insert(character.id.clone(), imported_id);
        summary.imported_characters += 1;
    }

    for location in &package.bible.locations {
        let imported_id = Database::new_uuid();
        let metadata = provenance_metadata(
            &location.metadata,
            &package.source_project_id,
            "location",
            &location.id,
        )?;
        tx.execute(
            "INSERT INTO locations
                (id, project_id, name, type, description, rules, connected_locations, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                imported_id,
                target_project_id,
                location.name,
                location.r#type,
                location.description,
                location.rules,
                location.connected_locations,
                location.status,
                metadata,
            ],
        )
        .map_err(|e| format!("Import location '{}': {}", location.name, e))?;
        location_id_map.insert(location.id.clone(), imported_id);
    }

    for organization in &package.bible.organizations {
        let metadata = provenance_metadata(
            &organization.metadata,
            &package.source_project_id,
            "organization",
            &organization.id,
        )?;
        tx.execute(
            "INSERT INTO organizations
                (id, project_id, name, description, hierarchy, goals, relationship_map, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                Database::new_uuid(),
                target_project_id,
                organization.name,
                organization.description,
                organization.hierarchy,
                organization.goals,
                organization.relationship_map,
                organization.status,
                metadata,
            ],
        )
        .map_err(|e| format!("Import organization '{}': {}", organization.name, e))?;
    }

    for item in &package.bible.items {
        let metadata =
            provenance_metadata(&item.metadata, &package.source_project_id, "item", &item.id)?;
        let owner_character_id = remap_optional_ref(&item.owner_character_id, &character_id_map);
        let location_id = remap_optional_ref(&item.location_id, &location_id_map);
        tx.execute(
            "INSERT INTO items
                (id, project_id, name, item_type, owner_character_id, location_id,
                 description, abilities, limitations, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                Database::new_uuid(),
                target_project_id,
                item.name,
                item.item_type,
                owner_character_id,
                location_id,
                item.description,
                item.abilities,
                item.limitations,
                item.status,
                metadata,
            ],
        )
        .map_err(|e| format!("Import item '{}': {}", item.name, e))?;
    }

    for lore in &package.bible.world_lore {
        let metadata = provenance_metadata(
            &lore.metadata,
            &package.source_project_id,
            "world_lore",
            &lore.id,
        )?;
        tx.execute(
            "INSERT INTO world_lore
                (id, project_id, lore_type, title, content, locked, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Database::new_uuid(),
                target_project_id,
                lore.lore_type,
                lore.title,
                lore.content,
                lore.locked as i32,
                lore.status,
                metadata,
            ],
        )
        .map_err(|e| {
            format!(
                "Import world lore '{}': {}",
                lore.title.as_deref().unwrap_or(""),
                e
            )
        })?;
        summary.imported_world_lore += 1;
    }

    for magic in &package.bible.magic_systems {
        let metadata = provenance_metadata(
            &magic.metadata,
            &package.source_project_id,
            "magic_system",
            &magic.id,
        )?;
        tx.execute(
            "INSERT INTO magic_or_power_systems
                (id, project_id, name, description, rules, limitations, progression, locked, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                Database::new_uuid(),
                target_project_id,
                magic.name,
                magic.description,
                magic.rules,
                magic.limitations,
                magic.progression,
                magic.locked as i32,
                magic.status,
                metadata,
            ],
        )
        .map_err(|e| {
            format!(
                "Import magic system '{}': {}",
                magic.name.as_deref().unwrap_or(""),
                e
            )
        })?;
    }

    for rule in &package.bible.canon_rules {
        let metadata = provenance_metadata(
            &rule.metadata,
            &package.source_project_id,
            "canon_rule",
            &rule.id,
        )?;
        tx.execute(
            "INSERT INTO canon_rules
                (id, project_id, rule_type, rule_text, severity, locked, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Database::new_uuid(),
                target_project_id,
                rule.rule_type,
                rule.rule_text,
                rule.severity,
                rule.locked as i32,
                rule.status,
                metadata,
            ],
        )
        .map_err(|e| format!("Import canon rule '{}': {}", rule.id, e))?;
    }

    for thread in &package.bible.plot_threads {
        let metadata = provenance_metadata(
            &thread.metadata,
            &package.source_project_id,
            "plot_thread",
            &thread.id,
        )?;
        tx.execute(
            "INSERT INTO plot_threads
                (id, project_id, name, description, priority, arc_status, introduced_chapter_id,
                 expected_resolution_chapter_id, resolved_chapter_id, related_characters,
                 related_chapters, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                Database::new_uuid(),
                target_project_id,
                thread.name,
                thread.description,
                thread.priority,
                thread.arc_status,
                remap_chapter_ref(&thread.introduced_chapter_id, chapter_id_map),
                remap_chapter_ref(&thread.expected_resolution_chapter_id, chapter_id_map),
                remap_chapter_ref(&thread.resolved_chapter_id, chapter_id_map),
                thread.related_characters,
                thread.related_chapters,
                metadata,
            ],
        )
        .map_err(|e| format!("Import plot thread '{}': {}", thread.id, e))?;
    }

    for clue in &package.bible.foreshadowing {
        let metadata = provenance_metadata(
            &clue.metadata,
            &package.source_project_id,
            "foreshadowing",
            &clue.id,
        )?;
        tx.execute(
            "INSERT INTO foreshadowing
                (id, project_id, clue_text, intended_payoff, introduced_chapter_id,
                 expected_resolution_chapter_id, resolved_chapter_id, status, importance, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                Database::new_uuid(),
                target_project_id,
                clue.clue_text,
                clue.intended_payoff,
                remap_chapter_ref(&clue.introduced_chapter_id, chapter_id_map),
                remap_chapter_ref(&clue.expected_resolution_chapter_id, chapter_id_map),
                remap_chapter_ref(&clue.resolved_chapter_id, chapter_id_map),
                clue.status,
                clue.importance,
                metadata,
            ],
        )
        .map_err(|e| format!("Import foreshadowing '{}': {}", clue.id, e))?;
    }

    for guide in &package.bible.style_guides {
        let metadata = provenance_metadata(
            &guide.metadata,
            &package.source_project_id,
            "style_guide",
            &guide.id,
        )?;
        tx.execute(
            "INSERT INTO style_guides
                (id, project_id, name, style_text, positive_examples, negative_examples, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Database::new_uuid(),
                target_project_id,
                guide.name,
                guide.style_text,
                guide.positive_examples,
                guide.negative_examples,
                guide.status,
                metadata,
            ],
        )
        .map_err(|e| format!("Import style guide '{}': {}", guide.id, e))?;
    }

    for event in &package.bible.timeline_events {
        let metadata = provenance_metadata(
            &event.metadata,
            &package.source_project_id,
            "timeline_event",
            &event.id,
        )?;
        tx.execute(
            "INSERT INTO timeline_events
                (id, project_id, chapter_id, event_time_label, sequence, event_summary,
                 involved_characters, involved_locations, consequences, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                Database::new_uuid(),
                target_project_id,
                remap_chapter_ref(&event.chapter_id, chapter_id_map),
                event.event_time_label,
                event.sequence,
                event.event_summary,
                event.involved_characters,
                event.involved_locations,
                event.consequences,
                event.status,
                metadata,
            ],
        )
        .map_err(|e| format!("Import timeline event '{}': {}", event.id, e))?;
    }

    for asset in &package.style_assets {
        let metadata = provenance_metadata_from_value(
            &asset.metadata,
            &package.source_project_id,
            "style_asset",
            &asset.id,
        )?;
        tx.execute(
            "INSERT INTO style_assets
                (id, project_id, name, asset_type, scope_type, scope_id, features,
                 positive_examples, negative_examples, anti_ai_rules, enabled, priority, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                Database::new_uuid(),
                target_project_id,
                asset.name,
                asset.asset_type,
                asset.scope_type,
                asset.scope_id,
                serde_json::to_string(&asset.features)
                    .map_err(|e| format!("Serialize style asset features: {}", e))?,
                serde_json::to_string(&asset.positive_examples)
                    .map_err(|e| format!("Serialize style asset positive examples: {}", e))?,
                serde_json::to_string(&asset.negative_examples)
                    .map_err(|e| format!("Serialize style asset negative examples: {}", e))?,
                serde_json::to_string(&asset.anti_ai_rules)
                    .map_err(|e| format!("Serialize style asset anti AI rules: {}", e))?,
                asset.enabled as i32,
                asset.priority,
                metadata,
            ],
        )
        .map_err(|e| format!("Import style asset '{}': {}", asset.id, e))?;
        summary.imported_style_assets += 1;
    }

    for fact in &package.hard_facts {
        let metadata = provenance_metadata_from_value(
            &fact.metadata,
            &package.source_project_id,
            "hard_fact",
            &fact.id,
        )?;
        let imported_chapter_id = fact
            .chapter_id
            .as_ref()
            .and_then(|source_id| chapter_id_map.and_then(|map| map.get(source_id)))
            .cloned();
        let imported_version_id = fact
            .chapter_version_id
            .as_ref()
            .and_then(|source_id| version_id_map.and_then(|map| map.get(source_id)))
            .cloned();
        tx.execute(
            "INSERT INTO hard_facts
                (id, project_id, chapter_id, chapter_version_id, fact_type, subject, predicate,
                 object, value_text, certainty, source_quote, scope, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                Database::new_uuid(),
                target_project_id,
                imported_chapter_id,
                imported_version_id,
                fact.fact_type,
                fact.subject,
                fact.predicate,
                fact.object,
                fact.value_text,
                fact.certainty,
                fact.source_quote,
                fact.scope,
                fact.status,
                metadata,
            ],
        )
        .map_err(|e| format!("Import hard fact '{}': {}", fact.id, e))?;
        summary.imported_hard_facts += 1;
    }
    Ok(summary)
}

fn remap_optional_ref(
    source_id: &Option<String>,
    id_map: &HashMap<String, String>,
) -> Option<String> {
    source_id
        .as_ref()
        .map(|id| id_map.get(id).cloned().unwrap_or_else(|| id.clone()))
}

fn remap_chapter_ref(
    source_id: &Option<String>,
    chapter_id_map: Option<&HashMap<String, String>>,
) -> Option<String> {
    source_id
        .as_ref()
        .and_then(|id| chapter_id_map.and_then(|map| map.get(id)).cloned())
}

fn validate_novel_bible_package(package: &NovelBiblePackage) -> Result<(), String> {
    if package.format != "ai_novel_factory.novel_bible" {
        return Err(format!(
            "Unsupported novel bible package '{}'",
            package.format
        ));
    }
    if package.format_version != 1 {
        return Err(format!(
            "Unsupported novel bible package version {}",
            package.format_version
        ));
    }
    if package.source_project_id.trim().is_empty() {
        return Err("source_project_id is required".to_string());
    }
    for character in &package.bible.characters {
        if character.name.trim().is_empty() {
            return Err("character name is required".to_string());
        }
    }
    for lore in &package.bible.world_lore {
        if lore.title.as_deref().unwrap_or("").trim().is_empty() {
            return Err("world lore title is required".to_string());
        }
        if lore.content.as_deref().unwrap_or("").trim().is_empty() {
            return Err("world lore content is required".to_string());
        }
    }
    for asset in &package.style_assets {
        if asset.name.trim().is_empty() || asset.asset_type.trim().is_empty() {
            return Err("style asset name and asset_type are required".to_string());
        }
    }
    for fact in &package.hard_facts {
        if fact.fact_type.trim().is_empty()
            || fact.subject.trim().is_empty()
            || fact.predicate.trim().is_empty()
            || fact.object.trim().is_empty()
        {
            return Err(
                "hard fact fact_type, subject, predicate, and object are required".to_string(),
            );
        }
    }
    Ok(())
}

fn provenance_metadata(
    raw_metadata: &str,
    source_project_id: &str,
    source_type: &str,
    source_id: &str,
) -> Result<String, String> {
    let mut metadata = serde_json::from_str::<Value>(raw_metadata).unwrap_or_else(|_| json!({}));
    if !metadata.is_object() {
        metadata = json!({});
    }
    metadata["source_provenance"] = json!({
        "format": "ai_novel_factory.novel_bible",
        "source_project_id": source_project_id,
        "source_type": source_type,
        "source_id": source_id,
    });
    serde_json::to_string(&metadata).map_err(|e| format!("Serialize provenance metadata: {}", e))
}

fn provenance_metadata_from_value(
    raw_metadata: &Value,
    source_project_id: &str,
    source_type: &str,
    source_id: &str,
) -> Result<String, String> {
    let mut metadata = raw_metadata.clone();
    if !metadata.is_object() {
        metadata = json!({});
    }
    metadata["source_provenance"] = json!({
        "format": "ai_novel_factory.project",
        "source_project_id": source_project_id,
        "source_type": source_type,
        "source_id": source_id,
    });
    serde_json::to_string(&metadata).map_err(|e| format!("Serialize provenance metadata: {}", e))
}
