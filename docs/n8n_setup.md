# n8n 工作流设置指南

## 1. 前提条件

- n8n 实例（Cloud 或 self-hosted）。
- Neon 数据库已初始化（参考 `docs/neon_setup.md`）。
- writer-service 已部署（参考 `writer-service/` 目录）。
- API keys 已配置（Anthropic、OpenAI、Gemini）。

## 2. n8n Credentials 配置

在 n8n 中创建以下 credentials：

### 2.1 Postgres — Neon Pooled (n8n)

| 字段 | 值 |
|------|-----|
| Credential Name | `Neon Pooled (n8n)` |
| Host | `ep-xxx-pooler.region.aws.neon.tech` |
| Port | `5432` |
| Database | `novel_factory` |
| User | Neon user |
| Password | Neon password |
| SSL | Require |

### 2.2 HTTP Header Auth — OpenAI API Key

| 字段 | 值 |
|------|-----|
| Credential Name | `OpenAI API Key` |
| Name | `Authorization` |
| Value | `Bearer {{$env.OPENAI_API_KEY}}` |

### 2.3 HTTP Header Auth — Writer Service Token

| 字段 | 值 |
|------|-----|
| Credential Name | `Writer Service Token` |
| Name | `Authorization` |
| Value | `Bearer {{$env.WRITER_SERVICE_TOKEN}}` |

### 2.4 HTTP Header Auth — WordPress Auth

| 字段 | 值 |
|------|-----|
| Credential Name | `WordPress Auth` |
| Name | `Authorization` |
| Value | `Basic {{btoa($env.WORDPRESS_USERNAME + ':' + $env.WORDPRESS_APP_PASSWORD)}}` |

或者使用 n8n WordPress 节点自带的 credentials。

## 3. 导入工作流

1. 进入 n8n → **Workflows** → **Import from File**。
2. 依次导入：
   - `n8n/01_novel_bootstrap.workflow.json`
   - `n8n/02_bible_ingestion.workflow.json`
   - `n8n/03_daily_chapter_production.workflow.json`
   - `n8n/04_review_and_repair.workflow.json`
   - `n8n/05_weekly_arc_planner.workflow.json`

3. 导入后，每个工作流会自动匹配 credentials（名称匹配）。
4. 如果 credential 名称不匹配，手动选择对应的 credential。

## 4. 工作流依赖关系

```
01_novel_bootstrap (手动)
    ↓ 创建 project + 初始 bible
02_bible_ingestion (手动/被调用)
    ↓ 维护 canon
05_weekly_arc_planner (每周一 10:00)
    ↓ 生成 chapter_plans
03_daily_chapter_production (每天 09:00)
    ↓ 生成 + 审稿 + 发布
04_review_and_repair (手动)
    ↓ 修订失败章节
```

## 5. 激活定时工作流

1. **05_weekly_arc_planner** — 切换右上角 **Active** toggle。
2. **03_daily_chapter_production** — 切换右上角 **Active** toggle。

> ⚠️ 定时工作流必须先保存并激活，然后才会按照 Schedule Trigger 运行。

## 6. 运行初始化

1. 确保数据库已 migration。
2. 手动触发 **01_novel_bootstrap**：
   - 输入项目名称、类型、风格等。
   - 运行。
3. 等待 bible 生成完成。
4. 手动将 project status 改为 `active`（通过 Postgres 或 n8n）。
5. 可选：运行 **05_weekly_arc_planner** 生成首批章节计划。
6. 激活 **03_daily_chapter_production** 开始每日自动生产。

## 7. 环境变量

确保 n8n 能访问 `.env` 中定义的环境变量：

- **n8n Cloud**：通过 n8n Cloud 的 Environment Variables 设置。
- **self-hosted n8n (Docker)**：通过 `docker run --env-file .env` 传入。
- **self-hosted n8n (npm)**：在启动前 `source .env` 或 `export`。

## 8. 自托管 n8n 的 Execute Command 说明

如果你使用 self-hosted n8n 并希望直接用 Execute Command 调用 Claude Code：

1. 在 `03_daily_chapter_production` 中替换 `Call Writer Service` 节点。
2. 使用 Execute Command 节点：
   ```bash
   cat /tmp/n8n_novel_prompt_{{ $json.job_id }}.json | claude --bare -p --output-format json --max-turns 3
   ```
3. 需要先用 Code 节点将 prompt 写入临时文件。
4. 仍然推荐使用 writer-service，因为：
   - 兼容 n8n Cloud。
   - 并发控制更好。
   - 错误处理和日志更完善。
