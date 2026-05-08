const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

const newCode = [
  'var chapterPlan = $("Select Next Chapter Plan").first().json;',
  'var project = $("Load Active Project").first().json;',
  '',
  '// genre is a string in DB, not array',
  'var genre = project.genre || "";',
  'if (Array.isArray(genre)) genre = genre.join(" ");',
  '',
  '// tone is inside style_profile JSONB',
  'var style = {};',
  'try { style = typeof project.style_profile === "string" ? JSON.parse(project.style_profile) : (project.style_profile || {}); } catch(e) {}',
  'var tone = style.tone || "";',
  '',
  'var queryParts = [',
  '  chapterPlan.title || "",',
  '  (chapterPlan.plot_goals || []).join(" "),',
  '  genre,',
  '  tone',
  '].filter(Boolean);',
  '',
  'var retrievalQuery = queryParts.join(" ");',
  'return [{ json: { retrieval_query: retrievalQuery, project_id: project.id, chapter_plan: chapterPlan, top_k: 12 } }];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();
  const node = wf.nodes.find(n => n.name === 'Build Retrieval Query');
  if (node) { node.parameters.jsCode = newCode; console.log('Fixed'); }

  // Also check Build Writing Brief - likely has similar issues
  const bwb = wf.nodes.find(n => n.name === 'Build Writing Brief');
  if (bwb) {
    const code = bwb.parameters.jsCode;
    if (code.includes('project.genre.join') || code.includes('project.tone')) {
      console.log('Build Writing Brief also needs genre/tone fix');
    }
  }

  wf.settings = { executionOrder: 'v1' };
  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });
  console.log(r.ok ? 'Updated!' : 'FAIL');
}
main().catch(e => console.error(e));
