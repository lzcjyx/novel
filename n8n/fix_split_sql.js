// Fix Split Bible to DB Rows SQL to match actual database schema
const fs = require('fs');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };
const WF_ID = 'CtEvELW9y6XbmbVi';

const fixedCode = [
  'const data = $input.first().json;',
  'const project = data.project;',
  'const bible = data.bible;',
  'const items = [];',
  '',
  '// style_guides',
  'if (bible.style_guide) {',
  '  items.push({ json: {',
  '    table: \x27style_guides\x27,',
  '    values: [project.id, \x27\\u521d\\u59cb\\u98ce\\u683c\\u6307\\u5357\x27, bible.style_guide.narrative_perspective || \x27\x27, JSON.stringify(bible.style_guide.positive_examples || []), JSON.stringify(bible.style_guide.negative_examples || [])],',
  '    sql: \x27INSERT INTO style_guides (project_id, name, style_text, positive_examples, negative_examples) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING RETURNING id\x27',
  '  }});',
  '}',
  '',
  '// characters - matching actual schema (name, aliases, role, personality, motivation, speech_style, appearance, backstory)',
  '(bible.characters || []).forEach(function(c) {',
  '  items.push({ json: {',
  '    table: \x27characters\x27,',
  '    values: [project.id, c.name, c.role || \x27\x27, c.personality || \x27\x27, c.motivation || \x27\x27, c.speech_style || \x27\x27, c.appearance || \x27\x27, c.backstory || \x27\x27],',
  '    sql: \x27INSERT INTO characters (project_id, name, role, personality, motivation, speech_style, appearance, backstory) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING RETURNING id\x27',
  '  }});',
  '});',
  '',
  '// locations',
  '(bible.locations || []).forEach(function(l) {',
  '  items.push({ json: {',
  '    table: \x27locations\x27,',
  '    values: [project.id, l.name, l.location_type || \x27\x27, l.description || \x27\x27],',
  '    sql: \x27INSERT INTO locations (project_id, name, type, description) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id\x27',
  '  }});',
  '});',
  '',
  '// organizations',
  '(bible.organizations || []).forEach(function(o) {',
  '  items.push({ json: {',
  '    table: \x27organizations\x27,',
  '    values: [project.id, o.name, o.description || \x27\x27, JSON.stringify(o.goals || [])],',
  '    sql: \x27INSERT INTO organizations (project_id, name, description, goals) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id\x27',
  '  }});',
  '});',
  '',
  '// items',
  '(bible.items || []).forEach(function(it) {',
  '  items.push({ json: {',
  '    table: \x27items\x27,',
  '    values: [project.id, it.name, it.item_type || \x27\x27, it.description || \x27\x27],',
  '    sql: \x27INSERT INTO items (project_id, name, item_type, description) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id\x27',
  '  }});',
  '});',
  '',
  '// world_lore',
  'if (bible.world_overview) {',
  '  items.push({ json: {',
  '    table: \x27world_lore\x27,',
  '    values: [project.id, \x27\\u4e16\\u754c\\u89c2\\u6982\\u8ff0\x27, \x27world_overview\x27, bible.world_overview],',
  '    sql: \x27INSERT INTO world_lore (project_id, title, lore_type, content) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id\x27',
  '  }});',
  '}',
  '',
  '// magic_or_power_systems',
  'if (bible.power_system) {',
  '  items.push({ json: {',
  '    table: \x27magic_or_power_systems\x27,',
  '    values: [project.id, bible.power_system.name || \x27\\u529b\\u91cf\\u4f53\\u7cfb\x27, bible.power_system.description || \x27\x27, JSON.stringify(bible.power_system.rules || []), JSON.stringify(bible.power_system.limitations || [])],',
  '    sql: \x27INSERT INTO magic_or_power_systems (project_id, name, description, rules, limitations) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING RETURNING id\x27',
  '  }});',
  '}',
  '',
  '// canon_rules - FIXED: removed rationale, reorder (rule_type, rule_text, severity)',
  '(bible.canon_rules || []).forEach(function(r) {',
  '  items.push({ json: {',
  '    table: \x27canon_rules\x27,',
  '    values: [project.id, r.rule_type || \x27hard\x27, r.rule_text || \x27\x27, r.rule_type === \x27soft\x27 ? \x27soft\x27 : \x27hard\x27],',
  '    sql: \x27INSERT INTO canon_rules (project_id, rule_type, rule_text, severity) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id\x27',
  '  }});',
  '});',
  '',
  '// plot_threads - FIXED: match actual schema (name, description)',
  '(bible.main_plot_threads || []).forEach(function(pt) {',
  '  items.push({ json: {',
  '    table: \x27plot_threads\x27,',
  '    values: [project.id, pt.title || \x27\x27, pt.description || \x27\x27, pt.thread_type || \x27main\x27],',
  '    sql: \x27INSERT INTO plot_threads (project_id, name, description, priority) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id\x27',
  '  }});',
  '});',
  '',
  '// chapter_plans - FIXED: match actual schema (outline instead of summary, no arc_stage)',
  '(bible.chapter_plans || []).forEach(function(cp) {',
  '  items.push({ json: {',
  '    table: \x27chapter_plans\x27,',
  '    values: [project.id, cp.sequence, cp.title || \x27\x27, JSON.stringify(cp.plot_goals || []), cp.summary || \x27\x27, cp.target_word_count || 3000],',
  '    sql: \x27INSERT INTO chapter_plans (project_id, sequence, title, plot_goals, outline, target_word_count) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (project_id, sequence) DO NOTHING RETURNING id\x27',
  '  }});',
  '});',
  '',
  'return items;'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/' + WF_ID, { headers: HEADERS });
  const wf = await resp.json();

  const node = wf.nodes.find(n => n.name === 'Split Bible to DB Rows');
  if (node) {
    // Verify by checking \x27 won't cause issues - \x27 in JS source string produces ' in output
    // But this code goes through JSON.stringify in the API call, so single quotes are fine
    node.parameters.jsCode = fixedCode;
    console.log('Fixed: Split Bible to DB Rows SQL matches DB schema');
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
