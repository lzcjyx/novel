-- ============================================================================
-- 002_pgvector_indexes.sql
-- pgvector HNSW 索引
--
-- 执行方式：Neon Direct Connection
--   psql "NEON_DATABASE_URL_DIRECT" -f 002_pgvector_indexes.sql
--
-- 注意：
--   - 此文件可重复执行（幂等），使用 IF NOT EXISTS。
--   - HNSW 索引比 IVFFlat 构建更快、查询更快，适合 Neon。
--   - 向量维度当前为 1536（OpenAI text-embedding-3-small）。
--   - 如果更换 embedding 模型，需先 DROP 旧索引，更新表列维度，再重建索引。
--   - 所有向量检索查询必须带 ORDER BY embedding <=> $query LIMIT N。
-- ============================================================================

INSERT INTO schema_migrations (version, description)
VALUES ('002_pgvector_indexes', 'Create HNSW indexes on vector_documents embedding column')
ON CONFLICT (version) DO NOTHING;

-- ============================================================================
-- HNSW 索引 — vector_documents.embedding
--
-- HNSW 优势（vs IVFFlat）：
--   1. 不需要先插入数据再建索引，建表后可直接创建。
--   2. 查询速度通常更快。
--   3. 构建时间更短。
--   4. 不需要定期 REINDEX / ANALYZE。
--
-- IVFFlat 备选（如果 HNSW 不可用）：
--   CREATE INDEX IF NOT EXISTS idx_vector_documents_embedding_ivfflat
--   ON vector_documents
--   USING ivfflat (embedding vector_cosine_ops)
--   WITH (lists = 100);
--   注意：IVFFlat 需要先有一定数据量再建索引，建完后运行 ANALYZE vector_documents;
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_vector_documents_embedding_hnsw
ON vector_documents
USING hnsw (embedding vector_cosine_ops);

-- ============================================================================
-- 可选：为未来的其他向量表预留索引模板
-- 如果有多个向量表，按相同模式创建：
--
-- CREATE INDEX IF NOT EXISTS idx_<table>_embedding_hnsw
-- ON <table>
-- USING hnsw (embedding vector_cosine_ops);
-- ============================================================================

-- ============================================================================
-- 向量检索示例查询（供 n8n Code 节点或 Postgres 节点参考）
-- ============================================================================

-- 示例 1：基础相似度检索
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

-- 示例 2：按 source_type 过滤
-- SELECT
--   id,
--   title,
--   content,
--   metadata,
--   1 - (embedding <=> $1::vector) AS similarity
-- FROM vector_documents
-- WHERE project_id = $2
--   AND source_type = ANY($3::text[])
-- ORDER BY embedding <=> $1::vector
-- LIMIT 12;

-- 示例 3：带相似度阈值过滤
-- SELECT
--   id,
--   title,
--   content,
--   metadata,
--   1 - (embedding <=> $1::vector) AS similarity
-- FROM vector_documents
-- WHERE project_id = $2
--   AND 1 - (embedding <=> $1::vector) > 0.7
-- ORDER BY embedding <=> $1::vector
-- LIMIT 12;

-- ============================================================================
-- HNSW 索引维护说明
-- ============================================================================
-- HNSW 索引基本不需要维护，但如果查询性能下降：
-- 1. 检查索引是否膨胀：SELECT pg_size_pretty(pg_relation_size('idx_vector_documents_embedding_hnsw'));
-- 2. 如需重建：DROP INDEX idx_vector_documents_embedding_hnsw; 然后重新 CREATE。
-- 3. Neon 管理索引存储，通常不需要手动 VACUUM。

-- ============================================================================
-- 完成
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '002_pgvector_indexes: HNSW indexes created on vector_documents.';
END $$;
