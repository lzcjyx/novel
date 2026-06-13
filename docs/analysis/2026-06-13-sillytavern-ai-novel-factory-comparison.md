# AI Novel Factory 与 SillyTavern 对比分析

## 结论

SillyTavern 的优势不在“自动写长篇小说”，而在它作为 LLM power-user runtime 的成熟度：提示词可视化编排、世界书条件注入、宏与斜杠命令、扩展系统、模型连接配置、角色卡和预设生态都很完整。

AI Novel Factory 的优势在另一侧：它已经围绕长篇小说生产建立了更专门的自动化流水线，包括项目结构、小说圣经、章节计划、草稿、评审、修订、Canon 更新、RAG、Graph-RAG、任务耗时和成本追踪。它比 SillyTavern 更像“生产工厂”，但比 SillyTavern 缺少“可配置运行时”和“创作者生态”。

后续改进不应把 AI Novel Factory 改成聊天软件。更合适的方向是吸收 SillyTavern 的可配置能力，把它们改造成面向长篇小说生产的 Prompt Runtime、Context Activation、Provider Profile、Operator Automation、Import/Export 和 Extension Host。

## 调研范围

本项目重点参考：

- `README.md`
- `DESIGN.md`
- `docs/specs/2026-06-08-ai-novel-factory-improvement-spec.md`
- `docs/plans/2026-06-08-ai-novel-factory-improvement-plan.md`
- `tauri-app/src/App.tsx`
- `tauri-app/src-tauri/src/lib.rs`
- `tauri-app/src-tauri/src/workflow/chapter_production.rs`
- `tauri-app/src-tauri/src/workflow/writing_context.rs`
- `tauri-app/src-tauri/src/workflow/prompt_rendering.rs`
- `tauri-app/src-tauri/src/workflow/review_agents.rs`
- `tauri-app/src-tauri/src/db/vector_store.rs`
- `tauri-app/src-tauri/src/db/knowledge_graph.rs`
- `tauri-app/src-tauri/src/ai/client.rs`
- `tauri-app/src-tauri/migrations/001_init_sqlite.sql`

SillyTavern 重点参考本地克隆 `.external/SillyTavern` 中的：

- `README.md`
- `src/server-main.js`
- `src/server-startup.js`
- `src/endpoints/worldinfo.js`
- `src/endpoints/vectors.js`
- `src/vectors/embedding.js`
- `src/endpoints/extensions.js`
- `src/endpoints/characters.js`
- `src/character-card-parser.js`
- `public/scripts/PromptManager.js`
- `public/scripts/world-info.js`
- `public/scripts/extensions.js`
- `public/scripts/st-context.js`
- `public/scripts/slash-commands.js`
- `public/scripts/macros/engine/MacroEngine.js`

## 产品定位差异

| 维度 | SillyTavern | AI Novel Factory | 判断 |
| --- | --- | --- | --- |
| 核心定位 | 面向 power user 的 LLM 聊天、角色扮演和提示词运行时 | 面向长篇小说的本地自动生产桌面应用 | 两者不是同类产品，不能按功能数量直接判胜负 |
| 主工作流 | 对话、角色卡、世界书、提示词预设、扩展脚本 | 项目、小说圣经、章节计划、生成、评审、修订、Canon 更新 | 本项目长篇生产链更强 |
| 可配置性 | 很强，提示词、模型、宏、扩展、命令都面向用户开放 | 中等，核心能力强但运行时配置较少 | 本项目需要学习酒馆的 runtime 思路 |
| 自动化深度 | 主要服务交互式聊天和用户脚本 | 内建章节生产、评审、修订和知识更新 | 本项目不应弱化自动生产能力 |
| 生态 | 角色卡、世界书、提示词预设、扩展和插件生态成熟 | 目前更像单体本地应用，互操作弱 | 本项目需要导入导出和扩展机制 |

## 能力矩阵

| 能力 | SillyTavern 做法 | AI Novel Factory 当前状态 | 本项目可学习点 |
| --- | --- | --- | --- |
| 提示词编排 | Prompt Manager 支持 prompt unit、角色、顺序、启用状态、injection position、token 统计和导入导出 | `prompt_rendering.rs` 主要做严格 `{{KEY}}` 替换，prompt registry 偏编译期 | 建立 Prompt Runtime Workbench，让操作者能看见、编辑和预览最终 prompt |
| 上下文激活 | World Info/Lorebook 支持关键词、次级关键词、递归扫描、sticky、cooldown、probability、group、depth、budget | 本项目有结构化 canon、RAG 和 Graph-RAG，但缺少显式的条件激活规则层 | 增加 Canon Activation Rules，把规则触发、RAG、Graph-RAG 纳入同一条可追踪链路 |
| 宏和变量 | MacroEngine 有解析、注册、前后处理和动态宏 | 只有模板键替换，缺少可扩展宏接口 | 先做受控宏，不直接开放任意脚本 |
| 操作者自动化 | slash command 和 STscript 支持命令注册、自动补全、变量、闭包、暂停和中止 | 内建工作流较强，但用户难以组合自己的批处理动作 | 增加安全的 Operator Recipes，再考虑 DSL |
| 扩展系统 | manifest、启用禁用、依赖检查、CSS/JS/i18n 加载、Git 安装更新、生成拦截器和 `getContext()` | 基本没有插件或扩展 host | 先做受限 Extension Host，避免桌面端直接执行不可信代码 |
| 模型连接 | 覆盖大量聊天、补全、图像、embedding 后端和连接预设 | 支持 DeepSeek/OpenAI/OpenAI-compatible/Anthropic/Gemini 等，设置 UI 较轻 | 建立 Provider Capability 和 Profile Manager，表达上下文长度、JSON 能力、embedding 能力、价格和风险 |
| 向量检索 | vectors 扩展支持多 embedding source、chunk 配置、阈值、注入模板、统计、清理 | 本项目已有 SQLite 向量存储、内容哈希去重、RAG 和 Graph-RAG | 本项目检索语义更贴合小说，但配置面和可观测面可继续增强 |
| 角色和世界资料生态 | TavernCard、PNG metadata、JSON/YAML/CharX、校验器、bulk merge | 本项目有更严肃的小说圣经表结构，但导入导出生态弱 | 做项目包、圣经包、Prompt preset 和 Lorebook 互操作 |
| 版本探索 | 聊天 swipes、regenerate、分支式候选输出 | 本项目有章节版本和评审修订，但“候选稿比较”体验不突出 | 增加候选草稿池、对比、选择和保留理由 |
| 可观测性 | 扩展和 UI 中有 token、上下文、状态信息 | 本项目已有 job timeline、耗时、token/cost 和 review dashboard | 本项目这一点已经强，应把 prompt/context 的 trace 也提升到同等水平 |

## AI Novel Factory 已经强于 SillyTavern 的部分

1. 长篇生产流水线更完整。
   本项目围绕章节计划、草稿、评审、修订、Canon 更新和学习写入形成闭环。SillyTavern 更偏实时交互，不能直接替代这种生产流水线。

2. 结构化 Canon 更严肃。
   本项目的 characters、locations、organizations、items、world lore、power systems、canon rules、plot threads、foreshadowing、style guides、timeline 等表更适合长期维护一部长篇小说。SillyTavern 的 world info 更灵活，但结构化程度低。

3. Graph-RAG 是更贴合长篇小说的方向。
   SillyTavern 的向量和世界书很成熟，但本项目把知识图谱关系纳入检索解释，更适合追踪人物、地点、组织、伏笔和设定之间的因果关系。

4. 质量评审和修订链路更专门。
   本项目已有多评审 agent、确定性 canon precheck、review aggregation 和 repair workflow。SillyTavern 更强调让用户在对话中调教输出。

5. 本地工厂运维信息更清晰。
   本项目已有任务阶段、耗时、失败原因、token 和成本记录。SillyTavern 的运行时状态丰富，但不是为“批量生产章节”设计。

## AI Novel Factory 目前的主要缺口

1. Prompt Runtime 太浅。
   当前 prompt 模板渲染的接口接近实现细节：调用者需要知道有哪些键、替换顺序和失败模式。相比 SillyTavern 的 Prompt Manager，本项目缺少可编辑 prompt unit、顺序控制、注入位置、token 预算、dry run 和导入导出。

2. 上下文激活缺少规则层。
   本项目有 RAG 和 Graph-RAG，但操作者还不能像使用世界书一样定义“某些关键词、人物、地点、剧情阶段出现时，固定注入某些资料”。这会让长篇生产过度依赖隐式检索。

3. 宏和变量系统太弱。
   `{{KEY}}` 替换简单可靠，但无法支持日期、章节状态、角色视角、叙事阶段、审稿结果、候选稿编号等动态变量。需要一个受控 Macro module，而不是把复杂字符串逻辑散落到调用处。

4. 没有扩展 seam。
   SillyTavern 有 extension manifest、加载、启用禁用、依赖、hook 和上下文对象。本项目目前更像封闭单体。缺少扩展 seam 会限制导出目标、评审 agent、上下文来源、提示词包和工作流 recipe 的生态化。

5. 操作者无法组合自己的自动化。
   本项目内建 workflow 强，但用户很难定义“生成三个开头候选稿”“只重跑风格评审”“为某章只构建上下文预览”“把世界观资料转成 canon rule”等批处理动作。

6. 互操作和导入导出弱。
   SillyTavern 的角色卡、世界书、预设和扩展有明确文件格式。本项目如果没有项目包、小说圣经包、Prompt preset 包和世界书导入，就很难形成可迁移资产。

7. UI 的局部性不足。
   `tauri-app/src/App.tsx` 体量较大，多个页面、状态和命令调用集中在同一文件。`tauri-app/src-tauri/src/lib.rs` 也聚合了大量 Tauri command。随着 Prompt Runtime 和 Context Activation 增加，继续堆在这些文件会降低局部性。

## 架构深挖机会

以下建议使用“Module、Interface、Implementation、Depth、Seam、Adapter、Leverage、Locality”作为架构词汇。

1. Prompt Runtime Module
   - Files: `tauri-app/src-tauri/src/workflow/prompt_rendering.rs`、`tauri-app/src-tauri/prompts/`、未来的 prompt preset 数据库表和前端 workbench。
   - Problem: 现有 Module 偏浅，Interface 暴露的是模板键和替换规则，Leverage 低。
   - Solution: 把 prompt unit、顺序、角色、注入位置、token 预算、变量解析和 dry run 放进一个更深的 Module。外部调用只提交“章节生产意图”和“上下文包”，由该 Module 返回可审计的 assembled prompt。
   - Benefits: Prompt 变化集中在一个 Implementation 内，调用者和测试只跨一个稳定 Interface，Locality 更好。

2. Context Activation Module
   - Files: `writing_context.rs`、`knowledge_graph.rs`、`vector_store.rs`、未来的 `context_activation.rs`。
   - Problem: RAG、Graph-RAG、canon summary 和固定上下文规则可能分散在多个调用点。
   - Solution: 建立一个 Context Activation Module，统一处理关键词、实体、章节阶段、检索命中、图谱邻域和预算裁剪。
   - Benefits: 上下文选择的解释可以从同一个 Interface 输出，给生成、预览 UI 和测试复用，Leverage 高。

3. Provider Capability Module
   - Files: `ai/client.rs`、`ai/factory.rs`、设置 UI、未来的 provider profile 表。
   - Problem: provider 选择现在主要是连接配置，模型能力、上下文窗口、JSON 能力、embedding 能力和价格没有形成深 Interface。
   - Solution: 用 Provider Capability Module 表达模型能力，再用具体 Adapter 对接 OpenAI-compatible、Anthropic、Gemini、DeepSeek 等。
   - Benefits: 新模型接入不需要把能力判断散落到 workflow，Locality 更好。

4. Extension Host Module
   - Files: 未来的 `extensions/manifest.rs`、`extensions/host.rs`、前端 extension 管理页。
   - Problem: 没有 extension seam，所有新能力都要改核心代码。
   - Solution: 先开放受限 hook，如 `before_context_build`、`after_context_build`、`before_review`、`after_review`、`export_target`。第一阶段优先支持声明式扩展，不执行任意 JS。
   - Benefits: 形成真实 seam 后，Prompt preset、导出目标和评审扩展可以通过 Adapter 接入核心流程。

5. UI Workbench Modules
   - Files: `tauri-app/src/App.tsx` 和未来的 `src/pages/*`、`src/features/*`、typed Tauri client。
   - Problem: 大文件让状态、页面和命令调用混在一起，Implementation 变化影响面大。
   - Solution: 以项目、圣经、章节、Graph、Jobs、Reviews、Prompt Workbench、Provider Profiles 为 Module 拆分页面和状态。
   - Benefits: 每个页面的 Interface 更小，改动和回归测试的 Locality 更好。

## 不建议照搬 SillyTavern 的部分

1. 不要把主体验改成聊天。
   聊天是 SillyTavern 的主工作流，但 AI Novel Factory 的核心价值是长篇生产流水线。可以增加调试式对话面板，但不应替代项目、章节和 Canon workflow。

2. 不要一开始开放任意脚本扩展。
   SillyTavern 的 JS 扩展适合 Web power-user 生态。桌面本地小说工厂如果直接执行未知扩展，会带来文件、网络和隐私风险。更稳妥的顺序是先做声明式规则和受限 hook。

3. 不要让宏语言先于可视化 prompt 预览膨胀。
   如果没有 assembled prompt preview、token budget 和 trace，宏越强，问题越难诊断。

4. 不要复制所有 provider 和格式。
   应优先选择对长篇小说生产有价值的模型能力：长上下文、稳定 JSON、低成本批量生成、embedding、可审计 usage metadata。

## 优先级建议

最高优先级是 Prompt Runtime Workbench 和 Context Activation。它们能直接改善章节生成质量、上下文可解释性和操作者控制感，而且能复用本项目已有的 RAG、Graph-RAG、job metadata 和 review pipeline。

第二优先级是 Provider Capability、Import/Export 和候选稿版本探索。这些能力能提升日常生产效率和资产可迁移性。

Extension Host 和完整 DSL 应放到后面。它们需要稳定 Interface 承接，否则会把核心 workflow 的复杂度暴露给用户和扩展作者。
