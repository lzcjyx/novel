// Aggressively slim ALL Code nodes to get under n8n editor size limit
const fs = require('fs');

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

function minifyJS(code) {
  // Remove comment lines (// ...)
  code = code.replace(/^\s*\/\/.*$/gm, '');
  // Remove block comments
  code = code.replace(/\/\*[\s\S]*?\*\//g, '');
  // Remove blank lines
  code = code.replace(/^\s*\n/gm, '');
  // Remove trailing whitespace
  code = code.replace(/[ \t]+$/gm, '');
  // Collapse multiple blank lines
  code = code.replace(/\n{3,}/g, '\n\n');
  return code.trim();
}

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: HEADERS });
  const wf = await resp.json();

  let totalSaved = 0;
  for (const node of wf.nodes) {
    if (node.type === 'n8n-nodes-base.code' && node.parameters.jsCode) {
      const old = node.parameters.jsCode;
      const slim = minifyJS(old);
      const saved = old.length - slim.length;
      if (saved > 50) {
        node.parameters.jsCode = slim;
        totalSaved += saved;
        console.log('Slimmed:', node.name, old.length, '->', slim.length, '(saved', saved, ')');
      }
    }
  }
  console.log('Total saved:', totalSaved, 'bytes');

  if (wf.settings) { for (const k of Object.keys(wf.settings)) { if (wf.settings[k] === null) delete wf.settings[k]; } }

  const updateResp = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: HEADERS,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });

  if (updateResp.ok) {
    const sz = JSON.stringify({nodes:wf.nodes,connections:wf.connections}).length;
    console.log('WF03 updated! New size:', sz, 'bytes');
    fs.writeFileSync('D:/novel/n8n/03_daily_chapter_production.workflow.json', JSON.stringify({name:wf.name,nodes:wf.nodes,connections:wf.connections,settings:wf.settings}, null, 2));
  } else {
    const err = await updateResp.json();
    console.log('FAIL:', JSON.stringify(err).slice(0, 300));
  }
}

main().catch(e => console.error(e));
