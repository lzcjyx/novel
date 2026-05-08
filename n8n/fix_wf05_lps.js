const fs = require('fs');
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/RGbsc6NtL3uHqKSk', { headers: H });
  const wf = await resp.json();

  // Fix Load Project State - use proper $input (no bash mangling)
  const lps = wf.nodes.find(n => n.name === 'Load Project State');
  if (lps) {
    lps.type = 'n8n-nodes-base.code';
    lps.typeVersion = 2;
    lps.parameters = {
      jsCode: 'const proj = $input.first().json;\nconst pid = proj.id;\nif (!pid) return [];\nreturn [{ json: { project: proj, project_id: pid, chapter_count: 0, plot_thread_count: 0 } }];'
    };
    delete lps.credentials;
    console.log('Fixed Load Project State code');
  }

  if (wf.settings) { for (const k of Object.keys(wf.settings)) { if (wf.settings[k] === null) delete wf.settings[k]; } }

  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  const d = await r.json();
  console.log(r.ok ? 'Updated! Run WF05' : JSON.stringify(d).slice(0, 300));
}
main().catch(e => console.error(e));
