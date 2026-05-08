// Add chapter_plans status update after chapter is written
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  // Update Execute Chapter Insert to also update chapter_plans status
  const eci = wf.nodes.find(n => n.name === 'Execute Chapter Insert');
  if (eci) {
    // The current query runs cte from Save Chapter.
    // We need to ALSO update chapter_plans status.
    // Easiest: add a second Postgres node after Execute Chapter Insert
    // Or modify the CTE to include the plan update
    // Simpler: just append plan update to the SQL
    eci.parameters.query = '={{ $json.sql + "; UPDATE chapter_plans SET status = \'draft\', updated_at = now() WHERE id = \'" + $json.plan_id + "\'" }}';
    console.log('Fixed: Execute Chapter Insert now also updates plan status');
  }

  // But wait - we need the plan_id in the SQL context. Let's add it in Save Chapter instead.
  const sc = wf.nodes.find(n => n.name === 'Save Chapter');
  if (sc) {
    let code = sc.parameters.jsCode;
    // Add plan_id to the output json
    code = code.replace(
      "return [{ json: { title: title, sequence: seq, word_count: wordCount, sql: sql } }];",
      "return [{ json: { title: title, sequence: seq, word_count: wordCount, sql: sql, plan_id: plan.id } }];"
    );
    sc.parameters.jsCode = code;
    console.log('Fixed: Save Chapter now outputs plan_id');
  }

  wf.settings = { executionOrder: 'v1' };
  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });
  console.log(r.ok ? 'Updated!' : 'FAIL');
}
main().catch(e => console.error(e));
