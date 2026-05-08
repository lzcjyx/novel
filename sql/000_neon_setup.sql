-- ============================================================================
-- 000_neon_setup.sql
-- Neon PostgreSQL 初始化 — 扩展与基础配置
--
-- 执行方式：Neon Direct Connection
--   psql "NEON_DATABASE_URL_DIRECT" -f 000_neon_setup.sql
--
-- 不要使用 Neon Pooled Connection 执行此文件。
-- ============================================================================

-- 记录 migration（幂等，001 会用 CREATE IF NOT EXISTS）
CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    description TEXT,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO schema_migrations (version, description)
VALUES ('000_neon_setup', 'Enable extensions and configure database defaults')
ON CONFLICT (version) DO NOTHING;

-- ============================================================================
-- 扩展
-- ============================================================================

CREATE EXTENSION IF NOT EXISTS vector;       -- pgvector: embedding 存储与相似度检索
CREATE EXTENSION IF NOT EXISTS pgcrypto;     -- gen_random_uuid(), 加密函数

-- ============================================================================
-- 时区设置（Neon 使用 ALTER ROLE 而非 ALTER DATABASE）
-- ============================================================================

-- Neon 不允许 ALTER DATABASE SET。使用 ALTER ROLE 代替。
DO $$
BEGIN
    EXECUTE 'ALTER ROLE ' || current_user || ' SET timezone TO ''Asia/Tokyo''';
END $$;

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
-- 完成标记
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '000_neon_setup: Extensions enabled, base configuration complete.';
END $$;
