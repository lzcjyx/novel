// Make WF05 robust against varying AI output key names
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
  '// Robust: try multiple possible key names',
  'var chapters = plan.chapter_plans || plan.chapters_plan || plan.new_chapter_plans || plan.chapters || [];',
  '',
  'for (var i = 0; i < chapters.length; i++) {',
  '  var cp = chapters[i];',
  '  var seq = (data.next_sequence_start || plan.next_sequence_start || 1) + i;',
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
  '// Flexible key matching',
  'var chapters = plan.chapter_plans || plan.chapters_plan || plan.new_chapter_plans || plan.chapters || [];',
  'var plotThreads = plan.plot_threads || plan.plot_threads_update || plan.plot_thread_updates || [];',
  'var foreshadow = plan.foreshadowing || plan.foreshadowing_update || plan.foreshadowing_updates || [];',
  '',
  '// Human review items - check in plot threads or top-level',
  'var humanReview = plan.human_review_required;',
  'if (typeof humanReview === "boolean") {',
  '  humanReview = humanReview ? [{ note: "human_review_required" }] : [];',
  '}',
  'if (!Array.isArray(humanReview)) humanReview = [];',
  'var plotHumanReview = (plotThreads || []).filter(function(p) { return p.human_review_required; });',
  'var allHumanReview = humanReview.concat(plotHumanReview);',
  '',
  'return [{',
  '  json: {',
  '    project_id: data.project_id,',
  '    chapters_planned: chapters.length,',
  '    plot_threads_updated: plotThreads.length,',
  '    foreshadowing_updated: foreshadow.length,',
  '    human_review_count: allHumanReview.length,',
  '    human_review_items: allHumanReview,',
  '    weekly_summary: plan.notes || plan.weekly_summary || "",',
  '    pacing_analysis: (plan.analysis || {}).current_pacing || (plan.analysis || {}).pacing_analysis || "",',
  '    status: "weekly_plan_complete"',
  '  }',
  '}];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/RGbsc6NtL3uHqKSk', { headers: H });
  const wf = await resp.json();

  const bps = wf.nodes.find(n => n.name === 'Build Plan SQLs');
  if (bps) { bps.parameters.jsCode = newBuildSQL; console.log('Fixed Build Plan SQLs (flex keys)'); }

  const wps = wf.nodes.find(n => n.name === 'Weekly Plan Summary');
  if (wps) { wps.parameters.jsCode = newSummary; console.log('Fixed Weekly Plan Summary (flex keys)'); }

  if (wf.settings) { for (const k of Object.keys(wf.settings)) { if (wf.settings[k] === null) delete wf.settings[k]; } }

  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r.ok ? 'Updated! Run WF05 again' : 'FAIL');
}
main().catch(e => console.error(e));
