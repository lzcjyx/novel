// Deploy all 5 workflows + create DeepSeek credential
const fs = require('fs');
const crypto = require('crypto');

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

async function main() {
  // Step 1: Create DeepSeek credential
  console.log('=== Creating DeepSeek credential ===');
  // First check if credential already exists
  const credList = await fetch(BASE + '/credentials?limit=50', { headers: HEADERS });
  const creds = await credList.json();
  const existing = (creds.data || []).find(c => c.name === 'DeepSeek account');
  let credentialId;
  if (existing) {
    credentialId = existing.id;
    console.log('Existing credential:', credentialId.slice(0, 8));
  } else {
    // Create new credential
    const credPayload = {
      name: 'DeepSeek account',
      type: 'deepSeekApi',
      data: {
        apiKey: '{{$env.DEEPSEEK_API_KEY}}'
      }
    };
    const createResp = await fetch(BASE + '/credentials', {
      method: 'POST', headers: HEADERS, body: JSON.stringify(credPayload)
    });
    const created = await createResp.json();
    if (createResp.ok) {
      credentialId = created.data?.id || created.id;
      console.log('Created credential:', credentialId.slice(0, 8));
    } else {
      console.log('Failed to create credential:', JSON.stringify(created).slice(0, 200));
    }
  }

  // Step 2: Delete old workflows
  console.log('\n=== Cleaning old workflows ===');
  const list = await fetch(BASE + '/workflows?limit=50', { headers: HEADERS });
  const all = await list.json();
  for (const w of (all.data || [])) {
    await fetch(BASE + '/workflows/' + w.id, { method: 'DELETE', headers: HEADERS });
    console.log('Deleted:', w.id.slice(0, 8), w.name);
  }

  // Step 3: Deploy all workflows
  console.log('\n=== Deploying workflows ===');
  for (const file of files) {
    const wf = JSON.parse(fs.readFileSync(file, 'utf-8'));
    // Update credential ID reference
    for (const node of wf.nodes) {
      if (node.credentials?.deepSeekApi && credentialId) {
        node.credentials.deepSeekApi.id = credentialId;
      }
    }
    // Clean settings
    if (wf.settings) {
      for (const k of Object.keys(wf.settings)) {
        if (wf.settings[k] === null) delete wf.settings[k];
      }
    }
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
      console.log('OK', result.id?.slice(0, 8) || result.data?.id?.slice(0, 8), wf.name);
    } else {
      console.log('FAIL', file, JSON.stringify(result).slice(0, 200));
    }
  }

  console.log('\n=== Done ===');
}

main().catch(e => console.error(e));
