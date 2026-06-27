-- ============================================================================
-- 001_init_sqlite.sql
-- AI Novel Factory — SQLite Schema (ported from PostgreSQL)
--
-- Type mappings:
--   UUID          → TEXT
--   JSONB         → TEXT (JSON string)
--   TIMESTAMPTZ   → TEXT (ISO 8601)
--   BOOLEAN       → INTEGER (0/1)
--   INTEGER       → INTEGER
--   NUMERIC       → REAL
--   vector(1536)  → handled by sqlite-vec virtual table
--   UUID[]        → TEXT (JSON array string, default '[]')
--   TEXT[]        → TEXT (JSON array string, default '[]')
--   gen_random_uuid() → hex(randomblob(16)) with dashes
--   now()         → datetime('now')
--   SERIAL/sequences → AUTOINCREMENT or application-level
--   Trigger functions → application-level updated_at management
-- ============================================================================

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;

-- ============================================================================
-- 0. schema_migrations
-- ============================================================================

CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    description TEXT,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- 1. projects — 小说项目主表
-- ============================================================================

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    genre TEXT,
    target_audience TEXT,
    style_profile TEXT NOT NULL DEFAULT '{}',
    total_target_words INTEGER,
    daily_target_words INTEGER,
    auto_publish INTEGER NOT NULL DEFAULT 0,
    quality_threshold INTEGER NOT NULL DEFAULT 85,
    blog_provider TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);

-- ============================================================================
-- 2. volumes — 卷
-- ============================================================================

CREATE TABLE IF NOT EXISTS volumes (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL,
    title TEXT NOT NULL,
    summary TEXT,
    target_word_count INTEGER,
    status TEXT NOT NULL DEFAULT 'planned',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_volumes_project_id ON volumes(project_id);

-- ============================================================================
-- 3. chapter_plans — 章节计划
-- ============================================================================

CREATE TABLE IF NOT EXISTS chapter_plans (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    volume_id TEXT REFERENCES volumes(id) ON DELETE SET NULL,
    sequence INTEGER NOT NULL,
    title TEXT,
    outline TEXT,
    pov_character_id TEXT,
    target_word_count INTEGER,
    required_characters TEXT NOT NULL DEFAULT '[]',
    required_locations TEXT NOT NULL DEFAULT '[]',
    plot_goals TEXT NOT NULL DEFAULT '[]',
    required_foreshadowing TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'planned'
        CHECK (status IN ('planned','in_progress','completed','skipped','archived')),
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_chapter_plans_project_id ON chapter_plans(project_id);
CREATE INDEX IF NOT EXISTS idx_chapter_plans_sequence ON chapter_plans(project_id, sequence);
CREATE INDEX IF NOT EXISTS idx_chapter_plans_status ON chapter_plans(status);

-- ============================================================================
-- 4. chapters — 章节主表
-- ============================================================================

CREATE TABLE IF NOT EXISTS chapters (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_plan_id TEXT REFERENCES chapter_plans(id) ON DELETE SET NULL,
    sequence INTEGER NOT NULL,
    title TEXT,
    final_version_id TEXT,
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft','reviewing','revised','final','published','needs_human_review','failed')),
    word_count INTEGER,
    summary TEXT,
    published_at TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_chapters_project_id ON chapters(project_id);
CREATE INDEX IF NOT EXISTS idx_chapters_sequence ON chapters(project_id, sequence);
CREATE INDEX IF NOT EXISTS idx_chapters_status ON chapters(project_id, status);
CREATE INDEX IF NOT EXISTS idx_chapters_published_at ON chapters(published_at);

-- ============================================================================
-- 5. chapter_versions — 章节版本历史
-- ============================================================================

CREATE TABLE IF NOT EXISTS chapter_versions (
    id TEXT PRIMARY KEY,
    chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    version_type TEXT NOT NULL DEFAULT 'draft'
        CHECK (version_type IN ('draft','revised','final','accepted_candidate')),
    title TEXT,
    body_markdown TEXT,
    summary TEXT,
    word_count INTEGER,
    model_provider TEXT,
    model_name TEXT,
    prompt_hash TEXT,
    context_hash TEXT,
    created_by_agent TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_chapter_versions_chapter_id ON chapter_versions(chapter_id);
CREATE INDEX IF NOT EXISTS idx_chapter_versions_number ON chapter_versions(chapter_id, version_number);

-- ============================================================================
-- 6. characters — 人物表
-- ============================================================================

CREATE TABLE IF NOT EXISTS characters (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    aliases TEXT NOT NULL DEFAULT '[]',
    role TEXT,
    personality TEXT,
    motivation TEXT,
    speech_style TEXT,
    appearance TEXT,
    backstory TEXT,
    relationship_map TEXT NOT NULL DEFAULT '{}',
    locked_fields TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_characters_project_id ON characters(project_id);
CREATE INDEX IF NOT EXISTS idx_characters_name ON characters(project_id, name);
CREATE INDEX IF NOT EXISTS idx_characters_status ON characters(status);

-- ============================================================================
-- 7. character_states — 人物状态快照
-- ============================================================================

CREATE TABLE IF NOT EXISTS character_states (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    after_chapter_id TEXT REFERENCES chapters(id) ON DELETE SET NULL,
    physical_state TEXT,
    emotional_state TEXT,
    knowledge_state TEXT,
    relationship_state TEXT NOT NULL DEFAULT '{}',
    location_id TEXT,
    inventory TEXT NOT NULL DEFAULT '[]',
    open_conflicts TEXT NOT NULL DEFAULT '[]',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_character_states_character_id ON character_states(character_id);
CREATE INDEX IF NOT EXISTS idx_character_states_chapter_id ON character_states(after_chapter_id);

-- ============================================================================
-- 8. locations — 地点
-- ============================================================================

CREATE TABLE IF NOT EXISTS locations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    type TEXT,
    description TEXT,
    rules TEXT,
    connected_locations TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_locations_project_id ON locations(project_id);
CREATE INDEX IF NOT EXISTS idx_locations_name ON locations(project_id, name);

-- ============================================================================
-- 9. organizations — 组织
-- ============================================================================

CREATE TABLE IF NOT EXISTS organizations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    hierarchy TEXT NOT NULL DEFAULT '{}',
    goals TEXT,
    relationship_map TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_organizations_project_id ON organizations(project_id);

-- ============================================================================
-- 10. items — 道具/物品
-- ============================================================================

CREATE TABLE IF NOT EXISTS items (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    item_type TEXT,
    owner_character_id TEXT REFERENCES characters(id) ON DELETE SET NULL,
    location_id TEXT REFERENCES locations(id) ON DELETE SET NULL,
    description TEXT,
    abilities TEXT,
    limitations TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_items_project_id ON items(project_id);
CREATE INDEX IF NOT EXISTS idx_items_name ON items(project_id, name);

-- ============================================================================
-- 11. world_lore — 世界观设定
-- ============================================================================

CREATE TABLE IF NOT EXISTS world_lore (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    lore_type TEXT,
    title TEXT,
    content TEXT,
    locked INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_world_lore_project_id ON world_lore(project_id);

-- ============================================================================
-- 12. magic_or_power_systems — 力量体系
-- ============================================================================

CREATE TABLE IF NOT EXISTS magic_or_power_systems (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT,
    description TEXT,
    rules TEXT,
    limitations TEXT,
    progression TEXT NOT NULL DEFAULT '{}',
    locked INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_magic_or_power_systems_project_id ON magic_or_power_systems(project_id);

-- ============================================================================
-- 13. timeline_events — 时间线事件
-- ============================================================================

CREATE TABLE IF NOT EXISTS timeline_events (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_id TEXT REFERENCES chapters(id) ON DELETE SET NULL,
    event_time_label TEXT,
    sequence INTEGER,
    event_summary TEXT,
    involved_characters TEXT NOT NULL DEFAULT '[]',
    involved_locations TEXT NOT NULL DEFAULT '[]',
    consequences TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_timeline_events_project_id ON timeline_events(project_id);
CREATE INDEX IF NOT EXISTS idx_timeline_events_chapter_id ON timeline_events(chapter_id);

-- ============================================================================
-- 14. plot_threads — 剧情线
-- ============================================================================

CREATE TABLE IF NOT EXISTS plot_threads (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT,
    description TEXT,
    priority INTEGER NOT NULL DEFAULT 3,
    arc_status TEXT NOT NULL DEFAULT 'open'
        CHECK (arc_status IN ('open','active','paused','resolved','abandoned')),
    introduced_chapter_id TEXT REFERENCES chapters(id) ON DELETE SET NULL,
    expected_resolution_chapter_id TEXT REFERENCES chapters(id) ON DELETE SET NULL,
    resolved_chapter_id TEXT REFERENCES chapters(id) ON DELETE SET NULL,
    related_characters TEXT NOT NULL DEFAULT '[]',
    related_chapters TEXT NOT NULL DEFAULT '[]',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_plot_threads_project_id ON plot_threads(project_id);
CREATE INDEX IF NOT EXISTS idx_plot_threads_arc_status ON plot_threads(arc_status);

-- ============================================================================
-- 15. foreshadowing — 伏笔管理
-- ============================================================================

CREATE TABLE IF NOT EXISTS foreshadowing (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    clue_text TEXT,
    intended_payoff TEXT,
    introduced_chapter_id TEXT REFERENCES chapters(id) ON DELETE SET NULL,
    expected_resolution_chapter_id TEXT REFERENCES chapters(id) ON DELETE SET NULL,
    resolved_chapter_id TEXT REFERENCES chapters(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'open',
    importance INTEGER NOT NULL DEFAULT 3,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_foreshadowing_project_id ON foreshadowing(project_id);
CREATE INDEX IF NOT EXISTS idx_foreshadowing_status ON foreshadowing(status);
CREATE INDEX IF NOT EXISTS idx_foreshadowing_introduced ON foreshadowing(introduced_chapter_id);
CREATE INDEX IF NOT EXISTS idx_foreshadowing_resolved ON foreshadowing(resolved_chapter_id);

-- ============================================================================
-- 16. canon_rules — 圣经锁定规则
-- ============================================================================

CREATE TABLE IF NOT EXISTS canon_rules (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    rule_type TEXT,
    rule_text TEXT,
    severity TEXT NOT NULL DEFAULT 'hard'
        CHECK (severity IN ('hard','soft')),
    locked INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_canon_rules_project_id ON canon_rules(project_id);
CREATE INDEX IF NOT EXISTS idx_canon_rules_severity ON canon_rules(severity);

-- ============================================================================
-- 17. style_guides — 风格指南
-- ============================================================================

CREATE TABLE IF NOT EXISTS style_guides (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT,
    style_text TEXT,
    positive_examples TEXT NOT NULL DEFAULT '[]',
    negative_examples TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_style_guides_project_id ON style_guides(project_id);

-- ============================================================================
-- 18. generation_jobs — 生成任务（幂等锁）
-- ============================================================================

CREATE TABLE IF NOT EXISTS generation_jobs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_plan_id TEXT NOT NULL REFERENCES chapter_plans(id) ON DELETE CASCADE,
    job_date TEXT NOT NULL DEFAULT (date('now')),
    status TEXT NOT NULL DEFAULT 'started'
        CHECK (status IN ('started','draft_created','reviewing','revising','publishing','completed','failed','needs_human_review','skipped','cancelled')),
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, chapter_plan_id, job_date)
);

CREATE INDEX IF NOT EXISTS idx_generation_jobs_project_id ON generation_jobs(project_id);
CREATE INDEX IF NOT EXISTS idx_generation_jobs_status ON generation_jobs(status);
CREATE INDEX IF NOT EXISTS idx_generation_jobs_date ON generation_jobs(job_date);

-- ============================================================================
-- 19. agent_reviews — Agent 审稿记录
-- ============================================================================

CREATE TABLE IF NOT EXISTS agent_reviews (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    chapter_version_id TEXT REFERENCES chapter_versions(id) ON DELETE SET NULL,
    agent_name TEXT,
    score INTEGER,
    pass INTEGER,
    blocking_issues TEXT NOT NULL DEFAULT '[]',
    minor_issues TEXT NOT NULL DEFAULT '[]',
    recommendations TEXT NOT NULL DEFAULT '[]',
    raw_output TEXT NOT NULL DEFAULT '{}',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_agent_reviews_chapter_id ON agent_reviews(chapter_id);
CREATE INDEX IF NOT EXISTS idx_agent_reviews_agent_name ON agent_reviews(agent_name);

-- ============================================================================
-- 20. review_scores — 评分汇总
-- ============================================================================

CREATE TABLE IF NOT EXISTS review_scores (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    chapter_version_id TEXT REFERENCES chapter_versions(id) ON DELETE SET NULL,
    average_score REAL,
    final_score REAL,
    decision TEXT,
    publish_allowed INTEGER NOT NULL DEFAULT 0,
    blocking_issue_count INTEGER NOT NULL DEFAULT 0,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_review_scores_chapter_id ON review_scores(chapter_id);

-- ============================================================================
-- 21. blog_posts — 博客发布记录
-- ============================================================================

CREATE TABLE IF NOT EXISTS blog_posts (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    provider TEXT,
    external_post_id TEXT,
    title TEXT,
    slug TEXT,
    url TEXT,
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft','publish','published','failed','archived')),
    published_at TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, chapter_id)
);

CREATE INDEX IF NOT EXISTS idx_blog_posts_chapter_id ON blog_posts(chapter_id);
CREATE INDEX IF NOT EXISTS idx_blog_posts_status ON blog_posts(status);
CREATE INDEX IF NOT EXISTS idx_blog_posts_published_at ON blog_posts(published_at);

-- ============================================================================
-- 22. reader_feedback — 读者反馈
-- ============================================================================

CREATE TABLE IF NOT EXISTS reader_feedback (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_id TEXT REFERENCES chapters(id) ON DELETE SET NULL,
    source TEXT,
    external_id TEXT,
    rating REAL,
    comment_text TEXT,
    sentiment TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_reader_feedback_chapter_id ON reader_feedback(chapter_id);

-- ============================================================================
-- 23. publication_queue — 发布队列
-- ============================================================================

CREATE TABLE IF NOT EXISTS publication_queue (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    chapter_version_id TEXT REFERENCES chapter_versions(id) ON DELETE SET NULL,
    provider TEXT,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending','publishing','published','failed','cancelled','needs_human_review')),
    scheduled_at TEXT,
    published_at TEXT,
    error_message TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_publication_queue_chapter_id ON publication_queue(chapter_id);
CREATE INDEX IF NOT EXISTS idx_publication_queue_status ON publication_queue(status);

-- ============================================================================
-- 24. system_settings — 系统全局设置
-- ============================================================================

CREATE TABLE IF NOT EXISTS system_settings (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    key TEXT NOT NULL,
    value TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_system_settings_key ON system_settings(key);
CREATE INDEX IF NOT EXISTS idx_system_settings_project_id ON system_settings(project_id);

-- ============================================================================
-- 25. vector_document_metadata — 向量文档元数据
-- ============================================================================

CREATE TABLE IF NOT EXISTS vector_document_metadata (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL,
    source_id TEXT,
    title TEXT,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL DEFAULT '',
    metadata TEXT NOT NULL DEFAULT '{}',
    embedding BLOB,
    embedding_provider TEXT NOT NULL DEFAULT '',
    embedding_model TEXT NOT NULL DEFAULT '',
    embedding_kind TEXT NOT NULL DEFAULT 'document',
    embedding_dim INTEGER NOT NULL DEFAULT 0,
    indexed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_vector_docs_project_id ON vector_document_metadata(project_id);
CREATE INDEX IF NOT EXISTS idx_vector_docs_source ON vector_document_metadata(project_id, source_type, source_id);

-- ============================================================================
-- 26. advisory_locks — SQLite 模拟 advisory lock
-- ============================================================================

CREATE TABLE IF NOT EXISTS advisory_locks (
    lock_id INTEGER PRIMARY KEY,
    acquired_at TEXT NOT NULL,
    holder TEXT NOT NULL
);

-- ============================================================================
-- Seed data
-- ============================================================================

INSERT OR IGNORE INTO system_settings (id, key, value, status) VALUES
    ('seed_embedding_model', 'embedding_model', '"text-embedding-3-small"', 'active'),
    ('seed_embedding_dim', 'embedding_dimension', '1536', 'active'),
    ('seed_quality_threshold', 'default_quality_threshold', '85', 'active'),
    ('seed_auto_publish', 'default_auto_publish', 'false', 'active'),
    ('seed_max_revise_count', 'max_revise_count', '2', 'active'),
    ('seed_timezone', 'timezone', '"Asia/Shanghai"', 'active');

-- Record migration
INSERT OR IGNORE INTO schema_migrations (version, description) VALUES
    ('001_init_sqlite', 'Create all business tables, indexes, and seed data');

-- ============================================================================
-- 27. knowledge_graph_edges — interactive knowledge graph
-- ============================================================================

CREATE TABLE IF NOT EXISTS knowledge_graph_edges (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_node_id TEXT NOT NULL,
    source_node_type TEXT NOT NULL,
    target_node_id TEXT NOT NULL,
    target_node_type TEXT NOT NULL,
    edge_type TEXT NOT NULL,
    description TEXT,
    auto_inferred INTEGER NOT NULL DEFAULT 1,
    confidence REAL DEFAULT 1.0,
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_kg_edges_project ON knowledge_graph_edges(project_id);
CREATE INDEX IF NOT EXISTS idx_kg_edges_source ON knowledge_graph_edges(project_id, source_node_id, source_node_type);
CREATE INDEX IF NOT EXISTS idx_kg_edges_target ON knowledge_graph_edges(project_id, target_node_id, target_node_type);

-- ============================================================================
-- 28. pipeline_metrics — cost/latency tracking
-- ============================================================================

CREATE TABLE IF NOT EXISTS pipeline_metrics (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    chapter_id TEXT,
    job_id TEXT,
    step TEXT NOT NULL,
    provider TEXT,
    model TEXT,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    latency_ms INTEGER,
    estimated_cost_usd REAL,
    success INTEGER NOT NULL DEFAULT 1,
    error_type TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_pipeline_metrics_project ON pipeline_metrics(project_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_metrics_step ON pipeline_metrics(step);

-- ============================================================================
-- 29. learning_entries — self-learning knowledge base
-- ============================================================================

CREATE TABLE IF NOT EXISTS learning_entries (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL,          -- manual | web | self_reflection
    source_url TEXT,
    source_title TEXT,
    category TEXT NOT NULL,             -- plot_pattern | character_archetype | dialogue_style | sentence_structure | pacing_technique | description_method | narrative_device | improvement_note
    pattern_name TEXT NOT NULL,
    pattern_description TEXT NOT NULL,
    example_text TEXT,
    application_notes TEXT,
    confidence REAL DEFAULT 0.7,
    usage_count INTEGER DEFAULT 0,
    last_used_at TEXT,
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_learning_project ON learning_entries(project_id);
CREATE INDEX IF NOT EXISTS idx_learning_category ON learning_entries(project_id, category);

-- ============================================================================
-- 30. prompt_presets / prompt_preset_units — operator-editable prompt runtime
-- ============================================================================

CREATE TABLE IF NOT EXISTS prompt_presets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    scope TEXT NOT NULL DEFAULT 'project',
    is_builtin INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_prompt_presets_status ON prompt_presets(status);
CREATE INDEX IF NOT EXISTS idx_prompt_presets_scope ON prompt_presets(scope);

CREATE TABLE IF NOT EXISTS prompt_preset_units (
    id TEXT PRIMARY KEY,
    preset_id TEXT NOT NULL REFERENCES prompt_presets(id) ON DELETE CASCADE,
    identifier TEXT NOT NULL,
    role TEXT NOT NULL,
    unit_order INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    injection_position TEXT NOT NULL DEFAULT 'main',
    generation_phase TEXT NOT NULL DEFAULT 'all',
    content TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(preset_id, identifier)
);

CREATE INDEX IF NOT EXISTS idx_prompt_units_preset ON prompt_preset_units(preset_id, unit_order);
CREATE INDEX IF NOT EXISTS idx_prompt_units_phase ON prompt_preset_units(preset_id, generation_phase);

-- ============================================================================
-- 31. context_rules — deterministic canon activation rules
-- ============================================================================

CREATE TABLE IF NOT EXISTS context_rules (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    primary_keywords TEXT NOT NULL DEFAULT '[]',
    secondary_keywords TEXT NOT NULL DEFAULT '[]',
    entity_refs TEXT NOT NULL DEFAULT '[]',
    chapter_ranges TEXT NOT NULL DEFAULT '[]',
    priority INTEGER NOT NULL DEFAULT 0,
    token_budget INTEGER NOT NULL DEFAULT 160,
    sticky_chapters INTEGER NOT NULL DEFAULT 0,
    cooldown_chapters INTEGER NOT NULL DEFAULT 0,
    content TEXT NOT NULL,
    source_type TEXT NOT NULL DEFAULT 'context_rule',
    source_id TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_context_rules_project ON context_rules(project_id, enabled);
CREATE INDEX IF NOT EXISTS idx_context_rules_priority ON context_rules(project_id, priority);

-- ============================================================================
-- 32. model_profiles — provider capability and pricing profiles
-- ============================================================================

CREATE TABLE IF NOT EXISTS model_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    base_url TEXT NOT NULL,
    model TEXT NOT NULL,
    context_window INTEGER NOT NULL DEFAULT 8192,
    supports_json INTEGER NOT NULL DEFAULT 0,
    supports_streaming INTEGER NOT NULL DEFAULT 0,
    supports_embeddings INTEGER NOT NULL DEFAULT 0,
    input_cost_per_million REAL,
    output_cost_per_million REAL,
    intended_use TEXT NOT NULL DEFAULT 'draft',
    status TEXT NOT NULL DEFAULT 'active',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_model_profiles_provider ON model_profiles(provider, model);
CREATE INDEX IF NOT EXISTS idx_model_profiles_use ON model_profiles(intended_use, status);

-- ============================================================================
-- 33. draft_alternatives — candidate draft exploration
-- ============================================================================

CREATE TABLE IF NOT EXISTS draft_alternatives (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_plan_id TEXT NOT NULL REFERENCES chapter_plans(id) ON DELETE CASCADE,
    candidate_number INTEGER NOT NULL,
    title TEXT NOT NULL,
    body_markdown TEXT NOT NULL,
    summary TEXT,
    word_count INTEGER NOT NULL DEFAULT 0,
    prompt_hash TEXT NOT NULL,
    context_hash TEXT NOT NULL,
    model_profile_id TEXT,
    review_notes TEXT NOT NULL DEFAULT '{}',
    estimated_cost_usd REAL,
    status TEXT NOT NULL DEFAULT 'candidate'
        CHECK (status IN ('candidate','selected','rejected','archived')),
    selection_reason TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, chapter_plan_id, candidate_number)
);

CREATE INDEX IF NOT EXISTS idx_draft_alternatives_plan ON draft_alternatives(project_id, chapter_plan_id, candidate_number);
CREATE INDEX IF NOT EXISTS idx_draft_alternatives_status ON draft_alternatives(project_id, status);

-- ============================================================================
-- 34. extension_packages — declarative extension host registry
-- ============================================================================

CREATE TABLE IF NOT EXISTS extension_packages (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT,
    manifest TEXT NOT NULL,
    contributions TEXT NOT NULL DEFAULT '[]',
    enabled INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'installed',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_extension_packages_enabled ON extension_packages(enabled, status);
