const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  // Fix Prepare Review Input - references stale node names
  const pri = wf.nodes.find(n => n.name === 'Prepare Review Input');
  if (pri) {
    pri.parameters.jsCode = [
      'var ch = $("Save Chapter").first().json;',
      'var plan = $("Select Next Chapter Plan").first().json;',
      '',
      'return [{ json: {',
      '  chapter_id: ch.id || "new",',
      '  title: ch.title,',
      '  sequence: plan.sequence || 1,',
      '  word_count: ch.word_count || 0,',
      '  content: ch.content || ""',
      '}}];'
    ].join('\n');
    console.log('Fixed Prepare Review Input');
  }

  // Fix Collect All Reviews - references stale nodes
  const car = wf.nodes.find(n => n.name === 'Collect All Reviews');
  if (car) {
    let code = car.parameters.jsCode;
    // Replace Save Draft Version with Execute Chapter Insert
    code = code.replace(/Save Draft Version/g, 'Execute Chapter Insert');
    // Replace Build Writing Brief refs if any
    car.parameters.jsCode = code;
    console.log('Fixed Collect All Reviews node refs');
  }

  // Fix Build Markdown Output - references stale nodes
  const bmo = wf.nodes.find(n => n.name === 'Build Markdown Output');
  if (bmo) {
    let code2 = bmo.parameters.jsCode;
    code2 = code2.replace(/Save Draft Version/g, 'Save Chapter');
    bmo.parameters.jsCode = code2;
    console.log('Fixed Build Markdown Output refs');
  }

  wf.settings = { executionOrder: 'v1' };
  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });
  console.log(r.ok ? 'Updated!' : 'FAIL');
}
main().catch(e => console.error(e));
