const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

const promptCode = [
  'var proj = $("Load Project").first().json;',
  'var input = $input.first().json;',
  'return [{ json: {',
  '  project: proj,',
  '  prompt_input: {',
  '    project: proj,',
  '    input_type: input.input_type || "chapter_update",',
  '    content: input.content || "",',
  '    title: input.title || "",',
  '    existing_canon: { characters: [], locations: [], items: [] }',
  '  },',
  '  system_prompt: "You are a novel editor extracting canon from new content. Output valid JSON only.",',
  '  prompt: JSON.stringify({ project: proj, input: input, task: "extract_canon" })',
  '}}];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/aCN37gN7682p0h9W', { headers: H });
  const wf = await resp.json();

  const bep = wf.nodes.find(n => n.name === 'Build Extraction Prompt');
  if (bep) {
    bep.parameters.jsCode = promptCode;
    console.log('Fixed Build Extraction Prompt');
  }

  // Also fix DeepSeek user message to use the right field
  const ds = wf.nodes.find(n => n.type === 'n8n-nodes-deepseek.deepSeek');
  if (ds && ds.parameters.prompt && ds.parameters.prompt.messages) {
    const userMsg = ds.parameters.prompt.messages.find(m => m.role === 'user');
    if (userMsg) {
      userMsg.content = '={{ JSON.stringify($json.prompt_input) }}';
      console.log('Fixed DeepSeek user message');
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
