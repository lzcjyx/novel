// Add JSON repair to Parse Bible Response
const fs = require('fs');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };
const WF_ID = 'CtEvELW9y6XbmbVi';

const newCode = [
  'const response = $input.first().json;',
  'let rawText = "";',
  '',
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
  'if (rawText.startsWith("```")) {',
  '  var idx = rawText.indexOf("\\n");',
  '  if (idx >= 0) rawText = rawText.substring(idx + 1);',
  '}',
  'if (rawText.endsWith("```")) {',
  '  rawText = rawText.substring(0, rawText.length - 3).trim();',
  '}',
  '',
  '// ---- JSON repair for common LLM errors ----',
  'function repairJSON(text) {',
  '  // 1. Quote unquoted values after colon: "key": value -> "key": "value"',
  '  // Matches pattern: colon + whitespace + unquoted non-JSON value',
  '  text = text.replace(/:\\s*([^"\\[\\{\\d\\-tf][^\n,\\]}]*)/g, function(match, p1) {',
  '    var val = p1.trim();',
  '    // Skip if already quoted, is null/true/false, number, array, or object',
  '    if (val === "" || val === "null" || val === "true" || val === "false") return match;',
  '    if (/^-?\\d/.test(val)) return match;',
  '    // Quote the value',
  '    return ": \\"" + val.replace(/"/g, \x27\\\\"\x27) + "\\"";',
  '  });',
  '  // 2. Remove trailing commas before } or ]',
  '  text = text.replace(/,(\\s*[}\\]])/g, "$1");',
  '  return text;',
  '}',
  '',
  'let bible;',
  'try {',
  '  bible = JSON.parse(rawText);',
  '} catch (e1) {',
  '  // Try repair',
  '  try {',
  '    var repaired = repairJSON(rawText);',
  '    bible = JSON.parse(repaired);',
  '  } catch (e2) {',
  '    var len = rawText.length;',
  '    var prefix = rawText.substring(0, 400);',
  '    var suffix = rawText.substring(Math.max(0, len - 400));',
  '    throw new Error(',
  '      "JSON parse failed after repair. Length=" + len + ". " +',
  '      "First 400: " + prefix + ". " +',
  '      "Last 400: " + suffix + ". " +',
  '      "Original error: " + e1.message + ". " +',
  '      "Repair error: " + e2.message',
  '    );',
  '  }',
  '}',
  '',
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
    pbr.parameters.jsCode = newCode;
    console.log('Updated: Parse Bible Response with JSON repair');
  }

  if (wf.settings) {
    for (const k of Object.keys(wf.settings)) { if (wf.settings[k] === null) delete wf.settings[k]; }
  }

  const updateResp = await fetch(BASE + '/workflows/' + WF_ID, {
    method: 'PUT',
    headers: HEADERS,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
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
