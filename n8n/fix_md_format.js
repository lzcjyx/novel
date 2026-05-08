// Fix Build Markdown Output: pure novel content, no metadata headers/footers
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

const buildMdCode = [
  'var writerResp = $("Call Writer Service").first().json;',
  'var chData = writerResp.data || writerResp;',
  'var plan = $("Select Next Chapter Plan").first().json;',
  '',
  'var title = chData.title || plan.title || "Untitled";',
  'var content = chData.body_markdown || chData.content || "";',
  'var wordCount = chData.word_count || content.length;',
  '',
  '// Strip any markdown headers/footers from AI output',
  '// Remove leading # Title if present (will re-add cleanly)',
  'content = content.replace(/^# [^\\n]*\\n?/, "").trim();',
  '',
  'var seq = plan.sequence || 1;',
  '',
  '// Write pure novel content — just the chapter title + body',
  'var md = "# " + title + "\\n\\n" + content + "\\n";',
  '',
  'var slug = "my-novel";',
  'var filename = "/data/paper/" + slug + "-ch" + String(seq).padStart(3,"0") + ".md";',
  '',
  'return [{ json: { filename: filename, content: md, title: title, sequence: seq, word_count: wordCount } }];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  const bmo = wf.nodes.find(n => n.name === 'Build Markdown Output');
  if (bmo) {
    bmo.parameters.jsCode = buildMdCode;
    console.log('Fixed: Build Markdown Output - pure content only');
  }

  wf.settings = { executionOrder: 'v1' };
  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });
  console.log(r.ok ? 'Updated!' : 'FAIL');
}
main().catch(e => console.error(e));
