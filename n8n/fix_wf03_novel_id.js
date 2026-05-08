// Make WF03 accept project_id from webhook via Code node adapter
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  // Add "Get Project ID" Code node BEFORE Load Active Project
  // It checks webhook input for project_id, defaults to active project
  const getPidNode = {
    parameters: { jsCode: [
      'var input = $input.first().json;',
      'var pid = input.project_id;',
      'if (pid) {',
      '  // Use the project_id from webhook/orchestrator',
      '  return [{ json: { project_id: pid, sql: "SELECT * FROM projects WHERE id = \x27" + pid + "\x27" } }];',
      '} else {',
      '  // No project_id specified, use active project',
      '  return [{ json: { project_id: "auto", sql: "SELECT * FROM projects WHERE status = \x27active\x27 ORDER BY created_at DESC LIMIT 1" } }];',
      '}'
    ].join('\n') },
    id: 'node-get-pid',
    name: 'Get Project ID',
    type: 'n8n-nodes-base.code',
    typeVersion: 2,
    position: [580, 300]
  };
  wf.nodes.push(getPidNode);
  console.log('Added Get Project ID Code node');

  // Change Load Active Project to execute the SQL from Get Project ID
  const lap = wf.nodes.find(n => n.name === 'Load Active Project');
  if (lap) {
    lap.parameters.query = '={{ $json.sql }}';
    delete lap.parameters.additionalFields;
    console.log('Fixed Load Active Project → uses $json.sql from Get Project ID');
  }

  // Rewire: Webhook Trigger → Get Project ID → Load Active Project → ...
  wf.connections['Webhook Trigger'] = { main: [[{ node: 'Get Project ID', type: 'main', index: 0 }]] };
  wf.connections['Get Project ID'] = { main: [[{ node: 'Load Active Project', type: 'main', index: 0 }]] };

  wf.settings = { executionOrder: 'v1' };
  const r2 = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r2.ok ? 'WF03 updated! Now accepts project_id from webhook' : 'FAIL: ' + JSON.stringify(await r2.json()).slice(0,200));
}
main().catch(e => console.error(e));
