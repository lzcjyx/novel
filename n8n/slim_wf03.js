// Slim down WF03 by minifying large Code nodes
const fs = require('fs');

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

// Minified Collect All Reviews (removed comments, shortened vars)
const slimCollectAllReviews = [
  'const allReviews = $input.all();',
  'const reviewData = [];',
  'for (const item of allReviews) {',
  '  try {',
  '    const j = item.json;',
  '    if (j.choices?.[0]?.message?.content) reviewData.push(JSON.parse(j.choices[0].message.content));',
  '    else if (j.agent_name) reviewData.push(j);',
  '    else if (j.data?.agent_name) reviewData.push(j.data);',
  '    else reviewData.push({agent_name:"unknown",score:0,pass:false,blocking_issues:[{id:"PARSE_ERR",severity:"high",issue:"Unable to parse review output"}],minor_issues:[],recommendations:[]});',
  '  } catch (e) {',
  '    reviewData.push({agent_name:"parse_error",score:0,pass:false,blocking_issues:[{id:"PARSE_ERR",severity:"high",issue:"Failed to parse: "+e.message}],minor_issues:[],recommendations:[]});',
  '  }',
  '}',
  'const safetyReview = reviewData.find(r => r.agent_name === "safety_reviewer");',
  'const scores = reviewData.map(r => r.score).filter(s => typeof s === "number" && !isNaN(s));',
  'const avgScore = scores.length > 0 ? Math.round((scores.reduce((a,b)=>a+b,0)/scores.length)*100)/100 : 0;',
  'const allBlocking = [], allMinor = [], recsByAgent = {};',
  'for (const r of reviewData) {',
  '  (r.blocking_issues||[]).forEach(i => allBlocking.push({...i,agent_name:r.agent_name}));',
  '  (r.minor_issues||[]).forEach(i => allMinor.push({...i,agent_name:r.agent_name}));',
  '  if (r.recommendations?.length) recsByAgent[r.agent_name] = r.recommendations;',
  '}',
  'const hasBlocking = allBlocking.length > 0;',
  'const safetyFailed = safetyReview?.pass === false;',
  'const ctx = $("Build Writing Brief").first()?.json || $input.first().json;',
  'const revCnt = ctx.revise_count ?? 0;',
  'const maxRev = ctx.max_revise_count ?? 2;',
  'const qThresh = ctx.quality_threshold ?? 85;',
  'let decision, finalScore;',
  'if (safetyFailed) { decision = "needs_human_review"; }',
  'else if (revCnt >= maxRev && hasBlocking) { decision = "needs_human_review"; }',
  'else if (hasBlocking) { decision = "revise"; }',
  'else if (avgScore >= qThresh) { decision = "publish_ready"; }',
  'else { decision = "revise"; }',
  'if (safetyFailed) finalScore = Math.min(avgScore, 40);',
  'else if (hasBlocking) finalScore = Math.min(avgScore, 74);',
  'else finalScore = avgScore;',
  'finalScore = Math.round(finalScore * 100) / 100;',
  'const prep = $("Prepare Review Input").first().json;',
  'return [{ json: {',
  '  chapter_id: prep.chapter_id, version_id: prep.version_id,',
  '  average_score: avgScore, final_score: finalScore,',
  '  decision: decision, publish_allowed: decision === "publish_ready",',
  '  safety_pass: !safetyFailed, has_blocking: hasBlocking,',
  '  all_pass: reviewData.every(r => r.pass !== false),',
  '  blocking_issues: allBlocking, minor_issues: allMinor,',
  '  must_fix: allBlocking.filter(i => i.severity === "high" || safetyFailed),',
  '  recommendations: recsByAgent, reviews: reviewData,',
  '  agent_count: reviewData.length, blocking_count: allBlocking.length, minor_count: allMinor.length',
  '}}];'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/hCkthWAH1GxEvLYU', { headers: HEADERS });
  const wf = await resp.json();

  // Replace the big Code node
  const node = wf.nodes.find(n => n.name === 'Collect All Reviews');
  if (node) {
    const oldSize = node.parameters.jsCode.length;
    node.parameters.jsCode = slimCollectAllReviews;
    const newSize = node.parameters.jsCode.length;
    console.log('Collect All Reviews:', oldSize, '->', newSize, 'bytes (saved', oldSize-newSize, ')');
  }

  // Also trim other large Code nodes if they exist
  const largeNodes = wf.nodes.filter(n => n.type === 'n8n-nodes-base.code' && (n.parameters.jsCode||'').length > 1500);
  console.log('Other large Code nodes:');
  largeNodes.forEach(n => console.log(' ', n.name, ':', (n.parameters.jsCode||'').length, 'bytes'));

  // Update
  if (wf.settings) { for (const k of Object.keys(wf.settings)) { if (wf.settings[k] === null) delete wf.settings[k]; } }

  const updateResp = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: HEADERS,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: wf.settings })
  });

  if (updateResp.ok) {
    console.log('WF03 updated! New total size:', JSON.stringify({nodes:wf.nodes,connections:wf.connections}).length, 'bytes');
    fs.writeFileSync('D:/novel/n8n/03_daily_chapter_production.workflow.json', JSON.stringify({name:wf.name,nodes:wf.nodes,connections:wf.connections,settings:wf.settings}, null, 2));
  } else {
    const err = await updateResp.json();
    console.log('FAIL:', JSON.stringify(err).slice(0, 300));
  }
}

main().catch(e => console.error(e));
