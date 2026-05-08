const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

// Build the save code as a simple string with everything properly quoted
const saveCode = 'var writerResp = $input.first().json;\n' +
  'var chData = writerResp.data || writerResp;\n' +
  'var plan = $("Select Next Chapter Plan").first().json;\n' +
  'var proj = $("Load Active Project").first().json;\n' +
  'var title = chData.title || plan.title || "Untitled";\n' +
  'var content = chData.body_markdown || chData.content || "";\n' +
  'var wordCount = chData.word_count || content.length;\n' +
  'function esc(val) {\n' +
  '  if (val === undefined || val === null) return "NULL";\n' +
  '  if (typeof val === "number") return String(val);\n' +
  '  var str = typeof val === "object" ? JSON.stringify(val) : String(val);\n' +
  '  return "\x27" + str.replace(/\x27/g, "\x27\x27") + "\x27";\n' +
  '}\n' +
  'var seq = plan.sequence || 1;\n' +
  'var sql = "WITH ins_ch AS (" +\n' +
  '  "INSERT INTO chapters (project_id, chapter_plan_id, sequence, title, status, word_count) VALUES (" +\n' +
  '  esc(proj.id) + ", " + esc(plan.id) + ", " + esc(seq) + ", " +\n' +
  '  esc(title) + ", \x27draft\x27, " + esc(wordCount) +\n' +
  '  ") RETURNING id" +\n' +
  '  "), ins_ver AS (" +\n' +
  '  "INSERT INTO chapter_versions (chapter_id, project_id, version_number, version_type, title, body_markdown, word_count)" +\n' +
  '  " SELECT id, " + esc(proj.id) + ", 1, \x27draft\x27, " + esc(title) + ", " + esc(content) + ", " + esc(wordCount) + " FROM ins_ch RETURNING id, chapter_id" +\n' +
  '  ") UPDATE chapters SET final_version_id = ins_ver.id FROM ins_ver WHERE chapters.id = ins_ver.chapter_id RETURNING chapters.id AS chapter_id, ins_ver.id AS version_id";\n' +
  'return [{ json: { title: title, sequence: seq, word_count: wordCount, sql: sql } }];';

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  const sc = wf.nodes.find(n => n.name === 'Save Chapter');
  if (sc) {
    sc.parameters.jsCode = saveCode;
    console.log('Fixed Save Chapter');
  }

  wf.settings = { executionOrder: 'v1' };
  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });
  console.log(r.ok ? 'Updated!' : 'FAIL');
}
main().catch(e => console.error(e));
