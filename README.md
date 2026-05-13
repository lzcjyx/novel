# AI Novel Factory

AI 长篇小说自动生产桌面应用。本地运行，零外部依赖（仅需 API Key）。

## 架构

```
Tauri v2 Desktop App
├── React + TypeScript (PlayStation-style UI)
└── Rust Backend
    ├── SQLite (rusqlite bundled) — 全部状态存储
    ├── reqwest — 直接调用 AI API
    ├── keyring — OS 系统密钥链存储 API Key
    └── workflow_runner — 代替 n8n 流程编排
```

**不依赖**: Docker、n8n、Neon PostgreSQL、Node.js 服务、Claude Code CLI

## 支持的模型

| 用途 | 支持 |
|------|------|
| 小说创作 | DeepSeek / OpenAI / Anthropic Claude / Gemini / OpenAI Compatible |
| 向量化 (RAG) | OpenAI / 智谱 GLM / OpenAI Compatible（独立配置） |
| 本地模型 | Ollama / vLLM（OpenAI Compatible 预设） |

## 快速开始

1. 下载 GitHub Release 安装包，安装
2. 打开 Settings → 选择模型提供商 → 填入 API Key → Save & Test Connection
3. Projects → + New Novel → 填写世界观设定 → 生成小说圣经
4. Dashboard → Generate Weekly Plan → 生成 10 章章节计划
5. Dashboard → Write Chapter Now → AI 生成章节 → 7 Agent 审稿 → 自动修订 → 导出 .md

## 核心功能

- **多小说项目管理** — 独立圣经、角色、地点、世界观
- **结构化小说圣经** — 角色/地点/组织/道具/力量体系/世界设定/硬性规则/剧情线/伏笔/风格指南/时间线
- **章节版本管理** — draft → review → revised → final 完整状态机
- **7 Agent 审稿** — continuity / character / plot_logic / pacing / style / safety / publication + review_arbiter
- **自动修订循环** — score < threshold → 修订 → 重审 → 直到达标或耗尽重试次数
- **RAG 向量检索** — 嵌入圣经数据，写作时检索相关上下文
- **知识图谱** — 交互式人物/地点/组织关系图（Obsidian 风格）
- **自我学习** — 用户输入范文/网页学习 → AI 提取写作技巧 → 注入后续章节
- **进度可视化** — 实时 pipeline 时间线，每步耗时统计
- **人工审核参与** — 编辑章节内容、修改圣经数据、审阅/驳回 Agent 评审
- **Markdown 导出** — 自动导出 .md 文件到本地目录
- **幂等保护** — generation_jobs 唯一约束，重复点击不重复生成
- **API Key 安全** — OS 系统密钥链存储 + SQLite 加密回退，日志脱敏

## 项目结构

```
tauri-app/
├── src/                          # React 前端
│   ├── App.tsx                   # 主组件
│   ├── index.css                 # DESIGN.md PlayStation 风格
│   └── main.tsx
├── src-tauri/
│   ├── Cargo.toml
│   ├── prompts/                  # 14 AI Prompt 模板
│   ├── migrations/
│   │   └── 001_init_sqlite.sql   # 29 个业务表
│   ├── tests/                    # 集成测试
│   └── src/
│       ├── lib.rs                # 25+ Tauri 命令
│       ├── db/                   # SQLite CRUD (12 模块)
│       ├── models/               # 数据模型
│       ├── ai/                   # ModelClient + 5 providers + ProviderFactory
│       ├── workflow/             # 流程引擎 (10 模块)
│       ├── prompts/              # 加载 + 渲染
│       ├── security/             # 密钥链 + 脱敏
│       └── export/               # Markdown 导出
├── package.json
└── vite.config.ts
```

## 测试

```bash
cd tauri-app
cargo test --manifest-path src-tauri/Cargo.toml
```

测试覆盖: 全流程基准测试 (7 项) + 数据库 CRUD (4 项) + 审稿仲裁器 (2 项) = **13 项测试**

## 数据存储

- **SQLite**: `Documents/AI-Novels/ai-novel-factory.db`
- **Markdown 导出**: `Documents/AI-Novels/novel-XXXXXXXX/ch001.md`
- **API Key**: OS 系统密钥链 + SQLite 加密回退

## 开发

```bash
cd tauri-app
npm install
npm run tauri dev
npm run tauri build
```

## 技术栈

React 19 + TypeScript + Vite · Rust + Tauri v2 · SQLite (rusqlite) · reqwest · keyring · async-trait · chrono · serde_json · uuid · sha2 · regex · dirs
