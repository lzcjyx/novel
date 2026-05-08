// Make WF04 actually save revised content back to DB and .md file
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/GLfaMt9VH3vXSREy', { headers: H });
  const wf = await resp.json();

  // Fix: Save Revised Version → actually build inline SQL to insert new version + update chapter
  const srv = wf.nodes.find(n => n.name === 'Save Revised Version');
  if (srv) {
    srv.type = 'n8n-nodes-base.code';
    srv.typeVersion = 2;
    srv.parameters = { jsCode: [
      'var writerResp = $input.first().json;',
      'var chData = writerResp.data || writerResp;',
      'var input = $("Prepare Input").first().json;',
      'var ch = $("Load Chapter").first().json;',
      '',
      'var title = chData.title || ch.title || "Revised";',
      'var content = chData.body_markdown || chData.content || "";',
      'var wordCount = chData.word_count || content.length;',
      'var summary = chData.summary || "";',
      '',
      'function esc(val) {',
      '  if (val === undefined || val === null) return "NULL";',
      '  if (typeof val === "number") return String(val);',
      '  var str = typeof val === "object" ? JSON.stringify(val) : String(val);',
      '  return "\x27" + str.replace(/\x27/g, "\x27\x27") + "\x27";',
      '}',
      '',
      '// CTE: insert new version + update chapter in one query',
      'var sql = "WITH ins_ver AS ("',
      '  + "INSERT INTO chapter_versions (chapter_id, project_id, version_number, version_type, title, body_markdown, summary, word_count) VALUES ("',
      '  + esc(ch.id) + ", " + esc(ch.project_id) + ", 2, \x27revised\x27, "',
      '  + esc(title) + ", " + esc(content) + ", " + esc(summary) + ", " + esc(wordCount)',
      '  + ") RETURNING id, chapter_id"',
      '  + ") UPDATE chapters SET status = \x27revised\x27, final_version_id = ins_ver.id, updated_at = now() FROM ins_ver WHERE chapters.id = ins_ver.chapter_id RETURNING chapters.id, ins_ver.id AS version_id";',
      '',
      'return [{ json: {',
      '  title: title,',
      '  content: content,',
      '  word_count: wordCount,',
      '  sql: sql',
      '}}];'
    ].join('\n') };
    delete srv.credentials;
    console.log('Fixed: Save Revised Version → inline SQL');
  }

  // Add a Postgres node after Save Revised Version to execute the SQL
  // Change "Prepare Re-Review" (which is a pass-through) to Postgres executor
  const prr = wf.nodes.find(n => n.name === 'Prepare Re-Review');
  if (prr) {
    prr.type = 'n8n-nodes-base.postgres';
    prr.typeVersion = 2;
    prr.name = 'Execute Revision Save';
    prr.parameters = { operation: 'executeQuery', query: '={{ $json.sql }}' };
    prr.credentials = { postgres: { id: '0w5EYts2acIZab9Y', name: 'Neon Pooled (n8n)' } };
    console.log('Fixed: Prepare Re-Review → Execute Revision Save');
  }

  // Change "Quick Safety Check" position is fine
  // Add a node after Execute Revision Save to write .md file
  // Actually, let me add a new node: Update .md File

  // Add "Update .md File" as a new Code node
  const updateMd = {
    parameters: { jsCode: [
      'var fs = require("fs");',
      'var data = $("Save Revised Version").first().json;',
      'var ch = $("Load Chapter").first().json;',
      '',
      'var seq = ch.sequence || 1;',
      'var slug = "my-novel";',
      'var filename = "/data/paper/" + slug + "-ch" + String(seq).padStart(3,"0") + ".md";',
      '',
      '// Read existing file',
      'var existing = "";',
      'try { existing = fs.readFileSync(filename, "utf8"); } catch(e) {}',
      '',
      '// Build revised content',
      'var md = "# " + data.title + " (Revised)\\n\\n";',
      'md += "> 章节: " + seq + " | 字数: " + data.word_count + "\\n\\n";',
      'md += data.content + "\\n\\n";',
      'md += "---\\n";',
      'md += "*Revised by AI Novel Factory*\\n";',
      '',
      'fs.writeFileSync(filename, md, "utf8");',
      '',
      'return [{ json: { updated: true, filename: filename, title: data.title } }];'
    ].join('\n') },
    id: 'node-update-md',
    name: 'Update .md File',
    type: 'n8n-nodes-base.code',
    typeVersion: 2,
    position: [1800, 700]
  };
  wf.nodes.push(updateMd);
  console.log('Added: Update .md File');

  // Update connections: Execute Revision Save → Update .md File → Quick Safety Check
  wf.connections['Execute Revision Save'] = { main: [[{ node: 'Update .md File', type: 'main', index: 0 }]] };
  wf.connections['Update .md File'] = { main: [[{ node: 'Quick Safety Check', type: 'main', index: 0 }]] };

  // Fix Build Revision Input to include proper data for writer
  const bri = wf.nodes.find(n => n.name === 'Build Revision Input');
  if (bri) {
    bri.parameters.jsCode = [
      'var ch = $("Load Chapter").first().json;',
      'var lvr = $("Load Version & Reviews").first().json;',
      'return [{ json: {',
      '  chapter_id: ch.id,',
      '  project_id: ch.project_id,',
      '  title: ch.title,',
      '  sequence: ch.sequence,',
      '  body_markdown: lvr.content || "",',
      '  mode: "full_revision",',
      '  revision_notes: "Revise based on safety review"',
      '}}];'
    ].join('\n');
    console.log('Fixed: Build Revision Input');
  }

  // Fix Final Decision
  const fd = wf.nodes.find(n => n.name === 'Final Decision');
  if (fd) {
    fd.parameters.jsCode = 'var qsc = $("Quick Safety Check").first().json;\nvar ok = qsc.message ? qsc.message.indexOf("pass")>=0 : true;\nreturn [{ json: { decision: ok ? "publish" : "revise", safety_ok: ok } }];';
    console.log('Fixed: Final Decision');
  }

  wf.settings = { executionOrder: 'v1' };
  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });
  console.log(r.ok ? 'Updated! Run WF04 to revise chapter' : 'FAIL');
}
main().catch(e => console.error(e));
