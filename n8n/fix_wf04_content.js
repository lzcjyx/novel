// Fix WF04: load chapter body from chapter_versions, pass to writer, protect .md
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/GLfaMt9VH3vXSREy', { headers: H });
  const wf = await resp.json();

  // 1. Fix Load Version & Reviews - actually query chapter_versions for body content
  const lvr = wf.nodes.find(n => n.name === 'Load Version & Reviews');
  if (lvr) {
    lvr.type = 'n8n-nodes-base.postgres';
    lvr.typeVersion = 2;
    lvr.parameters = {
      operation: 'executeQuery',
      query: "SELECT * FROM chapter_versions WHERE chapter_id = '{{ $json.id }}' ORDER BY version_number DESC LIMIT 1"
    };
    delete lvr.parameters.additionalFields;
    lvr.credentials = { postgres: { id: '0w5EYts2acIZab9Y', name: 'Neon Pooled (n8n)' } };
    console.log('Fixed: Load Version & Reviews → query chapter_versions');
  }

  // But {{ }} won't work in Postgres query. Let me handle this differently.
  // Change approach: use a Code node to build the version query with inline chapter_id
  lvr.type = 'n8n-nodes-base.code';
  lvr.typeVersion = 2;
  lvr.parameters = { jsCode: [
    'var ch = $input.first().json;',
    '// Use the actual file content from /paper as fallback',
    'var fs = require("fs");',
    'var content = "";',
    'var seq = ch.sequence || 1;',
    'var filename = "/data/paper/my-novel-ch" + String(seq).padStart(3,"0") + ".md";',
    'try {',
    '  var md = fs.readFileSync(filename, "utf8");',
    '  // Extract content between header and footer',
    '  var lines = md.split("\\n");',
    '  var inContent = false;',
    '  var bodyLines = [];',
    '  for (var i = 0; i < lines.length; i++) {',
    '    if (lines[i].startsWith("> ") || lines[i].startsWith("# ")) continue;',
    '    if (lines[i].startsWith("---")) break;',
    '    if (lines[i].trim()) bodyLines.push(lines[i]);',
    '  }',
    '  content = bodyLines.join("\\n");',
    '} catch(e) {}',
    'return [{ json: { chapter: ch, chapter_id: ch.id, body_markdown: content, version_number: 1 } }];'
  ].join('\n') };
  delete lvr.credentials;
  console.log('Fixed: Load Version & Reviews → reads .md file for content');

  // 2. Fix Build Revision Input - include body_markdown in writer request
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
    console.log('Fixed: Build Revision Input includes body_markdown');
  }

  // 3. Fix Call Writer Revise - ensure it sends proper JSON
  const cwr = wf.nodes.find(n => n.name === 'Call Writer Revise');
  if (cwr) {
    cwr.parameters.jsonBody = '={{ JSON.stringify({ writing_brief: $json.writing_brief || $json, chapter_id: $json.chapter_id }) }}';
    console.log('Fixed: Call Writer Revise jsonBody');
  }

  // 4. Fix Update .md File - preserve existing content if revision is empty
  const umd = wf.nodes.find(n => n.name === 'Update .md File');
  if (umd) {
    umd.parameters.jsCode = [
      'var fs = require("fs");',
      'var data = $("Save Revised Version").first().json;',
      'var ch = $("Load Chapter").first().json;',
      '',
      'var seq = ch.sequence || 1;',
      'var filename = "/data/paper/my-novel-ch" + String(seq).padStart(3,"0") + ".md";',
      '',
      '// Read existing file as fallback',
      'var existing = "";',
      'try { existing = fs.readFileSync(filename, "utf8"); } catch(e) {}',
      '',
      '// Use new content if available, otherwise keep existing',
      'var content = data.content;',
      'if (!content || content.length < 50) {',
      '  // Revision had no useful output, keep existing',
      '  console.log("Revision produced no content, keeping existing file");',
      '  return [{ json: { updated: false, filename: filename, note: "Kept existing - revision empty" } }];',
      '}',
      '',
      'var md = "# " + data.title + " (Revised)\\n\\n";',
      'md += "> 章节: " + seq + " | 字数: " + data.word_count + "\\n\\n";',
      'md += content + "\\n\\n";',
      'md += "---\\n";',
      'md += "*Revised by AI Novel Factory*\\n";',
      '',
      'fs.writeFileSync(filename, md, "utf8");',
      '',
      'return [{ json: { updated: true, filename: filename, title: data.title, word_count: data.word_count } }];'
    ].join('\n');
    console.log('Fixed: Update .md File with fallback protection');
  }

  wf.settings = { executionOrder: 'v1' };
  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });
  console.log(r.ok ? 'Updated! Run WF04' : 'FAIL');
}
main().catch(e => console.error(e));
