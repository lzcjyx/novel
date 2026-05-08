// Replace Generate Embeddings HTTP Request with Code node (placeholder vector)
// DeepSeek doesn't support embeddings API, so we generate a zero vector
const fs = require('fs');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };
const WF_ID = 'CtEvELW9y6XbmbVi';

async function main() {
  const resp = await fetch(BASE + '/workflows/' + WF_ID, { headers: HEADERS });
  const wf = await resp.json();

  // Find the Generate Embeddings node
  const embNode = wf.nodes.find(n => n.name === 'Generate Embeddings');
  if (!embNode) { console.log('Generate Embeddings node not found'); return; }

  // Convert from HTTP Request to Code node
  embNode.type = 'n8n-nodes-base.code';
  embNode.typeVersion = 2;
  embNode.parameters = {
    jsCode: [
      '// Generate placeholder embedding vector (1536 dimensions of zeros)',
      '// Replace with real OpenAI embedding when API key is available',
      'const items = $input.all();',
      'return items.map(function(item) {',
      '  var vec = new Array(1536).fill(0);',
      '  return {',
      '    json: {',
      '      ...item.json,',
      '      embedding: vec,',
      '      embedding_model: \x27placeholder-zero-vector\x27',
      '      embedding_dimension: 1536',
      '    }',
      '  };',
      '});'
    ].join('\n')
  };
  delete embNode.credentials;
  console.log('Converted Generate Embeddings to Code node (placeholder vector)');

  if (wf.settings) {
    for (const k of Object.keys(wf.settings)) { if (wf.settings[k] === null) delete wf.settings[k]; }
  }

  const updateResp = await fetch(BASE + '/workflows/' + WF_ID, {
    method: 'PUT',
    headers: HEADERS,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });

  if (updateResp.ok) {
    console.log('Workflow updated!');
    fs.writeFileSync(dir + '01_novel_bootstrap.workflow.json', JSON.stringify(wf, null, 2));
  } else {
    const err = await updateResp.json();
    console.log('FAIL:', JSON.stringify(err).slice(0, 300));
  }
}

main().catch(e => console.error(e));
