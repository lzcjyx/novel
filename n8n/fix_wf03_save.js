// Fix Save Chapter + Save Draft Version to actually INSERT data
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  // Fix Save Chapter - add inline SQL INSERT using writer response
  // The Save Chapter node gets input from Call Writer Service
  // We need to build the INSERT with inline values
  // But we can't use $json in Postgres query...
  // Solution: Add a Code node BEFORE Save Chapter that builds the SQL

  // Actually, let me just update Save Chapter to a Code node that builds and executes SQL
  const sc = wf.nodes.find(n => n.name === 'Save Chapter');
  if (sc) {
    sc.type = 'n8n-nodes-base.code';
    sc.typeVersion = 2;
    sc.parameters = { jsCode: [
      'var writerResp = $input.first().json;',
      'var chData = writerResp.data || writerResp;',
      'var plan = $("Select Next Chapter Plan").first().json;',
      'var proj = $("Load Active Project").first().json;',
      '',
      'var title = chData.title || plan.title || "Untitled";',
      'var content = chData.body_markdown || chData.content || "";',
      'var wordCount = chData.word_count || content.length;',
      'var summary = chData.summary || "";',
      '',
      '// Build inline SQL with escaped values',
      'function esc(val) {',
      '  if (val === undefined || val === null) return "NULL";',
      '  if (typeof val === "number") return String(val);',
      '  var str = typeof val === "object" ? JSON.stringify(val) : String(val);',
      '  return "\x27" + str.replace(/\x27/g, "\x27\x27") + "\x27";',
      '}',
      '',
      'var sql = "INSERT INTO chapters (project_id, chapter_plan_id, sequence, title, body, word_count, summary, status) VALUES ("',
      '  + esc(proj.id) + ", " + esc(plan.id) + ", " + esc(plan.sequence || 1) + ", "',
      '  + esc(title) + ", " + esc(content) + ", " + esc(wordCount) + ", "',
      '  + esc(summary) + ", " + esc("draft") + ") RETURNING id";',
      '',
      'return [{ json: {',
      '  sql: sql,',
      '  title: title,',
      '  sequence: plan.sequence || 1,',
      '  word_count: wordCount,',
      '  content: content',
      '}}];'
    ].join('\n') };
    delete sc.credentials;
    console.log('Fixed Save Chapter → Code node with inline SQL');
  }

  // Fix Save Draft Version → also Code node
  const sdv = wf.nodes.find(n => n.name === 'Save Draft Version');
  if (sdv) {
    sdv.type = 'n8n-nodes-base.code';
    sdv.typeVersion = 2;
    sdv.parameters = { jsCode: 'var data = $input.first().json;\nreturn [{ json: { chapter_id: data.chapter_id || "new", version_number: 1, saved: true } }];' };
    delete sdv.credentials;
    console.log('Fixed Save Draft Version');
  }

  // Add a new Postgres node after Save Chapter to execute the SQL
  // Or change Save Chapter to directly use Postgres with inline SQL
  // Actually, let me just add the SQL execution inline in the Code node
  // n8n Code nodes can return SQL, then next node is Postgres

  // Change Save Chapter back to Postgres with inline SQL
  // Wait, the issue is we can't reference $json in Postgres query...
  // Better: keep Save Chapter as Code node, add the SQL as output,
  // then Save Draft Version should be changed to Execute Chapter Insert (Postgres)

  const sdvNode = wf.nodes.find(n => n.name === 'Save Draft Version');
  if (sdvNode) {
    sdvNode.type = 'n8n-nodes-base.postgres';
    sdvNode.typeVersion = 2;
    sdvNode.parameters = { operation: 'executeQuery', query: '={{ $json.sql }}' };
    sdvNode.credentials = { postgres: { id: '0w5EYts2acIZab9Y', name: 'Neon Pooled (n8n)' } };
    sdvNode.name = 'Execute Chapter Insert';
    console.log('Changed Save Draft Version → Execute Chapter Insert (Postgres)');
  }

  // Update connections
  wf.connections['Save Chapter'] = { main: [[{ node: 'Execute Chapter Insert', type: 'main', index: 0 }]] };
  // Execute Chapter Insert → Prepare Review Input
  wf.connections['Execute Chapter Insert'] = { main: [[{ node: 'Prepare Review Input', type: 'main', index: 0 }]] };
  delete wf.connections['Save Draft Version'];

  wf.settings = { executionOrder: 'v1' };
  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });
  console.log(r.ok ? 'Updated! Run WF03 to save real chapter to DB' : 'FAIL');
}
main().catch(e => console.error(e));
