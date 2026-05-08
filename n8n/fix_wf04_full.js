// Fix WF04 completely: file paths + node references
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/GLfaMt9VH3vXSREy', { headers: H });
  const wf = await resp.json();

  // 1. Fix Load Version & Reviews - use per-novel subdirectory path
  const lvr = wf.nodes.find(n => n.name === 'Load Version & Reviews');
  if (lvr) {
    lvr.parameters.jsCode = [
      'var ch = $input.first().json;',
      'var content = "";',
      'var seq = ch.sequence || 1;',
      '// Per-novel subdirectory',
      'var slug = "my-novel";',
      'var filename = "/data/paper/" + slug + "/ch" + String(seq).padStart(3,"0") + ".md";',
      'try {',
      '  var fs = require("fs");',
      '  var md = fs.readFileSync(filename, "utf8");',
      '  var lines = md.split("\\n");',
      '  var bodyLines = [];',
      '  for (var i = 0; i < lines.length; i++) {',
      '    if (lines[i].startsWith("> ") || lines[i].startsWith("# ")) continue;',
      '    if (lines[i].startsWith("---")) break;',
      '    if (lines[i].trim()) bodyLines.push(lines[i]);',
      '  }',
      '  content = bodyLines.join("\\n");',
      '} catch(e) {}',
      'return [{ json: { chapter: ch, chapter_id: ch.id, body_markdown: content, version_number: 1 } }];'
    ].join('\n');
    console.log('Fixed Load Version & Reviews (per-novel path)');
  }

  // 2. Fix Save Revised Version - use $input chain instead of $("Load Chapter")
  const srv = wf.nodes.find(n => n.name === 'Save Revised Version');
  if (srv) {
    srv.parameters.jsCode = [
      'var writerResp = $input.first().json;',
      'var chData = writerResp.data || writerResp;',
      'var verData = $("Load Version & Reviews").first().json;',
      'var ch = verData.chapter || verData;',
      '',
      'var title = chData.title || ch.title || "Revised";',
      'var content = chData.body_markdown || chData.content || "";',
      'var wordCount = chData.word_count || content.length;',
      'var summary = chData.summary || "";',
      '',
      'if (!ch.id) throw new Error("Missing chapter id — Load Chapter may have returned empty");',
      '',
      'function esc(val) {',
      '  if (val === undefined || val === null) return "NULL";',
      '  if (typeof val === "number") return String(val);',
      '  var str = typeof val === "object" ? JSON.stringify(val) : String(val);',
      '  return "\x27" + str.replace(/\x27/g, "\x27\x27") + "\x27";',
      '}',
      '',
      'var sql = "WITH ins_ver AS ("',
      '  + "INSERT INTO chapter_versions (chapter_id, project_id, version_number, version_type, title, body_markdown, summary, word_count) VALUES ("',
      '  + esc(ch.id) + ", " + esc(ch.project_id) + ", 2, \x27revised\x27, "',
      '  + esc(title) + ", " + esc(content) + ", " + esc(summary) + ", " + esc(wordCount)',
      '  + ") RETURNING id, chapter_id"',
      '  + ") UPDATE chapters SET status = \x27revised\x27, final_version_id = ins_ver.id, updated_at = now() FROM ins_ver WHERE chapters.id = ins_ver.chapter_id RETURNING chapters.id, ins_ver.id AS version_id";',
      '',
      'return [{ json: { title: title, content: content, word_count: wordCount, sql: sql } }];'
    ].join('\n');
    console.log('Fixed Save Revised Version');
  }

  // 3. Simplify Build Revision Input
  const bri = wf.nodes.find(n => n.name === 'Build Revision Input');
  if (bri) {
    bri.parameters.jsCode = [
      'var ch = $("Load Chapter").first().json;',
      'var ver = $("Load Version & Reviews").first().json;',
      'return [{ json: {',
      '  writing_brief: {',
      '    chapter_id: ch.id,',
      '    project_id: ch.project_id,',
      '    title: ch.title,',
      '    sequence: ch.sequence,',
      '    body_markdown: ver.body_markdown || "",',
      '    style_guide: {},',
      '    characters: []',
      '  },',
      '  chapter_id: ch.id,',
      '  mode: "full_revision"',
      '}}];'
    ].join('\n');
    console.log('Fixed Build Revision Input');
  }

  wf.settings = { executionOrder: 'v1' };
  const r2 = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r2.ok ? 'WF04 fully fixed!' : 'FAIL');
}
main().catch(e => console.error(e));
