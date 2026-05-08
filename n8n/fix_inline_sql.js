// Change Build Insert SQL to produce inline SQL (no $params)
// Uses \x27 to avoid quote escaping madness
const fs = require('fs');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };
const WF_ID = 'CtEvELW9y6XbmbVi';

const buildInsertSQL = [
  '// Build inline SQL with properly escaped PostgreSQL values',
  'const item = $input.first().json;',
  '',
  'function esc(val) {',
  '  if (val === undefined || val === null) return "NULL";',
  '  if (typeof val === "boolean") return val ? "TRUE" : "FALSE";',
  '  if (typeof val === "number") return String(val);',
  '  var str = typeof val === "object" ? JSON.stringify(val) : String(val);',
  '  return "\x27" + str.replace(/\x27/g, "\x27\x27") + "\x27";',
  '}',
  '',
  'var vals = item.values || [];',
  'var sql = item.sql || "";',
  '',
  '// Replace $1, $2, etc. with escaped values',
  'for (var i = 0; i < vals.length; i++) {',
  '  sql = sql.replace("$" + (i + 1), esc(vals[i]));',
  '}',
  '',
  'return [{ json: { table: item.table, sql: sql } }];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/' + WF_ID, { headers: HEADERS });
  const wf = await resp.json();

  // Update Build Insert SQL
  const bis = wf.nodes.find(n => n.name === 'Build Insert SQL');
  if (bis) {
    bis.parameters.jsCode = buildInsertSQL;
    console.log('Updated: Build Insert SQL');
  }

  // Update Insert Bible Data - inline SQL only, no params
  const ibd = wf.nodes.find(n => n.name === 'Insert Bible Data');
  if (ibd) {
    ibd.parameters.query = '={{ $json.sql }}';
    delete ibd.parameters.additionalFields;
    console.log('Updated: Insert Bible Data (no params)');
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
  } else {
    const err = await updateResp.json();
    console.log('FAIL:', JSON.stringify(err).slice(0, 300));
  }
}

main().catch(e => console.error(e));
