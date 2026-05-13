# AI Novel Factory v2 — Design Spec

> 目标：补齐大模型应用开发春招学习计划的最终目标 1–8，增加知识图谱、进度流式推送、人工审核参与、本地模型支持。

## 1. Bug 修复与 Pipeline 鲁棒性

### 1.1 问题根因

| Bug | 根因 | 位置 |
|-----|------|------|
| Jobs 卡在 reviewing | `update_job_status("completed")` 在 revison 失败路径中不可达；pipeline 中间任何 panic 都会导致 job 状态永久停滞 | `chapter_production.rs:226→414` |
| Reviews 为空 | `let _ = save_agent_review(...)` 静默丢弃错误；若 save 失败无重试；ReviewPage 无空状态提示 | `chapter_production.rs:247`, `App.tsx` |
| 单个 review agent 挂起阻塞全部 | `tokio::join!` 等待所有 future 完成，任一超时不返回则全部等待 | `review_agents.rs:35` |

### 1.2 修复方案

1. **全局 try/catch 包裹** — 在 `generate_next_chapter` 最外层添加 `if let Err(e) = ...` 统一错误处理：任何未捕获错误都会 `update_job_status("failed")` 并释放锁
2. **Review agent 超时隔离** — 每个 agent 包一层 `tokio::time::timeout(300s)`，超时则返回带 `score: 0` 的占位 review，不阻塞其他 agent
3. **Save review 非静默** — 错误记录到 log，重试一次，仍失败则标记 job 为 `"completed_warnings"` 而非 `"failed"`
4. **Revision 空内容保护** — 已实现（<100 chars 保留原稿）；补上 job status 更新
5. **前端空状态** — Reviews/Jobs 查询返回空时显示 "数据尚未生成，可能在运行中" + 手动刷新按钮

---

## 2. 进度流式推送

### 2.1 后端：Tauri Events

Rust 端在每个 pipeline 步骤前后 emit `pipeline-step` 事件：

```rust
#[derive(Clone, Serialize)]
struct PipelineEvent {
    step: String,           // acquire_lock | load_canon | retrieve_context |
                            // generate_draft | review_a1..review_a7 |
                            // aggregate_reviews | revise | export | update_canon | complete
    status: String,         // running | done | failed
    elapsed_ms: Option<u64>,
    detail: Option<String>, // e.g. "3239 words", "score=65 pass=true"
    progress_pct: f64,      // 0-100
    timestamp: String,
}
```

Event 从 `generate_next_chapter` 函数内通过 `app_handle: AppHandle` 发射。`app_handle` 作为参数传入 `generate_next_chapter`。

### 2.2 前端：Pipeline 时间线

Dashboard 进度区域替换 spinner，渲染为垂直步骤列表：

```
○ 获取生成锁 .............................. 0.1s
○ 加载结构化圣经 .......................... 0.3s  
○ 检索向量上下文 .......................... 1.2s
● 生成初稿 ................................ 23s (运行中)
○ 审稿: 连续性 ............................ 待定
○ 审稿: 人物 .............................. 待定
... (7 个 agent 各自一行)
```

- ○ 待定 (pending) / ◌ 运行中 (running, 带 spinner) / ✓ 完成 (done, 绿色) / ✗ 失败 (failed, 红色)
- 失败步骤可展开查看错误详情
- 顶部 **取消按钮** 调用 `reset_running` 中止 pipeline

### 2.3 工程指标实时面板

时间线下方显示实时指标：

```
总耗时: 2m 34s  |  Prompt tokens: 8,421  |  Completion tokens: 3,892
估算成本: ¥1.02  |  初稿延迟: 58s         |  审稿延迟: 22s
```

数据持久化到 `pipeline_metrics` 表（见 §5.2）。

- **字体和颜色遵循 DESIGN.md token 系统**：`var(--font-display)` 用于标题，`var(--on-dark-body)` 用于正文，`var(--primary)` 用于运行中状态
- **暗色基底**：`var(--canvas-dark)` (#000000)，卡片 `var(--surface-dark-card)` (#181818)
- **PlayStation 风格按钮**：圆角 `var(--radius-full)`，Primary 蓝 `var(--primary)` (#0070d1)

---

## 3. 交互式知识图谱

### 3.1 数据模型

新增 `knowledge_graph_edges` 表：

```sql
CREATE TABLE knowledge_graph_edges (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_node_id TEXT NOT NULL,
    source_node_type TEXT NOT NULL,     -- character|location|organization|item|world_lore|magic_system|chapter|timeline_event|plot_thread|foreshadowing
    target_node_id TEXT NOT NULL,
    target_node_type TEXT NOT NULL,
    edge_type TEXT NOT NULL,            -- appears_in|located_at|owns|belongs_to|conflict|ally|discovers|triggers|resolves|custom
    description TEXT,
    auto_inferred INTEGER NOT NULL DEFAULT 1,
    confidence REAL DEFAULT 1.0,
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Node 类型**：Character, Location, Organization, Item, WorldLore, MagicSystem, Chapter, TimelineEvent, PlotThread, Foreshadowing

**Edge 类型（自动推断）**：`appears_in`（角色→章节）, `located_at`（角色→地点）, `owns`（角色→物品）, `belongs_to`（角色→组织）, `triggers`（事件→剧情线）, `resolves`（事件→伏笔）

**Edge 类型（用户创建）**：任何 custom 关系

### 3.2 关系提取

每章完成后，`canon_updater` 之后执行 `extract_relationships()`：

1. 加载章节正文 + 所有角色/地点/组织列表
2. 调用 AI (`generate_json`) 输出：
```json
{
  "new_relationships": [
    {"source": "char_id", "target": "char_id", "type": "conflict", "description": "赵无极在拍卖会上公然羞辱林风"},
    {"source": "char_id", "target": "loc_id", "type": "discovered", "description": "林风发现龙脉秘境的入口"}
  ],
  "updated_relationships": [
    {"edge_id": "...", "new_description": "..."}
  ]
}
```
3. INSERT 到 `knowledge_graph_edges`，`auto_inferred = 1`

### 3.3 前端：Cytoscape.js 图谱

- **Canvas** 占满 Bible 标签页区域，暗色背景 `var(--canvas-dark)`
- **节点颜色按类型**：角色 = `var(--primary)` (#0070d1), 地点 = `var(--commerce)` (#d53b00), 组织 = 灰色, 物品 = 金色, 章节 = 绿色
- **边** 细线 `var(--hairline-dark)`，关系越强越粗
- **力导向布局** + 碰撞检测避免标签重叠
- **单击节点** → 右侧滑入详情面板：名称、描述、所有关联节点和关系
- **双击空白** → "添加关系" 对话框：选择源节点、目标节点、关系类型、描述 → 创建用户边 (`auto_inferred = 0`)
- **拖拽节点** 重新排列，布局持久化
- **搜索栏** 顶部聚焦/高亮节点
- **过滤 pill** 显示/隐藏节点类型

### 3.4 与现有 Bible 编辑器的集成

节点详情面板即现有 Bible 编辑面板。图谱和标签页列表是同一数据的两个视图——一处修改另一处同步。

---

## 4. 用户参与：人工审核

### 4.1 世界观设定（增强现有表单）

+New Novel 表单新增字段：

| 新增字段 | 类型 | 对应 Bible 生成 |
|----------|------|----------------|
| 目标总字数 | number (e.g. 500000) | `projects.total_target_words` |
| 每日章节字数 | number (e.g. 3000) | `projects.daily_target_words` |
| 叙事视角 | 选择：第一人称/第三人称/多视角 | `style_profile` |
| 文风 | 多选：热血/轻松/悬疑/暗黑/史诗/幽默 | `style_profile.tone` |
| 禁用短语 | tag 输入 (e.g. "眼中闪过", "嘴角上扬") | `style_profile.forbidden_phrases` |
| 推荐技巧 | tag 输入 (e.g. "动作链", "对话推进") | `style_profile.preferred_techniques` |
| 核心前提取 | textarea: "如果...会怎样？" | Bible prompt |
| 初始角色构想 | textarea: "主角：..., 反派：..., ..." | Bible prompt |

**所有 UI 文本使用中文。**

### 4.2 审稿人工审核

Reviews 页面新增操作：

- **"通过" 按钮** — 标记该 review 为已验证。若全部通过且 score ≥ 85，可强制发布
- **"驳回" 按钮** — 标记该 review 为不准。若 ≥3 个被驳回，重算分数：`new_avg = sum(未被驳回的 reviews 的 score) / count(未被驳回)`。若 new_avg ≥ 85 且无 blocking，可发布
- **"添加评审意见"** — 用户写人工评审，下次修订时注入到 revision prompt

章节级操作：

- **"强制发布"** — 绕过 arbiter 决策，标记为 `published`
- **"手动修订指令"** — 用户写具体修订要求（"打斗场景写长一些"，"增加林风和赵无极的对话"），注入到 revision writer prompt
- **"换模型重试"** — 用不同 model/temperature 重跑 draft writer

### 4.3 新增 Tauri 命令

```rust
async fn approve_review(review_id: String) -> Result<(), String>
async fn reject_review(review_id: String) -> Result<(), String>
async fn add_review_note(review_id: String, note: String) -> Result<(), String>
async fn force_publish_chapter(chapter_id: String) -> Result<(), String>
async fn retry_with_prompt(chapter_id: String, user_instructions: String) -> Result<GenerationResult, String>
```

---

## 5. 评测与工程指标

### 5.1 RAG / Agent 评测（Goal 6）

**Settings → 评测** 面板：

- **RAG 评测**：用户创建测试集（5-10 条 query + 期望 doc ID），运行后显示 Recall@5 和 MRR
- **Agent 诊断**：自动分析 `agent_reviews` 数据——JSON 解析成功率、各 agent 分数分布、方差最大的 agent（不稳定）、平均 token 消耗
- 不需要新增 AI 调用——仅分析已有数据

### 5.2 工程指标表（Goal 7）

```sql
CREATE TABLE pipeline_metrics (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    chapter_id TEXT,
    job_id TEXT,
    step TEXT NOT NULL,              -- draft_generation | review_continuity | ...
    provider TEXT,
    model TEXT,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    latency_ms INTEGER,
    estimated_cost_usd REAL,
    success INTEGER NOT NULL DEFAULT 1,
    error_type TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

每次 AI 调用写入一行。Dashboard 的"工程指标"卡片显示：

```
今日: 3 次调用 | 12,400 tokens | ~¥1.28 | 平均延迟: 32s
本小说: 47 次调用 | 198K tokens | ~¥20.58
```

含 token 柱状图（prompt vs completion/chapter）和延迟趋势线。

---

## 6. 本地模型支持（Goal 8）

现有 `OpenAICompatibleProvider` 已支持任意 `base_url`。新增：

| 新增 | 目的 |
|------|------|
| **Ollama 预设** | 选择 "Ollama" → 自动填写 `http://localhost:11434/v1`，调用 `/api/tags` 获取已安装模型列表 |
| **vLLM 预设** | 选择 "vLLM" → 自动填写 `http://localhost:8000/v1`，调用 `/v1/models` 获取模型列表 |
| **本地模型标识** | Dashboard 上若 provider 为本地，显示 "🖥️ 本地" badge |
| **README 章节** | 逐步指南：安装 Ollama → pull 模型 → 配置 → 测试连接 |

---

## 7. 实现文件清单

| 模块 | 文件 | 变更类型 |
|------|------|---------|
| Pipeline 鲁棒性 | `src/workflow/chapter_production.rs` | 修改 — try/catch + timeout |
| Review 超时 | `src/workflow/review_agents.rs` | 修改 — `tokio::time::timeout` 包裹 |
| 进度事件 | `src/lib.rs` | 修改 — 新增 app_handle 参数 + emit |
| 进度事件 | `src/workflow/chapter_production.rs` | 修改 — 每步骤 emit |
| 知识图谱 | `migrations/001_init_sqlite.sql` | 修改 — 新增 `knowledge_graph_edges` |
| 知识图谱 | `src/workflow/canon_updater.rs` | 修改 — 新增 `extract_relationships()` |
| 知识图谱 | `src/db/knowledge_graph.rs` | 新增 — CRUD |
| 知识图谱 | `src/App.tsx` | 修改 — Bible 页切换为图谱视图 |
| 用户参与 | `src/App.tsx` | 修改 — 增强创建表单 + Reviews 操作 |
| 用户参与 | `src/lib.rs` | 修改 — 新增 5 个 Tauri 命令 |
| 评测 | `src/db/eval.rs` | 新增 — RAG 评测逻辑 |
| 指标 | `src/lib.rs` | 修改 — 每次 AI 调用写入 metrics |
| 指标 | `src/App.tsx` | 修改 — Dashboard 指标卡片 |
| 本地模型 | `src/ai/ollama.rs` | 新增 — 模型列表获取 |
| 本地模型 | `src/App.tsx` | 修改 — Settings 新增预设 |

---

## 8. 验证

1. `cargo test` — 全部已有测试通过 + 新增 `knowledge_graph`、`eval` 测试
2. `npx tsc --noEmit` — TS 编译无错误
3. 手动：创建小说 → 填写完整世界观设定 → 生成 Bible → 图谱展示角色/地点关系
4. 手动：Write Chapter Now → 进度时间线实时更新 → 每步显示耗时 → 指标面板更新
5. 手动：审稿完成 → 用户驳回一个 agent → 分数重算 → 用户写手动修订指令 → 重新修订
6. 手动：Jobs 页面不再卡在 reviewing；Reviews 页面有数据或显示空状态
7. 手动：Settings 选择 Ollama → 自动获取模型列表 → Test Connection 成功
