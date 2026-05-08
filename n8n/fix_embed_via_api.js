// Fix embeddings HTTP Request nodes via the n8n API
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

const OPENAI_CRED = { id: 'BGOK2MJV1ZG3nBmK', name: 'OpenAI API' };

async function main() {
  // Get current workflow IDs
  const listResp = await fetch(BASE + '/workflows?limit=10', { headers: HEADERS });
  const list = await listResp.json();
  const nameToId = {};
  for (const w of (list.data || [])) {
    nameToId[w.name] = w;
  }

  // Fix each workflow
  for (const [name, meta] of Object.entries(nameToId)) {
    // Get full workflow
    const resp = await fetch(BASE + '/workflows/' + meta.id, { headers: HEADERS });
    const wf = await resp.json();

    let changed = false;

    for (const node of (wf.nodes || [])) {
      if (node.type === 'n8n-nodes-base.httpRequest') {
        const cred = node.credentials?.httpHeaderAuth;
        // Fix if it references the wrong credential (DeepSeek API as httpHeaderAuth)
        if (cred && cred.id === 'si5XfG0zsr4yuxSz') {
          console.log('Fixing', node.name, 'in', name);
          node.credentials.httpHeaderAuth = { ...OPENAI_CRED };
          changed = true;
        }
      }
    }

    if (changed) {
      // Clean settings
      if (wf.settings) {
        for (const k of Object.keys(wf.settings)) {
          if (wf.settings[k] === null) delete wf.settings[k];
        }
      }
      // Update workflow
      const updateResp = await fetch(BASE + '/workflows/' + meta.id, {
        method: 'PUT',
        headers: HEADERS,
        body: JSON.stringify({
          name: wf.name,
          nodes: wf.nodes,
          connections: wf.connections,
          settings: wf.settings
        })
      });
      if (updateResp.ok) {
        console.log('  -> OK');
      } else {
        const err = await updateResp.json();
        console.log('  -> FAIL', JSON.stringify(err).slice(0, 200));
      }
    } else {
      console.log('No httpRequest fix needed in', name);
    }
  }

  console.log('\nDone');
}

main().catch(e => console.error(e));
