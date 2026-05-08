# AI Novel Factory

生产级、可复制、可扩展的 AI 长篇小说自动生产流水线。

## 架构

```
n8n (流程编排)
  ├──→ Neon PostgreSQL + pgvector (状态存储 / 向量检索)
  ├──→ writer-service → Claude Code CLI (章节生成)
  ├──→ Claude / OpenAI / Gemini API (多 Agent 审稿)
  └──→ WordPress / REST API (博客发布)
```

## 项目结构

```
novel/
├── sql/
│   ├── 000_neon_setup.sql          # 扩展 + 基础配置
│   ├── 001_init_schema.sql         # 全部表 + 索引 + 触发器 + 种子数据
│   └── 002_pgvector_indexes.sql    # pgvector HNSW 索引
├── n8n/
│   ├── 01_novel_bootstrap.workflow.json       # 项目初始化
│   ├── 02_bible_ingestion.workflow.json       # 圣经更新
│   ├── 03_daily_chapter_production.workflow.json  # 每日章节生产
│   ├── 04_review_and_repair.workflow.json     # 手动返修
│   └── 05_weekly_arc_planner.workflow.json    # 每周剧情规划
├── prompts/                       # AI prompt 模板 (12 个)
├── writer-service/
│   ├── package.json
│   ├── server.js                  # Express HTTP 服务
│   ├── Dockerfile
│   └── .env.example
├── docs/
│   ├── neon_setup.md              # Neon 数据库设置
│   ├── n8n_setup.md               # n8n 工作流设置
│   ├── operations.md              # 日常运维手册
│   └── troubleshooting.md         # 故障排除
├── .env.example
└── README.md
```

## 快速开始

### 1. Neon 数据库

```bash
# 创建 Neon 项目 → 获取连接字符串 → 设置环境变量
export NEON_DATABASE_URL_DIRECT="postgresql://USER:PASSWORD@ep-xxx.region.aws.neon.tech/novel_factory?sslmode=require"

# 执行 migration
psql "$NEON_DATABASE_URL_DIRECT" -f sql/000_neon_setup.sql
psql "$NEON_DATABASE_URL_DIRECT" -f sql/001_init_schema.sql
psql "$NEON_DATABASE_URL_DIRECT" -f sql/002_pgvector_indexes.sql
```

详见 [docs/neon_setup.md](docs/neon_setup.md)。

### 2. Writer Service

```bash
cd writer-service
cp .env.example .env    # 编辑 .env 填入真实值
npm install
npm start               # 监听 :8787
```

### 3. n8n 工作流

1. 复制 `.env.example` → `.env`，填入所有真实值。
2. 在 n8n 中创建 credentials（参考 [docs/n8n_setup.md](docs/n8n_setup.md)）。
3. 导入 `n8n/` 目录下的 5 个 workflow JSON。
4. 首次运行：手动触发 Workflow 01 初始化项目。
5. 手动运行 Workflow 05 生成首批章节计划。
6. 激活 Workflow 03 和 Workflow 05 的定时触发。

详见 [docs/n8n_setup.md](docs/n8n_setup.md)。

## 5 个工作流

| # | 工作流 | 触发方式 | 说明 |
|---|--------|---------|------|
| 01 | Novel Bootstrap | 手动 | 创建项目、生成初始圣经、写入数据库 |
| 02 | Bible Ingestion | 手动/被调用 | 提取新设定、更新 canon、写入向量 |
| 03 | Daily Chapter Production | 每天 09:00 JST | 完整的 21 节点生产管线 |
| 04 | Review and Repair | 手动 | 修订失败章节、通过后发布 |
| 05 | Weekly Arc Planner | 每周一 10:00 JST | 分析节奏、生成未来 7-14 章计划 |

## 7 个审稿 Agent

| Agent | 职责 | Blocking 条件 |
|-------|------|-------------|
| continuity_reviewer | 时间线、地点、道具、设定一致性 | 违反 hard canon |
| character_reviewer | 人物性格、口吻、动机一致性 | 核心性格背离 |
| plot_logic_reviewer | 情节因果、冲突推进、机械降神 | 核心剧情目标未完成 |
| pacing_reviewer | 节奏、爽点、钩子、连载适配 | 无实质冲突或推进 |
| style_reviewer | 文风、语言质量、AI 味 | 文风严重偏离 |
| safety_reviewer | 安全、版权、密钥泄露 | 任何敏感内容 |
| publication_reviewer | Markdown、博客适配 | 格式严重损坏 |

## 关键约束

- **Neon 数据库**：不用 Docker PostgreSQL。Pooled connection 用于 n8n，Direct connection 用于 migration。
- **幂等控制**：`INSERT ... ON CONFLICT DO NOTHING RETURNING id`，不用 session-level advisory lock。
- **发布安全**：默认 draft，仅当 `auto_publish=true AND final_score>=85` 时 publish。
- **密钥管理**：所有 API key 使用 n8n credentials 或环境变量，绝不硬编码。
- **向量检索**：始终限定 `project_id`，防止跨项目上下文污染。
- **Locked canon**：不自动覆盖。低置信度变更进入 `human_review_required`。
- **防重复发布**：`blog_posts` 有 `UNIQUE(project_id, chapter_id)`，发布前检查。

## 环境变量

见 `.env.example`。关键变量：

| 变量 | 说明 |
|------|------|
| `NEON_DATABASE_URL_POOLED` | n8n 日常读写连接 |
| `NEON_DATABASE_URL_DIRECT` | migration/管理连接 |
| `EMBEDDING_DIMENSION` | 向量维度，默认 1536 |
| `EMBEDDING_MODEL` | Embedding 模型 |
| `WRITER_SERVICE_URL` | writer-service 地址 |
| `WRITER_SERVICE_TOKEN` | writer-service 认证 token |
| `ANTHROPIC_API_KEY` | Claude API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `WORDPRESS_BASE_URL` | WordPress 站点 URL |

## License

MIT
