// Final WF04 fix: replace Load Chapter Postgres with Code node that works reliably
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/GLfaMt9VH3vXSREy', { headers: H });
  const wf = await resp.json();

  // Replace Load Chapter → Code node that reads from filesystem
  const lc = wf.nodes.find(n => n.name === 'Load Chapter');
  if (lc) {
    lc.type = 'n8n-nodes-base.code';
    lc.typeVersion = 2;
    lc.parameters = { jsCode: [
      'var fs = require("fs");',
      'var path = require("path");',
      'var paperDir = "/data/paper";',
      'var slugs = [];',
      'try { slugs = fs.readdirSync(paperDir).filter(function(d) { return d.startsWith("novel-"); }); } catch(e) {}',
      '',
      '// Find the newest chapter across all novels',
      'var newest = null;',
      'var newestTime = 0;',
      'slugs.forEach(function(slug) {',
      '  var dir = paperDir + "/" + slug;',
      '  try {',
      '    fs.readdirSync(dir).filter(function(f) { return f.endsWith(".md"); }).forEach(function(f) {',
      '      var stat = fs.statSync(dir + "/" + f);',
      '      if (stat.mtimeMs > newestTime) {',
      '        newestTime = stat.mtimeMs;',
      '        var seq = parseInt((f.match(/ch(\\d+)/) || [])[1]) || 1;',
      '        var content = fs.readFileSync(dir + "/" + f, "utf8");',
      '        var title = "";',
      '        var lines = content.split("\\n");',
      '        if (lines[0].startsWith("# ")) title = lines[0].replace("# ", "");',
      '        newest = { filename: f, sequence: seq, title: title, slug: slug, content: content };',
      '      }',
      '    });',
      '  } catch(e) {}',
      '});',
      '',
      'if (!newest) { throw new Error("No chapter files found — run Write Chapter Now first"); }',
      '',
      'return [{ json: { id: newest.filename, sequence: newest.sequence, title: newest.title, project_id: "auto", slug: newest.slug, chapter_id: newest.filename } }];'
    ].join('\n') };
    delete lc.credentials;
    console.log('Load Chapter → Code node (reads filesystem)');
  }

  // Simplify Save Revised Version — just touch the file, no DB write needed for review
  const srv = wf.nodes.find(n => n.name === 'Save Revised Version');
  if (srv) {
    srv.parameters.jsCode = [
      'var writerResp = $input.first().json;',
      'var chData = writerResp.data || writerResp;',
      'var ch = $("Load Chapter").first().json;',
      '',
      'var title = chData.title || ch.title || "Revised";',
      'var content = chData.body_markdown || chData.content || "";',
      'var wordCount = chData.word_count || content.length;',
      '',
      '// Write revised content back to the file',
      'var fs = require("fs");',
      'var paperDir = "/data/paper";',
      'var filename = paperDir + "/" + (ch.slug || "my-novel") + "/ch" + String(ch.sequence || 1).padStart(3,"0") + ".md";',
      'if (content && content.length > 50) {',
      '  fs.writeFileSync(filename, "# " + title + "\\n\\n" + content + "\\n", "utf8");',
      '}',
      '',
      'return [{ json: { title: title, content: content, word_count: wordCount, revised: content.length > 50, filename: filename } }];'
    ].join('\n');
    console.log('Save Revised Version → writes revised file directly');
  }

  // Simplify Execute Revision Save — just pass through
  const ers = wf.nodes.find(n => n.name === 'Execute Revision Save');
  if (ers) {
    ers.type = 'n8n-nodes-base.code';
    ers.typeVersion = 2;
    ers.parameters = { jsCode: 'return $input.all();' };
    delete ers.credentials;
    console.log('Execute Revision Save → pass-through');
  }

  wf.settings = { executionOrder: 'v1' };
  const r2 = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r2.ok ? 'WF04 rebuilt!' : 'FAIL');
}
main().catch(e => console.error(e));
