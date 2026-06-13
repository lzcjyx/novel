# SillyTavern 启发的 AI Novel Factory 改进计划

## 状态

这是只供审阅的路线计划，尚未执行。它把 `docs/analysis/2026-06-13-sillytavern-ai-novel-factory-comparison.md` 中的比较结论转成 AI Novel Factory 后续改进阶段。

本计划吸收 SillyTavern 的成熟运行时能力，但目标仍是长篇小说生产应用，不是把产品改成聊天优先的角色扮演前端。

## 目标

把 AI Novel Factory 从“已有自动写作流水线的本地应用”，提升为“可配置、可检查、可扩展的长篇小说生产运行时”。

## 设计原则

- 保留现有核心流程：章节计划、上下文构建、草稿、评审、修订、Canon 更新、学习写入、任务追踪。
- 优先建设深 Module 和小 Interface，避免 prompt、context、provider、UI 逻辑继续分散。
- 在生成前让操作者看见 prompt 和 context 的最终组装结果。
- 增加可配置性，但不直接开放不安全的任意代码执行。
- 尽早建立导入导出格式，让项目资产可迁移。
- 每个未来实现切片都要能用非真实模型的确定性测试验证。

## Phase 0: 基线与契约

Goal: 在引入 SillyTavern 式运行时能力前，定义哪些现有能力不能退化。

Scope:

- 梳理章节生产、RAG、Graph-RAG、评审 agents、任务元数据、Canon 更新的当前工作流契约。
- 识别哪些测试已经覆盖确定性生成行为，哪些地方需要补 fixture。
- 第一批审计文件包括 `writing_context.rs`、`prompt_rendering.rs`、`review_agents.rs`、`vector_store.rs`、`knowledge_graph.rs`、`ai/client.rs`、`ai/factory.rs`、`src/App.tsx`、`src-tauri/src/lib.rs`。

Acceptance criteria:

- 维护者能说清章节生产、prompt rendering、context assembly、provider call、job event 的稳定 Interface。
- 未来任何 prompt/context 改动都能通过确定性测试和 snapshot 风格输出验证。
- 新功能不能削弱现有 RAG、Graph-RAG、review、job observability。

## Phase 1: Prompt Runtime Workbench

Goal: 用操作者可见的 Prompt Runtime Module 替代浅层模板替换。

Likely files:

- Modify: `tauri-app/src-tauri/src/workflow/prompt_rendering.rs`
- Modify: `tauri-app/src-tauri/prompts/`
- Create: `tauri-app/src-tauri/src/workflow/prompt_runtime.rs`
- Create: `tauri-app/src-tauri/src/db/prompt_presets.rs`
- Modify: `tauri-app/src-tauri/migrations/001_init_sqlite.sql`
- Create or split: `tauri-app/src/` 下的 prompt workbench 前端文件

Capabilities:

- 存储 prompt preset 和 prompt unit，并给每个 unit 稳定 identifier。
- 支持 role、order、enabled state、injection position、generation phase。
- 针对选中的项目和章节计划渲染 dry-run assembled prompt。
- 在生成前显示 token 估算和未解析变量错误。
- 导入导出 prompt preset package。
- 内置 prompt 作为只读默认值，用户覆盖写入 SQLite。

Acceptance criteria:

- 操作者能在章节草稿或评审运行前预览准确的 prompt 和 context package。
- 必需变量缺失时，prompt rendering 给出清晰失败原因。
- Prompt preset 导入导出后 identifier 和 order 不漂移。
- 测试覆盖 prompt unit 排序、变量解析、禁用 unit、dry-run 输出。

## Phase 2: Context Activation 与 Canon Rules

Goal: 把 SillyTavern 世界书能力改造成面向长篇小说的 Canon Activation system。

Likely files:

- Modify: `tauri-app/src-tauri/src/workflow/writing_context.rs`
- Create: `tauri-app/src-tauri/src/workflow/context_activation.rs`
- Create: `tauri-app/src-tauri/src/db/context_rules.rs`
- Modify: `tauri-app/src-tauri/src/db/knowledge_graph.rs`
- Modify: `tauri-app/src-tauri/src/db/vector_store.rs`
- Modify: `tauri-app/src-tauri/migrations/001_init_sqlite.sql`
- Create or split: `tauri-app/src/` 下的 context rule 前端文件

Capabilities:

- 增加 Canon Activation Rules，字段包括 primary keywords、secondary keywords、entity references、chapter ranges、priority、token budget、sticky behavior、cooldown、enabled state。
- 扫描章节计划、简介、上一章摘要、选中角色、选中地点、操作者备注作为 activation targets。
- 通过一个 Context Activation Module 合并规则命中、RAG、Graph-RAG。
- 在 chapter version 和 context preview 中持久化 activation trace。
- 展示每个上下文来源被纳入的原因：rule hit、vector score、graph path、manual pin。

Acceptance criteria:

- 给定固定项目数据和章节计划，激活出的上下文是确定性的。
- Context preview 能解释每条来源的纳入原因。
- 没有规则命中时，RAG 和 Graph-RAG 仍正常工作。
- 测试覆盖关键词激活、secondary-key 过滤、budget clipping、cooldown、trace persistence。

## Phase 3: Provider Capability 与 Profile Manager

Goal: 把模型配置从连接字段提升为 capability-aware Provider Capability Module。

Likely files:

- Modify: `tauri-app/src-tauri/src/ai/client.rs`
- Modify: `tauri-app/src-tauri/src/ai/factory.rs`
- Create: `tauri-app/src-tauri/src/ai/provider_capabilities.rs`
- Create: `tauri-app/src-tauri/src/db/model_profiles.rs`
- Modify: `tauri-app/src-tauri/migrations/001_init_sqlite.sql`
- Modify or split: `tauri-app/src/` 下的 settings UI

Capabilities:

- 存储 provider profiles：base URL、model name、context window、JSON reliability、streaming support、embedding support、price snapshots、intended use。
- 区分 draft、review、repair、embedding、summarization 模型。
- 当 profile 不适合某工作流时给出风险提示，例如小上下文跑 Graph-RAG，或结构化抽取模型缺少稳定 JSON 能力。
- 保留 OpenAI-compatible Adapter，同时让能力判断不依赖具体 provider。

Acceptance criteria:

- 操作者能在命名 profile 之间切换，而不是反复编辑原始设置。
- Job metadata 记录每个阶段使用的 model profile。
- 成本估算使用生成时的 profile price snapshot。
- 测试覆盖 capability validation 和 workflow profile selection。

## Phase 4: Operator Recipes

Goal: 在完整脚本语言之前，先提供安全的操作者自动化。

Likely files:

- Create: `tauri-app/src-tauri/src/workflow/operator_recipes.rs`
- Create: `tauri-app/src-tauri/src/db/operator_recipes.rs`
- Modify: `tauri-app/src-tauri/src/workflow/chapter_production.rs`
- Modify: `tauri-app/src-tauri/src/workflow/review_agents.rs`
- Modify: `tauri-app/src-tauri/migrations/001_init_sqlite.sql`
- Create or split: `tauri-app/src/` 下的 recipe UI

Capabilities:

- 增加内置 recipes：生成同一章的三个候选稿、只重跑风格评审、只构建 context preview、不生成正文、只修复 Canon consistency 问题、把选中素材总结成 Canon candidate。
- 用结构化 action 表示 recipe，不使用任意 JavaScript。
- 接入 progress、cancellation、retry、job timeline。

Acceptance criteria:

- 每个内置 recipe 都产生 job events，并且可取消。
- Recipe step 失败后留下可检查的原因。
- 测试能用 deterministic fake model adapters 执行 recipes。

## Phase 5: Import、Export 与 Interop

Goal: 让 AI Novel Factory 资产可迁移，并能吸收 SillyTavern 生态格式。

Likely files:

- Create: `tauri-app/src-tauri/src/workflow/package_io.rs`
- Create: `tauri-app/src-tauri/src/workflow/lorebook_import.rs`
- Create: `tauri-app/src-tauri/src/db/import_export.rs`
- Modify: `tauri-app/src-tauri/migrations/001_init_sqlite.sql`
- Create or split: `tauri-app/src/` 下的 import/export UI

Capabilities:

- 导出导入完整 project package。
- 导出导入 novel bible package。
- 导出导入 prompt preset package。
- 把 SillyTavern-style world info 或 lorebook JSON 导入为 Canon Activation Rules 和 source documents。
- 导入时保留 source provenance。
- 写入数据库前校验 package。

Acceptance criteria:

- Project package 对稳定字段能确定性 round trip。
- 无效 package 失败时不产生部分写入。
- 导入的 lorebook entries 记录原始格式和映射决策。
- 测试覆盖 package validation、失败 rollback、lorebook-to-rule conversion。

## Phase 6: Draft Alternatives 与 Selection

Goal: 把 SillyTavern swipes 思路改造成章节候选稿工作流。

Likely files:

- Modify: `tauri-app/src-tauri/src/workflow/chapter_production.rs`
- Create: `tauri-app/src-tauri/src/workflow/draft_alternatives.rs`
- Create: `tauri-app/src-tauri/src/db/draft_alternatives.rs`
- Modify: `tauri-app/src-tauri/migrations/001_init_sqlite.sql`
- Create or split: `tauri-app/src/` 下的 chapter comparison UI

Capabilities:

- 用同一章节计划和 context package 生成多个候选稿。
- 每个候选稿保留 prompt、context trace、model profile、review notes、cost。
- 支持候选稿并排比较。
- 选择 winning candidate，并记录选择理由。
- 只有操作者明确选择时，才把 rejected candidates 作为项目记忆保留。

Acceptance criteria:

- 候选稿生成不会覆盖当前 accepted chapter。
- 候选稿 metadata 足够审计生成过程。
- 测试覆盖 candidate creation、selection、rejection、accepted-version promotion。

## Phase 7: Extension Host

Goal: 在 prompt、context、provider、recipe Interface 稳定之后，再建立受限 Extension Host Module。

Likely files:

- Create: `tauri-app/src-tauri/src/extensions/manifest.rs`
- Create: `tauri-app/src-tauri/src/extensions/host.rs`
- Create: `tauri-app/src-tauri/src/extensions/permissions.rs`
- Modify: `tauri-app/src-tauri/src/lib.rs`
- Modify: `tauri-app/src-tauri/migrations/001_init_sqlite.sql`
- Create or split: `tauri-app/src/` 下的 extension manager UI

Capabilities:

- 支持声明式 extension manifest，包含 name、version、description、permissions、hooks、package contents。
- 第一阶段只支持非代码扩展类型：prompt packs、export targets、review rubrics、context rule packs、recipe packs。
- 增加明确权限：filesystem、network、model calls、project read、project write。
- Extension 导入后默认禁用。
- 提供 hook：`before_context_build`、`after_context_build`、`before_review`、`after_review`、`export_target`。

Acceptance criteria:

- Extension import 在激活前校验 manifest 和 permissions。
- Disabled extensions 不能影响 workflow output。
- Hook execution 写入 job metadata，可追踪。
- 测试覆盖 permission denial、disabled extension behavior、hook ordering。

## Phase 8: UI 与 Command Locality

Goal: 在新增 workbench 继续扩大 UI 之前，降低大文件维护摩擦。

Likely files:

- Modify and split: `tauri-app/src/App.tsx`
- Create: `tauri-app/src/pages/*`
- Create: `tauri-app/src/features/*`
- Create: `tauri-app/src/lib/tauriClient.ts`
- Modify and split: `tauri-app/src-tauri/src/lib.rs`

Capabilities:

- 把主要 workbench 拆成聚焦的 UI Modules：Projects、Bible、Chapters、Graph、Jobs、Reviews、Prompt Workbench、Context Rules、Provider Profiles、Recipes、Import/Export、Extensions。
- 增加 typed Tauri client Module，避免页面里重复 command names 和 payload shapes。
- 按领域拆分 backend command registration，同时尽量保留现有 Tauri command names。

Acceptance criteria:

- 修改一个页面时，不需要理解整个 `App.tsx` Implementation。
- Backend command registration 能按领域查找。
- Frontend build 和现有 smoke checks 继续通过。
- 测试和 type checks 能捕获 command payload 字段重命名。

## 推荐执行顺序

1. Phase 0: 基线与契约。
2. Phase 1: Prompt Runtime Workbench。
3. Phase 2: Context Activation 与 Canon Rules。
4. Phase 3: Provider Capability 与 Profile Manager。
5. Phase 5: Import、Export 与 Interop。
6. Phase 6: Draft Alternatives 与 Selection。
7. Phase 4: Operator Recipes。
8. Phase 8: UI 与 Command Locality，随各 workbench 增量推进。
9. Phase 7: Extension Host，等前面的 Interface 稳定后再做。

Extension Host 应该延后，因为从架构上看，一个 Adapter 只是 hypothetical seam；当 prompt packs、context packs、export targets、review rubrics、recipes 等多个具体 Adapter 出现后，才能证明哪些 seam 真实存在。

## 未来执行时的验证策略

未来进入实现时，应先用确定性测试，再做真实 provider 检查：

- Prompt assembly snapshot tests。
- Context activation fixture tests。
- RAG 和 Graph-RAG trace tests。
- Provider profile capability validation tests。
- Fake model adapter 下的 recipe execution tests。
- Import/export round-trip tests。
- Extension manifest 和 permission tests。
- Frontend production build。
- 改动 workflow 对应的 Tauri smoke checks。
- `git diff --check`。

## 主要风险

1. Prompt 和 macro 太灵活会降低可靠性。
   Mitigation: 先做 Prompt Runtime Workbench，再扩大 macro 能力。

2. Context rules 可能和 RAG、Graph-RAG 冲突。
   Mitigation: 由一个 Context Activation Module 统一负责排序、预算和 trace output。

3. Extension Host 可能引入安全问题。
   Mitigation: 先支持声明式 packages 和显式权限，不执行任意代码。

4. Provider 配置可能变得过于噪声。
   Mitigation: 用 named profiles 和 capability warnings，而不是一次性暴露所有 provider 选项。

5. UI 扩张可能让 `App.tsx` 更难维护。
   Mitigation: 每新增一个 workbench，就顺手拆分对应页面和 typed command calls，不做无关大重写。

## 第一切片建议

建议第一阶段把 Phase 1 和 Phase 2 合成一个产品里程碑：“Prompt And Context Workbench”。

这个里程碑的 Leverage 最高，因为它让生成过程中最关键的隐藏部分可见：最终 prompt 是什么，纳入了哪些 context，为什么纳入，占用了多少预算。它也会形成 provider profiles、recipes、import/export、candidate drafts、future extensions 需要依赖的稳定 Interface。
