你是一位资深的中文网络小说世界观架构师。

根据下面的小说项目描述，生成一套完整的小说圣经设定。全文必须使用中文。

小说项目信息：
{{PROJECT_INPUT_JSON}}

请严格按以下 JSON schema 输出（只输出 JSON，不要其他文字）：

{
  "world_overview": "200-500字的世界背景概述，包括时代、社会结构、核心冲突",
  "power_system": {
    "name": "力量/修炼体系名称",
    "description": "体系描述",
    "rules": "核心规则",
    "limitations": "限制与代价",
    "progression": "进阶路线"
  },
  "main_plot_threads": [
    {"name": "剧情线名称", "description": "描述", "priority": 3}
  ],
  "characters": [
    {"name": "角色名（中文）", "role": "主角/反派/配角", "personality": "性格特征", "motivation": "动机", "speech_style": "说话风格", "appearance": "外貌描写", "backstory": "背景故事"}
  ],
  "locations": [
    {"name": "地点名（中文）", "type": "类型", "description": "描述"}
  ],
  "organizations": [
    {"name": "组织名（中文）", "description": "描述", "goals": "目标"}
  ],
  "items": [
    {"name": "物品名（中文）", "description": "描述", "abilities": "特殊能力", "limitations": "限制"}
  ],
  "style_guide": {
    "narrative_perspective": "叙事视角（如第三人称）",
    "tense": "时态（如过去时）",
    "tone": "文风基调（如热血/悬疑/轻松）",
    "forbidden_phrases": ["禁止使用的短语"],
    "preferred_techniques": ["推荐使用的技巧"]
  },
  "canon_rules": [
    {"rule_type": "规则类型", "rule_text": "具体规则", "severity": "hard或soft"}
  ],
  "chapter_plans": [
    {"title": "章节标题（中文）", "outline": "2-3句大纲", "target_word_count": 3500}
  ]
}

要求：
1. 全部内容必须是中文。
2. 至少生成 6 个有深度的角色（至少 2 个反派、2 个配角）。
3. 至少 4 个地点。
4. 至少 2 个组织/势力。
5. 3-5 条剧情主线。
6. 5-10 条硬性世界规则。
7. chapter_plans 必须是 10 章，但它们是长篇小说的 first 10 immediate chapter plans，只覆盖 opening movement（开篇推进），不是整本书大纲压缩版。
8. 角色名、地名、组织名必须是原创中文名。
9. 力量体系必须有明确的规则、限制和代价。
10. 严禁输出除 JSON 以外的任何文字。
11. 如果 target_total_words 是几十万字级别，这 10 章只应覆盖约 2-5% 的长篇进度：建立人物、世界规则、第一层冲突、代价、钩子和短期目标。
12. 这 10 章 must not resolve 核心矛盾、最终反派、终局谜底、最终感情归宿、力量体系终点或主角最终胜利。
13. 长线高潮、反转和终局应写入 main_plot_threads 的长期方向；chapter_plans 只写接下来可执行的近期开篇节拍。
