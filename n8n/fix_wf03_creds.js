// Fix WF03 credential issues causing blank canvas
const fs = require('fs');

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };
const WF_ID = 'ufROdOe1mPpXn77j';

async function main() {
  const resp = await fetch(BASE + '/workflows/' + WF_ID, { headers: HEADERS });
  const wf = await resp.json();

  let fixes = 0;

  for (const node of wf.nodes) {
    // Fix 1: Publish to WordPress - string credential -> null (remove)
    if (node.name === 'Publish to WordPress' && node.credentials) {
      if (typeof node.credentials.httpHeaderAuth === 'string') {
        delete node.credentials.httpHeaderAuth;
        fixes++;
        console.log('Fixed: Publish to WordPress - removed broken credential string');
      }
    }

    // Fix 2: Embed Retrieval Query - wrong credential type (httpHeaderAuth with DeepSeek ID)
    if (node.name === 'Embed Retrieval Query' && node.credentials?.httpHeaderAuth) {
      if (node.credentials.httpHeaderAuth.id === 'si5XfG0zsr4yuxSz') {
        node.credentials.httpHeaderAuth = { id: 'BGOK2MJV1ZG3nBmK', name: 'OpenAI API' };
        fixes++;
        console.log('Fixed: Embed Retrieval Query -> OpenAI API credential');
      }
    }

    // Fix 3: Check for any other string-type credential values
    if (node.credentials) {
      for (const [key, val] of Object.entries(node.credentials)) {
        if (typeof val === 'string') {
          console.log('WARNING:', node.name, 'has string credential:', key, '=', val);
          delete node.credentials[key];
          fixes++;
        }
      }
    }
  }

  console.log('Total fixes:', fixes);

  if (fixes > 0) {
    if (wf.settings) {
      for (const k of Object.keys(wf.settings)) { if (wf.settings[k] === null) delete wf.settings[k]; }
    }
    const updateResp = await fetch(BASE + '/workflows/' + WF_ID, {
      method: 'PUT',
      headers: HEADERS,
      body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
    });
    if (updateResp.ok) {
      console.log('WF03 updated!');
      fs.writeFileSync('D:/novel/n8n/03_daily_chapter_production.workflow.json', JSON.stringify(wf, null, 2));
    } else {
      const err = await updateResp.json();
      console.log('FAIL:', JSON.stringify(err).slice(0, 300));
    }
  }
}

main().catch(e => console.error(e));
