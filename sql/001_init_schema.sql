-- ============================================================================
-- 001_init_schema.sql
-- AI Novel Factory — 完整数据库 Schema
--
-- 执行方式：Neon Direct Connection
--   psql "NEON_DATABASE_URL_DIRECT" -f 001_init_schema.sql
--
-- 约束：
--   - 可重复执行（幂等），所有 CREATE 使用 IF NOT EXISTS。
--   - 日常 n8n 读写请使用 Neon Pooled Connection。
--   - 不要使用 session-level advisory lock。
--   - 所有 Neon 连接必须 sslmode=require。
--   - 幂等控制通过 unique 约束 + ON CONFLICT DO NOTHING 实现。
--   - 默认向量维度 1536（OpenAI text-embedding-3-small）。
--   - 使用 gen_random_uuid()，不依赖 uuid-ossp。
-- ============================================================================

-- 记录 migration
INSERT INTO schema_migrations (version, description)
VALUES ('001_init_schema', 'Create all business tables, indexes, triggers, and seed data')
ON CONFLICT (version) DO NOTHING;

-- ============================================================================
-- 扩展（如 000_neon_setup.sql 未执行，此处幂等补建）
-- ============================================================================

CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ============================================================================
-- 通用触发器函数：自动更新 updated_at
-- ============================================================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- 1. schema_migrations — migration 版本记录（全局表）
-- ============================================================================

CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    description TEXT,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE schema_migrations IS '数据库 migration 版本记录。';

-- ============================================================================
-- 2. projects — 小说项目主表
-- ============================================================================

CREATE TABLE IF NOT EXISTS projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    genre TEXT,
    target_audience TEXT,
    style_profile JSONB NOT NULL DEFAULT '{}'::jsonb,
    total_target_words INTEGER,
    daily_target_words INTEGER,
    auto_publish BOOLEAN NOT NULL DEFAULT false,
    quality_threshold INTEGER NOT NULL DEFAULT 85,
    blog_provider TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE projects IS '小说项目主表。一个 project = 一部小说。';
COMMENT ON COLUMN projects.name IS '项目名称 / 书名。';
COMMENT ON COLUMN projects.style_profile IS '文风配置 JSON，包括叙事视角、时态、语调、禁忌短语等。';
COMMENT ON COLUMN projects.auto_publish IS '是否允许自动发布。false 时只发布为 draft。';
COMMENT ON COLUMN projects.quality_threshold IS '最低质量分阈值。低于此值不发布。';
COMMENT ON COLUMN projects.blog_provider IS '发布平台标识，如 wordpress、custom_api。';

CREATE TRIGGER trg_projects_updated_at
    BEFORE UPDATE ON projects
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 3. volumes — 卷
-- ============================================================================

CREATE TABLE IF NOT EXISTS volumes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL,
    title TEXT NOT NULL,
    summary TEXT,
    target_word_count INTEGER,
    status TEXT NOT NULL DEFAULT 'planned',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE volumes IS '小说分卷。';

CREATE TRIGGER trg_volumes_updated_at
    BEFORE UPDATE ON volumes
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 4. chapter_plans — 章节计划
-- ============================================================================

CREATE TABLE IF NOT EXISTS chapter_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    volume_id UUID REFERENCES volumes(id) ON DELETE SET NULL,
    sequence INTEGER NOT NULL,
    title TEXT,
    outline TEXT,
    pov_character_id UUID,
    target_word_count INTEGER,
    required_characters UUID[] NOT NULL DEFAULT '{}',
    required_locations UUID[] NOT NULL DEFAULT '{}',
    plot_goals JSONB NOT NULL DEFAULT '[]'::jsonb,
    required_foreshadowing JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL DEFAULT 'planned'
        CHECK (status IN ('planned','in_progress','completed','skipped','archived')),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE chapter_plans IS '章节规划，每周由 Weekly Arc Planner 更新。';
COMMENT ON COLUMN chapter_plans.pov_character_id IS '本章主视角角色。';
COMMENT ON COLUMN chapter_plans.required_characters IS '本章必须出场的角色 UUID 数组。';
COMMENT ON COLUMN chapter_plans.required_locations IS '本章涉及的地点 UUID 数组。';
COMMENT ON COLUMN chapter_plans.plot_goals IS '本章剧情目标 JSON 数组。';
COMMENT ON COLUMN chapter_plans.required_foreshadowing IS '本章需要埋入/推进/回收的伏笔。';

CREATE TRIGGER trg_chapter_plans_updated_at
    BEFORE UPDATE ON chapter_plans
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- FK: pov_character_id → characters（characters 尚未创建，后续 ALTER）
-- 见文件末尾 "延迟外键" 区块。

-- ============================================================================
-- 5. chapters — 章节主表
-- ============================================================================

CREATE TABLE IF NOT EXISTS chapters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_plan_id UUID REFERENCES chapter_plans(id) ON DELETE SET NULL,
    sequence INTEGER NOT NULL,
    title TEXT,
    final_version_id UUID,
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft','reviewing','revised','final','published','needs_human_review','failed')),
    word_count INTEGER,
    summary TEXT,
    published_at TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE chapters IS '章节主表，记录每章的最新状态。';
COMMENT ON COLUMN chapters.final_version_id IS '指向 chapter_versions.id。FK 在 chapter_versions 创建后通过 ALTER TABLE 添加。';
COMMENT ON COLUMN chapters.status IS 'draft=初稿 reviewing=审稿中 revised=修订后 final=定稿 published=已发布 needs_human_review=人工审核 failed=失败。';

CREATE TRIGGER trg_chapters_updated_at
    BEFORE UPDATE ON chapters
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 6. chapter_versions — 章节版本历史（chapters 1:N）
-- ============================================================================

CREATE TABLE IF NOT EXISTS chapter_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chapter_id UUID NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    version_type TEXT NOT NULL DEFAULT 'draft'
        CHECK (version_type IN ('draft','revised','final')),
    title TEXT,
    body_markdown TEXT,
    summary TEXT,
    word_count INTEGER,
    model_provider TEXT,
    model_name TEXT,
    prompt_hash TEXT,
    context_hash TEXT,
    created_by_agent TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE chapter_versions IS '章节版本历史。chapters : chapter_versions = 1 : N。';
COMMENT ON COLUMN chapter_versions.version_type IS 'draft=初稿 revised=修订稿 final=定稿。';
COMMENT ON COLUMN chapter_versions.prompt_hash IS 'writing_brief 的 hash，用于追踪同一 brief 的多次生成。';
COMMENT ON COLUMN chapter_versions.context_hash IS '检索上下文（向量结果）的 hash。';
COMMENT ON COLUMN chapter_versions.created_by_agent IS '创建者标识，如 draft_writer / revision_writer。';

CREATE TRIGGER trg_chapter_versions_updated_at
    BEFORE UPDATE ON chapter_versions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 延迟外键：chapters.final_version_id → chapter_versions.id
-- ============================================================================

ALTER TABLE chapters
    ADD CONSTRAINT fk_chapters_final_version
    FOREIGN KEY (final_version_id)
    REFERENCES chapter_versions(id)
    ON DELETE SET NULL;

COMMENT ON CONSTRAINT fk_chapters_final_version ON chapters
    IS 'chapters.final_version_id → chapter_versions.id。';

-- ============================================================================
-- 7. characters — 人物表
-- ============================================================================

CREATE TABLE IF NOT EXISTS characters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    aliases TEXT[] NOT NULL DEFAULT '{}',
    role TEXT,
    personality TEXT,
    motivation TEXT,
    speech_style TEXT,
    appearance TEXT,
    backstory TEXT,
    relationship_map JSONB NOT NULL DEFAULT '{}'::jsonb,
    locked_fields TEXT[] NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE characters IS '人物设定表。';
COMMENT ON COLUMN characters.relationship_map IS '人物关系图 JSON，key=角色名，value=关系描述。';
COMMENT ON COLUMN characters.locked_fields IS '锁定的字段名数组。AI 不得覆盖这些字段。如 {personality,motivation,backstory}。';

CREATE TRIGGER trg_characters_updated_at
    BEFORE UPDATE ON characters
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 8. character_states — 人物状态快照（characters 1:N）
-- ============================================================================

CREATE TABLE IF NOT EXISTS character_states (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    character_id UUID NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    after_chapter_id UUID REFERENCES chapters(id) ON DELETE SET NULL,
    physical_state TEXT,
    emotional_state TEXT,
    knowledge_state TEXT,
    relationship_state JSONB NOT NULL DEFAULT '{}'::jsonb,
    location_id UUID,
    inventory JSONB NOT NULL DEFAULT '[]'::jsonb,
    open_conflicts JSONB NOT NULL DEFAULT '[]'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE character_states IS '角色在某章结束后的状态快照。characters : character_states = 1 : N。';
COMMENT ON COLUMN character_states.open_conflicts IS '该角色当前未解决的冲突 JSON。';

CREATE TRIGGER trg_character_states_updated_at
    BEFORE UPDATE ON character_states
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- character_states.location_id FK 在 locations 创建后补充。

-- ============================================================================
-- 9. locations — 地点
-- ============================================================================

CREATE TABLE IF NOT EXISTS locations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    type TEXT,
    description TEXT,
    rules TEXT,
    connected_locations JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE locations IS '地点设定表。';
COMMENT ON COLUMN locations.rules IS '地点特殊规则。';
COMMENT ON COLUMN locations.connected_locations IS '关联地点 JSON 数组。';

CREATE TRIGGER trg_locations_updated_at
    BEFORE UPDATE ON locations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- character_states 的 location_id FK（locations 已存在，现在补充）
ALTER TABLE character_states
    ADD CONSTRAINT fk_character_states_location
    FOREIGN KEY (location_id)
    REFERENCES locations(id)
    ON DELETE SET NULL;

-- ============================================================================
-- 10. organizations — 组织
-- ============================================================================

CREATE TABLE IF NOT EXISTS organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    hierarchy JSONB NOT NULL DEFAULT '{}'::jsonb,
    goals TEXT,
    relationship_map JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE organizations IS '组织/势力表。';
COMMENT ON COLUMN organizations.hierarchy IS '组织层级结构 JSON。';
COMMENT ON COLUMN organizations.relationship_map IS '与其他组织的关系 JSON。';

CREATE TRIGGER trg_organizations_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 11. items — 道具/物品
-- ============================================================================

CREATE TABLE IF NOT EXISTS items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    item_type TEXT,
    owner_character_id UUID REFERENCES characters(id) ON DELETE SET NULL,
    location_id UUID REFERENCES locations(id) ON DELETE SET NULL,
    description TEXT,
    abilities TEXT,
    limitations TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE items IS '道具/物品表。';
COMMENT ON COLUMN items.owner_character_id IS '当前持有角色。';
COMMENT ON COLUMN items.location_id IS '当前所在地点。';

CREATE TRIGGER trg_items_updated_at
    BEFORE UPDATE ON items
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 12. world_lore — 世界观设定
-- ============================================================================

CREATE TABLE IF NOT EXISTS world_lore (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    lore_type TEXT,
    title TEXT,
    content TEXT,
    locked BOOLEAN NOT NULL DEFAULT false,
    status TEXT NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE world_lore IS '世界观设定，包括历史、地理、文化、社会规则等。';
COMMENT ON COLUMN world_lore.locked IS '锁定后 AI 不得自动修改。';

CREATE TRIGGER trg_world_lore_updated_at
    BEFORE UPDATE ON world_lore
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 13. magic_or_power_systems — 力量体系
-- ============================================================================

CREATE TABLE IF NOT EXISTS magic_or_power_systems (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT,
    description TEXT,
    rules TEXT,
    limitations TEXT,
    progression JSONB NOT NULL DEFAULT '{}'::jsonb,
    locked BOOLEAN NOT NULL DEFAULT false,
    status TEXT NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE magic_or_power_systems IS '力量/魔法/超能力体系设定。';
COMMENT ON COLUMN magic_or_power_systems.progression IS '进阶体系 JSON。';

CREATE TRIGGER trg_magic_or_power_systems_updated_at
    BEFORE UPDATE ON magic_or_power_systems
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 14. timeline_events — 时间线事件
-- ============================================================================

CREATE TABLE IF NOT EXISTS timeline_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_id UUID REFERENCES chapters(id) ON DELETE SET NULL,
    event_time_label TEXT,
    sequence INTEGER,
    event_summary TEXT,
    involved_characters UUID[] NOT NULL DEFAULT '{}',
    involved_locations UUID[] NOT NULL DEFAULT '{}',
    consequences JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE timeline_events IS '小说时间线，记录所有重要事件及发生顺序。';
COMMENT ON COLUMN timeline_events.event_time_label IS '时间标签，如 "Day 3 上午" 或 "第一卷 第五章"。';
COMMENT ON COLUMN timeline_events.sequence IS '排序用，允许小数插入。';

CREATE TRIGGER trg_timeline_events_updated_at
    BEFORE UPDATE ON timeline_events
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 15. plot_threads — 剧情线
-- ============================================================================

CREATE TABLE IF NOT EXISTS plot_threads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT,
    description TEXT,
    priority INTEGER NOT NULL DEFAULT 3,
    arc_status TEXT NOT NULL DEFAULT 'open'
        CHECK (arc_status IN ('open','active','paused','resolved','abandoned')),
    introduced_chapter_id UUID REFERENCES chapters(id) ON DELETE SET NULL,
    expected_resolution_chapter_id UUID REFERENCES chapters(id) ON DELETE SET NULL,
    resolved_chapter_id UUID REFERENCES chapters(id) ON DELETE SET NULL,
    related_characters UUID[] NOT NULL DEFAULT '{}',
    related_chapters UUID[] NOT NULL DEFAULT '{}',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE plot_threads IS '剧情线索/主线支线管理。';
COMMENT ON COLUMN plot_threads.priority IS '1-5，1 最高。';
COMMENT ON COLUMN plot_threads.arc_status IS 'open=待开启 active=进行中 paused=暂停 resolved=已解决 abandoned=废弃。';

CREATE TRIGGER trg_plot_threads_updated_at
    BEFORE UPDATE ON plot_threads
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 16. foreshadowing — 伏笔管理
-- ============================================================================

CREATE TABLE IF NOT EXISTS foreshadowing (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    clue_text TEXT,
    intended_payoff TEXT,
    introduced_chapter_id UUID REFERENCES chapters(id) ON DELETE SET NULL,
    expected_resolution_chapter_id UUID REFERENCES chapters(id) ON DELETE SET NULL,
    resolved_chapter_id UUID REFERENCES chapters(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'open',
    importance INTEGER NOT NULL DEFAULT 3,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE foreshadowing IS '伏笔管理。追踪每个伏笔从埋入到回收的完整生命周期。';
COMMENT ON COLUMN foreshadowing.status IS 'open=未回收 resolved=已回收 abandoned=废弃。';
COMMENT ON COLUMN foreshadowing.importance IS '1-5，1 最高重要度。';

CREATE TRIGGER trg_foreshadowing_updated_at
    BEFORE UPDATE ON foreshadowing
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 17. canon_rules — 圣经锁定规则
-- ============================================================================

CREATE TABLE IF NOT EXISTS canon_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    rule_type TEXT,
    rule_text TEXT,
    severity TEXT NOT NULL DEFAULT 'hard'
        CHECK (severity IN ('hard','soft')),
    locked BOOLEAN NOT NULL DEFAULT true,
    status TEXT NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE canon_rules IS '圣经锁定规则。hard=不可违背 soft=尽量避免。';
COMMENT ON COLUMN canon_rules.severity IS 'hard=硬规则不可违背 soft=软规则尽量遵守。';
COMMENT ON COLUMN canon_rules.locked IS 'true=AI 不得修改此规则。';

CREATE TRIGGER trg_canon_rules_updated_at
    BEFORE UPDATE ON canon_rules
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 18. style_guides — 风格指南
-- ============================================================================

CREATE TABLE IF NOT EXISTS style_guides (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT,
    style_text TEXT,
    positive_examples TEXT[] NOT NULL DEFAULT '{}',
    negative_examples TEXT[] NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE style_guides IS '风格指南。';
COMMENT ON COLUMN style_guides.positive_examples IS '正面范例文本。';
COMMENT ON COLUMN style_guides.negative_examples IS '反面范例文本。';

CREATE TRIGGER trg_style_guides_updated_at
    BEFORE UPDATE ON style_guides
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 19. generation_jobs — 生成任务（幂等锁）
-- ============================================================================

CREATE TABLE IF NOT EXISTS generation_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_plan_id UUID NOT NULL REFERENCES chapter_plans(id) ON DELETE CASCADE,
    job_date DATE NOT NULL DEFAULT CURRENT_DATE,
    status TEXT NOT NULL DEFAULT 'started'
        CHECK (status IN ('started','draft_created','reviewing','revising','publishing','completed','failed','needs_human_review','skipped')),
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(project_id, chapter_plan_id, job_date)
);

COMMENT ON TABLE generation_jobs IS '每日章节生成任务。幂等锁通过 project_id + chapter_plan_id + job_date 唯一约束实现。';
COMMENT ON COLUMN generation_jobs.job_date IS '生成日期。每日同一 plan 只能有一条记录。';
COMMENT ON COLUMN generation_jobs.status IS 'started=已获取锁 draft_created=初稿完成 reviewing=审稿中 revising=修订中 publishing=发布中 completed=完成 failed=失败 needs_human_review=需人工审核 skipped=跳过。';

CREATE TRIGGER trg_generation_jobs_updated_at
    BEFORE UPDATE ON generation_jobs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 20. agent_reviews — Agent 审稿记录
-- ============================================================================

CREATE TABLE IF NOT EXISTS agent_reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_id UUID NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    chapter_version_id UUID REFERENCES chapter_versions(id) ON DELETE SET NULL,
    agent_name TEXT,
    score INTEGER,
    pass BOOLEAN,
    blocking_issues JSONB NOT NULL DEFAULT '[]'::jsonb,
    minor_issues JSONB NOT NULL DEFAULT '[]'::jsonb,
    recommendations JSONB NOT NULL DEFAULT '[]'::jsonb,
    raw_output JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE agent_reviews IS '审稿 Agent 完整输出记录。';
COMMENT ON COLUMN agent_reviews.agent_name IS 'Agent 标识：continuity_reviewer, character_reviewer, plot_logic_reviewer, pacing_reviewer, style_reviewer, safety_reviewer, publication_reviewer, review_arbiter。';
COMMENT ON COLUMN agent_reviews.blocking_issues IS '阻塞性问题 JSON 数组。';
COMMENT ON COLUMN agent_reviews.raw_output IS 'Agent 原始输出 JSON。';

CREATE TRIGGER trg_agent_reviews_updated_at
    BEFORE UPDATE ON agent_reviews
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 21. review_scores — 评分汇总
-- ============================================================================

CREATE TABLE IF NOT EXISTS review_scores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_id UUID NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    chapter_version_id UUID REFERENCES chapter_versions(id) ON DELETE SET NULL,
    average_score NUMERIC,
    final_score NUMERIC,
    decision TEXT,
    publish_allowed BOOLEAN NOT NULL DEFAULT false,
    blocking_issue_count INTEGER NOT NULL DEFAULT 0,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE review_scores IS '审稿评分汇总。冗余存储以加速查询和决策。';
COMMENT ON COLUMN review_scores.decision IS 'publish_ready / revise / needs_human_review / stop。';
COMMENT ON COLUMN review_scores.publish_allowed IS 'review_arbiter 最终是否允许发布。';

CREATE TRIGGER trg_review_scores_updated_at
    BEFORE UPDATE ON review_scores
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 22. blog_posts — 博客发布记录
-- ============================================================================

CREATE TABLE IF NOT EXISTS blog_posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_id UUID NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    provider TEXT,
    external_post_id TEXT,
    title TEXT,
    slug TEXT,
    url TEXT,
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft','publish','published','failed','archived')),
    published_at TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(project_id, chapter_id)
);

COMMENT ON TABLE blog_posts IS '博客发布记录。防重复发布通过 project_id + chapter_id 唯一约束实现。';
COMMENT ON COLUMN blog_posts.provider IS '发布平台标识。';
COMMENT ON COLUMN blog_posts.external_post_id IS '外部博客平台的文章 ID。';
COMMENT ON COLUMN blog_posts.status IS 'draft=草稿 publish=待发布 published=已发布 failed=发布失败 archived=已归档。';

CREATE TRIGGER trg_blog_posts_updated_at
    BEFORE UPDATE ON blog_posts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 23. reader_feedback — 读者反馈
-- ============================================================================

CREATE TABLE IF NOT EXISTS reader_feedback (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_id UUID REFERENCES chapters(id) ON DELETE SET NULL,
    source TEXT,
    external_id TEXT,
    rating NUMERIC,
    comment_text TEXT,
    sentiment TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE reader_feedback IS '读者反馈汇总，用于调整写作方向和节奏。';
COMMENT ON COLUMN reader_feedback.source IS '来源：wordpress_comment / manual / social / other。';
COMMENT ON COLUMN reader_feedback.sentiment IS '情感分析结果：positive / neutral / negative。';

CREATE TRIGGER trg_reader_feedback_updated_at
    BEFORE UPDATE ON reader_feedback
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 24. publication_queue — 发布队列
-- ============================================================================

CREATE TABLE IF NOT EXISTS publication_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    chapter_id UUID NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    chapter_version_id UUID REFERENCES chapter_versions(id) ON DELETE SET NULL,
    provider TEXT,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending','publishing','published','failed','cancelled','needs_human_review')),
    scheduled_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    error_message TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE publication_queue IS '发布队列。确保每章发布可追踪、可重试。';
COMMENT ON COLUMN publication_queue.status IS 'pending=待发布 publishing=发布中 published=已发布 failed=失败 cancelled=取消 needs_human_review=需人工审核。';

CREATE TRIGGER trg_publication_queue_updated_at
    BEFORE UPDATE ON publication_queue
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 25. system_settings — 系统全局设置
-- ============================================================================

CREATE TABLE IF NOT EXISTS system_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID,
    key TEXT NOT NULL,
    value JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE system_settings IS '系统全局/项目级配置。project_id 为 NULL 表示全局设置。';
COMMENT ON COLUMN system_settings.key IS '配置键，如 embedding_model、default_quality_threshold。';
COMMENT ON COLUMN system_settings.value IS '配置值 JSON。';

CREATE TRIGGER trg_system_settings_updated_at
    BEFORE UPDATE ON system_settings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 26. vector_documents — 向量化文档（pgvector）
--
-- 向量维度默认 1536（OpenAI text-embedding-3-small）。
-- 如果更换 embedding 模型，需：
--   1. ALTER TABLE vector_documents ALTER COLUMN embedding TYPE vector(NEW_DIM);
--   2. DROP INDEX idx_vector_documents_embedding_hnsw;
--   3. 重新 CREATE INDEX;
--   4. 重新生成所有 embedding。
-- ============================================================================

CREATE TABLE IF NOT EXISTS vector_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL,
    source_id UUID,
    title TEXT,
    content TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    embedding vector(1536),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE vector_documents IS '向量化文档。用于写作前 RAG 检索上下文。';
COMMENT ON COLUMN vector_documents.source_type IS '来源类型：chapter_summary / character_profile / location / organization / item / world_lore / power_system / timeline_event / plot_thread / foreshadowing / canon_rule / style_guide / custom。';
COMMENT ON COLUMN vector_documents.source_id IS '多态引用。结合 source_type 在业务层关联来源对象。不给 source_id 添加单一 FK。';
COMMENT ON COLUMN vector_documents.embedding IS '向量维度 1536（text-embedding-3-small）。更换模型时需调整。';

CREATE TRIGGER trg_vector_documents_updated_at
    BEFORE UPDATE ON vector_documents
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- 延迟外键：chapter_plans.pov_character_id → characters.id
-- ============================================================================

ALTER TABLE chapter_plans
    ADD CONSTRAINT fk_chapter_plans_pov_character
    FOREIGN KEY (pov_character_id)
    REFERENCES characters(id)
    ON DELETE SET NULL;

COMMENT ON CONSTRAINT fk_chapter_plans_pov_character ON chapter_plans
    IS 'chapter_plans.pov_character_id → characters.id。';

-- ============================================================================
-- 索引：业务查询
-- ============================================================================

-- projects
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);

-- volumes
CREATE INDEX IF NOT EXISTS idx_volumes_project_id ON volumes(project_id);

-- chapter_plans
CREATE INDEX IF NOT EXISTS idx_chapter_plans_project_id ON chapter_plans(project_id);
CREATE INDEX IF NOT EXISTS idx_chapter_plans_sequence ON chapter_plans(project_id, sequence);
CREATE INDEX IF NOT EXISTS idx_chapter_plans_status ON chapter_plans(status);

-- chapters
CREATE INDEX IF NOT EXISTS idx_chapters_project_id ON chapters(project_id);
CREATE INDEX IF NOT EXISTS idx_chapters_sequence ON chapters(project_id, sequence);
CREATE INDEX IF NOT EXISTS idx_chapters_status ON chapters(project_id, status);
CREATE INDEX IF NOT EXISTS idx_chapters_published_at ON chapters(published_at);

-- chapter_versions
CREATE INDEX IF NOT EXISTS idx_chapter_versions_chapter_id ON chapter_versions(chapter_id);
CREATE INDEX IF NOT EXISTS idx_chapter_versions_number ON chapter_versions(chapter_id, version_number);

-- characters
CREATE INDEX IF NOT EXISTS idx_characters_project_id ON characters(project_id);
CREATE INDEX IF NOT EXISTS idx_characters_name ON characters(project_id, lower(name));
CREATE INDEX IF NOT EXISTS idx_characters_status ON characters(status);

-- character_states
CREATE INDEX IF NOT EXISTS idx_character_states_character_id ON character_states(character_id);
CREATE INDEX IF NOT EXISTS idx_character_states_chapter_id ON character_states(after_chapter_id);

-- locations
CREATE INDEX IF NOT EXISTS idx_locations_project_id ON locations(project_id);
CREATE INDEX IF NOT EXISTS idx_locations_name ON locations(project_id, lower(name));

-- organizations
CREATE INDEX IF NOT EXISTS idx_organizations_project_id ON organizations(project_id);

-- items
CREATE INDEX IF NOT EXISTS idx_items_project_id ON items(project_id);
CREATE INDEX IF NOT EXISTS idx_items_name ON items(project_id, lower(name));

-- world_lore
CREATE INDEX IF NOT EXISTS idx_world_lore_project_id ON world_lore(project_id);

-- magic_or_power_systems
CREATE INDEX IF NOT EXISTS idx_magic_or_power_systems_project_id ON magic_or_power_systems(project_id);

-- timeline_events
CREATE INDEX IF NOT EXISTS idx_timeline_events_project_id ON timeline_events(project_id);
CREATE INDEX IF NOT EXISTS idx_timeline_events_chapter_id ON timeline_events(chapter_id);

-- plot_threads
CREATE INDEX IF NOT EXISTS idx_plot_threads_project_id ON plot_threads(project_id);
CREATE INDEX IF NOT EXISTS idx_plot_threads_arc_status ON plot_threads(arc_status);

-- foreshadowing
CREATE INDEX IF NOT EXISTS idx_foreshadowing_project_id ON foreshadowing(project_id);
CREATE INDEX IF NOT EXISTS idx_foreshadowing_status ON foreshadowing(status);
CREATE INDEX IF NOT EXISTS idx_foreshadowing_introduced ON foreshadowing(introduced_chapter_id);
CREATE INDEX IF NOT EXISTS idx_foreshadowing_resolved ON foreshadowing(resolved_chapter_id);

-- canon_rules
CREATE INDEX IF NOT EXISTS idx_canon_rules_project_id ON canon_rules(project_id);
CREATE INDEX IF NOT EXISTS idx_canon_rules_severity ON canon_rules(severity);

-- style_guides
CREATE INDEX IF NOT EXISTS idx_style_guides_project_id ON style_guides(project_id);

-- generation_jobs
CREATE INDEX IF NOT EXISTS idx_generation_jobs_project_id ON generation_jobs(project_id);
CREATE INDEX IF NOT EXISTS idx_generation_jobs_status ON generation_jobs(status);
CREATE INDEX IF NOT EXISTS idx_generation_jobs_date ON generation_jobs(job_date);
-- 幂等唯一索引 UNIQUE(project_id, chapter_plan_id, job_date) 已在表定义中包含

-- agent_reviews
CREATE INDEX IF NOT EXISTS idx_agent_reviews_chapter_id ON agent_reviews(chapter_id);
CREATE INDEX IF NOT EXISTS idx_agent_reviews_agent_name ON agent_reviews(agent_name);

-- review_scores
CREATE INDEX IF NOT EXISTS idx_review_scores_chapter_id ON review_scores(chapter_id);

-- blog_posts
CREATE INDEX IF NOT EXISTS idx_blog_posts_chapter_id ON blog_posts(chapter_id);
CREATE INDEX IF NOT EXISTS idx_blog_posts_provider_ext ON blog_posts(provider, external_post_id);
CREATE INDEX IF NOT EXISTS idx_blog_posts_status ON blog_posts(status);
CREATE INDEX IF NOT EXISTS idx_blog_posts_published_at ON blog_posts(published_at);
-- 防重复唯一索引 UNIQUE(project_id, chapter_id) 已在表定义中包含

-- reader_feedback
CREATE INDEX IF NOT EXISTS idx_reader_feedback_chapter_id ON reader_feedback(chapter_id);

-- publication_queue
CREATE INDEX IF NOT EXISTS idx_publication_queue_chapter_id ON publication_queue(chapter_id);
CREATE INDEX IF NOT EXISTS idx_publication_queue_status ON publication_queue(status);

-- system_settings
CREATE INDEX IF NOT EXISTS idx_system_settings_key ON system_settings(key);
CREATE INDEX IF NOT EXISTS idx_system_settings_project_id ON system_settings(project_id);

-- vector_documents
CREATE INDEX IF NOT EXISTS idx_vector_documents_project_id ON vector_documents(project_id);
CREATE INDEX IF NOT EXISTS idx_vector_documents_source ON vector_documents(project_id, source_type, source_id);

-- ============================================================================
-- JSONB GIN 索引
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_projects_metadata_gin ON projects USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_chapter_plans_plot_goals_gin ON chapter_plans USING GIN (plot_goals);
CREATE INDEX IF NOT EXISTS idx_chapters_metadata_gin ON chapters USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_chapter_versions_metadata_gin ON chapter_versions USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_characters_relationship_map_gin ON characters USING GIN (relationship_map);
CREATE INDEX IF NOT EXISTS idx_characters_metadata_gin ON characters USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_character_states_relationship_state_gin ON character_states USING GIN (relationship_state);
CREATE INDEX IF NOT EXISTS idx_locations_metadata_gin ON locations USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_organizations_hierarchy_gin ON organizations USING GIN (hierarchy);
CREATE INDEX IF NOT EXISTS idx_items_metadata_gin ON items USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_world_lore_metadata_gin ON world_lore USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_magic_or_power_systems_metadata_gin ON magic_or_power_systems USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_timeline_events_metadata_gin ON timeline_events USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_plot_threads_metadata_gin ON plot_threads USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_foreshadowing_metadata_gin ON foreshadowing USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_canon_rules_metadata_gin ON canon_rules USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_style_guides_metadata_gin ON style_guides USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_generation_jobs_metadata_gin ON generation_jobs USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_agent_reviews_blocking_gin ON agent_reviews USING GIN (blocking_issues);
CREATE INDEX IF NOT EXISTS idx_agent_reviews_minor_gin ON agent_reviews USING GIN (minor_issues);
CREATE INDEX IF NOT EXISTS idx_agent_reviews_recommendations_gin ON agent_reviews USING GIN (recommendations);
CREATE INDEX IF NOT EXISTS idx_agent_reviews_raw_output_gin ON agent_reviews USING GIN (raw_output);
CREATE INDEX IF NOT EXISTS idx_review_scores_metadata_gin ON review_scores USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_blog_posts_metadata_gin ON blog_posts USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_publication_queue_metadata_gin ON publication_queue USING GIN (metadata);
CREATE INDEX IF NOT EXISTS idx_system_settings_value_gin ON system_settings USING GIN (value);
CREATE INDEX IF NOT EXISTS idx_vector_documents_metadata_gin ON vector_documents USING GIN (metadata);

-- ============================================================================
-- pgvector HNSW 索引
--
-- HNSW vs IVFFlat：
--   HNSW 优势：不需要先插入数据再建索引，构建更快，查询更快。
--   如果 HNSW 不可用（pgvector < 0.5.0），使用下面的 IVFFlat 替代。
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_vector_documents_embedding_hnsw
ON vector_documents
USING hnsw (embedding vector_cosine_ops);

-- IVFFlat 备选方案（如果 HNSW 不支持）：
--   1. 先插入足够数据（建议 > 1000 行）。
--   2. CREATE INDEX IF NOT EXISTS idx_vector_documents_embedding_ivfflat
--      ON vector_documents
--      USING ivfflat (embedding vector_cosine_ops)
--      WITH (lists = 100);
--   3. ANALYZE vector_documents;

-- ============================================================================
-- 示例向量检索 SQL（供 n8n Code / Postgres 节点参考）
-- ============================================================================

-- 示例：按相似度检索与当前章节最相关的设定
--
-- SELECT
--   id,
--   title,
--   content,
--   metadata,
--   1 - (embedding <=> $1::vector) AS similarity
-- FROM vector_documents
-- WHERE project_id = $2
-- ORDER BY embedding <=> $1::vector
-- LIMIT 12;
--
-- 注意：
--   - 所有向量检索必须包含 ORDER BY embedding <=> $query::vector
--   - 所有向量检索必须包含 LIMIT
--   - 所有向量检索必须限定 project_id，避免不同小说项目上下文污染

-- ============================================================================
-- Seed 数据
-- ============================================================================

-- 1. 插入示例 project
INSERT INTO projects (name, genre, target_audience, style_profile, total_target_words, daily_target_words, blog_provider, status)
VALUES (
    '示例小说',
    'fantasy',
    '18-35岁网络小说读者',
    '{"narrative_perspective":"第三人称","tense":"过去时","tone":"热血","dialogue_style":"符合人物性格","forbidden_phrases":["眼中闪过","嘴角上扬","不由得"],"preferred_techniques":["动作链","对话推进","感官细节"]}'::jsonb,
    500000,
    3000,
    'wordpress',
    'active'
)
ON CONFLICT DO NOTHING;

-- 2. 插入 style_guide（引用示例 project）
WITH inserted_project AS (
    SELECT id FROM projects WHERE name = '示例小说' LIMIT 1
)
INSERT INTO style_guides (project_id, name, style_text, positive_examples, negative_examples, status)
SELECT
    ip.id,
    '默认风格指南',
    '第三人称限制视角。过去时。信息密度高，不水文，不空话。对话有性格，冲突推进快。每章结尾有自然钩子。',
    ARRAY['他握紧刀柄，没有回头。身后传来脚步声，越来越近。'],
    ARRAY['他的眼中闪过一丝复杂的神色，嘴角微微上扬，心中不由得暗道一声不好。'],
    'active'
FROM inserted_project ip
WHERE NOT EXISTS (
    SELECT 1 FROM style_guides WHERE project_id = ip.id AND name = '默认风格指南'
);

-- 3. 插入 system_setting（全局，不绑定 project）
INSERT INTO system_settings (key, value, status)
VALUES
    ('embedding_model', '"text-embedding-3-small"', 'active'),
    ('embedding_dimension', '1536', 'active'),
    ('default_quality_threshold', '85', 'active'),
    ('default_auto_publish', 'false', 'active'),
    ('max_revise_count', '2', 'active'),
    ('timezone', '"Asia/Tokyo"', 'active')
ON CONFLICT DO NOTHING;

-- ============================================================================
-- 完成
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '001_init_schema: All tables, constraints, indexes, triggers, and seed data created.';
    RAISE NOTICE '  - 26 tables created';
    RAISE NOTICE '  - HNSW vector index on vector_documents.embedding';
    RAISE NOTICE '  - GIN indexes on all JSONB columns';
    RAISE NOTICE '  - Seed data: 1 project, 1 style_guide, 6 system_settings';
    RAISE NOTICE '  - All constraints are idempotent (IF NOT EXISTS / ON CONFLICT DO NOTHING)';
END $$;
