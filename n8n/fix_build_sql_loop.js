// Fix Build Insert SQL: use $input.all() to process ALL items, not just the first
const fs = require('fs');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };
const WF_ID = 'CtEvELW9y6XbmbVi';

const newCode = [
  '// Process ALL items from Split Bible to DB Rows',
  'const items = $input.all();',
  '',
  'function esc(val) {',
  '  if (val === undefined || val === null) return "NULL";',
  '  if (typeof val === "boolean") return val ? "TRUE" : "FALSE";',
  '  if (typeof val === "number") return String(val);',
  '  var str = typeof val === "object" ? JSON.stringify(val) : String(val);',
  '  return "\x27" + str.replace(/\x27/g, "\x27\x27") + "\x27";',
  '}',
  '',
  'return items.map(function(item) {',
  '  var j = item.json;',
  '  var vals = j.values || [];',
  '  var sql = j.sql || "";',
  '',
  '  for (var i = 0; i < vals.length; i++) {',
  '    sql = sql.replace("$" + (i + 1), esc(vals[i]));',
  '  }',
  '',
  '  return { json: { table: j.table, sql: sql } };',
  '});'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/' + WF_ID, { headers: HEADERS });
  const wf = await resp.json();

  const bis = wf.nodes.find(n => n.name === 'Build Insert SQL');
  if (bis) {
    bis.parameters.jsCode = newCode;
    console.log('Fixed: Build Insert SQL now processes ALL items');
  }

  if (wf.settings) {
    for (const k of Object.keys(wf.settings)) { if (wf.settings[k] === null) delete wf.settings[k]; }
  }

  const updateResp = await fetch(BASE + '/workflows/' + WF_ID, {
    method: 'PUT',
    headers: HEADERS,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
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
