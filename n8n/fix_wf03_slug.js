// Fix WF03 Build Markdown Output: use ID-based slug matching orchestrator
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

const bmoCode = [
  'var writerResp = $("Call Writer Service").first().json;',
  'var chData = writerResp.data || writerResp;',
  'var plan = $("Select Next Chapter Plan").first().json;',
  'var proj = $("Load Active Project").first().json;',
  '',
  'var title = chData.title || plan.title || "Untitled";',
  'var content = chData.body_markdown || chData.content || "";',
  'content = content.replace(/^# [^\\n]*\\n?/, "").trim();',
  'var wordCount = chData.word_count || content.length;',
  '',
  '// Use ID-based slug (matches orchestrator convention)',
  'var slug = "novel-" + (proj.id || "unknown").substring(0, 8);',
  'var dir = "/data/paper/" + slug;',
  'var fs = require("fs");',
  'try { fs.mkdirSync(dir, { recursive: true }); } catch(e) {}',
  '',
  'var seq = plan.sequence || 1;',
  'var filename = dir + "/ch" + String(seq).padStart(3,"0") + ".md";',
  '',
  'fs.writeFileSync(filename, "# " + title + "\\n\\n" + content + "\\n", "utf8");',
  '',
  'return [{ json: { filename: filename, content: content, title: title, sequence: seq, word_count: wordCount, slug: slug, project_id: proj.id } }];'
].join('\n');

const lscCode = [
  'var fs = require("fs");',
  'var proj = $("Load Active Project").first().json;',
  'var plan = $("Select Next Chapter Plan").first().json;',
  '',
  '// ID-based slug matching orchestrator',
  'var slug = "novel-" + (proj.id || "0").substring(0, 8);',
  '',
  'var prevChapters = [];',
  'for (var i = 1; i <= 20; i++) {',
  '  var fn = "/data/paper/" + slug + "/ch" + String(i).padStart(3,"0") + ".md";',
  '  try {',
  '    var md = fs.readFileSync(fn, "utf8");',
  '    var t = "";',
  '    var lines = md.split("\\n");',
  '    for (var j = 0; j < Math.min(lines.length, 5); j++) {',
  '      if (lines[j].startsWith("# ")) t = lines[j].replace("# ","");',
  '    }',
  '    var body = md.substring(md.indexOf("\\n\\n") + 2, md.indexOf("\\n\\n") + 302);',
  '    if (t) prevChapters.push({ sequence: i, title: t, preview: body });',
  '  } catch(e) {}',
  '}',
  '',
  'var context = "";',
  'if (prevChapters.length > 0) {',
  '  var last = prevChapters[prevChapters.length - 1];',
  '  context += "Previous: Ch." + last.sequence + " " + last.title + "\\n" + (last.preview || "") + "\\n\\n";',
  '} else {',
  '  context += "First chapter. Establish world and protagonist.\\n";',
  '}',
  '',
  'return [{ json: {',
  '  project_id: proj.id,',
  '  previous_chapters: prevChapters,',
  '  context: context,',
  '  slug: slug',
  '}}];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  const bmo = wf.nodes.find(n => n.name === 'Build Markdown Output');
  if (bmo) { bmo.parameters.jsCode = bmoCode; console.log('Fixed Build Markdown Output (ID slug)'); }

  const lsc = wf.nodes.find(n => n.name === 'Load Structured Canon');
  if (lsc) { lsc.parameters.jsCode = lscCode; console.log('Fixed Load Structured Canon (ID slug)'); }

  wf.settings = { executionOrder: 'v1' };
  const r2 = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r2.ok ? 'WF03 slug synced!' : 'FAIL');
}
main().catch(e => console.error(e));
