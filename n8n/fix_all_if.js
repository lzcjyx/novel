// Fix ALL IF nodes across all workflows for n8n v2.19.3 compatibility
// n8n v2.19.3 IF V1 valid operations:
//   number: smaller, smallerEqual, larger, largerEqual
//   string: contains, notContains, endsWith, isEmpty, isNotEmpty, regex, startsWith, equals
//   boolean: true, false  (NOT 'equals'!)
//   dateTime: before, after

const fs = require('fs');
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI1Y2JjODlhNS1iZmQyLTQ3YWQtODkxOC02M2E1NWZjNjdlNDQiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiY2FmOGVjMDktN2FhYS00ZTRmLWI0NDktOTRjYWZmYmRlNzAzIiwiaWF0IjoxNzc4MDc5NTgwfQ.9S7oWFJj7OHSXt31tbAtvt-N9olWrdIH9ld6TuBAZyg';

const files = [
  '01_novel_bootstrap.workflow.json',
  '02_bible_ingestion.workflow.json',
  '03_daily_chapter_production.workflow.json',
  '04_review_and_repair.workflow.json',
  '05_weekly_arc_planner.workflow.json',
];

for (const file of files) {
  const wf = JSON.parse(fs.readFileSync(file, 'utf-8'));
  let fixed = 0;

  for (const node of wf.nodes) {
    if (node.type !== 'n8n-nodes-base.if') continue;
    const conds = node.parameters.conditions;
    if (!conds) continue;

    // Fix boolean conditions: 'equals' is not valid, use 'true'/'false' directly
    if (conds.boolean) {
      const newBools = [];
      let changed = false;
      for (const c of conds.boolean) {
        if (c.operation === 'equals' && c.value2 === true) {
          newBools.push({ value1: c.value1, operation: 'true' });
          changed = true;
        } else if (c.operation === 'equals' && c.value2 === false) {
          newBools.push({ value1: c.value1, operation: 'false' });
          changed = true;
        } else {
          newBools.push(c);
        }
      }
      if (changed) {
        conds.boolean = newBools;
        fixed++;
        console.log('  Fixed boolean in:', node.name);
      }
    }

    // Fix number conditions: 'equals' is not valid, use smaller/larger combos
    if (conds.number) {
      const newNums = [];
      let changed = false;
      for (const c of conds.number) {
        if (c.operation === 'equals' && c.value2 === 0) {
          newNums.push({ value1: c.value1, operation: 'smaller', value2: 1 });
          changed = true;
        } else if (c.operation === 'equals' && c.value2 === 1) {
          newNums.push({ value1: c.value1, operation: 'larger', value2: 0 });
          changed = true;
        } else {
          newNums.push(c);
        }
      }
      if (changed) {
        conds.number = newNums;
        fixed++;
        console.log('  Fixed number in:', node.name);
      }
    }
  }

  if (fixed > 0) {
    // Clean settings
    if (wf.settings) {
      for (const k of Object.keys(wf.settings)) {
        if (wf.settings[k] === null) delete wf.settings[k];
      }
    }
    fs.writeFileSync(file, JSON.stringify(wf, null, 2));
    console.log(file + ': ' + fixed + ' IF nodes fixed');
  } else {
    console.log(file + ': no fixes needed');
  }
}

// Deploy all
async function deploy() {
  const nameToId = {};
  // First list existing NF workflows
  const list = await fetch('http://localhost:5678/api/v1/workflows?limit=20', {
    headers: { 'X-N8N-API-KEY': API_KEY }
  });
  const data = await list.json();
  for (const w of data.data) {
    if (w.name.startsWith('[NF]')) {
      nameToId[w.name] = w.id;
    }
  }

  for (const file of files) {
    const wf = JSON.parse(fs.readFileSync(file, 'utf-8'));
    const oldId = nameToId[wf.name];
    if (oldId) {
      await fetch('http://localhost:5678/api/v1/workflows/' + oldId, {
        method: 'DELETE', headers: { 'X-N8N-API-KEY': API_KEY }
      });
      console.log('Deleted old', wf.name, oldId.slice(0,8));
    }
    const payload = JSON.stringify({name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings});
    const resp = await fetch('http://localhost:5678/api/v1/workflows', {
      method: 'POST', headers: { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY }, body: payload
    });
    const result = await resp.json();
    console.log('Created', wf.name, '->', resp.ok ? 'OK ' + result.id.slice(0,8) : 'FAIL ' + (result.message || ''));
  }
}
deploy();
