// Update all workflow JSON files: replace old PostgreSQL credential ID with new one
const fs = require('fs');

const OLD_PG_ID = 'FFfm0dWYLSjUyWk8';
const NEW_PG_ID = '0w5EYts2acIZab9Y';
const NEW_DEEPSEEK_ID = 'si5XfG0zsr4yuxSz';
const OLD_DEEPSEEK_ID = 'O06O4ws1byCbKxp5';
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

// Step 1: Update local JSON files
for (const file of files) {
  const content = fs.readFileSync(file, 'utf-8');
  let updated = content;
  // Replace old PG credential with new one
  while (updated.includes(OLD_PG_ID)) {
    updated = updated.replace(OLD_PG_ID, NEW_PG_ID);
  }
  // Replace old DeepSeek credential with new one
  while (updated.includes(OLD_DEEPSEEK_ID)) {
    updated = updated.replace(OLD_DEEPSEEK_ID, NEW_DEEPSEEK_ID);
  }
  if (updated !== content) {
    fs.writeFileSync(file, updated);
    console.log('Updated:', file);
  } else {
    console.log('No changes:', file);
  }
}

// Step 2: Redeploy all workflows
async function redeploy() {
  // Get existing workflows
  const list = await fetch(BASE + '/workflows?limit=20', { headers: HEADERS });
  const all = await list.json();

  for (const file of files) {
    const wf = JSON.parse(fs.readFileSync(file, 'utf-8'));
    // Find existing workflow by name
    const old = (all.data || []).find(w => w.name === wf.name);
    if (old) {
      // Delete old
      await fetch(BASE + '/workflows/' + old.id, { method: 'DELETE', headers: HEADERS });
    }
    // Clean settings
    if (wf.settings) {
      for (const k of Object.keys(wf.settings)) {
        if (wf.settings[k] === null) delete wf.settings[k];
      }
    }
    // Create new
    const payload = JSON.stringify({
      name: wf.name,
      nodes: wf.nodes,
      connections: wf.connections,
      settings: wf.settings
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
