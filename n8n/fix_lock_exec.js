// Fix: Acquire Lock Code node builds SQL, but no Postgres node executes it!
// Add Postgres node to execute the lock SQL
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  // Add Execute Lock Postgres node
  wf.nodes.push({
    parameters: { operation: 'executeQuery', query: '={{ $json.sql }}' },
    id: 'node-exec-lock',
    name: 'Execute Lock',
    type: 'n8n-nodes-base.postgres',
    typeVersion: 2,
    position: [1200, 640],
    credentials: { postgres: { id: '0w5EYts2acIZab9Y', name: 'Neon Pooled (n8n)' } }
  });
  console.log('Added Execute Lock Postgres node');

  // Rewire: Acquire Lock → Execute Lock → Load Project Config
  wf.connections['Acquire Lock'] = { main: [[{ node: 'Execute Lock', type: 'main', index: 0 }]] };
  wf.connections['Execute Lock'] = { main: [[{ node: 'Load Project Config', type: 'main', index: 0 }]] };

  wf.settings = { executionOrder: 'v1' };
  const r2 = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r2.ok ? 'Updated!' : 'FAIL');
}
main().catch(e => console.error(e));
