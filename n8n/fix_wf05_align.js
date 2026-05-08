// Fix Build Plan SQLs + Weekly Plan Summary: align keys with AI output
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

const newBuildSQL = [
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
  '// AI outputs plan.chapter_plans (not new_chapter_plans)',
  'var plans = plan.chapter_plans || [];',
  '',
  'for (var i = 0; i < plans.length; i++) {',
  '  var cp = plans[i];',
  '  var seq = (data.next_sequence_start || 1) + i;',
  '  items.push({ json: { sql: "INSERT INTO chapter_plans (project_id, sequence, title, plot_goals, outline, target_word_count, status) VALUES ("',
  '    + esc(pid) + ", " + esc(seq) + ", "',
  '    + esc(cp.title || ("Chapter " + seq)) + ", "',
  '    + esc(cp.plot_goals || cp.goals || []) + ", "',
  '    + esc(cp.summary || cp.outline || cp.description || "") + ", "',
  '    + esc(cp.target_word_count || cp.word_count || 3000) + ", "',
  '    + esc("planned") + ") RETURNING id"',
  '  }});',
  '}',
  '',
  'if (items.length === 0) {',
  '  items.push({ json: { sql: "SELECT 1 AS note" } });',
  '}',
  '',
  'return items;'
].join('\n');

const newSummary = [
  'var data = $("Parse Arc Plan").first().json;',
  'var plan = data.plan;',
  '',
  'return [{',
  '  json: {',
  '    project_id: data.project_id,',
  '    chapters_planned: (plan.chapter_plans || []).length,',
  '    plot_threads_updated: (plan.plot_threads || []).length,',
  '    foreshadowing_updated: (plan.foreshadowing || []).length,',
  '    human_review_count: (plan.human_review_required || plan.plot_threads || []).filter(function(p) { return p.human_review_required; }).length,',
  '    human_review_items: (plan.plot_threads || []).filter(function(p) { return p.human_review_required; }),',
  '    weekly_summary: plan.notes || plan.weekly_summary || "",',
  '    pacing_analysis: (plan.analysis || {}).current_pacing || "",',
  '    status: "weekly_plan_complete"',
  '  }',
  '}];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/RGbsc6NtL3uHqKSk', { headers: H });
  const wf = await resp.json();

  const bps = wf.nodes.find(n => n.name === 'Build Plan SQLs');
  if (bps) { bps.parameters.jsCode = newBuildSQL; console.log('Fixed Build Plan SQLs'); }

  const wps = wf.nodes.find(n => n.name === 'Weekly Plan Summary');
  if (wps) { wps.parameters.jsCode = newSummary; console.log('Fixed Weekly Plan Summary'); }

  if (wf.settings) { for (const k of Object.keys(wf.settings)) { if (wf.settings[k] === null) delete wf.settings[k]; } }

  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r.ok ? 'Updated! Run WF05' : 'FAIL');
}
main().catch(e => console.error(e));
