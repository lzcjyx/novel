// Convert all AI HTTP Request nodes to native DeepSeek nodes
const fs = require('fs');
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI1Y2JjODlhNS1iZmQyLTQ3YWQtODkxOC02M2E1NWZjNjdlNDQiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiY2FmOGVjMDktN2FhYS00ZTRmLWI0NDktOTRjYWZmYmRlNzAzIiwiaWF0IjoxNzc4MDc5NTgwfQ.9S7oWFJj7OHSXt31tbAtvt-N9olWrdIH9ld6TuBAZyg';

const DEEPSEEK_CRED = { id: 'O06O4ws1byCbKxp5', name: 'DeepSeek account' };

function isAIHttpRequest(node) {
  if (node.type !== 'n8n-nodes-base.httpRequest') return false;
  const url = node.parameters.url || '';
  return url.includes('deepseek.com') || url.includes('openai.com');
}

function convertToDeepSeekNode(httpNode, suffix) {
  // Extract messages from the jsonBody
  const body = httpNode.parameters.jsonBody || '{}';
  let messages = [];
  let temperature = 0.7;

  try {
    // Try to parse the jsonBody template
    const tempBody = body.replace(/\{\{.*?\}\}/g, '""'); // Replace n8n expressions
    const parsed = JSON.parse(tempBody);
    if (parsed.messages) messages = parsed.messages;
    if (parsed.temperature !== undefined) temperature = parsed.temperature;
  } catch (_) {
    // Can't parse, just create default messages
    messages = [{ role: 'user', content: body }];
  }

  // Build the DeepSeek node parameters
  const systemMsg = messages.find(m => m.role === 'system');
  const userMsg = messages.find(m => m.role === 'user');

  // Extract actual n8n expressions from the original body
  const systemContent = systemMsg ? extractExpression(body, 'system') : '';
  const userContent = userMsg ? extractExpression(body, 'user') : body;

  return {
    parameters: {
      model: 'deepseek-chat',
      prompt: {
        messages: systemContent
          ? [
              { role: 'system', content: systemContent },
              { role: 'user', content: userContent }
            ]
          : [
              { role: 'user', content: userContent }
            ]
      },
      options: {
        temperature: temperature
      },
      requestOptions: {}
    },
    id: httpNode.id,
    name: httpNode.name,
    type: 'n8n-nodes-deepseek.deepseek',
    typeVersion: 1,
    position: httpNode.position,
    credentials: {
      deepSeekApi: DEEPSEEK_CRED
    }
  };
}

function extractExpression(body, role) {
  // Extract the content for a specific role from the jsonBody template
  try {
    const parsed = JSON.parse(body);
    const msg = parsed.messages.find(m => m.role === role);
    return msg ? msg.content : '';
  } catch (_) {
    return body;
  }
}

// Process all 5 workflows
const files = [
  '01_novel_bootstrap.workflow.json',
  '02_bible_ingestion.workflow.json',
  '03_daily_chapter_production.workflow.json',
  '04_review_and_repair.workflow.json',
  '05_weekly_arc_planner.workflow.json',
];

for (const file of files) {
  const wf = JSON.parse(fs.readFileSync(file, 'utf-8'));
  let converted = 0;

  for (let i = 0; i < wf.nodes.length; i++) {
    const node = wf.nodes[i];
    if (isAIHttpRequest(node)) {
      // Check if this is an embedding node (keep as HTTP)
      if (node.parameters.url && node.parameters.url.includes('embeddings')) {
        continue; // Skip embedding nodes - they need HTTP Request
      }
      // Convert to DeepSeek node
      wf.nodes[i] = convertToDeepSeekNode(node, '');
      converted++;
      console.log('  Converted:', node.name, 'in', file);
    }
  }

  if (converted > 0) {
    // Clean settings
    for (const k of Object.keys(wf.settings || {})) {
      if (wf.settings[k] === null) delete wf.settings[k];
    }
    fs.writeFileSync(file, JSON.stringify(wf, null, 2));
    console.log(file + ': ' + converted + ' nodes converted');
  }
}

// Deploy all
async function deploy() {
  const list = await fetch('http://localhost:5678/api/v1/workflows?limit=20', {
    headers: { 'X-N8N-API-KEY': API_KEY }
  });
  const d = await list.json();
  const nameToId = {};
  for (const w of d.data) {
    if (w.name.startsWith('[NF]')) nameToId[w.name] = w.id;
  }

  for (const file of files) {
    const wf = JSON.parse(fs.readFileSync(file, 'utf-8'));
    const oldId = nameToId[wf.name];
    if (oldId) {
      await fetch('http://localhost:5678/api/v1/workflows/' + oldId, {
        method: 'DELETE', headers: { 'X-N8N-API-KEY': API_KEY }
      });
    }
    const payload = JSON.stringify({
      name: wf.name,
      nodes: wf.nodes,
      connections: wf.connections,
      settings: wf.settings
    });
    const resp = await fetch('http://localhost:5678/api/v1/workflows', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY },
      body: payload
    });
    const result = await resp.json();
    console.log(wf.name, '->', resp.ok ? 'OK ' + result.id.slice(0,8) : 'FAIL');
  }
}

deploy();
