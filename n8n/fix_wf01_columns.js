// Fix workflow 01 column name mismatches between workflow and actual DB schema
const fs = require('fs');
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI1Y2JjODlhNS1iZmQyLTQ3YWQtODkxOC02M2E1NWZjNjdlNDQiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiY2FmOGVjMDktN2FhYS00ZTRmLWI0NDktOTRjYWZmYmRlNzAzIiwiaWF0IjoxNzc4MDc5NTgwfQ.9S7oWFJj7OHSXt31tbAtvt-N9olWrdIH9ld6TuBAZyg';

const wf = JSON.parse(fs.readFileSync('01_novel_bootstrap.workflow.json', 'utf-8'));

// ==== 1. Fix Prepare Project Input ====
const prepInput = wf.nodes.find(n => n.name === 'Prepare Project Input');
prepInput.parameters.jsCode = prepInput.parameters.jsCode
  .replace(/project_title/g, 'project_name')
  .replace(/slug:/g, 'project_slug:')
  .replace(/project_title/g, 'project_name')
  .replace(/synopsis/g, 'description')
  .replace(/style_summary/g, 'style_profile_desc')
  .replace(/total_target_words/g, 'target_word_count_total')
  .replace(/daily_word_count/g, 'daily_target_words_count')
  .replace(/publish_platform/g, 'blog_provider_val');

// ==== 2. Fix Create Project SQL ====
const createProject = wf.nodes.find(n => n.name === 'Create Project');
createProject.parameters.query = `INSERT INTO projects (
  name, genre, target_audience, style_profile,
  total_target_words, daily_target_words,
  blog_provider, auto_publish,
  quality_threshold, status, metadata
) VALUES (
  $1, $2, $3, $4,
  $5, $6,
  $7, $8,
  $9, 'draft', $10
)
RETURNING id, name, status, created_at;`;

// Update values to match new query
createProject.parameters.additionalFields.values = [
  { name: "1", value: "={{ $json.project_name }}" },
  { name: "2", value: "={{ ($json.genre || ['fantasy']).join(', ') }}" },
  { name: "3", value: "={{ $json.target_audience }}" },
  { name: "4", value: "={{ JSON.stringify({ tone: $json.tone, narrative_style: $json.style_profile_desc, forbidden_phrases: ['眼中闪过','嘴角上扬','不由得'], preferred_techniques: ['动作链','对话推进','感官细节'] }) }}" },
  { name: "5", value: "={{ $json.target_word_count_total }}" },
  { name: "6", value: "={{ $json.daily_target_words_count }}" },
  { name: "7", value: "={{ $json.blog_provider_val }}" },
  { name: "8", value: "={{ $json.auto_publish }}" },
  { name: "9", value: "={{ $json.quality_threshold }}" },
  { name: "10", value: "={{ JSON.stringify({ synopsis: $json.description, sub_genre: $json.sub_genre, slug: $json.project_slug }) }}" }
];

// ==== 3. Fix Build Bible Prompt ====
const biblePrompt = wf.nodes.find(n => n.name === 'Build Bible Prompt');
// Replace project.title → project.name
biblePrompt.parameters.jsCode = biblePrompt.parameters.jsCode
  .replace(/project\.title/g, 'project.name')
  .replace(/project\.ton/g, 'project.style_profile?.tone || project.tone');

// ==== 4. Fix Build Insert SQL node ====
const buildInserts = wf.nodes.find(n => n.name === 'Build Insert SQL');
if (buildInserts) {
  buildInserts.parameters.jsCode = buildInserts.parameters.jsCode
    .replace(/c\.abilities \|\| \[\]/g, 'c.abilities || []');
}

// ==== 5. Fix Bootstrap Summary ====
const summary = wf.nodes.find(n => n.name === 'Bootstrap Summary');
summary.parameters.jsCode = summary.parameters.jsCode
  .replace(/project\.name/g, 'project.name')
  .replace(/project\.title/g, 'project.name');

// ==== 6. Fix Parse Bible Response ====
const parseBible = wf.nodes.find(n => n.name === 'Parse Bible Response');
parseBible.parameters.jsCode = parseBible.parameters.jsCode
  .replace(/project\.title/g, 'project.name');

// ==== 7. Fix Split Bible to DB Rows ====
const splitBible = wf.nodes.find(n => n.name === 'Split Bible to DB Rows');
splitBible.parameters.jsCode = splitBible.parameters.jsCode
  .replace(/project\.title/g, 'project.name');

// ==== 8. Fix Prepare Vector Documents ====
const prepVectors = wf.nodes.find(n => n.name === 'Prepare Vector Documents');
prepVectors.parameters.jsCode = prepVectors.parameters.jsCode
  .replace(/project\.title/g, 'project.name');

// ==== 9. Fix Gen Bible via AI to use correct model ====
const genBible = wf.nodes.find(n => n.name === 'Gen Bible via OpenAI');
if (genBible) {
  genBible.parameters.jsonBody = genBible.parameters.jsonBody
    .replace(/"gpt-4o"/g, '"deepseek-chat"');
}

// ==== 10. Fix Load Active Project SQL (node 002) ====
const loadProj = wf.nodes.find(n => n.name === 'Load Active Project');
if (loadProj) {
  // This node might not exist in 01 — it's in 03. But just in case:
}

// Clean settings
for (const k of Object.keys(wf.settings || {})) {
  if (wf.settings[k] === null) delete wf.settings[k];
}

fs.writeFileSync('01_novel_bootstrap.workflow.json', JSON.stringify(wf, null, 2));
console.log('Workflow 01 fixed and saved.');

// Also fix the other workflows that reference projects columns
const files = ['02_bible_ingestion.workflow.json', '03_daily_chapter_production.workflow.json', '04_review_and_repair.workflow.json', '05_weekly_arc_planner.workflow.json'];
for (const file of files) {
  const w = JSON.parse(fs.readFileSync(file, 'utf-8'));
  let changed = false;
  for (const node of w.nodes) {
    // Fix SQL queries that reference projects.title
    if (node.parameters?.query) {
      let q = node.parameters.query;
      if (q.includes('projects') && (q.includes('.title') || q.includes('p.title') || q.includes('p.slug'))) {
        q = q.replace(/\btitle\b/g, 'name').replace(/\bslug\b/g, 'name');
        node.parameters.query = q;
        changed = true;
      }
      if (q.includes('style_summary') || q.includes('daily_word_count')) {
        q = q.replace(/style_summary/g, 'style_profile').replace(/daily_word_count/g, 'daily_target_words');
        node.parameters.query = q;
        changed = true;
      }
    }
    // Fix JS code that references projects.title
    if (node.parameters?.jsCode) {
      let js = node.parameters.jsCode;
      if (js.includes('project.title') || js.includes('project.slug')) {
        js = js.replace(/project\.title/g, 'project.name').replace(/project\.slug/g, 'project.name');
        node.parameters.jsCode = js;
        changed = true;
      }
      if (js.includes('style_summary') || js.includes('daily_word_count')) {
        js = js.replace(/style_summary/g, 'style_profile').replace(/daily_word_count/g, 'daily_target_words');
        node.parameters.jsCode = js;
        changed = true;
      }
    }
  }
  if (changed) {
    for (const k of Object.keys(w.settings || {})) { if (w.settings[k] === null) delete w.settings[k]; }
    fs.writeFileSync(file, JSON.stringify(w, null, 2));
    console.log(file + ': fixed column references');
  } else {
    console.log(file + ': no fixes needed');
  }
}

// Deploy all
async function deploy() {
  const list = await fetch('http://localhost:5678/api/v1/workflows?limit=20', {
    headers: { 'X-N8N-API-KEY': API_KEY }
  });
  const data = await list.json();
  const nameToId = {};
  for (const w of data.data) {
    if (w.name.startsWith('[NF]')) nameToId[w.name] = w.id;
  }

  const allFiles = ['01_novel_bootstrap.workflow.json', ...files];
  for (const file of allFiles) {
    const wf2 = JSON.parse(fs.readFileSync(file, 'utf-8'));
    const oldId = nameToId[wf2.name];
    if (oldId) {
      await fetch('http://localhost:5678/api/v1/workflows/' + oldId, { method: 'DELETE', headers: { 'X-N8N-API-KEY': API_KEY } });
    }
    const payload = JSON.stringify({name: wf2.name, nodes: wf2.nodes, connections: wf2.connections, settings: wf2.settings});
    const resp = await fetch('http://localhost:5678/api/v1/workflows', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY }, body: payload });
    const result = await resp.json();
    console.log(wf2.name, '->', resp.ok ? 'OK ' + result.id.slice(0,8) : 'FAIL ' + (result.message||'').slice(0,100));
  }
}
deploy();
