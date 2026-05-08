const fs = require('fs');
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI1Y2JjODlhNS1iZmQyLTQ3YWQtODkxOC02M2E1NWZjNjdlNDQiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiY2FmOGVjMDktN2FhYS00ZTRmLWI0NDktOTRjYWZmYmRlNzAzIiwiaWF0IjoxNzc4MDc5NTgwfQ.9S7oWFJj7OHSXt31tbAtvt-N9olWrdIH9ld6TuBAZyg';

const wf = JSON.parse(fs.readFileSync('01_novel_bootstrap.workflow.json', 'utf-8'));

// Fix the Schema Missing? IF node condition
// n8n v2.19.3 IF V1 number type does NOT support 'equals' — only 'smaller'/'larger'/'smallerEqual'/'largerEqual'
// Use smaller: schema_ready < 1 means schema_ready = 0 → schema missing
const ifNode = wf.nodes.find(n => n.name === 'Schema Missing?');
ifNode.parameters.conditions = {
  number: [
    {
      value1: '={{ $json.schema_ready }}',
      operation: 'smaller',
      value2: 1
    }
  ]
};
console.log('Fixed conditions:', JSON.stringify(ifNode.parameters.conditions));

// Check all other IF nodes for similar issues
for (const node of wf.nodes) {
  if (node.type === 'n8n-nodes-base.if' && node.parameters.conditions) {
    const checkFix = (obj) => {
      if (typeof obj !== 'object' || obj === null) return;
      for (const [k, v] of Object.entries(obj)) {
        if (k === 'value1' && typeof v === 'string' && v.includes('={{ .')) {
          console.log('Found broken expression in', node.name + ':', v);
        }
        if (Array.isArray(v)) v.forEach(checkFix);
        else if (typeof v === 'object') checkFix(v);
      }
    };
    checkFix(node.parameters.conditions);
  }
}

// Save local
for (const k of Object.keys(wf.settings)) {
  if (wf.settings[k] === null) delete wf.settings[k];
}
fs.writeFileSync('01_novel_bootstrap.workflow.json', JSON.stringify(wf, null, 2));
console.log('Saved locally.');

// Deploy
async function deploy() {
  const payload = JSON.stringify({name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings});
  const resp = await fetch('http://localhost:5678/api/v1/workflows', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY },
    body: payload
  });
  const data = await resp.json();
  console.log('Deploy status:', resp.status);
  if (resp.ok) {
    console.log('New ID:', data.id);
    // Fetch to verify
    const vrfy = await fetch('http://localhost:5678/api/v1/workflows/' + data.id, {
      headers: { 'X-N8N-API-KEY': API_KEY }
    });
    const vdata = await vrfy.json();
    const ifn = vdata.nodes.find(n => n.name === 'Schema Missing?');
    console.log('Deployed IF conditions:', JSON.stringify(ifn.parameters.conditions));
  } else {
    console.log('Error:', JSON.stringify(data).slice(0,300));
  }
}
deploy();
