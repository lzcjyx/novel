// Fix Parse Bible Response: DeepSeek node outputs the choice object directly
const fs = require('fs');
const path = require('path');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

const WF_ID = 'CtEvELW9y6XbmbVi';

const newParseCode = [
  'const response = $input.first().json;',
  'let rawText = "";',
  '',
  '// DeepSeek community node outputs the FIRST choice object directly:',
  '// {index, message: {role, content}, logprobs, finish_reason}',
  '// NOT the full OpenAI response {choices: [...]}',
  'if (response.message && response.message.content) {',
  '  rawText = response.message.content;',
  '} else if (response.choices && response.choices[0] && response.choices[0].message) {',
  '  rawText = response.choices[0].message.content;',
  '} else if (typeof response === "string") {',
  '  rawText = response;',
  '} else {',
  '  rawText = JSON.stringify(response);',
  '}',
  '',
  '// Strip markdown code blocks if present',
  'rawText = rawText.trim();',
  'rawText = rawText.replace(/^\\s*```[a-z]*\\s*\\n?/, "").replace(/\\n?```\\s*$/, "");',
  '',
  'let bible;',
  'try {',
  '  bible = JSON.parse(rawText);',
  '} catch (e) {',
  '  throw new Error("Failed to parse bible JSON. Raw (first 500): " + rawText.substring(0, 500));',
  '}',
  '',
  '// Validate required fields',
  'if (!bible.world_overview || !bible.characters || !bible.main_plot_threads) {',
  '  throw new Error("Bible missing required fields. Got: " + Object.keys(bible).join(", "));',
  '}',
  '',
  'return [{ json: { project: $("Create Project").first().json, bible: bible } }];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/' + WF_ID, { headers: HEADERS });
  const wf = await resp.json();

  const pbr = wf.nodes.find(n => n.name === 'Parse Bible Response');
  if (pbr) {
    pbr.parameters.jsCode = newParseCode;
    console.log('Updated: Parse Bible Response');
  }

  if (wf.settings) {
    for (const k of Object.keys(wf.settings)) {
      if (wf.settings[k] === null) delete wf.settings[k];
    }
  }

  const updateResp = await fetch(BASE + '/workflows/' + WF_ID, {
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
    console.log('Workflow updated!');
    fs.writeFileSync(dir + '01_novel_bootstrap.workflow.json', JSON.stringify(wf, null, 2));
  } else {
    const err = await updateResp.json();
    console.log('FAIL:', JSON.stringify(err).slice(0, 300));
  }
}

main().catch(e => console.error(e));
