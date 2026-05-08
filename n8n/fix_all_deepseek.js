// Fix all DeepSeek nodes: proper system prompts + all review agents
const fs = require('fs');
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI1Y2JjODlhNS1iZmQyLTQ3YWQtODkxOC02M2E1NWZjNjdlNDQiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiY2FmOGVjMDktN2FhYS00ZTRmLWI0NDktOTRjYWZmYmRlNzAzIiwiaWF0IjoxNzc4MDc5NTgwfQ.9S7oWFJj7OHSXt31tbAtvt-N9olWrdIH9ld6TuBAZyg';

// Regex for extracting message content from n8n expression templates
function extractMsgContent(bodyStr, role) {
  // Match: "role": "system", "content": "VALUE" or 'role':'system','content':'VALUE'
  const roleEsc = role.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  // Try to find content for this role in the JSON body
  const patterns = [
    new RegExp('"' + roleEsc + '"\\s*:\\s*"content"\\s*:\\s*"([^"]*(?:\\{[^}]*\\}[^"]*)*)"', 'm'),
    new RegExp('"' + roleEsc + '"\\s*,\\s*"content"\\s*:\\s*"([^"]*)"', 'm'),
  ];
  for (const p of patterns) {
    const m = bodyStr.match(p);
    if (m) return m[1];
  }
  return null;
}

// Simpler approach: just extract between "content":" and the next "}
function extractContent(bodyStr, role) {
  // Find the section for this role
  const roleIdx = bodyStr.indexOf('"role": "' + role + '"');
  if (roleIdx === -1) {
    const roleIdx2 = bodyStr.indexOf('"role":"' + role + '"');
    if (roleIdx2 === -1) return null;
  }
  const idx = roleIdx !== -1 ? roleIdx : bodyStr.indexOf('"role":"' + role + '"');
  // Find content after this role
  const contentIdx = bodyStr.indexOf('"content"', idx);
  if (contentIdx === -1) return null;
  // Find the value between content": " and the closing "
  const valueStart = bodyStr.indexOf('"', contentIdx + 10) + 1;
  if (valueStart === 0) return null;
  // Find the closing quote (before }, or next field)
  let valueEnd = valueStart;
  let depth = 0;
  let inString = true;
  for (let i = valueStart; i < bodyStr.length; i++) {
    const ch = bodyStr[i];
    if (ch === '\\') { i++; continue; }
    if (ch === '"' && depth === 0) { valueEnd = i; break; }
    if (ch === '{') depth++;
    if (ch === '}') depth--;
  }
  return bodyStr.substring(valueStart, valueEnd);
}

const SYS = "$json.system_prompt";
const JSON_STR = "$json";

function fixDeepSeekMessages(node, systemContent, userContent) {
  const messages = [];
  if (systemContent) {
    messages.push({ role: "system", content: systemContent });
  }
  messages.push({ role: "user", content: userContent });
  node.parameters.prompt.messages = messages;
}

// ==== Fix Workflow 01: Gen Bible via OpenAI ====
const wf1 = JSON.parse(fs.readFileSync('01_novel_bootstrap.workflow.json', 'utf-8'));
const gb = wf1.nodes.find(n => n.name === 'Gen Bible via OpenAI');
fixDeepSeekMessages(gb,
  '={{ ' + SYS + ' }}',
  '={{ ' + JSON_STR + '.prompt }}'
);
for (const k of Object.keys(wf1.settings || {})) { if (wf1.settings[k] === null) delete wf1.settings[k]; }
fs.writeFileSync('01_novel_bootstrap.workflow.json', JSON.stringify(wf1, null, 2));
console.log('Fixed wf01: Gen Bible with system+user messages');

// ==== Fix Workflow 02: Extract Canon via AI ====
const wf2 = JSON.parse(fs.readFileSync('02_bible_ingestion.workflow.json', 'utf-8'));
const ec = wf2.nodes.find(n => n.name === 'Extract Canon via AI');
fixDeepSeekMessages(ec,
  '你是一位资深小说编辑，负责从新内容中提取结构化 canon 信息。请严格按 JSON schema 输出，不要包含任何解释。关键规则：1. 不要覆盖 existing_canon 中标记为 locked=true 的内容 2. 对不确定的内容设置 confidence < 0.7 3. 不得输出解释、分析、寒暄或 markdown 代码块',
  '={{ JSON.stringify(' + JSON_STR + '.prompt_input) }}'
);
for (const k of Object.keys(wf2.settings || {})) { if (wf2.settings[k] === null) delete wf2.settings[k]; }
fs.writeFileSync('02_bible_ingestion.workflow.json', JSON.stringify(wf2, null, 2));
console.log('Fixed wf02: Extract Canon with system+user messages');

// ==== Fix Workflow 03: All review agents ====
const wf3 = JSON.parse(fs.readFileSync('03_daily_chapter_production.workflow.json', 'utf-8'));

const reviewSpecs = {
  'A1 Continuity Review': {
    system: '你是 continuity_reviewer，负责检查长篇小说章节的连续性问题。你只输出合法 JSON。检查：时间线、地点、伤势、道具、人物关系、伏笔、世界观是否矛盾。blocking：违反 hard canon、关键状态矛盾、擅自改变主线。评分：90-100无问题 75-89小问题 60-74需要修订 0-59禁止发布。',
    user: 'writing_brief: {{ JSON.stringify(' + JSON_STR + '.writing_brief) }}\\n\\nchapter: {{ JSON.stringify(' + JSON_STR + '.chapter) }}'
  },
  'A2 Character Review': {
    system: '你是 character_reviewer，负责检查人物一致性。你只输出合法 JSON。检查：人物行为是否符合 personality、动机、speech_style、关系变化是否有铺垫。blocking：主角核心设定背离、关键关系突变、行为严重矛盾。',
    user: 'writing_brief: {{ JSON.stringify(' + JSON_STR + '.writing_brief) }}\\n\\nchapter: {{ JSON.stringify(' + JSON_STR + '.chapter) }}'
  },
  'A3 Plot Logic Review': {
    system: '你是 plot_logic_reviewer，负责检查情节因果和章节结构。你只输出合法 JSON。检查：是否完成 plot_goals、冲突因果、转折铺垫、是否有机械降神。blocking：核心目标未完成、完全无因果、机械降神解决冲突。',
    user: 'writing_brief: {{ JSON.stringify(' + JSON_STR + '.writing_brief) }}\\n\\nchapter: {{ JSON.stringify(' + JSON_STR + '.chapter) }}'
  },
  'A4 Pacing Review': {
    system: '你是 pacing_reviewer，负责检查中文连载网文的节奏、爽点、钩子和读者留存。你只输出合法 JSON。检查：开头、冲突、情绪曲线、信息增量、爽点、段落长度、说明文字、章节结尾钩子。blocking：无实质冲突、结尾无阅读动力、大量说明不可读。',
    user: 'writing_brief: {{ JSON.stringify(' + JSON_STR + '.writing_brief) }}\\n\\nchapter: {{ JSON.stringify(' + JSON_STR + '.chapter) }}'
  },
  'A5 Style Review': {
    system: '你是 style_reviewer，负责检查文风、语言质量和 AI 味。你只输出合法 JSON。检查：是否符合 style_guide、重复表达、空泛形容、AI 套话、对话自然度、错别字、病句。blocking：文风完全偏离、语言质量极低、大量不可读。',
    user: 'style_guide: {{ JSON.stringify(' + JSON_STR + '.writing_brief.style_guide) }}\\n\\nchapter: {{ JSON.stringify(' + JSON_STR + '.chapter) }}'
  },
  'A6 Safety Review': {
    system: '你是 safety_reviewer，负责检查公开发布风险、原创性风险和安全风险。你只输出合法 JSON。检查：违法/危险内容、过度模仿具体作者/作品、密钥泄露、隐私泄露、平台禁止内容。blocking：密钥/连接字符串/token泄露、版权抄袭、违法内容、隐私泄露。只要发现密钥或内部配置泄露，必须 blocking。',
    user: 'chapter: {{ JSON.stringify(' + JSON_STR + '.chapter) }}'
  },
  'A7 Publication Review': {
    system: '你是 publication_reviewer，负责检查章节是否适合发布到博客。你只输出合法 JSON。检查：Markdown 有效性、标题、摘要、slug、tags、是否包含内部信息。blocking：Markdown严重损坏、含内部prompt/审稿意见、标题正文为空。',
    user: 'chapter: {{ JSON.stringify(' + JSON_STR + '.chapter) }}'
  },
  'Generate Blog Metadata': {
    system: '你是 blog_publisher_metadata_agent，负责把小说章节转换成适合博客发布的元数据。你只输出合法 JSON。要求：生成标题、slug（小写英文+连字符）、excerpt（80-160中文字）、tags（3-8个）、category、seo_description（80-150中文字）、cover_prompt。不剧透结尾关键反转，不包含内部审稿信息。',
    user: 'chapter: {{ JSON.stringify(' + JSON_STR + '.chapter) }}\\n\\nproject: {{ JSON.stringify(' + JSON_STR + '.project) }}\\n\\nblog_config: {{ JSON.stringify(' + JSON_STR + '.blog_config) }}'
  }
};

for (const [name, spec] of Object.entries(reviewSpecs)) {
  const node = wf3.nodes.find(n => n.name === name);
  if (node && node.type === 'n8n-nodes-deepseek.deepseek') {
    fixDeepSeekMessages(node, spec.system, spec.user);
    console.log('Fixed wf03:', name);
  }
}

for (const k of Object.keys(wf3.settings || {})) { if (wf3.settings[k] === null) delete wf3.settings[k]; }
fs.writeFileSync('03_daily_chapter_production.workflow.json', JSON.stringify(wf3, null, 2));

// ==== Fix Workflow 04: Quick Safety Check ====
const wf4 = JSON.parse(fs.readFileSync('04_review_and_repair.workflow.json', 'utf-8'));
const qs = wf4.nodes.find(n => n.name === 'Quick Safety Check');
if (qs && qs.type === 'n8n-nodes-deepseek.deepseek') {
  fixDeepSeekMessages(qs,
    '你是 safety_reviewer。只输出合法 JSON。对章节进行安全审查，检查是否有密钥泄露、版权问题、平台禁止内容。blocking issue：出现密钥、连接字符串、token、版权抄袭、违法内容。',
    'chapter:\\n{{ JSON.stringify(' + JSON_STR + '.chapter) }}'
  );
  console.log('Fixed wf04: Quick Safety Check');
}
for (const k of Object.keys(wf4.settings || {})) { if (wf4.settings[k] === null) delete wf4.settings[k]; }
fs.writeFileSync('04_review_and_repair.workflow.json', JSON.stringify(wf4, null, 2));

// ==== Fix Workflow 05: Generate Arc Plan via AI ====
const wf5 = JSON.parse(fs.readFileSync('05_weekly_arc_planner.workflow.json', 'utf-8'));
const gap = wf5.nodes.find(n => n.name === 'Generate Arc Plan via AI');
if (gap && gap.type === 'n8n-nodes-deepseek.deepseek') {
  fixDeepSeekMessages(gap,
    '你是一位资深网络小说策划编辑，负责每周剧情规划。你只输出合法 JSON。任务：1.分析已发布章节节奏和读者反馈 2.检查未回收伏笔和活跃剧情线 3.生成未来7-14章的详细章节计划 4.更新剧情线和伏笔状态 5.不得覆盖locked canon 6.对重大主线变更标记human_review_required=true 7.每章计划必须包含plot_goals、预期冲突、涉及角色、涉及地点、情绪曲线目标。',
    '={{ JSON.stringify(' + JSON_STR + ') }}'
  );
  console.log('Fixed wf05: Generate Arc Plan');
}
for (const k of Object.keys(wf5.settings || {})) { if (wf5.settings[k] === null) delete wf5.settings[k]; }
fs.writeFileSync('05_weekly_arc_planner.workflow.json', JSON.stringify(wf5, null, 2));

// ==== Deploy all ====
async function deploy() {
  const list = await fetch('http://localhost:5678/api/v1/workflows?limit=20', {
    headers: { 'X-N8N-API-KEY': API_KEY }
  });
  const d = await list.json();
  const nameToId = {};
  for (const w of d.data) {
    if (w.name.startsWith('[NF]')) nameToId[w.name] = w.id;
  }

  const allFiles = [
    '01_novel_bootstrap.workflow.json',
    '02_bible_ingestion.workflow.json',
    '03_daily_chapter_production.workflow.json',
    '04_review_and_repair.workflow.json',
    '05_weekly_arc_planner.workflow.json',
  ];

  for (const file of allFiles) {
    const wf = JSON.parse(fs.readFileSync(file, 'utf-8'));
    const oldId = nameToId[wf.name];
    if (oldId) {
      await fetch('http://localhost:5678/api/v1/workflows/' + oldId, {
        method: 'DELETE', headers: { 'X-N8N-API-KEY': API_KEY }
      });
      console.log('Deleted old:', oldId.slice(0,8));
    }
    const payload = JSON.stringify({
      name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings
    });
    const resp = await fetch('http://localhost:5678/api/v1/workflows', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY },
      body: payload
    });
    const result = await resp.json();
    console.log(wf.name, '->', resp.ok ? 'OK ' + result.id.slice(0,8) : 'FAIL');
  }
}

deploy();
