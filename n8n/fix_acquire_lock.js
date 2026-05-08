const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const r = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await r.json();

  // Fix Acquire Lock: Code node with inline SQL
  const al = wf.nodes.find(n => n.name === 'Acquire Lock');
  if (al) {
    al.type = 'n8n-nodes-base.code';
    al.typeVersion = 2;
    al.parameters = { jsCode: [
      'var d = $input.first().json;',
      'var pid = d.project_id;',
      'var cid = d.id;',
      'if (!pid || !cid) return [];',
      'function esc(v) { return "\x27" + String(v).replace(/\x27/g, "\x27\x27") + "\x27"; }',
      'var sql = "INSERT INTO generation_jobs (project_id, chapter_plan_id, job_date, status) VALUES (" + esc(pid) + ", " + esc(cid) + ", CURRENT_DATE, \x27started\x27) ON CONFLICT (project_id, chapter_plan_id, job_date) DO NOTHING RETURNING id";',
      'return [{ json: { sql: sql } }];'
    ].join('\n') };
    delete al.credentials;
    console.log('Fixed Acquire Lock');
  }

  wf.settings = { executionOrder: 'v1' };
  const r2 = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r2.ok ? 'Updated!' : 'FAIL');
}
main().catch(e => console.error(e));
