# 审稿 Agent 配置汇总

本文档汇总所有 8 个审稿 Agent 的 system prompt、JSON schema、评分规则和 blocking issue 判定。

详细 prompt 模版请参见各 Agent 独立文件：
- [continuity_reviewer.md](continuity_reviewer.md)
- [character_reviewer.md](character_reviewer.md)
- [plot_logic_reviewer.md](plot_logic_reviewer.md)
- [pacing_reviewer.md](pacing_reviewer.md)
- [style_reviewer.md](style_reviewer.md)
- [safety_reviewer.md](safety_reviewer.md)
- [publication_reviewer.md](publication_reviewer.md)
- [review_arbiter.md](review_arbiter.md)

---

## A. continuity_reviewer — 连续性审查

**检查范围**：时间线、地点、伤势、道具、人物关系、伏笔、世界观设定。

**Score 规则**：
- 90-100：无明显连续性问题
- 75-89：有小问题，不影响发布
- 60-74：有明显矛盾，需要修订
- 0-59：严重破坏 canon，禁止发布

**Blocking issue 判定**：
- 违反 hard canon
- 关键人物状态矛盾
- 关键时间线矛盾
- 擅自改变主线设定
- 擅自回收或否定重要伏笔

**JSON Schema**：
```json
{
  "agent_name": "continuity_reviewer",
  "score": 0,
  "pass": true,
  "blocking_issues": [
    {
      "id": "C001",
      "severity": "high",
      "issue": "string",
      "evidence": "string",
      "canon_reference": "string",
      "recommendation": "string"
    }
  ],
  "minor_issues": [],
  "recommendations": [],
  "canon_update_suggestions": []
}
```

---

## B. character_reviewer — 人物一致性审查

**检查范围**：人物性格、口吻、动机、成长线。

**Blocking issue 判定**：
- 主角做出与核心设定完全相反的选择且无铺垫
- 关键角色关系突变
- 角色已知信息与行为严重矛盾
- 重要角色口吻明显错乱

**JSON Schema**：
```json
{
  "agent_name": "character_reviewer",
  "score": 0,
  "pass": true,
  "blocking_issues": [
    {
      "id": "CH001",
      "character": "string",
      "issue": "string",
      "evidence": "string",
      "expected_behavior": "string",
      "recommendation": "string"
    }
  ],
  "minor_issues": [],
  "recommendations": [],
  "dialogue_fixes": [
    {
      "original": "string",
      "suggested": "string",
      "reason": "string"
    }
  ]
}
```

---

## C. plot_logic_reviewer — 情节逻辑审查

**检查范围**：情节因果、冲突推进、信息披露、转折合理性。

**Blocking issue 判定**：
- 本章核心剧情目标未完成
- 情节转折完全无因果
- 用机械降神解决主要冲突
- 关键剧情与主线方向冲突

**JSON Schema**：
```json
{
  "agent_name": "plot_logic_reviewer",
  "score": 0,
  "pass": true,
  "blocking_issues": [
    {
      "id": "P001",
      "issue": "string",
      "evidence": "string",
      "impact": "string",
      "recommendation": "string"
    }
  ],
  "minor_issues": [],
  "recommendations": [],
  "plot_goal_completion": [
    {
      "goal": "string",
      "completed": true,
      "evidence": "string"
    }
  ]
}
```

---

## D. pacing_reviewer — 节奏/爽点审查

**检查范围**：网文章节节奏、钩子、爽点、情绪曲线。

**Blocking issue 判定**：
- 本章没有任何实质冲突或推进
- 结尾完全没有继续阅读动力
- 大量说明文字导致章节不可读

**JSON Schema**：
```json
{
  "agent_name": "pacing_reviewer",
  "score": 0,
  "pass": true,
  "hook_score": 0,
  "conflict_score": 0,
  "emotional_curve_score": 0,
  "blocking_issues": [],
  "minor_issues": [],
  "recommendations": [
    {
      "type": "opening | conflict | payoff | hook | paragraph | exposition",
      "issue": "string",
      "recommendation": "string"
    }
  ],
  "suggested_ending_hook": "string"
}
```

---

## E. style_reviewer — 文风润色审查

**检查范围**：文风、语言质量、AI 味、错别字、病句。

**Blocking issue 判定**：
- 文风完全偏离项目定位
- 语言质量低到影响发布
- 大量重复、错乱或不可读文本

**JSON Schema**：
```json
{
  "agent_name": "style_reviewer",
  "score": 0,
  "pass": true,
  "blocking_issues": [],
  "minor_issues": [
    {
      "id": "S001",
      "issue": "string",
      "evidence": "string",
      "recommendation": "string"
    }
  ],
  "line_edits": [
    {
      "original": "string",
      "suggested": "string",
      "reason": "string"
    }
  ],
  "global_recommendations": []
}
```

---

## F. safety_reviewer — 安全/版权审查

**检查范围**：公开发布风险、原创性风险、安全风险、密钥泄露。

**Blocking issue 判定**：
- 出现密钥、密码、内部配置
- 出现 Neon connection string、API key、writer-service token
- 明显要求复刻具体受版权保护文本
- 存在高风险违法或平台禁止内容
- 存在真实个人隐私泄露

> **安全审查是最严格的**。只要发现密钥或内部配置泄露，必须 blocking，且 decision 必须是 `needs_human_review`。

**JSON Schema**：
```json
{
  "agent_name": "safety_reviewer",
  "score": 0,
  "pass": true,
  "blocking_issues": [
    {
      "id": "SAFE001",
      "risk_type": "policy | copyright | privacy | secret | platform",
      "issue": "string",
      "evidence": "string",
      "recommendation": "string"
    }
  ],
  "minor_issues": [],
  "recommendations": []
}
```

---

## G. publication_reviewer — 博客发布审查

**检查范围**：Markdown 格式、标题、摘要、标签、发布字段合规。

**Blocking issue 判定**：
- Markdown 严重损坏
- 含有内部 prompt、审稿意见、系统信息
- 标题或正文为空

**JSON Schema**：
```json
{
  "agent_name": "publication_reviewer",
  "score": 0,
  "pass": true,
  "blocking_issues": [],
  "minor_issues": [],
  "blog_metadata": {
    "title": "string",
    "slug": "string",
    "excerpt": "string",
    "tags": ["string"],
    "category": "string",
    "seo_description": "string",
    "status_recommendation": "draft | publish"
  },
  "recommendations": []
}
```

---

## H. review_arbiter — 审稿裁决

**职责**：汇总所有审稿 Agent 输出，决定章节下一步状态。

**决策规则**（按优先级）：
1. `safety_reviewer.pass === false` → decision = `needs_human_review`
2. 任一 Agent 有 `blocking_issues` → decision = `revise`
3. `average_score >= quality_threshold` 且无 blocking → decision = `publish_ready`
4. `average_score < quality_threshold` → decision = `revise`
5. `revise_count >= 2` 仍未通过 → decision = `needs_human_review`
6. `publication_reviewer` 标记格式严重错误 → decision = `revise`
7. 不得忽略任何 high severity 问题

**JSON Schema**：
```json
{
  "agent_name": "review_arbiter",
  "average_score": 0,
  "final_score": 0,
  "decision": "publish_ready | revise | needs_human_review | stop",
  "blocking_issues": [],
  "must_fix": [],
  "minor_issues": [],
  "summary": "string",
  "publish_allowed": false
}
```
