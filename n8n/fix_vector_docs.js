// Fix Prepare Vector Documents - use bible data directly from Parse Bible Response
const fs = require('fs');
const dir = __dirname + '/';

const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const HEADERS = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };
const WF_ID = 'CtEvELW9y6XbmbVi';

const newCode = [
  '// Get bible data from Parse Bible Response',
  'const bibleData = $(\"Parse Bible Response\").first().json;',
  'const bible = bibleData.bible;',
  'const project = bibleData.project;',
  'const vectors = [];',
  '',
  '// Characters',
  '(bible.characters || []).forEach(function(c) {',
  '  vectors.push({',
  '    json: {',
  '      project_id: project.id,',
  '      source_type: \"character_profile\",',
  '      title: c.name,',
  '      content: JSON.stringify({name: c.name, role: c.role, personality: c.personality, speech_style: c.speech_style, motivation: c.motivation, backstory: c.backstory}),',
  '      input_text: c.name + \" - \" + (c.role || \"\") + \" - \" + (c.personality || \"\")',
  '    }',
  '  });',
  '});',
  '',
  '// World overview',
  'if (bible.world_overview) {',
  '  vectors.push({',
  '    json: {',
  '      project_id: project.id,',
  '      source_type: \"world_lore\",',
  '      title: \"World Overview\",',
  '      content: bible.world_overview,',
  '      input_text: bible.world_overview',
  '    }',
  '  });',
  '}',
  '',
  '// Power system',
  'if (bible.power_system) {',
  '  vectors.push({',
  '    json: {',
  '      project_id: project.id,',
  '      source_type: \"world_lore\",',
  '      title: \"Power System: \" + (bible.power_system.name || \"\"),',
  '      content: JSON.stringify(bible.power_system),',
  '      input_text: (bible.power_system.name || \"\") + \" - \" + (bible.power_system.description || \"\")',
  '    }',
  '  });',
  '}',
  '',
  '// Locations',
  '(bible.locations || []).forEach(function(l) {',
  '  vectors.push({',
  '    json: {',
  '      project_id: project.id,',
  '      source_type: \"world_lore\",',
  '      title: \"Location: \" + l.name,',
  '      content: JSON.stringify(l),',
  '      input_text: l.name + \" - \" + (l.description || \"\")',
  '    }',
  '  });',
  '});',
  '',
  '// Organizations',
  '(bible.organizations || []).forEach(function(o) {',
  '  vectors.push({',
  '    json: {',
  '      project_id: project.id,',
  '      source_type: \"world_lore\",',
  '      title: \"Organization: \" + o.name,',
  '      content: JSON.stringify(o),',
  '      input_text: o.name + \" - \" + (o.description || \"\")',
  '    }',
  '  });',
  '});',
  '',
  'return vectors;'
].join('\n');

async function main() {
  const resp = await fetch(BASE + '/workflows/' + WF_ID, { headers: HEADERS });
  const wf = await resp.json();

  const pvd = wf.nodes.find(n => n.name === 'Prepare Vector Documents');
  if (pvd) {
    pvd.parameters.jsCode = newCode;
    console.log('Updated: Prepare Vector Documents');
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
