const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

const parseCode = [
  'const response = $input.first().json;',
  'let rawText = "";',
  '',
  'if (response.message && response.message.content) {',
  '  rawText = response.message.content;',
  '} else if (response.choices && response.choices[0] && response.choices[0].message) {',
  '  rawText = response.choices[0].message.content;',
  '} else if (typeof response === "string") {',
  '  rawText = response;',
  '} else {',
  '  rawText = JSON.stringify(response);',
  '}',
  '',
  'rawText = rawText.trim();',
  'if (rawText.startsWith("```")) {',
  '  var nl = rawText.indexOf("\\n");',
  '  if (nl >= 0) rawText = rawText.substring(nl + 1);',
  '}',
  'if (rawText.endsWith("```")) {',
  '  rawText = rawText.substring(0, rawText.length - 3).trim();',
  '}',
  '',
  'let plan;',
  'try {',
  '  plan = JSON.parse(rawText);',
  '} catch (e) {',
  '  throw new Error("Failed to parse arc plan. Raw (first 300): " + rawText.substring(0, 300));',
  '}',
  '',
  'const projectId = $("Load Active Project").first().json.id;',
  'const context = $("Build Planning Context").first().json;',
  '',
  'return [{',
  '  json: {',
  '    project_id: projectId,',
  '    plan: plan,',
  '    next_sequence_start: context.next_sequence_start,',
  '    has_human_review: (plan.human_review_required || []).length > 0',
  '  }',
  '}];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/RGbsc6NtL3uHqKSk', { headers: H });
  const wf = await resp.json();

  const node = wf.nodes.find(n => n.name === 'Parse Arc Plan');
  if (node) {
    node.parameters.jsCode = parseCode;
    console.log('Fixed Parse Arc Plan');
  }

  if (wf.settings) { for (const k of Object.keys(wf.settings)) { if (wf.settings[k] === null) delete wf.settings[k]; } }

  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r.ok ? 'Updated!' : 'FAIL');
}
main().catch(e => console.error(e));
