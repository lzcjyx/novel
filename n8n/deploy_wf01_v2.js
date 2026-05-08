const fs = require('fs');
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI1Y2JjODlhNS1iZmQyLTQ3YWQtODkxOC02M2E1NWZjNjdlNDQiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiY2FmOGVjMDktN2FhYS00ZTRmLWI0NDktOTRjYWZmYmRlNzAzIiwiaWF0IjoxNzc4MDc5NTgwfQ.9S7oWFJj7OHSXt31tbAtvt-N9olWrdIH9ld6TuBAZyg';

const wf = JSON.parse(fs.readFileSync('01_novel_bootstrap.workflow.json', 'utf-8'));

// Fix Merge node: correct field names + proper SQL quoting
const mergeJsCode = [
'// Build INSERT SQL with proper PostgreSQL single-quote escaping',
'const input = $("Prepare Project Input").first().json;',
'',
'function esc(val) {',
'  if (val === undefined || val === null) return "NULL";',
'  if (typeof val === "boolean") return val ? "TRUE" : "FALSE";',
'  if (typeof val === "number") return String(val);',
'  const str = typeof val === "object" ? JSON.stringify(val) : String(val);',
'  // PostgreSQL: single quotes, doubling any embedded single quotes',
'  return "\'" + str.replace(/\'/g, "\'\'") + "\'";',
'}',
'',
'const name = esc(input.project_name);',
'const genre = esc(input.genre ? input.genre.join(", ") : "fantasy");',
'const audience = esc(input.target_audience);',
'const style = esc(JSON.stringify({',
'  tone: input.tone || "热血",',
'  narrative_style: input.style_profile_desc || "",',
'  forbidden_phrases: ["眼中闪过","嘴角上扬","不由得"],',
'  preferred_techniques: ["动作链","对话推进","感官细节"]',
'}));',
'const totalWords = esc(input.target_total_words || 500000);',
'const dailyWords = esc(input.daily_target_words_count || 3000);',
'const blog = esc(input.blog_provider_val || "wordpress");',
'const autoPub = esc(input.auto_publish || false);',
'const qThreshold = esc(input.quality_threshold || 85);',
'const meta = esc(JSON.stringify({',
'  synopsis: input.description || "",',
'  sub_genre: input.sub_genre || [],',
'  slug: input.project_slug || "",',
'  max_revise_count: input.max_revise_count || 2',
'}));',
'',
'const sql = "INSERT INTO projects (name, genre, target_audience, style_profile, total_target_words, daily_target_words, blog_provider, auto_publish, quality_threshold, status, metadata) VALUES (" + name + ", " + genre + ", " + audience + ", " + style + ", " + totalWords + ", " + dailyWords + ", " + blog + ", " + autoPub + ", " + qThreshold + ", " + esc("draft") + ", " + meta + ") RETURNING id, name, status, created_at;";',
'',
'return [{ json: { ...input, schema_ready: $json.schema_ready, _sql: sql } }];'
].join('\n');

const merge = wf.nodes.find(n => n.name === 'Merge Input Data');
merge.parameters.jsCode = mergeJsCode;

// Clean settings
for (const k of Object.keys(wf.settings || {})) {
  if (wf.settings[k] === null) delete wf.settings[k];
}
fs.writeFileSync('01_novel_bootstrap.workflow.json', JSON.stringify(wf, null, 2));
console.log('Fixed: proper SQL quoting with single quotes, correct field names.');

async function deploy() {
  const list = await fetch('http://localhost:5678/api/v1/workflows?limit=20', {
    headers: { 'X-N8N-API-KEY': API_KEY }
  });
  const d = await list.json();
  const old = d.data.find(w => w.name === '[NF] 01 — Novel Bootstrap');
  if (old) {
    await fetch('http://localhost:5678/api/v1/workflows/' + old.id, {
      method: 'DELETE', headers: { 'X-N8N-API-KEY': API_KEY }
    });
  }
  const payload = JSON.stringify({
    name: wf.name,
    nodes: wf.nodes,
    connections: wf.connections,
    settings: wf.settings
  });
  const resp = await fetch('http://localhost:5678/api/v1/workflows', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY },
    body: payload
  });
  const result = await resp.json();
  console.log('Deploy:', resp.ok ? 'OK ' + result.id.slice(0,8) : 'FAIL ' + (result.message||'').slice(0,100));
}

deploy();
