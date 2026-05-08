// Fix: Generate Embeddings now produces inline SQL, Store Vector Documents uses it
const fs = require('fs');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };
const WF_ID = 'CtEvELW9y6XbmbVi';

const embeddingCode = [
  '// Generate placeholder embedding vector + inline SQL for insert',
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
  '  var d = item.json;',
  '  var vec = new Array(1536).fill(0);',
  '  var vecStr = "\x27[" + vec.join(",") + "]\x27";',
  '  var sql = "INSERT INTO vector_documents (project_id, source_type, title, content, embedding) VALUES ("',
  '    + esc(d.project_id) + ", "',
  '    + esc(d.source_type) + ", "',
  '    + esc(d.title) + ", "',
  '    + esc(d.content) + ", "',
  '    + vecStr + "::vector"',
  '  + ") ON CONFLICT DO NOTHING RETURNING id;";',
  '  return { json: { sql: sql, embedding: vec } };',
  '});'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/' + WF_ID, { headers: HEADERS });
  const wf = await resp.json();

  // Update Generate Embeddings to produce inline SQL
  const emb = wf.nodes.find(n => n.name === 'Generate Embeddings');
  if (emb) {
    emb.parameters.jsCode = embeddingCode;
    console.log('Updated: Generate Embeddings (now produces inline SQL)');
  }

  // Update Store Vector Documents to use inline SQL
  const svd = wf.nodes.find(n => n.name === 'Store Vector Documents');
  if (svd) {
    svd.parameters.query = '={{ $json.sql }}';
    delete svd.parameters.additionalFields;
    console.log('Updated: Store Vector Documents (inline SQL)');
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
