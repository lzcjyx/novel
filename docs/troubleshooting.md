# 故障排除指南

## 常见问题

### 1. "schema not initialized" 错误

**症状**：Workflow 01 在 Check Schema Ready 节点处停止。

**解决**：
```bash
psql "$NEON_DATABASE_URL_DIRECT" -f sql/000_neon_setup.sql
psql "$NEON_DATABASE_URL_DIRECT" -f sql/001_init_schema.sql
psql "$NEON_DATABASE_URL_DIRECT" -f sql/002_pgvector_indexes.sql
```

### 2. n8n 连接 Neon 失败

**症状**：Postgres 节点报错 `connection refused` 或 `SSL error`。

**检查清单**：
- [ ] Host 使用的是 Pooled host（含 `-pooler`）。
- [ ] SSL 设置为 **Require**。
- [ ] Port 是 `5432`。
- [ ] Database 名称正确（默认 `novel_factory`）。
- [ ] IP 白名单：Neon 需要将 n8n 服务器 IP 加入 allowlist（Neon Console → Settings → IP Allow）。

### 3. writer-service 返回 401 Unauthorized

**症状**：所有 writer-service 请求返回 401。

**解决**：
- 检查 `WRITER_SERVICE_TOKEN` 是否在 `.env` 和 n8n environment variables 中一致。
- 检查 n8n HTTP Request 节点是否使用了正确的 Header Auth credential。

### 4. writer-service 超时

**症状**：writer-service 请求 600 秒后超时，Claude Code 未完成。

**解决**：
- 增加 `CLAUDE_CODE_TIMEOUT_MS`（例如 900000 = 15 分钟）。
- 减少 `CLAUDE_CODE_MAX_TURNS`（例如 2）。
- 检查 Claude Code CLI 是否已安装并在 PATH 中。
- 检查 `ANTHROPIC_API_KEY` 是否有效。

### 5. 章节重复生成（幂等失效）

**症状**：同一章节在同一天被生成多次。

**检查**：
```sql
SELECT project_id, chapter_plan_id, job_date, COUNT(*) AS cnt
FROM generation_jobs
GROUP BY project_id, chapter_plan_id, job_date
HAVING COUNT(*) > 1;
```

**解决**：
- 确认 `generation_jobs` 表有 `UNIQUE(project_id, chapter_plan_id, job_date)` 约束。
- 确认 n8n 工作流中 `Acquire Lock` 节点使用了 `ON CONFLICT DO NOTHING RETURNING id`。
- 删除重复记录，保留最早的一条。

### 6. 章节重复发布（博客重复文章）

**症状**：同一章在博客上有多个文章。

**检查**：
```sql
SELECT project_id, chapter_id, COUNT(*) AS cnt
FROM blog_posts
GROUP BY project_id, chapter_id
HAVING COUNT(*) > 1;
```

**解决**：
- 确认 `blog_posts` 表有 `UNIQUE(project_id, chapter_id)` 约束。
- 手动删除重复的博客文章。
- 保留一条 `blog_posts` 记录。

### 7. 审稿 Agent 返回非 JSON

**症状**：`Collect All Reviews` 节点解析失败。

**解决**：
- 确保所有审稿 Agent 的 API 调用使用了 `"response_format": { "type": "json_object" }`。
- 检查模型是否支持 JSON mode（GPT-4o、GPT-4 Turbo 支持）。
- 如果使用 Claude API，在 system prompt 中明确要求 JSON 输出。

### 8. embedding 维度不匹配

**症状**：向量检索报错或返回空结果。

**检查**：
```sql
SELECT attname, atttypid::regtype
FROM pg_attribute
WHERE attrelid = 'vector_documents'::regclass
  AND attname = 'embedding';
```

**解决**：
- 确认 `EMBEDDING_DIMENSION` 和 `EMBEDDING_MODEL` 一致：
  - `text-embedding-3-small` → 1536
  - `text-embedding-3-large` → 3072
  - `text-embedding-ada-002` → 1536

### 9. pgvector HNSW 索引不可用

**症状**：创建索引时报错 `access method "hnsw" does not exist`。

**解决**：
- 检查 pgvector 版本：`SELECT extversion FROM pg_extension WHERE extname = 'vector';`
- HNSW 需要 pgvector >= 0.5.0。
- Neon 默认支持 HNSW。
- 如果 HNSW 不可用，使用 IVFFlat 代替（参考 `sql/002_pgvector_indexes.sql` 中的备选方案）。

### 10. Workflow 执行顺序问题

**症状**：Workflow 3 在 Workflow 5 之前运行，没有章节计划可用。

**解决**：
- 确保 Workflow 5（Weekly Arc Planner）在 Workflow 3（Daily Chapter Production）之前运行。
- 首次运行时，先手动运行 Workflow 5 生成首批章节计划。
- 或手动插入几条 `chapter_plans`：
  ```sql
  INSERT INTO chapter_plans (project_id, sequence, title, plot_goals, target_word_count)
  VALUES
    ('PROJECT_ID', 1, '第一章', ARRAY['建立主角身份', '引入世界观', '设置第一个冲突'], 3000),
    ('PROJECT_ID', 2, '第二章', ARRAY['推进冲突', '展示力量体系', '引入关键配角'], 3000);
  ```

### 11. writer-service 并发耗尽

**症状**：返回 503 "Max concurrency reached"。

**解决**：
- 增加 `WRITER_SERVICE_MAX_CONCURRENT`。
- 确保多个小说项目错开 Schedule Trigger 时间。
- 检查是否有卡住的 Claude Code 进程：`ps aux | grep claude`。

### 12. Neon connection pooling 问题

**症状**：`remaining connection slots are reserved for non-replication superuser connections`。

**解决**：
- 使用 Pooled connection（含 `-pooler` 的 host）。
- 不要在 n8n 中使用 Direct connection 进行大量查询。
- 减少同时运行的 n8n 工作流实例数。

### 获取更多帮助

1. 检查 `generation_jobs` 中的 `error_message`、`stderr`、`exit_code` 字段。
2. 检查 n8n 工作流执行历史（Workflow → Executions）。
3. 检查 writer-service 日志（stdout）。
4. 检查 Neon 控制台的 Query Monitor 和 Connection Pooling 状态。
