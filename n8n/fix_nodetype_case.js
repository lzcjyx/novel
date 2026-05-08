// Fix DeepSeek node type case: deepseek -> deepSeek
const fs = require('fs');
const path = require('path');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

const files = [
  '01_novel_bootstrap.workflow.json',
  '02_bible_ingestion.workflow.json',
  '03_daily_chapter_production.workflow.json',
  '04_review_and_repair.workflow.json',
  '05_weekly_arc_planner.workflow.json',
];

const WRONG = 'n8n-nodes-deepseek.deepseek';
const CORRECT = 'n8n-nodes-deepseek.deepSeek';

let totalFixes = 0;

for (const file of files) {
  let content = fs.readFileSync(dir + file, 'utf-8');
  let count = 0;
  while (content.includes(WRONG)) {
    content = content.replace(WRONG, CORRECT);
    count++;
  }
  if (count > 0) {
    fs.writeFileSync(dir + file, content);
    console.log(`Fixed ${count} occurrences in ${file}`);
    totalFixes += count;
  } else {
    console.log(`No fixes needed in ${file}`);
  }
}
console.log(`\nTotal fixes: ${totalFixes}`);

// Redeploy
async function redeploy() {
  console.log('\n--- Redeploying ---');
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
      console.log('OK', result.id?.slice(0, 8) || result.data?.id?.slice(0, 8), wf.name);
    } else {
      console.log('FAIL', file, JSON.stringify(result).slice(0, 200));
    }
  }
  console.log('Done');
}

redeploy().catch(e => console.error(e));
