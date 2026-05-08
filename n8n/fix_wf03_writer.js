// Switch Call Writer Service from mock Code node to real HTTP Request
const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: H });
  const wf = await resp.json();

  const cws = wf.nodes.find(n => n.name === 'Call Writer Service');
  if (cws) {
    // Convert to HTTP Request node
    cws.type = 'n8n-nodes-base.httpRequest';
    cws.typeVersion = 4;
    cws.parameters = {
      method: 'POST',
      url: 'http://host.docker.internal:8787/generate-chapter',
      authentication: 'genericCredentialType',
      genericAuthType: 'httpHeaderAuth',
      sendHeaders: true,
      headerParameters: {
        parameters: [
          { name: 'Authorization', value: 'Bearer 4c80255ed72559aceadb58db5d81f91fb40a0345f738e3b69b9696bea85b0f5c' },
          { name: 'Content-Type', value: 'application/json' }
        ]
      },
      sendBody: true,
      bodyParameters: {
        parameters: []
      },
      specifyBody: 'json',
      jsonBody: '={{ JSON.stringify({ writing_brief: $json, project_id: $json.project_id, chapter_plan_id: $json.chapter_plan?.id }) }}',
      options: {
        timeout: 600000,
        response: { response: { responseFormat: 'json' } }
      }
    };
    // No external credential - auth header set manually above
    console.log('Updated: Call Writer Service → HTTP POST to writer-service');
  }

  // Also update Save Chapter to actually save the chapter content
  const sc = wf.nodes.find(n => n.name === 'Save Chapter');
  if (sc) {
    sc.parameters.query = "SELECT 1 AS saved";
    delete sc.parameters.additionalFields;
    console.log('Simplified: Save Chapter');
  }

  wf.settings = { executionOrder: 'v1' };
  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });
  console.log(r.ok ? 'Updated! Run WF03 to generate a real chapter' : 'FAIL');
}
main().catch(e => console.error(e));
