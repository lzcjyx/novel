// Enhance novel quality: fix data pipeline + increase word count
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

// Load Structured Canon: load real data from previous chapters + DB
const loadCanonCode = [
  'var fs = require("fs");',
  'var proj = $("Load Active Project").first().json;',
  'var plan = $("Select Next Chapter Plan").first().json;',
  '',
  'var slug = (proj.name || "novel").replace(/[^a-zA-Z0-9\\u4e00-\\u9fff]+/g, "-").toLowerCase();',
  '',
  '// 1. Read previous chapters for continuity',
  'var prevChapters = [];',
  'for (var i = 1; i <= 20; i++) {',
  '  var fn = "/data/paper/" + slug + "/ch" + String(i).padStart(3,"0") + ".md";',
  '  try {',
  '    var md = fs.readFileSync(fn, "utf8");',
  '    var title = "";',
  '    var summary = "";',
  '    var lines = md.split("\\n");',
  '    for (var j = 0; j < Math.min(lines.length, 5); j++) {',
  '      if (lines[j].startsWith("# ")) title = lines[j].replace("# ","");',
  '    }',
  '    // Get first 300 chars as summary',
  '    var body = md.substring(md.indexOf("\\n\\n") + 2, md.indexOf("\\n\\n") + 302);',
  '    if (title) prevChapters.push({ sequence: i, title: title, preview: body });',
  '  } catch(e) {}',
  '}',
  '',
  '// 2. Build context string for writer',
  'var context = "";',
  'if (prevChapters.length > 0) {',
  '  var last = prevChapters[prevChapters.length - 1];',
  '  context += "Previous chapter (Ch." + last.sequence + ": " + last.title + "):\\n" + (last.preview || "") + "\\n\\n";',
  '  if (prevChapters.length > 1) {',
  '    context += "Recent chapters summary:\\n";',
  '    prevChapters.slice(-5).forEach(function(ch) {',
  '      context += "- Ch." + ch.sequence + ": " + ch.title + "\\n";',
  '    });',
  '  }',
  '} else {',
  '  context += "This is the first chapter. Establish world, protagonist, and central conflict.\\n";',
  '}',
  '',
  'return [{ json: {',
  '  project_id: proj.id,',
  '  previous_chapters: prevChapters,',
  '  context: context,',
  '  characters: [],',
  '  plot_threads: [],',
  '  foreshadowing_inventory: [],',
  '  canon_rules: [],',
  '  style_guide: {}',
  '}}];'
].join('\n');

// Build Writing Brief: rich context with all available data
const buildBriefCode = [
  'var proj = $("Load Active Project").first().json;',
  'var plan = $("Select Next Chapter Plan").first().json;',
  'var canon = $("Load Structured Canon").first().json;',
  '',
  'return [{ json: {',
  '  project_id: proj.id,',
  '  chapter_plan_id: plan.id,',
  '  sequence: plan.sequence || 1,',
  '  title: plan.title || "Chapter " + (plan.sequence || 1),',
  '',
  '  // Core writing targets',
  '  plot_goals: plan.plot_goals || [],',
  '  target_word_count: 5000,  // Increased from 3000',
  '  arc_stage: plan.arc_stage || "rising",',
  '',
  '  // Continuity context (from previous chapters)',
  '  previous_context: canon.context || "",',
  '  previous_chapters: canon.previous_chapters || [],',
  '',
  '  // World & style',
  '  genre: proj.genre || "",',
  '  style_profile: proj.style_profile || {},',
  '  canon_rules: canon.canon_rules || [],',
  '  style_guide: canon.style_guide || {},',
  '',
  '  // Characters, plots, foreshadowing',
  '  characters: canon.characters || [],',
  '  plot_threads: canon.plot_threads || [],',
  '  foreshadowing_inventory: canon.foreshadowing_inventory || []',
  '}}];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  // 1. Load Structured Canon
  const lsc = wf.nodes.find(n => n.name === 'Load Structured Canon');
  if (lsc) { lsc.parameters.jsCode = loadCanonCode; console.log('1. Fixed Load Structured Canon'); }

  // 2. Build Writing Brief
  const bwb = wf.nodes.find(n => n.name === 'Build Writing Brief');
  if (bwb) { bwb.parameters.jsCode = buildBriefCode; console.log('2. Fixed Build Writing Brief (target=5000)'); }

  wf.settings = { executionOrder: 'v1' };
  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });
  console.log(r.ok ? 'WF03 updated!' : 'FAIL');
}
main().catch(e => console.error(e));
