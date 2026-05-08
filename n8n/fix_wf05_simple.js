const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

// Super simple - just count what exists, no assumptions about key names
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
  '// Find the chapters array - try all possible key names',
  'var chapters = plan.chapter_plans || plan.chapters_plan || plan.new_chapter_plans || plan.chapters || [];',
  '',
  '// Also look for chapters nested under any key that is an array of objects with "title"',
  'if (chapters.length === 0) {',
  '  var keys = Object.keys(plan);',
  '  for (var i = 0; i < keys.length; i++) {',
  '    var val = plan[keys[i]];',
  '    if (Array.isArray(val) && val.length > 0 && val[0].title) {',
  '      chapters = val;',
  '      break;',
  '    }',
  '  }',
  '}',
  '',
  'for (var i = 0; i < chapters.length; i++) {',
  '  var cp = chapters[i];',
  '  var seq = cp.sequence || (data.next_sequence_start || plan.next_sequence_start || 1) + i;',
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

// Simple summary - count arrays, handle boolean human_review
const newSummary = [
  'var data = $("Parse Arc Plan").first().json;',
  'var plan = data.plan;',
  '',
  '// Find chapters array',
  'var chapters = plan.chapter_plans || plan.chapters_plan || plan.new_chapter_plans || plan.chapters || [];',
  'if (chapters.length === 0) {',
  '  var keys = Object.keys(plan);',
  '  for (var i = 0; i < keys.length; i++) {',
  '    var val = plan[keys[i]];',
  '    if (Array.isArray(val) && val.length > 0 && val[0].title) chapters = val;',
  '  }',
  '}',
  '',
  '// Count plot-related arrays',
  'var plotArr = plan.plot_threads || plan.plot_threads_update || plan.plot_thread_updates || [];',
  'var foreshArr = plan.foreshadowing || plan.foreshadowing_update || plan.foreshadowing_updates || [];',
  '',
  '// Handle human_review - can be boolean or array',
  'var hr = plan.human_review_required;',
  'if (typeof hr === "boolean") hr = hr ? 1 : 0;',
  'else if (Array.isArray(hr)) hr = hr.length;',
  'else hr = 0;',
  '',
  'return [{',
  '  json: {',
  '    project_id: data.project_id,',
  '    chapters_planned: chapters.length,',
  '    plot_threads_updated: plotArr.length,',
  '    foreshadowing_updated: foreshArr.length,',
  '    human_review_count: hr,',
  '    human_review_items: Array.isArray(plan.human_review_required) ? plan.human_review_required : [],',
  '    weekly_summary: plan.notes || plan.weekly_summary || plan.plan_summary || "",',
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
