// Restore per-novel subdirectory logic in WF03 Build Markdown Output + Write Chapter File
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
  '// Per-novel subdirectory',
  'var slug = (proj.name || "novel").replace(/[^a-zA-Z0-9\\u4e00-\\u9fff]+/g, "-").toLowerCase();',
  'var dir = "/data/paper/" + slug;',
  'var fs = require("fs");',
  'try { fs.mkdirSync(dir, { recursive: true }); } catch(e) {}',
  '',
  'var seq = plan.sequence || 1;',
  'var filename = dir + "/ch" + String(seq).padStart(3,"0") + ".md";',
  '',
  'fs.writeFileSync(filename, "# " + title + "\\n\\n" + content + "\\n", "utf8");',
  '',
  'return [{ json: { filename: filename, content: content, title: title, sequence: seq, word_count: wordCount, slug: slug } }];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  // Fix Build Markdown Output
  const bmo = wf.nodes.find(n => n.name === 'Build Markdown Output');
  if (bmo) { bmo.parameters.jsCode = bmoCode; console.log('Fixed Build Markdown Output (per-novel)'); }

  // Remove separate Write Chapter File node (BMO now writes directly)
  const wcf = wf.nodes.find(n => n.name === 'Write Chapter File');
  if (wcf) {
    wcf.parameters = { jsCode: 'return $input.all();' };
    console.log('Write Chapter File → pass-through');
  }

  wf.settings = { executionOrder: 'v1' };
  const r2 = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r2.ok ? 'Updated!' : 'FAIL');
}
main().catch(e => console.error(e));
