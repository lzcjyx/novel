const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();
  const node = wf.nodes.find(n => n.name === 'Build Writing Brief');

  // Read current code, fix genre/tone references
  let code = node.parameters.jsCode;

  // Fix: extract tone from style_profile, handle genre as string
  // Add after var project = ... line
  code = code.replace(
    'const project = $(\'Load Active Project\').first().json;',
    'var project = $("Load Active Project").first().json;\n// Parse style_profile for tone\nvar style = {};\ntry { style = typeof project.style_profile === "string" ? JSON.parse(project.style_profile) : (project.style_profile || {}); } catch(e) {}\nproject.tone = style.tone || "";\n// genre is string, not array\nif (Array.isArray(project.genre)) project.genre = project.genre.join(", ");'
  );

  // Also remove references to project.title (should be project.name)
  code = code.replace(/project\.title/g, 'project.name');

  node.parameters.jsCode = code;
  console.log('Fixed Build Writing Brief');

  wf.settings = { executionOrder: 'v1' };
  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });
  console.log(r.ok ? 'Updated!' : 'FAIL');
}
main().catch(e => console.error(e));
