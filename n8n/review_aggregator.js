// ===================================================================
// n8n Code Node — Review Aggregator
// 用于聚合 7 个并行审稿 Agent 的结果并输出决策。
//
// 使用方式：
//   1. 在 n8n 中创建 Code 节点。
//   2. 将 7 个审稿 Agent 的输出连接到该节点的输入。
//   3. 复制下面的代码粘贴到 Code 节点。
//   4. 下游节点引用 $json.decision、$json.final_score 等字段。
//
// 输入：$input.all() — 每个 item 是一个审稿 Agent 的返回。
//   支持三种输入格式：
//     a. OpenAI chat/completions response（含 choices[0].message.content）
//     b. 已解析的 review JSON（含 agent_name 字段）
//     c. HTTP Request 节点返回的 { data: { agent_name, ... } }
//
// 输出：单一 item，包含：
//   - decision: publish_ready | revise | needs_human_review
//   - average_score, final_score
//   - must_fix (所有 blocking_issues 合并)
//   - minor_issues (所有 minor_issues 合并)
//   - recommendations (按 agent_name 分组)
//   - publish_allowed (boolean)
// ===================================================================

const allReviews = $input.all();
const reviewData = [];

// ── 1. 解析每个 Agent 的输出 ──
for (const item of allReviews) {
  try {
    const j = item.json;

    // OpenAI chat/completions 格式
    if (j.choices?.[0]?.message?.content) {
      const parsed = JSON.parse(j.choices[0].message.content);
      reviewData.push(parsed);
    }
    // 已解析的 review JSON
    else if (j.agent_name) {
      reviewData.push(j);
    }
    // HTTP Request 返回的 { data: { ... } }
    else if (j.data?.agent_name) {
      reviewData.push(j.data);
    }
    // 无法识别
    else {
      reviewData.push({
        agent_name: 'unknown',
        score: 0,
        pass: false,
        blocking_issues: [{
          id: 'PARSE_ERR',
          severity: 'high',
          issue: 'Unable to parse review output'
        }],
        minor_issues: [],
        recommendations: []
      });
    }
  } catch (e) {
    reviewData.push({
      agent_name: 'parse_error',
      score: 0,
      pass: false,
      blocking_issues: [{
        id: 'PARSE_ERR',
        severity: 'high',
        issue: 'Failed to parse review: ' + String(e.message)
      }],
      minor_issues: [],
      recommendations: []
    });
  }
}

// ── 2. 定位特殊 Agent ──
const safetyReview = reviewData.find(r => r.agent_name === 'safety_reviewer');
const publicationReview = reviewData.find(r => r.agent_name === 'publication_reviewer');

// ── 3. 计算 average_score ──
const scores = reviewData
  .map(r => r.score)
  .filter(s => typeof s === 'number' && !isNaN(s));

const averageScore = scores.length > 0
  ? Math.round((scores.reduce((a, b) => a + b, 0) / scores.length) * 100) / 100
  : 0;

// ── 4. 收集所有 issues ──
const allBlocking = [];
const allMinor = [];
const recommendationsByAgent = {};

for (const r of reviewData) {
  (r.blocking_issues || []).forEach(issue => {
    allBlocking.push({ ...issue, _agent: r.agent_name });
  });
  (r.minor_issues || []).forEach(issue => {
    allMinor.push({ ...issue, _agent: r.agent_name });
  });
  if (r.recommendations && r.recommendations.length > 0) {
    recommendationsByAgent[r.agent_name] = r.recommendations;
  }
}

// ── 5. 决策逻辑 ──
const hasBlocking = allBlocking.length > 0;
const safetyFailed = safetyReview && safetyReview.pass === false;

// 从上下文获取阈值和修订次数
// 方式 A：从上游 Build Writing Brief 节点获取
// 方式 B：从当前 workflow 的 staticData 或 input 获取
const briefNode = $('Build Writing Brief').first();
const reviseCount =
  briefNode?.json?.revise_count ??
  $input.first().json.revise_count ??
  0;
const maxRevise =
  briefNode?.json?.max_revise_count ??
  $input.first().json.max_revise_count ??
  2;
const qualityThreshold =
  briefNode?.json?.quality_threshold ??
  $input.first().json.quality_threshold ??
  85;

let decision;
if (safetyFailed) {
  // 规则 1：safety 不通过 → 强制人工审核
  decision = 'needs_human_review';
} else if (reviseCount >= maxRevise && hasBlocking) {
  // 规则 5：修订次数耗尽仍未通过 → 人工审核
  decision = 'needs_human_review';
} else if (hasBlocking) {
  // 规则 2：有 blocking issues → 修订
  decision = 'revise';
} else if (averageScore >= qualityThreshold) {
  // 规则 3：分数达标且无 blocking → 发布就绪
  decision = 'publish_ready';
} else {
  // 规则 4：分数不达标 → 修订
  decision = 'revise';
}

// ── 6. 计算 final_score（带钳制） ──
let finalScore;
if (safetyFailed) {
  finalScore = Math.min(averageScore, 40);
} else if (hasBlocking) {
  finalScore = Math.min(averageScore, 74);
} else {
  finalScore = averageScore;
}
finalScore = Math.round(finalScore * 100) / 100;

// ── 7. 发布允许判定 ──
const publishAllowed = decision === 'publish_ready';

// ── 8. 构建输出 ──
return [{
  json: {
    // 分数
    average_score: averageScore,
    final_score: finalScore,

    // 决策
    decision: decision,
    publish_allowed: publishAllowed,

    // 状态标记
    safety_pass: !safetyFailed,
    has_blocking: hasBlocking,
    all_pass: reviewData.every(r => r.pass !== false),

    // 合并后的 issues
    blocking_issues: allBlocking,
    must_fix: allBlocking.filter(
      i => i.severity === 'high' || safetyFailed
    ),
    minor_issues: allMinor,

    // 按 Agent 分组的建议
    recommendations: recommendationsByAgent,

    // 透传原始数据（用于写入 agent_reviews 表）
    reviews: reviewData,

    // 汇总信息
    agent_count: reviewData.length,
    blocking_count: allBlocking.length,
    minor_count: allMinor.length,
    safety_passed: !safetyFailed
  }
}];
