# AI 小说工厂故障排除指南

本指南适用于当前 Tauri + React + Rust + SQLite 桌面版。

## Dashboard 无法生成章节

**症状**：点击 `Write With Controls` 后返回 `No chapter plans available` 或 `No planned chapter found`。

**处理**：
- 先在 Dashboard 运行 `Generate Weekly Plan`。
- 到 Plans 页面确认至少有一条 `status = planned` 的章节计划。
- 如果某条计划误停在 `in_progress` 且没有生成章节，可按 `docs/operations.md` 的失败恢复 SQL 改回 `planned`。

## 生成任务一直显示运行中

**症状**：Dashboard 显示 RUNNING，但日志和 pipeline 不再变化。

**处理**：
- 先重启应用。
- 如果仍显示运行中，点击 `Reset Stuck Job`。
- 如果数据库中存在未结束 job，确认没有真实生成任务后将 job 标成 `failed`。

## 一天内重复生成被拦截

**症状**：返回 `Already generated a chapter today. Use force=true to override.`。

**说明**：Dashboard 的 `Write With Controls` 会用 `force=true` 立即写下一章；底层流水线仍会用 `generation_jobs(project_id, chapter_plan_id, job_date)` 保持幂等，避免同一计划同一天产生多个 job。

检查：

```sql
SELECT project_id, chapter_plan_id, job_date, COUNT(*) AS cnt
FROM generation_jobs
GROUP BY project_id, chapter_plan_id, job_date
HAVING COUNT(*) > 1;
```

## Context Preview 为空

**症状**：`Context Preview` 显示 `No context loaded` 或错误。

**处理**：
- 确认已经选择项目。
- 确认存在 `planned` 章节计划。
- 如果 Bible 为空，先创建项目或补充 Bible。
- 如果学习条目为空，到 Learn 页面导入样章；这不会阻塞生成，但会降低风格约束。

## 上下文不连贯

**检查顺序**：
- Chapters 页面最近 1 到 2 章是否有最终正文。
- 最近章节 `summary` 是否为空或失真。
- Bible 页面人物状态、伏笔、时间线是否过期。
- Settings 里的 Embedding Provider 是否开启；RAG 为 OFF 时只能使用结构化 canon 和最近正文。

生成时 draft prompt 会注入 `writing_context`，包含最近摘要、最近正文片段、上一章结尾钩子、canon、学习条目和 Dashboard 控制参数。

## 文风仍然俗套

**处理**：
- 在 `Chapter Controls` 的 `禁止动作` 中明确列出本书要禁的桥段和高频词。
- 在 `风格重点` 中写具体写法，不要只写“高级”“细腻”。
- 到 Learn 页面导入 2 到 5 段目标风格样章。
- 生成后在 Reviews 中查看 style/pacing 相关意见，再把反复出现的问题加入学习库。

## prompt 渲染失败

**症状**：错误里出现 `Unresolved placeholders`。

**原因**：某个 prompt 模板新增了 `{{PLACEHOLDER}}`，但代码没有提供变量。

**处理**：
- 检查对应 `tauri-app/src-tauri/prompts/*.md` 模板。
- 确认调用处使用 strict renderer 并传入所有变量。
- 不要让裸 `{{...}}` 进入模型调用。

## RAG 向量检索无结果

**处理**：
- Settings 中配置 Embedding Provider 和 API Key。
- 点击测试，确保 embedding API 可用。
- 修改 Bible 后重建索引。
- 如果仍为空，检查 `vector_document_metadata` 是否有当前项目数据。

```sql
SELECT source_type, COUNT(*) AS cnt
FROM vector_document_metadata
WHERE project_id = 'PROJECT_ID'
GROUP BY source_type;
```

## 章节生成成功但计划未完成

当前流水线在成功导出并更新 canon 后会调用 `mark_chapter_plan_completed`。如果状态异常，检查是否在 `update_canon` 前失败：

```sql
SELECT cp.id, cp.sequence, cp.status, gj.status AS job_status, gj.error_message
FROM chapter_plans cp
LEFT JOIN generation_jobs gj ON gj.chapter_plan_id = cp.id
WHERE cp.project_id = 'PROJECT_ID'
ORDER BY cp.sequence;
```

## 前端构建失败

运行：

```bash
cd tauri-app
npm run build
```

常见原因：
- Tauri invoke 参数名和 Rust command 参数不一致。
- TypeScript interface 漏了新增字段。
- JSX 标签未闭合。

## Rust 测试失败

运行：

```bash
cargo test --manifest-path tauri-app/src-tauri/Cargo.toml
```

重点关注 `core_writing_loop_tests`。这些测试覆盖 strict prompt rendering、weekly planner prompt、学习条目使用、writing context package、job 幂等、章节计划完成和核心生成流水线。
