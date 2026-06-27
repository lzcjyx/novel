# GitHub 长篇小说 Agent 框架对比分析

## 结论

AI Novel Factory 现在已经不是一个简单的 AI 写作壳子。它的核心优势是本地桌面化、结构化 Canon、章节生产流水线、Graph-RAG、7 Agent 审稿、自动修订、任务可观测性、SQLite 持久化和一批已经落地的运行时 Module，例如 Prompt Runtime、Context Activation、Model Profile、Operator Recipes、Import/Export、Draft Alternatives、Extension Host。

但和 GitHub 上热门长篇创作项目相比，本项目的短板也很明确：它更像“后端生产工厂已经成型，但作者工作台还不够强”。很多 Module 已有实现，却还缺少成熟的作者可见工作台、阶段化导演体验、写法资产系统、事实账本、用户自定义工作流、读者反馈闭环、显式记忆银行和更完整的互操作生态。

下一步不应重复造一个聊天产品，也不应把系统改成完全自治的一键出书机。更正确的方向是保留 AI Novel Factory 的本地生产流水线，把外部项目成熟的“作者控制面”和“运行时可配置性”吸收进来。

## 调研范围

GitHub 搜索关键词包括：

- `AI novel writing`
- `novel writing agent framework`
- `story writing agent`
- `long form writing agent`

GitHub REST API 在读取 README 阶段触发未认证限流，因此仓库元数据主要来自 Search API 首轮结果，README 内容改用 `raw.githubusercontent.com` 读取。星标数为 2026-06-15 调研时搜索结果中的近似值。

纳入重点比较的仓库：

| 仓库 | 星标约数 | 定位 | 调研判断 |
| --- | ---: | --- | --- |
| `ExplosiveCoderflome/AI-Novel-Writing-Assistant` | 1640 | AI Native 长篇小说生产系统 | 最接近本项目的完整竞品 |
| `mind-protocol/terminal-velocity` | 1100 | 10 个 AI agents 完成一部长篇小说的公开项目 | 更像案例，不是通用框架 |
| `iLearn-Lab/NovelClaw` | 320 | 长篇写作工作台、可检查运行、记忆控制 | 运行可观测性和记忆控制值得学习 |
| `Doriandarko/kimi-writer` | 280 | 单文件/CLI 自治写作 agent | 长上下文压缩和恢复机制值得学习 |
| `FlickeringLamp/ai-novelist` | 199 | 类 coding-agent 的 AI 写作桌面工具 | Tool Registry、MCP、skills 和人机协同值得学习 |
| `yuanbw2025/storyforge` | 109 | 本地优先 AI 小说创作工作台 | Prompt 透明化、题材模板、工作流和前端 UX 很强 |
| `MangoLion/plotbunni` | 94 | 小说写作套件和 AI 辅助工作台 | 轻量作者体验、概念库和场景级写作值得学习 |
| `ThomasHoussin/Claude-Book` | 89 | 基于 Claude Code 的多 agent 小说写作框架 | 文件化 bible/state/timeline 契约很清晰 |
| `maosi-wangle/long-web-novel-generator` | 1 | LangGraph/RAG 多 agent 网文生成骨架 | CLI、RAG、Human review skeleton 可参考 |
| `nax-sec/AuthorOS` | 2 | local-first CLI AI author system | 读者反馈、决策、记忆闭环值得关注 |
| `XINGANLIU/web-novel-writing-skill` | 2 | 给 Claude/Codex/Cursor/Gemini 用的网文 skill | 10 阶段流程、7 角色、4 层防幻觉机制可吸收 |

本项目参考文件：

- `README.md`
- `docs/analysis/2026-06-13-sillytavern-ai-novel-factory-comparison.md`
- `docs/plans/2026-06-13-sillytavern-inspired-ai-novel-factory-plan.md`
- `tauri-app/src-tauri/src/workflow/chapter_production.rs`
- `tauri-app/src-tauri/src/workflow/writing_context.rs`
- `tauri-app/src-tauri/src/workflow/prompt_runtime.rs`
- `tauri-app/src-tauri/src/workflow/context_activation.rs`
- `tauri-app/src-tauri/src/workflow/operator_recipes.rs`
- `tauri-app/src-tauri/src/workflow/package_io.rs`
- `tauri-app/src-tauri/src/workflow/lorebook_import.rs`
- `tauri-app/src-tauri/src/extensions/host.rs`
- `tauri-app/src-tauri/src/extensions/manifest.rs`
- `tauri-app/src-tauri/src/ai/provider_capabilities.rs`
- `tauri-app/src-tauri/src/db/prompt_presets.rs`
- `tauri-app/src-tauri/src/db/draft_alternatives.rs`
- `tauri-app/src/App.tsx`
- `tauri-app/src-tauri/tests/*`

## 总体能力矩阵

| 能力 | AI Novel Factory | 外部项目更强处 | 判断 |
| --- | --- | --- | --- |
| 长篇自动生产主链 | 已有章节计划、生成、审稿、修订、Canon 更新、导出 | AI-Novel-Writing-Assistant 的自动导演和整本生产入口更面向新手 | 本项目流水线强，但开书和整本掌控体验弱 |
| 结构化 Canon | 角色、地点、组织、道具、力量体系、规则、伏笔、时间线、图谱 | Claude-Book 的 bible/state/timeline 文件契约更容易人工审计 | 本项目数据库更严肃，人工可读性和状态提交视图不足 |
| Prompt Runtime | 已有 prompt unit、role、order、phase、token estimate、preview trace | StoryForge 的 prompt 可见、可调参、可克隆、few-shot、题材包更成熟 | 本项目 Module 有了，作者工作台还浅 |
| Context Activation | 已有关键词、secondary keywords、章节范围、sticky、cooldown、trace | SillyTavern/StoryForge 的世界书规则和 UI 更成熟 | 本项目方向正确，规则表达能力和可视化不足 |
| RAG/Graph-RAG | SQLite 向量、Graph context、retrieval trace、Graph rerank | NovelClaw 的 memory banks 和 run artifacts 更适合作为可编辑记忆面 | 本项目检索深度强，记忆编辑面弱 |
| 审稿和修订 | 7 Agent 审稿、arbiter、repair loop、review dashboard | Web Novel Writing Skill 的阶段质量门和反 AI 规则库更细 | 本项目工程闭环强，文学质检维度可继续深化 |
| 写法/风格资产 | 有 style guide、learning entries、自我反思学习 | AI-Novel-Writing-Assistant 和 StoryForge 的写法引擎、风格特征池更成熟 | 当前缺少“写法资产”作为一等 Module |
| 模型路由 | 有 provider profiles、workflow validation、cost snapshot | AI-Novel-Writing-Assistant 的任务路由产品化更明显 | 本项目底层有，配置体验和风险提示需要加强 |
| 运行可观测性 | job phase、usage/cost、prompt/context metadata、extension hook trace | NovelClaw 的 run 目录、worker.log、progress.log 和下载 artifacts 更直接 | 本项目数据库审计强，文件级 artifacts 可读性弱 |
| 作者工作台 | Dashboard、Context Preview、Graph、Reviews、Jobs、Knowledge Library | StoryForge、PlotBunni、NovelClaw 的工作台更完整、分区更清晰 | 本项目 UI 仍偏单体，作者控制面不够丰富 |
| 导入导出 | project package、bible package、prompt preset、SillyTavern lorebook | StoryForge 的 JSON/Markdown/TXT/HTML/File System/Gist 导出更多 | 本项目互操作起步好，但 package 覆盖仍不完整 |
| 扩展 | 声明式 extension manifest、hooks、permissions、metadata patch | SillyTavern 的扩展生态和 ai-novelist 的 MCP/skills 更开放 | 本项目安全保守正确，但 hook 实际影响面还弱 |

## AI Novel Factory 的主要优点

### 1. 真正围绕“长篇生产流水线”建模

多数外部项目仍在“写作工作台”或“agent prompt 框架”层面。AI Novel Factory 已经把长篇生产拆成项目、小说圣经、章节计划、写作上下文、草稿、审稿、修订、Canon 更新、学习沉淀、导出和任务追踪。这种主链是本项目最重要的 Leverage。

相关 Module：

- `workflow::chapter_production`
- `workflow::writing_context`
- `workflow::review_agents`
- `workflow::review_arbiter`
- `workflow::canon_updater`
- `workflow::task_transaction`

### 2. 本地桌面和 SQLite 使部署成本低

NovelClaw、AI-Novel-Writing-Assistant、ai-novelist 等项目常见 Express/FastAPI/Electron/Docker/Qdrant/Chroma 组合。AI Novel Factory 的 Tauri + Rust + SQLite 路线更适合本地个人创作者，部署 Interface 更小：安装桌面端、配置 API key、使用本地库。

这对目标用户很有价值，尤其是不会运维 Docker、Qdrant、Python venv 或 Node 服务的作者。

### 3. 结构化 Canon 和 Graph-RAG 比普通世界书更适合长篇

StoryForge、SillyTavern、PlotBunni 的 concept/lorebook/world info 更灵活，但很多是文本条目或前端对象。AI Novel Factory 的结构化表和知识图谱更适合长篇维护：角色状态、地点、组织、道具、规则、剧情线、伏笔、时间线都能被工作流读取。

`writing_context.rs` 还会根据章节计划匹配图谱 seed node，扩展邻居关系，并把图谱命中的 source keys 用于 rerank retrieval。这比单纯向量检索更贴合长篇一致性问题。

### 4. 审稿、修订和任务审计已经工程化

本项目不是简单“生成一章”。生成后会跑审稿 agents、聚合分数、决定 publish_ready/revise/needs_human_review，必要时自动修订并重审。模型 usage、token、cost、prompt/context metadata、learning context、job phase event 都有记录。

这比很多 GitHub 写作工具只提供“AI 写正文”要更接近生产系统。

### 5. 已经把 SillyTavern 式能力落成初版 Module

2026-06-13 的 SillyTavern 对比计划里列出的很多方向，现在代码里已经有初版：

- `prompt_runtime.rs`: prompt unit 编排、phase filtering、trace、token estimate
- `context_activation.rs`: Canon Activation Rules、sticky、cooldown、trace
- `provider_capabilities.rs`: workflow capability warnings
- `operator_recipes.rs`: structured recipes、context preview、draft candidates
- `package_io.rs`: project/bible package
- `lorebook_import.rs`: SillyTavern lorebook 到 context rules
- `extensions/host.rs`: 声明式扩展 hook、permissions、trace
- `draft_alternatives.rs`: 候选稿生成、选择、promotion

这说明本项目的架构方向不是空谈，已经有可测试的 Implementation。

### 6. 测试基线比很多同类项目更扎实

`tauri-app/src-tauri/tests` 里已经有 prompt runtime、context activation、provider capability、operator recipes、import/export、extension host、draft alternatives、Graph-RAG、generation job observability、task transaction、core writing loop 等测试。

这对 AI 应用很关键。外部很多项目 README 很亮，但测试和故障恢复不一定有同等成熟度。

## AI Novel Factory 的主要缺点

### 1. 新手开书体验弱于 AI-Novel-Writing-Assistant

AI-Novel-Writing-Assistant 强调“自动导演开书”：从一句灵感开始，生成多套整本方向、标题组、书级 framing、故事宏观规划、本书世界、角色准备、卷战略、节奏拆章，并设置检查点、继续推进和换模型重试。

AI Novel Factory 虽然有 novel bootstrap 和 weekly planner，但产品体验仍更像“先填项目，再生成圣经，再生成计划，再写章”。对新手来说，这比“导演模式”需要更多写作判断。

需要学习：

- 一句灵感到整本方向候选
- 书级 positioning/framing
- 前 30 章承诺
- 自动推进到可开写的检查点
- 多方案选择、定向重做和局部修订

### 2. Prompt Runtime Module 还不够深

本项目已有 `PromptUnit`、role、order、enabled、generation_phase 和 preview，但 Interface 仍偏底层。相比 StoryForge，缺少：

- 每个 AI 按钮背后的 prompt 全量可视编辑
- prompt 参数滑块和临时 override
- few-shot 好例/反例管理
- 题材包热切换
- 用户克隆内置模板为自定义版本
- prompt 版本历史和回滚
- prompt A/B 试写

当前 Dashboard 能看到 `system_prompt` 和 `user_prompt` preview，这很好，但它还不是完整的 Prompt Workbench。

### 3. Context Activation 有基础规则，但表达能力和 UI 不够

`context_activation.rs` 目前主要是对章节计划和 operator controls 做字符串包含匹配，支持 primary/secondary keyword、chapter range、sticky、cooldown 和 token budget。它还缺少：

- entity_refs 的真实参与
- 分组互斥、probability、depth、递归扫描
- 手动 pin/unpin
- 规则命中模拟器
- 预算竞争可视化
- 规则和 RAG/Graph-RAG 的统一排序解释
- 用户可编辑的世界书式工作台

这个 Module 已经有 Leverage，但 Interface 仍会让高级用户觉得控制力不足。

### 4. 写法引擎不如 AI-Novel-Writing-Assistant 和 StoryForge

本项目有 style guide、learned patterns、自我反思学习和 style reviewer，但“写法资产”还不是一等 Module。外部项目已经把写法拆成可保存、可绑定、可试写、可提取、可组合的资产。

当前缺口：

- 从范文提取风格特征后形成可编辑特征池
- 写法规则编译结果可预览
- 写法资产绑定到项目、卷、章节、角色 POV
- 反 AI 规则库和高频模板句检测
- 风格漂移检测
- 写法对生成结果的贡献 trace

### 5. 事实账本和硬设定继承不足

AI-Novel-Writing-Assistant README 明确提到“章节定稿时自动抽取正文中的关键硬事实并写入事实账本，下一章生成时读取真实前文”。AI Novel Factory 有 Canon 更新、timeline、character states 和 knowledge graph，但还缺少一个明确的 Hard Fact Ledger Module。

长篇中最容易出错的不是“大设定”，而是金额、票号、数量、交易性质、称谓、伤势、物品归属、地点距离、时间间隔等硬事实。它们不一定适合都写进 world lore，但需要被锁定和检索。

### 6. Operator Recipes 还像后端骨架，不像作者自动化

`operator_recipes.rs` 已有结构化 action，能构建 context preview、生成候选稿、支持取消、记录 job metadata。但当前很多 action 仍是 “queued” 或测试用确定性实现，缺少完整的用户自定义 recipe UI。

缺口：

- 用户创建/编辑 recipe
- recipe 参数表单
- step input/output 显示
- 支持只重跑某个审稿 agent 后立即比较差异
- 支持“生成三版开头并评审排序”
- 支持“只构建 prompt/context，不调用模型”
- 支持 recipe 模板导入导出

### 7. Extension Host 安全保守，但实际影响力弱

本项目的声明式 Extension Host 很谨慎，这是优点。它禁止 enabled_by_default，限定 permissions/hooks/package_kinds，并把 hook trace 写入 job metadata。

但当前 hook 的能力主要是 metadata_patch，很多 hook output 在 chapter production 中只是记录，并没有真正改变 context package、prompt unit、review rubric 或 export target。也就是说 Seam 已存在，但 Adapter 的真实 Leverage 还不够。

短期应继续保持“不执行任意 JS”的原则，但要让声明式扩展真正能贡献：

- prompt packs
- context rule packs
- review rubrics
- recipe packs
- export templates

### 8. Import/Export package 覆盖不完整

`package_io.rs` 已经能导出 project、chapter plan、chapter、version、context rules、prompt presets、model profiles、draft candidates、extension packages。但 `insert_bible_rows` 当前只导入 characters 和 world_lore。

这会导致 novel bible package 的 round trip 不完整：locations、organizations、items、magic systems、canon rules、plot threads、foreshadowing、timeline events、style guides 等结构化资产可能不能完整导入。相比 StoryForge 的完整 JSON 备份和 Claude-Book 的文件化 bible，这一点需要补齐。

### 9. UI Module Locality 不足

`tauri-app/src/App.tsx` 聚合了导航、Dashboard、Context Preview、Reviews、Jobs、Graph、Settings、Knowledge Library 等大量状态和 UI。随着 Prompt Workbench、Context Rules、Model Profiles、Recipes、Import/Export、Extensions 继续增加，单文件 Implementation 会让变更局部性变差。

这不是简单代码洁癖。作者工作台会继续扩张，如果 UI 仍聚合在大文件，后续 prompt/context/provider 的产品化速度会越来越慢。

### 10. 人机协同和读者反馈闭环不如 AuthorOS

AuthorOS 强调 reader feedback、preview revisions、approve changes、decide、memory update。AI Novel Factory 有人工审核和 review dashboard，但还缺少一个清晰的“反馈进入修订候选，不直接污染 Canon”的决策链。

对长篇连载，这很重要：读者反馈、编辑意见、作者临时想法都应该先进入 candidate/decision，再由操作者批准写入正文或 Canon。

## 各框架具体应学习什么

### AI-Novel-Writing-Assistant

应学习：

- 自动导演开书，把一句灵感转成多套整本方案和书级 framing。
- Creative Hub，把对话、规划、工具执行、审批和状态卡片收在统一中枢。
- 写法引擎，把风格从 prompt 文本升级为可保存、可绑定、可提取、可试写的资产。
- 事实账本，专门处理跨章硬事实继承。
- 模型路由，把规划、正文、审阅、修复、embedding 分开配置。

不应照搬：

- 不要引入过重的 Web/Server/Prisma/Qdrant 依赖栈，本项目本地桌面轻部署是优势。
- 不要把所有流程都做成自动推进，AI Novel Factory 仍应保留清晰的人工审核和可回滚状态。

### StoryForge

应学习：

- Prompt 全透明：每个按钮背后的 system/user template 都可见。
- Prompt 可编辑：参数、override、克隆、保存、few-shot、反例。
- 题材包和风格包：历史、仙侠、言情、现实、悬疑等可热切换。
- 三层记忆系统的产品表达：Working/Episodic/Semantic Memory。
- 版本历史、导入导出、HTML/TXT/Markdown/JSON 多格式输出。
- 作者友好的工作台布局。

不应照搬：

- 不要把核心数据只放到浏览器 IndexedDB。AI Novel Factory 的 SQLite 后端更适合桌面长期项目。
- 不要把 prompt 灵活性放到可靠性之前。先做 preview、trace、tests，再开放更强宏。

### NovelClaw

应学习：

- Run artifacts：每次运行都有 status、worker log、progress log、chapter files、download。
- Memory banks：记忆不只是内部 context，而是可编辑、可检查的作者资产。
- Manuscript/storyboard/world/character/style 多个可见 surface。
- Session continuation，让作者能从同一会话继续，而不是每次从按钮开始。

不应照搬：

- 不需要 Portal/MultiAgent/NovelClaw 多服务拆分。本项目可以在单桌面应用内实现同等工作台。

### Claude-Book

应学习：

- `bible/` 永久只读，`state/` 每章版本化，`timeline/history.md` append-only 的清晰契约。
- planner/writer/style-linter/character-reviewer/continuity-reviewer/state-updater 的 agent 分工。
- 每章生成后 state commit，再进入下一章。
- Perplexity improver 或类似“反 AI 可预测句”检查。
- EPUB/MOBI/AZW3 导出链路。

不应照搬：

- 不要完全文件化取代 SQLite。更适合做“可导出的审计镜像”或“项目包 human-readable sidecar”。

### Web Novel Writing Skill

应学习：

- 10 阶段流程：灵感、世界观、人物、全局大纲、分卷、章节细纲、正文、质量审查、记忆落盘、修订。
- 7 种专家角色的清晰职责。
- 4 层防幻觉机制：写前约束、写中引导、写后审查、长期记忆。
- 黄金三章策略、3:1 节奏法则、章节提交机制。
- 玄幻、都市、言情、科幻等中文网文 genre guides。

不应照搬：

- 它本质是 skill/prompt 框架，不是完整应用。适合转化为 Prompt Packs、Review Rubrics、Recipe Packs。

### AuthorOS

应学习：

- CLI core + web front desk 的双层设计。
- reader feedback 进入 preview revision，再 approve/apply。
- decide 和 memory update 作为独立阶段。
- 多书 bookshelf 和切换状态。
- 测试驱动的命令层。

不应照搬：

- 本项目不必转成 CLI-first，但可以增加脚本化命令或 headless workflow，服务批量生产和自动测试。

### kimi-writer

应学习：

- 长上下文 token monitoring。
- 自动 context compression。
- 中断后 recovery summary。
- 实时 streaming 和工具调用进度。

不应照搬：

- 不要让一个自治 agent 自由写文件替代结构化流水线。它适合补强长任务鲁棒性，不适合作为核心架构。

### ai-novelist

应学习：

- Tool Registry 和人在回路批准。
- Agentic RAG，让 AI 自主决定何时查询知识库。
- MCP client 和 skills 生态入口。
- 存档点、diff、回档。

不应照搬：

- 终端/命令执行/MCP 权限要非常谨慎。小说项目会含大量私密设定和 API key，本项目应默认最小权限。

### PlotBunni

应学习：

- 轻量概念库、场景级写作、计划视图、正文视图之间的顺滑切换。
- AI 对单场景写作的上下文组织。
- Prompt Manager 的简洁体验。
- 作者友好而非工程师友好的 UI。

不应照搬：

- 它更像写作套件，不是自动生产工厂。本项目应学习体验，不降低自动化深度。

### terminal-velocity

应学习：

- agent 角色公开化。
- 创作过程记录和可复盘。
- 质量、原创性、冗余、集成等审查角色。
- 完整 manuscript 和开发过程透明化。

不应照搬：

- 它是一次自治创作案例，不是通用产品架构。不要把“100% AI-generated”作为本项目目标。

## 推荐优先级

### P0: 补齐当前 Module 的产品化缺口

1. Prompt Workbench
   - 把当前 `prompt_runtime.rs` 和 `prompt_presets.rs` 做成完整 UI。
   - 支持 prompt unit 编辑、排序、禁用、phase、preview、token estimate、导入导出。
   - 增加参数、few-shot、题材包前，先保证 dry run 和 trace 稳定。

2. Context Rules Workbench
   - 把 `context_activation.rs` 做成规则编辑、命中模拟、trace 预览。
   - 让 entity_refs 真正参与激活。
   - 在 UI 中显示 rule hit、vector score、graph path、manual pin。

3. Model Profile Manager
   - 把 `provider_capabilities.rs` 的 warnings 产品化。
   - 按 draft/review/repair/embedding/summarization/Graph-RAG 配置 profile。
   - 在 job metadata 里继续保留 profile snapshot。

### P1: 加强长篇“开书到整本掌控”

1. Director Mode
   - 从一句灵感生成 2 到 3 套整本方向候选。
   - 形成书级 positioning、目标读者、前 30 章承诺、标题组。
   - 设置人工检查点和定向重做。

2. Volume Strategy 和 Golden Three Chapters
   - 增加卷战略、卷骨架、节奏段。
   - 单独强化前 3 章：钩子、爽点、角色吸引力、冲突承诺。

3. Hard Fact Ledger
   - 从定稿章节抽取硬事实。
   - 生成下一章时强制注入相关事实。
   - 对硬事实冲突做阻断式审查。

### P2: 强化文学质量和写法资产

1. Style Asset Module
   - 范文导入和写法特征提取。
   - 风格特征池、启用/禁用、绑定范围。
   - 生成前 preview 写法规则编译结果。

2. Anti-AI Pattern Gate
   - 高频套话、句式模板、低信息密度、过度解释、情绪直白化检查。
   - 可先做 rule-based，再接模型审查。

3. Genre Packs
   - 玄幻/都市/言情/科幻/悬疑/历史。
   - 每个 pack 包含 prompt units、context rules、review rubrics、operator recipes。

### P3: 提升作者控制面和 UI Locality

1. 拆分 `App.tsx`
   - Projects、Dashboard、Bible、Graph、Jobs、Reviews、Knowledge、Prompt Workbench、Context Rules、Model Profiles、Recipes、Import/Export、Extensions 各自成 UI Module。

2. 增加 typed Tauri client
   - 避免 command name 和 payload shape 散在页面中。

3. Memory Banks
   - 把 Canon、Hard Facts、Character State、Timeline、Learning Entries 做成可编辑记忆面。

### P4: 扩展和互操作

1. 补完整 Project/Bible package round trip
   - 导入 locations、organizations、items、magic systems、canon rules、plot threads、foreshadowing、timeline、style guides。

2. 让 Extension Host 真正影响 workflow
   - 声明式 prompt pack、context rule pack、review rubric、recipe pack、export template。
   - 保持默认禁用和显式权限。

3. 增加 human-readable audit export
   - 导出 `bible/`、`state/chapter-NN/`、`timeline/history.md` 风格 sidecar，学习 Claude-Book 的可读契约。

### P5: 长任务鲁棒性

1. Context compression
   - 长会话自动压缩成可恢复 summary。

2. Recovery mode
   - 中断后从 job snapshot、context hash、prompt hash、latest artifact 恢复。

3. Run artifacts
   - 可选把每次 job 的 prompt、context、logs、outputs 导出到本地目录，学习 NovelClaw 的 inspectable runs。

## 架构深挖机会

以下建议使用 Module、Interface、Implementation、Depth、Seam、Adapter、Leverage、Locality 术语。

### 1. Prompt Runtime Module 需要更深

Files:

- `tauri-app/src-tauri/src/workflow/prompt_runtime.rs`
- `tauri-app/src-tauri/src/db/prompt_presets.rs`
- `tauri-app/src/App.tsx`

Problem:

当前 Interface 已经比简单模板替换更好，但仍暴露 prompt unit 细节，作者只能 preview，不能完整操作。Deletion test 显示，如果删除这个 Module，复杂度会回到调用处，所以它值得继续加深。

Solution:

把 prompt unit 管理、参数、few-shot、版本、dry run、token budget、trace 集中在 Prompt Runtime Module 后面。调用方只提交“生成意图”和“上下文包”，返回 assembled prompt 和可审计 trace。

Benefits:

Leverage 提升，所有生成、审稿、修订、recipe、extension 都复用同一 Interface。Locality 改善，prompt 问题集中在一个 Implementation 和一组测试中。

### 2. Context Activation Module 需要统一 RAG、Graph-RAG 和 rules

Files:

- `tauri-app/src-tauri/src/workflow/context_activation.rs`
- `tauri-app/src-tauri/src/workflow/writing_context.rs`
- `tauri-app/src-tauri/src/db/context_rules.rs`
- `tauri-app/src-tauri/src/db/vector_store.rs`
- `tauri-app/src-tauri/src/db/knowledge_graph.rs`

Problem:

规则命中、向量检索、图谱邻居和学习条目目前在 `WritingContextPackage` 中并列，但排序、预算竞争和 UI 解释还没有统一的深 Interface。

Solution:

建立一个 Context Package Builder Module，让所有 context source 以统一 source record 进入排序、预算、trace。RAG、Graph-RAG、rules、manual pins、learning entries 都是 Adapter。

Benefits:

上下文选择的 Locality 更强，调试和测试都通过同一个 Interface。

### 3. Hard Fact Ledger 应成为独立 Module

Files:

- 未来 `workflow/hard_fact_ledger.rs`
- 未来 `db/hard_facts.rs`
- `workflow/canon_updater.rs`
- `workflow/chapter_production.rs`

Problem:

Canon 适合大设定，timeline 适合事件，character state 适合人物状态。但金额、编号、数量、物品归属等硬事实没有专门 Module，容易散落在摘要或 world lore。

Solution:

定稿后抽取 hard facts，生成前按章节计划和检索命中注入，审稿时做硬事实冲突检查。

Benefits:

长篇一致性的 Leverage 很高，尤其是悬疑、商战、权谋、科幻和历史题材。

### 4. Extension Host 的 Seam 已有，但 Adapter 不够真实

Files:

- `tauri-app/src-tauri/src/extensions/host.rs`
- `tauri-app/src-tauri/src/extensions/manifest.rs`
- `tauri-app/src-tauri/src/workflow/chapter_production.rs`

Problem:

当前 hook trace 能记录，但 metadata patch 很少真正改变核心 workflow 输出。一个 Adapter 只是 hypothetical seam，多个真实 Adapter 才能证明 seam 成立。

Solution:

先做非代码扩展 Adapter：prompt pack、context rule pack、review rubric、recipe pack、export template。每种都必须能改变明确的 workflow input，并写 trace。

Benefits:

扩展生态的 Leverage 增加，同时避免任意代码执行风险。

## 最短可执行路线

建议把下一阶段命名为“Author Control Runtime”：

1. Prompt Workbench
2. Context Rules Workbench
3. Model Profile Manager
4. Director Mode MVP
5. Hard Fact Ledger MVP
6. Style Asset MVP
7. UI Module split for the new workbenches

这样做的原因是：这些能力直接影响生成质量、可控性和用户留存，并且能复用本项目已经存在的 Rust Module 和测试。Extension Host、MCP、完整 DSL 可以后移，等 prompt/context/provider/recipe 的 Interface 更稳定后再开放。
