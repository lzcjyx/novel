// Fix embeddings HTTP Request nodes: use proper httpHeaderAuth credential
const fs = require('fs');
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

const OPENAI_CRED_ID = 'BGOK2MJV1ZG3nBmK';
const WRONG_HTTP_CRED_ID = 'si5XfG0zsr4yuxSz'; // DeepSeek cred used as httpHeaderAuth

const dir = __dirname + '/';
const files = [
  '01_novel_bootstrap.workflow.json',
  '03_daily_chapter_production.workflow.json',
  '04_review_and_repair.workflow.json',
];

let fixes = 0;

for (const file of files) {
  const content = fs.readFileSync(dir + file, 'utf-8');
  let updated = content;

  // Fix: httpHeaderAuth nodes referencing DeepSeek credential -> use OpenAI credential
  // Pattern: "httpHeaderAuth": { "id": "si5XfG0zsr4yuxSz", "name": "DeepSeek API" }
  const oldPattern = '"httpHeaderAuth":{"id":"' + WRONG_HTTP_CRED_ID + '","name":"DeepSeek API"}';
  const newPattern = '"httpHeaderAuth":{"id":"' + OPENAI_CRED_ID + '","name":"OpenAI API"}';

  while (updated.includes(oldPattern)) {
    updated = updated.replace(oldPattern, newPattern);
    fixes++;
    console.log('  Fixed httpHeaderAuth ref in', file);
  }

  if (updated !== content) {
    fs.writeFileSync(dir + file, updated);
  }
}

console.log('Total fixes:', fixes);

// Redeploy the modified workflows
async function redeploy() {
  const list = await fetch(BASE + '/workflows?limit=20', { headers: HEADERS });
  const all = await list.json();

  for (const file of files) {
    const wf = JSON.parse(fs.readFileSync(dir + file, 'utf-8'));
    const old = (all.data || []).find(w => w.name === wf.name);
    if (old) {
      await fetch(BASE + '/workflows/' + old.id, { method: 'DELETE', headers: HEADERS });
    }
    if (wf.settings) {
      for (const k of Object.keys(wf.settings)) {
        if (wf.settings[k] === null) delete wf.settings[k];
      }
    }
    const payload = JSON.stringify({
      name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings
    });
    const resp = await fetch(BASE + '/workflows', {
      method: 'POST', headers: HEADERS, body: payload
    });
    const result = await resp.json();
    if (resp.ok) {
      console.log('OK', result.id?.slice(0, 8), wf.name);
    } else {
      console.log('FAIL', file, JSON.stringify(result).slice(0, 200));
    }
  }
}

redeploy().catch(e => console.error(e));
