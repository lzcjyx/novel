// Fix: Simple JSON repair for LLM unquoted Chinese values
const fs = require('fs');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };
const WF_ID = 'CtEvELW9y6XbmbVi';

// Build code via JSON.stringify to avoid all escaping issues
const parseCode = JSON.stringify([
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
  '  var nl = rawText.indexOf("\\n");',
  '  if (nl >= 0) rawText = rawText.substring(nl + 1);',
  '}',
  'if (rawText.endsWith("```")) {',
  '  rawText = rawText.substring(0, rawText.length - 3).trim();',
  '}',
  '',
  '// JSON repair: quote unquoted values after colon',
  '// Fixes LLM errors like "age": \\u672a\\u77e5 -> "age": "\\u672a\\u77e5"',
  'function repairJSON(text) {',
  '  // Pattern: colon+space followed by non-quote, non-bracket, non-digit, non-t/f/n chars',
  '  // Match colon, spaces, then capture unquoted text until comma/newline/closing bracket',
  '  var fixed = text.replace(/:\\s*([^"\\[\\{\\-\\dtnf][^,\\n\\}\\]\\)]*)/g, function(m, val) {',
  '    var v = val.trim();',
  '    if (v === "" || v === "true" || v === "false" || v === "null") {',
  '      return ": " + v;',
  '    }',
  '    if (/^-?[\\d]/.test(v)) {',
  '      return ": " + v;',
  '    }',
  '    return ": " + JSON.stringify(v);',
  '  });',
  '  return fixed;',
  '}',
  '',
  'let bible;',
  'try {',
  '  bible = JSON.parse(rawText);',
  '} catch (e1) {',
  '  try {',
  '    var repaired = repairJSON(rawText);',
  '    bible = JSON.parse(repaired);',
  '  } catch (e2) {',
  '    var len = rawText.length;',
  '    var pre = rawText.substring(0, 400);',
  '    var suf = rawText.substring(Math.max(0, len - 400));',
  '    throw new Error(',
  '      "JSON failed after repair. Len=" + len + ". " +',
  '      "First: " + pre + ". " +',
  '      "Last: " + suf + ". " +',
  '      "Err1: " + e1.message + ". " +',
  '      "Err2: " + e2.message',
  '    );',
  '  }',
  '}',
  '',
  'if (!bible.world_overview || !bible.characters || !bible.main_plot_threads) {',
  '  throw new Error("Missing required fields. Got: " + Object.keys(bible).join(", "));',
  '}',
  '',
  'return [{ json: { project: $("Create Project").first().json, bible: bible } }];'
].join('\n'));

async function main() {
  const resp = await fetch(BASE + '/workflows/' + WF_ID, { headers: HEADERS });
  const wf = await resp.json();

  const pbr = wf.nodes.find(n => n.name === 'Parse Bible Response');
  if (pbr) {
    // parseCode is a JSON-escaped string, need to JSON.parse it
    pbr.parameters.jsCode = parseCode;
    console.log('Updated: Parse Bible Response with repair');
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
