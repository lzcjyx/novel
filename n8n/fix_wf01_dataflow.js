// Fix workflow 01 data flow: insert a Merge node so Create Project gets input data
const fs = require('fs');
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI1Y2JjODlhNS1iZmQyLTQ3YWQtODkxOC02M2E1NWZjNjdlNDQiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiY2FmOGVjMDktN2FhYS00ZTRmLWI0NDktOTRjYWZmYmRlNzAzIiwiaWF0IjoxNzc4MDc5NTgwfQ.9S7oWFJj7OHSXt31tbAtvt-N9olWrdIH9ld6TuBAZyg';

const wf = JSON.parse(fs.readFileSync('01_novel_bootstrap.workflow.json', 'utf-8'));

// Add a Code node that merges Prepare Project Input data into the flow
// This node goes between Schema Missing? false branch and Create Project
const mergeNode = {
  parameters: {
    jsCode: '// Merge original input data into the flow\n// At this point, $json is from Check Schema Ready ({schema_ready: 1})\n// We need the project input data from Prepare Project Input\nconst input = $(\"Prepare Project Input\").first().json;\nreturn [{ json: { ...input, schema_ready: $json.schema_ready } }];'
  },
  id: 'node-merge-001',
  name: 'Merge Input Data',
  type: 'n8n-nodes-base.code',
  typeVersion: 2,
  position: [1120, 520]
};

// Insert Merge node
wf.nodes.push(mergeNode);

// Update connections: Schema Missing? false → Merge → Create Project
// Old: Schema Missing? false → Create Project
// New: Schema Missing? false → Merge → Create Project
const smNode = wf.connections['Schema Missing?'];
if (smNode && smNode.main[0]) {
  // Move Create Project connection from Schema Missing? false to Merge
  smNode.main[0] = [{ node: 'Merge Input Data', type: 'main', index: 0 }];
}

// Add Merge → Create Project connection
wf.connections['Merge Input Data'] = {
  main: [[{ node: 'Create Project', type: 'main', index: 0 }]]
};

// Now Create Project can reference $json.project_name, $json.genre, etc.
// Fix Create Project values back to simple $json references
const cp = wf.nodes.find(n => n.name === 'Create Project');
cp.parameters.additionalFields.values = [
  { name: '1', value: '={{ $json.project_name }}' },
  { name: '2', value: '={{ ($json.genre || ["fantasy"]).join(", ") }}' },
  { name: '3', value: '={{ $json.target_audience }}' },
  { name: '4', value: '={{ JSON.stringify({ tone: $json.tone, narrative_style: $json.style_profile_desc, forbidden_phrases: ["眼中闪过","嘴角上扬","不由得"], preferred_techniques: ["动作链","对话推进","感官细节"] }) }}' },
  { name: '5', value: '={{ $json.target_word_count_total }}' },
  { name: '6', value: '={{ $json.daily_target_words_count }}' },
  { name: '7', value: '={{ $json.blog_provider_val }}' },
  { name: '8', value: '={{ $json.auto_publish }}' },
  { name: '9', value: '={{ $json.quality_threshold }}' },
  { name: '10', value: '={{ JSON.stringify({ synopsis: $json.description, sub_genre: $json.sub_genre, slug: $json.project_slug }) }}' }
];

// Also fix Build Bible Prompt to use $json (since it's downstream of Create Project)
// Create Project returns {id, name, status, created_at}
// But Build Bible Prompt needs the full project info including genre, tone, etc.
// It's now downstream of Create Project → $json has {id, name, status, created_at}
// It needs Merge data + Create Project data
// Best: have Build Bible Prompt also reference Merge
const bbp = wf.nodes.find(n => n.name === 'Build Bible Prompt');
bbp.parameters.jsCode = `// ============================================================
// Build the bible generation prompt.
// Uses merged project data from Merge Input Data node.
// ============================================================

const project = $('Create Project').first().json;
const input = $('Merge Input Data').first().json;

const prompt = '你是一位资深小说策划编辑。请为以下新小说项目生成完整的初始圣经。\\n\\n项目信息：\\n- 书名：' + project.name + '\\n- 类型：' + input.genre + '\\n- 目标读者：' + input.target_audience + '\\n- 文风基调：' + input.tone + '\\n\\n请生成以下内容（全部使用中文，输出严格 JSON）：\\n1. 世界观概述（500-800字）\\n2. 力量体系初步设计\\n3. 主线剧情概要（3-5条主线）\\n4. 主要人物设计（5-10个）\\n5. 重要地点（3-5个）\\n6. 重要组织（2-4个）\\n7. 核心道具（2-5个）\\n8. 风格指南\\n9. 禁写规则（至少5条hard规则）\\n10. 前10章章节计划\\n\\n只输出 JSON，不要解释。';

return [{
  json: {
    project: project,
    prompt: prompt,
    system_prompt: '你是一位资深小说策划编辑。请严格按 JSON schema 输出，不要包含任何解释或 markdown 代码块。'
  }
}];`;

// Fix Parse Bible Response to reference Merge Input Data
const pbr = wf.nodes.find(n => n.name === 'Parse Bible Response');
pbr.parameters.jsCode = `const response = $input.first().json;
let bible;
try {
  if (response.choices && response.choices[0]) {
    bible = JSON.parse(response.choices[0].message.content);
  } else {
    bible = response;
  }
} catch (e) {
  throw new Error('Failed to parse bible JSON: ' + e.message);
}

return [{ json: { project: $('Create Project').first().json, bible: bible } }];`;

// Fix Split Bible to DB Rows
const sbr = wf.nodes.find(n => n.name === 'Split Bible to DB Rows');
sbr.parameters.jsCode = `const data = $input.first().json;
const project = data.project;
const bible = data.bible;
const items = [];

if (bible.style_guide) {
  items.push({ json: { table: 'style_guides', sql: 'INSERT INTO style_guides (project_id, name, style_text, positive_examples, negative_examples) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING RETURNING id', values: [project.id, '初始风格指南', bible.style_guide.narrative_perspective || '', (bible.style_guide.positive_examples || []), (bible.style_guide.negative_examples || [])] } });
}

(bible.characters || []).forEach(c => {
  items.push({ json: { table: 'characters', sql: 'INSERT INTO characters (project_id, name, role, appearance, personality, speech_style, motivation, backstory) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING RETURNING id', values: [project.id, c.name, c.role, c.appearance, c.personality, c.speech_style, c.motivation, c.backstory] } });
});

(bible.locations || []).forEach(l => {
  items.push({ json: { table: 'locations', sql: 'INSERT INTO locations (project_id, name, type, description) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id', values: [project.id, l.name, l.location_type, l.description] } });
});

(bible.organizations || []).forEach(o => {
  items.push({ json: { table: 'organizations', sql: 'INSERT INTO organizations (project_id, name, description, goals) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id', values: [project.id, o.name, o.description, o.goals || []] } });
});

(bible.items || []).forEach(it => {
  items.push({ json: { table: 'items', sql: 'INSERT INTO items (project_id, name, item_type, description) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id', values: [project.id, it.name, it.item_type, it.description] } });
});

if (bible.world_overview) {
  items.push({ json: { table: 'world_lore', sql: 'INSERT INTO world_lore (project_id, title, lore_type, content) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id', values: [project.id, '世界观概述', 'world_overview', bible.world_overview] } });
}

if (bible.power_system) {
  items.push({ json: { table: 'magic_or_power_systems', sql: 'INSERT INTO magic_or_power_systems (project_id, name, description, rules, limitations) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING RETURNING id', values: [project.id, bible.power_system.name || '力量体系', bible.power_system.description || '', JSON.stringify(bible.power_system.rules || []), JSON.stringify(bible.power_system.limitations || [])] } });
}

(bible.canon_rules || []).forEach(r => {
  items.push({ json: { table: 'canon_rules', sql: 'INSERT INTO canon_rules (project_id, rule_type, severity, rule_text, rationale) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING RETURNING id', values: [project.id, r.rule_type || 'hard', r.rule_type === 'soft' ? 'soft' : 'hard', r.rule_text, r.rationale] } });
});

(bible.main_plot_threads || []).forEach(pt => {
  items.push({ json: { table: 'plot_threads', sql: 'INSERT INTO plot_threads (project_id, name, description, thread_type) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id', values: [project.id, pt.title, pt.description, pt.thread_type || 'main'] } });
});

(bible.chapter_plans || []).forEach(cp => {
  items.push({ json: { table: 'chapter_plans', sql: 'INSERT INTO chapter_plans (project_id, sequence, title, plot_goals, summary, target_word_count, arc_stage) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (project_id, sequence) DO NOTHING RETURNING id', values: [project.id, cp.sequence, cp.title, cp.plot_goals || [], cp.summary || '', cp.target_word_count || 3000, cp.arc_stage || ''] } });
});

return items;`;

// Fix Build Insert SQL to use correct character insert (no abilities column)
const bis = wf.nodes.find(n => n.name === 'Build Insert SQL');
bis.parameters.jsCode = `const item = $input.first().json;
const data = item.data;
let sql = '';
let values = [];

switch (item.table) {
  case 'style_guides':
    sql = 'INSERT INTO style_guides (project_id, name, style_text, positive_examples, negative_examples) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING RETURNING id';
    values = data.values;
    break;
  case 'characters':
    sql = 'INSERT INTO characters (project_id, name, role, appearance, personality, speech_style, motivation, backstory) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING RETURNING id';
    values = data.values;
    break;
  case 'locations':
    sql = 'INSERT INTO locations (project_id, name, type, description) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id';
    values = data.values;
    break;
  case 'organizations':
    sql = 'INSERT INTO organizations (project_id, name, description, goals) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id';
    values = data.values;
    break;
  case 'items':
    sql = 'INSERT INTO items (project_id, name, item_type, description) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id';
    values = data.values;
    break;
  case 'world_lore':
    sql = 'INSERT INTO world_lore (project_id, title, lore_type, content) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id';
    values = data.values;
    break;
  case 'magic_or_power_systems':
    sql = 'INSERT INTO magic_or_power_systems (project_id, name, description, rules, limitations) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING RETURNING id';
    values = data.values;
    break;
  case 'canon_rules':
    sql = 'INSERT INTO canon_rules (project_id, rule_type, severity, rule_text, rationale) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING RETURNING id';
    values = data.values;
    break;
  case 'plot_threads':
    sql = 'INSERT INTO plot_threads (project_id, name, description, thread_type) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id';
    values = data.values;
    break;
  case 'chapter_plans':
    sql = 'INSERT INTO chapter_plans (project_id, sequence, title, plot_goals, summary, target_word_count, arc_stage) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (project_id, sequence) DO NOTHING RETURNING id';
    values = data.values;
    break;
  default:
    return [];
}

return [{ json: { table: item.table, sql: sql, values: values } }];`;

// Fix Prepare Vector Documents to reference Create Project
const pvd = wf.nodes.find(n => n.name === 'Prepare Vector Documents');
pvd.parameters.jsCode = `const items = $input.all();
const project = $('Create Project').first().json;
const vectors = [];
const seen = new Set();

for (const item of items) {
  const j = item.json;
  if (seen.has(j.table + '-' + (j.data?.name || j.data?.title || ''))) continue;
  seen.add(j.table + '-' + (j.data?.name || j.data?.title || ''));

  let sourceType, title, content;
  if (j.table === 'characters' && j.data) {
    sourceType = 'character_profile';
    title = '角色: ' + j.data.name;
    content = '【' + j.data.role + '】' + j.data.name + '\\n性格:' + j.data.personality + '\\n口吻:' + j.data.speech_style + '\\n动机:' + j.data.motivation + '\\n背景:' + j.data.backstory;
  } else if (j.table === 'world_lore' && j.data) {
    sourceType = 'world_lore';
    title = j.data.title;
    content = j.data.content;
  } else continue;

  vectors.push({
    json: { project_id: project.id, source_type: sourceType, title: title, content: content, input_text: content }
  });
}
return vectors;`;

// Fix Bootstrap Summary
const bss2 = wf.nodes.find(n => n.name === 'Bootstrap Summary');
bss2.parameters.jsCode = `const project = $('Create Project').first().json;
const items = $input.all();

return [{
  json: {
    project_id: project.id,
    project_name: project.name,
    status: 'bootstrap_complete',
    message: 'Project \"' + project.name + '\" (ID: ' + project.id + ') bootstrapped. Set status to active, then run Weekly Arc Planner.',
    entities_created: items.length
  }
}];`;

// Also ensure the Gen Bible via OpenAI is renamed/referenced correctly
const gbo = wf.nodes.find(n => n.name === 'Gen Bible via OpenAI');
if (gbo) {
  // URL is already set to DeepSeek
}

// Clean settings
for (const k of Object.keys(wf.settings || {})) { if (wf.settings[k] === null) delete wf.settings[k]; }
fs.writeFileSync('01_novel_bootstrap.workflow.json', JSON.stringify(wf, null, 2));
console.log('Workflow 01 completely rebuilt with correct data flow.');

// Deploy
async function deploy() {
  const list = await fetch('http://localhost:5678/api/v1/workflows?limit=20', { headers: { 'X-N8N-API-KEY': API_KEY } });
  const d = await list.json();
  const old = d.data.find(w => w.name === '[NF] 01 — Novel Bootstrap');
  if (old) {
    await fetch('http://localhost:5678/api/v1/workflows/' + old.id, { method: 'DELETE', headers: { 'X-N8N-API-KEY': API_KEY } });
    console.log('Deleted old:', old.id.slice(0,8));
  }
  const payload = JSON.stringify({name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings});
  const resp = await fetch('http://localhost:5678/api/v1/workflows', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY }, body: payload });
  const result = await resp.json();
  console.log('Deployed:', resp.ok ? 'OK ' + result.id.slice(0,8) : 'FAIL ' + (result.message||'').slice(0,200));
}
deploy();
