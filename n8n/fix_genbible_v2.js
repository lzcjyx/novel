// Fix Gen Bible: maxTokens to 32768 + better error diagnostics in Parse
const fs = require('fs');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };
const WF_ID = 'CtEvELW9y6XbmbVi';

const betterParse = [
  'const response = $input.first().json;',
  'let rawText = "";',
  '',
  '// DeepSeek community node outputs: {index, message: {role, content}, logprobs, finish_reason}',
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
  'rawText = rawText.trim();',
  '',
  '// Strip markdown code blocks',
  'rawText = rawText.replace(/^[\\s]*```[a-z]*[\\s]*[\\n]?/, "");',
  'rawText = rawText.replace(/[\\n]?```[\\s]*$/, "");',
  'rawText = rawText.trim();',
  '',
  '// Fix: AI sometimes wraps JSON in curly quotes or includes BOM',
  'rawText = rawText.replace(/^[\\u200B\\uFEFF]+/, "");',
  '',
  'let bible;',
  'try {',
  '  bible = JSON.parse(rawText);',
  '} catch (e) {',
  '  const prefix = rawText.substring(0, 400);',
  '  const suffix = rawText.substring(Math.max(0, rawText.length - 400));',
  '  const len = rawText.length;',
  '  throw new Error(',
  '    "JSON parse failed. Length=" + len + ". ',
  '    + "First 400: " + prefix + ". ',
  '    + "Last 400: " + suffix + ". ',
  '    + "Error: " + e.message',
  '  );',
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

  // Fix Gen Bible maxTokens
  const gb = wf.nodes.find(n => n.name === 'Gen Bible via OpenAI');
  if (gb) {
    gb.parameters.options.maxTokens = 32768;
    console.log('Gen Bible: maxTokens -> 32768');
  }

  // Fix Parse Bible Response with better error diagnostics
  const pbr = wf.nodes.find(n => n.name === 'Parse Bible Response');
  if (pbr) {
    pbr.parameters.jsCode = betterParse;
    console.log('Parse Bible: better error diagnostics');
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
