# AI 小说工厂日常运维手册

当前项目是本地 Tauri 桌面应用，主数据库是 SQLite。默认数据目录在系统 Documents 下的 `AI-Novels`，数据库文件通常是 `AI-Novels/ai-novel-factory.db`，导出的章节 Markdown 也在该目录下。

## 每日生产流程

1. 打开 Dashboard，选择项目。
2. 如果 `Plans Left` 为 0，先运行 `Generate Weekly Plan`。
3. 在 `Chapter Controls` 里填写本章意图、必写节拍、禁止动作和风格重点。
4. 点 `Refresh` 检查 `Context Preview`，确认下一章计划、上一章钩子、canon 数量和学习条目正常。
5. 点 `Write With Controls`。流水线应依次出现 `acquire_lock`、`load_canon`、`retrieve_context`、`generate_draft`、`aggregate_reviews`、`export`、`update_canon`、`complete`。
6. 生成后到 Chapters 查看正文，到 Reviews 查看评分和阻塞意见。

## 状态检查

使用 SQLite 客户端打开本地数据库后可运行：

```sql
SELECT
  p.name AS project,
  cp.sequence,
  cp.title AS plan_title,
  gj.job_date,
  gj.status,
  gj.error_message,
  gj.updated_at
FROM generation_jobs gj
JOIN projects p ON gj.project_id = p.id
JOIN chapter_plans cp ON gj.chapter_plan_id = cp.id
ORDER BY gj.created_at DESC
LIMIT 20;
```

```sql
SELECT
  c.sequence,
  c.title,
  c.status,
  c.word_count,
  rs.final_score,
  rs.decision,
  c.updated_at
FROM chapters c
LEFT JOIN review_scores rs ON rs.chapter_id = c.id
WHERE c.project_id = 'PROJECT_ID'
ORDER BY c.sequence DESC;
```

## 失败恢复

常见恢复顺序：

1. Dashboard 如果显示运行中但没有进度，先重启应用。
2. 仍卡住时使用 `Reset Stuck Job`，只重置前端运行标记。
3. 如果数据库里 job 一直是 `started`、`reviewing`、`revising` 或 `publishing`，确认没有真实生成任务后再改状态：

```sql
UPDATE generation_jobs
SET status = 'failed',
    error_message = 'manually marked failed after local app recovery',
    completed_at = datetime('now'),
    updated_at = datetime('now')
WHERE id = 'JOB_ID';
```

4. 如果章节计划已经被置为 `in_progress` 但没有对应章节，可重新激活：

```sql
UPDATE chapter_plans
SET status = 'planned',
    updated_at = datetime('now')
WHERE id = 'PLAN_ID';
```

## 上下文质量维护

为了减少上下文断裂：

```sql
SELECT sequence, title, status, summary
FROM chapters
WHERE project_id = 'PROJECT_ID'
ORDER BY sequence DESC
LIMIT 8;
```

如果最近章节摘要为空或不准确，先在 Chapters 里人工修正文稿，再让后续生成使用新的最终版本。生成成功后，canon 更新使用最终稿，不再使用初稿。

学习库检查：

```sql
SELECT category, pattern_name, confidence, usage_count, last_used_at
FROM learning_entries
WHERE project_id = 'PROJECT_ID'
ORDER BY confidence DESC, usage_count ASC
LIMIT 20;
```

缺少风格样本时，到 Learn 页面加入人工样章或网页样章。Dashboard 的 `Context Preview` 会显示本次将注入的高置信学习条目。

## 向量索引维护

如果 Dashboard 显示 RAG 为 OFF，到 Settings 配置 Embedding Provider。修改 Bible 条目后可在 Bible 页面使用 `Apply to All & Rebuild Index`。

手动清空某项目向量元数据：

```sql
DELETE FROM vector_document_metadata
WHERE project_id = 'PROJECT_ID';
```

随后在应用里重建索引。

## 备份

关闭应用后复制整个 `AI-Novels` 目录即可备份数据库和导出章节。最小备份只需复制：

```text
AI-Novels/ai-novel-factory.db
```

恢复时先关闭应用，再用备份文件替换同名数据库。
