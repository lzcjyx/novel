// Fix Build Bible Prompt and Parse Bible Response for proper JSON schema
const fs = require('fs');
const path = require('path');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

const WF_ID = 'CtEvELW9y6XbmbVi';

// Build the bible prompt code as lines array (avoids template literal issues)
const bbpLines = [
  '// ============================================================',
  '// Build the bible generation prompt.',
  '// ============================================================',
  '',
  'const project = $(\"Create Project\").first().json;',
  'const input = $(\"Merge Input Data\").first().json;',
  '',
  'const schemaObj = {',
  '  world_overview: \"世界观概述（500-800中文字）\",',
  '  power_system: {',
  '    name: \"力量体系名称\",',
  '    description: \"力量体系描述\",',
  '    rules: [\"规则示例1\"],',
  '    limitations: [\"限制示例1\"]',
  '  },',
  '  main_plot_threads: [{',
  '    title: \"剧情线标题\",',
  '    description: \"剧情线描述\",',
  '    thread_type: \"main|sub|background\"',
  '  }],',
  '  characters: [{',
  '    name: \"角色名\",',
  '    role: \"protagonist|antagonist|supporting|minor\",',
  '    appearance: \"外貌描述\",',
  '    personality: \"性格描述\",',
  '    speech_style: \"说话风格\",',
  '    motivation: \"核心动机\",',
  '    backstory: \"背景故事\"',
  '  }],',
  '  locations: [{',
  '    name: \"地点名\",',
  '    location_type: \"city|dungeon|wilderness|organization|other\",',
  '    description: \"描述\"',
  '  }],',
  '  organizations: [{',
  '    name: \"组织名\",',
  '    description: \"描述\",',
  '    goals: [\"目标1\"]',
  '  }],',
  '  items: [{',
  '    name: \"道具名\",',
  '    item_type: \"weapon|consumable|quest|key|other\",',
  '    description: \"描述\"',
  '  }],',
  '  style_guide: {',
  '    narrative_perspective: \"叙事视角, 如 first_person|third_person\",',
  '    positive_examples: [\"好的句式示例\"],',
  '    negative_examples: [\"避免的句式示例\"]',
  '  },',
  '  canon_rules: [{',
  '    rule_type: \"hard|soft\",',
  '    rule_text: \"规则内容\",',
  '    rationale: \"规则理由\"',
  '  }],',
  '  chapter_plans: [{',
  '    sequence: 1,',
  '    title: \"章节标题\",',
  '    plot_goals: [\"本章目标\"],',
  '    summary: \"章节概要用中文写（一句话）\",',
  '    target_word_count: 3000,',
  '    arc_stage: \"setup|build|climax|resolution\"',
  '  }]',
  '};',
  '',
  'const prompt = ' + JSON.stringify(
    '你是一位资深小说策划编辑。请为以下新小说项目生成完整的初始圣经。\n\n' +
    '项目信息：\n- 书名：' + '${project.name}' + '\n- 类型：' + '${input.genre}' + '\n- 目标读者：' + '${input.target_audience}' + '\n- 文风基调：' + '${input.tone}' + '\n\n' +
    '请严格按以下 JSON schema 输出（所有 key 必须使用英文，所有内容的 value 用中文写）：\n' + '${JSON.stringify(schemaObj, null, 2)}' + '\n\n' +
    '输出要求：\n' +
    '1. 只输出合法 JSON，不要包在代码块里\n' +
    '2. world_overview 必须 500-800 中文字\n' +
    '3. 至少 5-10 个角色 characters\n' +
    '4. 至少 5 条 canon_rules（包含 hard 和 soft）\n' +
    '5. 至少 3-5 条 main_plot_threads\n' +
    '6. 前 10 章的 chapter_plans，sequence 从 1 到 10\n' +
    '7. 不要包含任何解释、寒暄或 markdown'
  ) + ';',
  '',
  'return [{',
  '  json: {',
  '    project: project,',
  '    prompt: prompt,',
  '    system_prompt: \"你是一位资深小说策划编辑。请严格按提供的 JSON schema 输出。所有 key 必须使用英文，所有内容用中文。不要包含解释或代码块。\",',
  '    schema: schemaObj',
  '  }',
  '}];'
];

// Build Parse Bible Response code
const pbrLines = [
  'const response = $input.first().json;',
  'let rawText = \"\";',
  '',
  '// DeepSeek node outputs standard chat completion format',
  'if (response.choices && response.choices[0]) {',
  '  rawText = response.choices[0].message.content || \"\";',
  '} else if (typeof response === \"string\") {',
  '  rawText = response;',
  '} else {',
  '  rawText = JSON.stringify(response);',
  '}',
  '',
  '// Strip markdown code blocks if present',
  'rawText = rawText.trim();',
  'rawText = rawText.replace(/^```[a-z]*\\s*\\n?/, \"\").replace(/\\n?```$/, \"\");',
  '',
  'let bible;',
  'try {',
  '  bible = JSON.parse(rawText);',
  '} catch (e) {',
  '  throw new Error(\"Failed to parse bible JSON. Raw (first 500): \" + rawText.substring(0, 500));',
  '}',
  '',
  '// Validate required fields',
  'if (!bible.world_overview || !bible.characters || !bible.main_plot_threads) {',
  '  throw new Error(\"Bible missing required fields. Got: \" + Object.keys(bible).join(\", \"));',
  '}',
  '',
  'return [{ json: { project: $(\"Create Project\").first().json, bible: bible } }];'
];

async function main() {
  const resp = await fetch(BASE + '/workflows/' + WF_ID, { headers: HEADERS });
  const wf = await resp.json();

  const bbp = wf.nodes.find(n => n.name === 'Build Bible Prompt');
  if (bbp) {
    bbp.parameters.jsCode = bbpLines.join('\n');
    console.log('Updated: Build Bible Prompt (' + bbpLines.length + ' lines)');
  }

  const pbr = wf.nodes.find(n => n.name === 'Parse Bible Response');
  if (pbr) {
    pbr.parameters.jsCode = pbrLines.join('\n');
    console.log('Updated: Parse Bible Response (' + pbrLines.length + ' lines)');
  }

  if (wf.settings) {
    for (const k of Object.keys(wf.settings)) {
      if (wf.settings[k] === null) delete wf.settings[k];
    }
  }

  const updateResp = await fetch(BASE + '/workflows/' + WF_ID, {
    method: 'PUT',
    headers: HEADERS,
    body: JSON.stringify({
      name: wf.name,
      nodes: wf.nodes,
      connections: wf.connections,
      settings: wf.settings
    })
  });

  if (updateResp.ok) {
    console.log('Workflow updated!');
    fs.writeFileSync(dir + '01_novel_bootstrap.workflow.json', JSON.stringify(wf, null, 2));
    console.log('Local file synced.');
  } else {
    const err = await updateResp.json();
    console.log('FAIL:', JSON.stringify(err).slice(0, 300));
  }
}

main().catch(e => console.error(e));
