// Simplify WF03: remove Get Project ID adapter, Load Active Project works directly
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  // Remove Get Project ID node
  wf.nodes = wf.nodes.filter(n => n.name !== 'Get Project ID');
  delete wf.connections['Get Project ID'];
  console.log('Removed Get Project ID');

  // Restore Load Active Project to simple active-project query
  const lap = wf.nodes.find(n => n.name === 'Load Active Project');
  if (lap) {
    lap.parameters.query = "SELECT * FROM projects WHERE status = 'active' ORDER BY created_at DESC LIMIT 1";
    delete lap.parameters.additionalFields;
    console.log('Restored Load Active Project');
  }

  // Rewire: Webhook Trigger → Load Active Project directly
  wf.connections['Webhook Trigger'] = { main: [[{ node: 'Load Active Project', type: 'main', index: 0 }]] };

  wf.settings = { executionOrder: 'v1' };
  const r2 = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r2.ok ? 'WF03 simplified!' : 'FAIL');
}
main().catch(e => console.error(e));
