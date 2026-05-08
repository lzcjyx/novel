// Add response_format: {type: "json_object"} to Gen Bible node
// This forces DeepSeek to output valid JSON
const fs = require('fs');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };
const WF_ID = 'CtEvELW9y6XbmbVi';

async function main() {
  const resp = await fetch(BASE + '/workflows/' + WF_ID, { headers: HEADERS });
  const wf = await resp.json();

  // Fix Gen Bible node
  const gb = wf.nodes.find(n => n.name === 'Gen Bible via OpenAI');
  if (gb) {
    // Add response_format to force JSON output
    gb.parameters.options.response_format = { type: 'json_object' };
    console.log('Added response_format: json_object');

    // Update system prompt to mention JSON (required by DeepSeek)
    const msgs = gb.parameters.prompt.messages;
    const sysMsg = msgs.find(m => m.role === 'system');
    if (sysMsg) {
      // The system message is "={{ $json.system_prompt }}" so we update Build Bible Prompt instead
      console.log('System prompt is dynamic - updating Build Bible Prompt');
    }
  }

  // Update Build Bible Prompt system_prompt to mention JSON
  const bbp = wf.nodes.find(n => n.name === 'Build Bible Prompt');
  if (bbp) {
    // Find the system_prompt value and ensure it says "JSON"
    // Current code sets system_prompt inline, let me update it
    const code = bbp.parameters.jsCode;
    // Make sure system_prompt mentions JSON output requirement
    const updated = code.replace(
      'system_prompt: "你是一位资深小说策划编辑。请严格按提供的 JSON schema 输出。所有 key 必须使用英文，所有内容用中文。不要包含解释或代码块。"',
      'system_prompt: "你是一位资深小说策划编辑。请输出严格合法的 JSON，所有字符串值必须用双引号括起来。所有 key 必须使用英文，所有内容用中文。不要输出任何解释、注释或代码块，只输出纯 JSON 对象。"'
    );
    if (updated !== code) {
      bbp.parameters.jsCode = updated;
      console.log('Updated system_prompt in Build Bible Prompt');
    }
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
