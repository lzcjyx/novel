# 日常运维手册

## 日常检查

### 1. 检查今天的章节是否生成

```sql
SELECT
  p.title AS project,
  cp.sequence,
  gj.job_date,
  gj.status,
  gj.final_score,
  gj.error_message
FROM generation_jobs gj
JOIN projects p ON gj.project_id = p.id
JOIN chapter_plans cp ON gj.chapter_plan_id = cp.id
WHERE gj.job_date = CURRENT_DATE
ORDER BY p.title, cp.sequence;
```

### 2. 检查待人工审核的章节

```sql
SELECT
  p.title,
  c.sequence,
  c.title AS chapter_title,
  c.status,
  c.updated_at
FROM chapters c
JOIN projects p ON c.project_id = p.id
WHERE c.status = 'needs_human_review';
```

### 3. 检查审稿评分趋势

```sql
SELECT
  p.title,
  c.sequence,
  rs.overall_score,
  rs.decision,
  rs.created_at
FROM review_scores rs
JOIN chapters c ON rs.chapter_id = c.id
JOIN projects p ON rs.project_id = p.id
ORDER BY rs.created_at DESC
LIMIT 20;
```

### 4. 检查生成失败

```sql
SELECT
  p.title,
  gj.job_date,
  gj.status,
  gj.error_message,
  gj.stderr,
  gj.updated_at
FROM generation_jobs gj
JOIN projects p ON gj.project_id = p.id
WHERE gj.status = 'failed'
ORDER BY gj.updated_at DESC
LIMIT 10;
```

## 手动操作

### 重新生成失败的章节

1. 在 `generation_jobs` 中找到失败的 job。
2. 删除该 job 记录（允许明天重新生成）：
   ```sql
   DELETE FROM generation_jobs WHERE id = 'FAILED_JOB_ID';
   ```
3. 或者在 `chapter_plans` 中确保该章节状态还是 `planned`。
4. 下次定时触发时会重试。

### 手动发布一章

1. 确保章节在 `chapters` 中 `status = 'final'`。
2. 检查 `blog_posts` 是否已有记录：
   ```sql
   SELECT * FROM blog_posts WHERE chapter_id = 'CHAPTER_ID';
   ```
3. 如果没有，手动运行 workfow 04 `review_and_repair`，设置 `force_publish=true`。
4. 或直接通过 WordPress / 博客 API 发布，然后手动写入 `blog_posts`：
   ```sql
   INSERT INTO blog_posts (project_id, chapter_id, external_post_id, slug, title, url, status, published_at)
   VALUES ('PROJECT_ID', 'CHAPTER_ID', 'WP_POST_ID', 'slug', 'Title', 'URL', 'published', now())
   ON CONFLICT (project_id, chapter_id) DO UPDATE SET
     external_post_id = EXCLUDED.external_post_id,
     status = 'published',
     published_at = now(),
     updated_at = now();
   ```

### 激活/暂停项目

```sql
-- 激活
UPDATE projects SET status = 'active', updated_at = now() WHERE id = 'PROJECT_ID';

-- 暂停
UPDATE projects SET status = 'paused', updated_at = now() WHERE id = 'PROJECT_ID';

-- 归档
UPDATE projects SET status = 'archived', updated_at = now() WHERE id = 'PROJECT_ID';
```

### 锁定/解锁 canon

```sql
-- 锁定一条 canon rule
UPDATE canon_rules SET is_locked = true, updated_at = now() WHERE id = 'RULE_ID';

-- 解锁
UPDATE canon_rules SET is_locked = false, updated_at = now() WHERE id = 'RULE_ID';
```

### 手动更新章节计划

```sql
-- 跳过一章
UPDATE chapter_plans SET status = 'skipped', updated_at = now() WHERE id = 'PLAN_ID';

-- 重新激活一章
UPDATE chapter_plans SET status = 'planned', updated_at = now() WHERE id = 'PLAN_ID';
```

## 数据库维护

### 查看表大小

```sql
SELECT
  tablename,
  pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

### 查看向量索引大小

```sql
SELECT
  indexname,
  pg_size_pretty(pg_relation_size(indexname::regclass)) AS size
FROM pg_indexes
WHERE indexname LIKE '%embedding%';
```

### 清理旧版本（可选）

```sql
-- 保留每个 chapter 最新 3 个 version，删除更旧的
DELETE FROM chapter_versions
WHERE id IN (
  SELECT id FROM (
    SELECT id,
      ROW_NUMBER() OVER (PARTITION BY chapter_id ORDER BY version_number DESC) AS rn
    FROM chapter_versions
  ) sub WHERE rn > 3
);
```

## 备份

### 导出 schema + 数据

```bash
# 使用 Direct Connection
pg_dump "$NEON_DATABASE_URL_DIRECT" \
  --no-owner --no-acl \
  --format=custom \
  --file=novel_factory_$(date +%Y%m%d).dump
```

### 仅导出 schema

```bash
pg_dump "$NEON_DATABASE_URL_DIRECT" \
  --schema-only --no-owner --no-acl \
  --file=novel_factory_schema_$(date +%Y%m%d).sql
```

### 恢复

```bash
pg_restore "$NEON_DATABASE_URL_DIRECT" \
  --clean --if-exists \
  --no-owner --no-acl \
  novel_factory_YYYYMMDD.dump
```

## 监控指标

### 每日生产健康度

```sql
SELECT
  job_date,
  COUNT(*) AS total_jobs,
  COUNT(*) FILTER (WHERE status = 'completed') AS completed,
  COUNT(*) FILTER (WHERE status = 'failed') AS failed,
  COUNT(*) FILTER (WHERE status = 'needs_human_review') AS needs_review,
  ROUND(AVG(final_score) FILTER (WHERE status = 'completed'), 1) AS avg_score
FROM generation_jobs
WHERE job_date >= CURRENT_DATE - INTERVAL '14 days'
GROUP BY job_date
ORDER BY job_date DESC;
```

### 审稿 Agent 表现

```sql
SELECT
  agent_name,
  COUNT(*) AS reviews,
  ROUND(AVG(score), 1) AS avg_score,
  COUNT(*) FILTER (WHERE pass = false) AS failures,
  ROUND(COUNT(*) FILTER (WHERE pass = false) * 100.0 / COUNT(*), 1) AS failure_rate
FROM agent_reviews
WHERE created_at >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY agent_name
ORDER BY agent_name;
```
