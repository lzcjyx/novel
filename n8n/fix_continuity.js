// Fix chapter continuity: load previous chapters context + pass to writer
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

// Load Structured Canon: query recent chapters + characters + plot threads
const loadCanonCode = [
  'var fs = require("fs");',
  'var proj = $("Load Active Project").first().json;',
  '',
  '// Read previous chapters from .md files for continuity',
  'var prevChapters = [];',
  'for (var i = 1; i <= 10; i++) {',
  '  var fn = "/data/paper/my-novel-ch" + String(i).padStart(3,"0") + ".md";',
  '  try {',
  '    var md = fs.readFileSync(fn, "utf8");',
  '    // Extract title and first paragraph as summary',
  '    var lines = md.split("\\n");',
  '    var title = "";',
  '    var summary = "";',
  '    for (var j = 0; j < lines.length; j++) {',
  '      if (lines[j].startsWith("# ") && !title) title = lines[j].replace("# ","");',
  '      if (lines[j].startsWith("> ") && lines[j].includes("章节:")) {',
  '        summary = lines[j];',
  '      }',
  '    }',
  '    if (title) {',
  '      var bodyStart = md.indexOf("\\n\\n", md.indexOf("> 章节:"));',
  '      var body = bodyStart > 0 ? md.substring(bodyStart, bodyStart + 300) : "";',
  '      prevChapters.push({ sequence: i, title: title, preview: body });',
  '    }',
  '  } catch(e) {}',
  '}',
  '',
  'var context = "Previous chapters summary:\\n";',
  'if (prevChapters.length > 0) {',
  '  prevChapters.forEach(function(ch) {',
  '    context += "- Ch." + ch.sequence + ": " + ch.title + " | " + (ch.preview || "").substring(0, 100) + "\\n";',
  '  });',
  '} else {',
  '  context += "This is the first chapter. No previous context.\\n";',
  '}',
  '',
  'return [{ json: {',
  '  project_id: proj.id,',
  '  previous_chapters: prevChapters,',
  '  context: context,',
  '  characters: [],',
  '  plot_threads: [],',
  '  foreshadowing: []',
  '}}];'
].join('\n');

// Build Writing Brief: include previous chapters context
const buildBriefCode = [
  'var proj = $("Load Active Project").first().json;',
  'var plan = $("Select Next Chapter Plan").first().json;',
  'var canon = $("Load Structured Canon").first().json;',
  'var retrieval = $("Build Retrieval Query").first().json;',
  '',
  '// Build writing brief for the writer-service',
  'return [{ json: {',
  '  project_id: proj.id,',
  '  chapter_plan_id: plan.id,',
  '  sequence: plan.sequence,',
  '  title: plan.title || "Chapter " + plan.sequence,',
  '  plot_goals: plan.plot_goals || [],',
  '  outline: plan.outline || plan.summary || "",',
  '  target_word_count: plan.target_word_count || 3000,',
  '',
  '  // Context for continuity',
  '  previous_context: canon.context || "",',
  '  previous_chapters: canon.previous_chapters || [],',
  '',
  '  // Style and genre',
  '  genre: proj.genre || "",',
  '  style_profile: proj.style_profile || {},',
  '',
  '  // Characters and world info (placeholder for now)',
  '  characters: canon.characters || [],',
  '  plot_threads: canon.plot_threads || [],',
  '',
  '  // Retrieval context',
  '  retrieval_context: retrieval.retrieval_query || ""',
  '}}];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  // 1. Load Structured Canon
  const lsc = wf.nodes.find(n => n.name === 'Load Structured Canon');
  if (lsc) { lsc.parameters.jsCode = loadCanonCode; console.log('Fixed Load Structured Canon'); }

  // 2. Build Writing Brief
  const bwb = wf.nodes.find(n => n.name === 'Build Writing Brief');
  if (bwb) { bwb.parameters.jsCode = buildBriefCode; console.log('Fixed Build Writing Brief'); }

  // 3. Update Call Writer Service jsonBody to pass full writing_brief
  const cws = wf.nodes.find(n => n.name === 'Call Writer Service');
  if (cws) {
    cws.parameters.jsonBody = '={{ JSON.stringify({ writing_brief: $json, chapter_plan_id: $json.chapter_plan_id, project_id: $json.project_id }) }}';
    console.log('Fixed Call Writer Service jsonBody');
  }

  wf.settings = { executionOrder: 'v1' };
  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });
  console.log(r.ok ? 'Updated! Continuity enabled' : 'FAIL');
}
main().catch(e => console.error(e));
