const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

const buildPlanSQL = [
  'var data = $input.first().json;',
  'var plan = data.plan;',
  'var pid = data.project_id;',
  'var items = [];',
  '',
  'function esc(val) {',
  '  if (val === undefined || val === null) return "NULL";',
  '  if (typeof val === "boolean") return val ? "TRUE" : "FALSE";',
  '  if (typeof val === "number") return String(val);',
  '  var str = typeof val === "object" ? JSON.stringify(val) : String(val);',
  '  return "\x27" + str.replace(/\x27/g, "\x27\x27") + "\x27";',
  '}',
  '',
  'var plans = plan.chapter_plans || plan.new_chapter_plans || plan.chapters || [];',
  '',
  'for (var i = 0; i < plans.length; i++) {',
  '  var cp = plans[i];',
  '  items.push({',
  '    json: {',
  '      sql: "INSERT INTO chapter_plans (project_id, sequence, title, plot_goals, outline, target_word_count, status) VALUES ("',
  '        + esc(pid) + ", "',
  '        + esc((data.next_sequence_start || 1) + i) + ", "',
  '        + esc(cp.title || ("Chapter " + ((data.next_sequence_start||1)+i))) + ", "',
  '        + esc(cp.plot_goals || []) + ", "',
  '        + esc(cp.summary || cp.outline || "") + ", "',
  '        + esc(cp.target_word_count || 3000) + ", "',
  '        + esc("planned")',
  '      + ") ON CONFLICT (project_id, sequence) DO UPDATE SET title = EXCLUDED.title, plot_goals = EXCLUDED.plot_goals, outline = EXCLUDED.outline, target_word_count = EXCLUDED.target_word_count, updated_at = now() RETURNING id"',
  '    }',
  '  });',
  '}',
  '',
  'if (items.length === 0) {',
  '  items.push({ json: { sql: "SELECT 1 AS placeholder", _note: "No chapter plans generated" } });',
  '}',
  '',
  'return items;'
].join('\n');

const executePlanSQL = '={{ $json.sql }}';

async function main() {
  const resp = await fetch(BASE + '/workflows/RGbsc6NtL3uHqKSk', { headers: H });
  const wf = await resp.json();

  const bps = wf.nodes.find(n => n.name === 'Build Plan SQLs');
  if (bps) {
    bps.parameters.jsCode = buildPlanSQL;
    console.log('Fixed Build Plan SQLs');
  }

  const epu = wf.nodes.find(n => n.name === 'Execute Plan Updates');
  if (epu) {
    epu.parameters.query = executePlanSQL;
    delete epu.parameters.additionalFields;
    console.log('Fixed Execute Plan Updates (inline SQL)');
  }

  if (wf.settings) { for (const k of Object.keys(wf.settings)) { if (wf.settings[k] === null) delete wf.settings[k]; } }

  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r.ok ? 'Updated!' : 'FAIL');
}
main().catch(e => console.error(e));
