// Fix Parse Bible Response with character-by-character JSON repair (no regex issues)
const fs = require('fs');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };
const WF_ID = 'CtEvELW9y6XbmbVi';

// Repair function: char-by-char, no complex regex
const repairFn = [
  'function repairJSON(text) {',
  '  var out = "";',
  '  var i = 0;',
  '  while (i < text.length) {',
  '    out += text[i];',
  '    if (text[i] === ":" && i + 1 < text.length) {',
  '      var j = i + 1;',
  '      while (j < text.length && (text[j] === " " || text[j] === "\\n" || text[j] === "\\r" || text[j] === "\\t")) j++;',
  '      if (j < text.length) {',
  '        var ch = text[j];',
  '        var isQuoted = (ch === \x27"\x27);',
  '        var isBracket = (ch === "[" || ch === "{");',
  '        var isNum = (ch >= "0" && ch <= "9") || ch === "-";',
  '        var tok4 = text.substring(j, j + 4);',
  '        var tok5 = text.substring(j, j + 5);',
  '        var isKeyword = (tok4 === "true" || tok5 === "false" || tok4 === "null");',
  '        if (!isQuoted && !isBracket && !isNum && !isKeyword) {',
  '          var k = j;',
  '          while (k < text.length && text[k] !== "," && text[k] !== "\\n" && text[k] !== "}" && text[k] !== "]") k++;',
  '          var val = text.substring(j, k).trim();',
  '          var escaped = val.replace(/"/g, \x27\\\\"\x27).replace(/\\\\/g, \x27\\\\\\\\\x27);',
  '          out += \x27"\x27 + val + \x27"\x27;',
  '          i = k - 1;',
  '        } else {',
  '          i = j - 1;',
  '        }',
  '      }',
  '    }',
  '    i++;',
  '  }',
  '  return out;',
  '}'
].join('\n');

const fullCode = [
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
  repairFn,
  '',
  '// Remove trailing commas',
  'rawText = rawText.replace(/,(\s*[}\\]])/g, "$1");',
  '',
  'let bible;',
  'try {',
  '  bible = JSON.parse(rawText);',
  '} catch (e1) {',
  '  try {',
  '    var repaired = repairJSON(rawText);',
  '    repaired = repaired.replace(/,(\s*[}\\]])/g, "$1");',
  '    bible = JSON.parse(repaired);',
  '  } catch (e2) {',
  '    throw new Error(',
  '      "JSON failed after repair. " +',
  '      "Err: " + e1.message + " | " +',
  '      "Repair err: " + e2.message',
  '    );',
  '  }',
  '}',
  '',
  'if (!bible.world_overview || !bible.characters || !bible.main_plot_threads) {',
  '  throw new Error("Missing fields: " + Object.keys(bible).join(", "));',
  '}',
  '',
  'return [{ json: { project: $("Create Project").first().json, bible: bible } }];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/' + WF_ID, { headers: HEADERS });
  const wf = await resp.json();

  const pbr = wf.nodes.find(n => n.name === 'Parse Bible Response');
  if (pbr) {
    pbr.parameters.jsCode = fullCode;
    console.log('Updated: Parse Bible Response with char-by-char repair');
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
