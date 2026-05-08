const API_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI3ZTYzYTYzMC00NzYwLTRiZjEtOTViOC0zNTBjMTQxZDZkZjkiLCJpc3MiOiJuOG4iLCJhdWQiOiJwdWJsaWMtYXBpIiwianRpIjoiZDJjMDgxODUtNDA3YS00NzVhLWJjZjMtMDM4ZmI1ZWEyMjAyIiwiaWF0IjoxNzc4MTQ3NTk3fQ.IbGlhTHCcVL5nM9wF3SbnmxgGhRnIsino1m0l3HdXSY';
const BASE = 'http://localhost:5678/api/v1';
const H = { 'Content-Type': 'application/json', 'X-N8N-API-KEY': API_KEY };

async function main() {
  const resp = await fetch(BASE + '/workflows/RGbsc6NtL3uHqKSk', { headers: H });
  const wf = await resp.json();

  // Fix DeepSeek node messages with explicit JSON schema
  const ds = wf.nodes.find(n => n.type === 'n8n-nodes-deepseek.deepSeek');
  if (ds && ds.parameters.prompt && ds.parameters.prompt.messages) {
    const msgs = ds.parameters.prompt.messages;

    // System prompt with exact JSON schema
    const sysMsg = msgs.find(m => m.role === 'system');
    if (sysMsg) {
      sysMsg.content = '你是一位资深网络小说策划编辑，负责每周剧情规划。你必须严格按照以下 JSON schema 输出，不要包含任何解释或代码块。所有字符串值必须用双引号括起来。';
    }

    // User prompt with context + schema
    const userMsg = msgs.find(m => m.role === 'user');
    if (userMsg) {
      userMsg.content = '={{ "项目上下文：\\n" + JSON.stringify($json, null, 2) + "\\n\\n请按以下 JSON schema 输出（严格使用这些 key 名，不要改变结构）：\\n{\\n  \\"chapter_plans\\": [\\n    {\\n      \\"sequence\\": 1,\\n      \\"title\\": \\"章节标题\\",\\n      \\"plot_goals\\": [\\"目标1\\", \\"目标2\\"],\\n      \\"summary\\": \\"章节概述（80-150字）\\",\\n      \\"target_word_count\\": 3000\\n    }\\n  ],\\n  \\"plot_thread_updates\\": [\\n    {\\n      \\"name\\": \\"剧情线名称\\",\\n      \\"status\\": \\"active|resolved|on_hold\\",\\n      \\"description\\": \\"更新描述\\"\\n    }\\n  ],\\n  \\"foreshadowing_updates\\": [\\n    {\\n      \\"description\\": \\"伏笔描述\\",\\n      \\"payoff_chapter\\": 5\\n    }\\n  ],\\n  \\"human_review_required\\": false,\\n  \\"weekly_summary\\": \\"本周规划概述（100-200字）\\"\\n}\\n\\n要求：\\n- 生成 7-10 章 chapter_plans\\n- sequence 从 ' + ($json.next_sequence_start || 1) + ' 开始递增\\n- 必须输出纯 JSON，不要包在代码块里\\n- 所有 key 名必须按上述 schema 使用" }}';
    }

    console.log('Updated DeepSeek messages with exact JSON schema');
  }

  // Also update Parse Arc Plan to handle this exact schema
  const pap = wf.nodes.find(n => n.name === 'Parse Arc Plan');
  if (pap) {
    pap.parameters.jsCode = [
      'var response = $input.first().json;',
      'var rawText = "";',
      'if (response["message"] && response["message"]["content"]) rawText = response["message"]["content"];',
      'else if (response["choices"] && response["choices"][0]) rawText = response["choices"][0]["message"]["content"];',
      'else if (typeof response === "string") rawText = response;',
      'else rawText = JSON.stringify(response);',
      '',
      'rawText = rawText.trim();',
      'if (rawText.startsWith("```")) { var nl = rawText.indexOf("\\n"); if (nl >= 0) rawText = rawText.substring(nl + 1); }',
      'if (rawText.endsWith("```")) rawText = rawText.substring(0, rawText.length - 3).trim();',
      '',
      'var plan;',
      'try { plan = JSON.parse(rawText); }',
      'catch (e) { throw new Error("Failed to parse arc plan. Raw (first 300): " + rawText.substring(0, 300)); }',
      '',
      'var proj = $("Load Active Project").first().json;',
      '',
      'return [{ json: {',
      '  project_id: proj["id"],',
      '  plan: plan,',
      '  next_sequence_start: ($("Build Planning Context").first().json["next_sequence_start"] || 1)',
      '}}];'
    ].join('\n');
    console.log('Updated Parse Arc Plan');
  }

  // Update Build Plan SQLs for the schema-defined structure
  const bps = wf.nodes.find(n => n.name === 'Build Plan SQLs');
  if (bps) {
    bps.parameters.jsCode = [
      'var data = $input.first().json;',
      'var plan = data.plan;',
      'var pid = data.project_id;',
      'var items = [];',
      '',
      'function esc(val) {',
      '  if (val === undefined || val === null) return "NULL";',
      '  if (typeof val === "boolean") return val ? "TRUE" : "FALSE";',
      '  if (typeof val === "number") return String(val);',
      '  var str = typeof val === "object" ? JSON.stringify(val) : String(val);',
      '  return "\x27" + str.replace(/\x27/g, "\x27\x27") + "\x27";',
      '}',
      '',
      'var chapters = plan["chapter_plans"] || [];',
      'for (var i = 0; i < chapters.length; i++) {',
      '  var cp = chapters[i];',
      '  var seq = cp["sequence"] || i + 1;',
      '  items.push({ json: { sql: "INSERT INTO chapter_plans (project_id, sequence, title, plot_goals, outline, target_word_count, status) VALUES ("',
      '    + esc(pid) + ", " + esc(seq) + ", "',
      '    + esc(cp["title"] || ("Chapter " + seq)) + ", "',
      '    + esc(cp["plot_goals"] || []) + ", "',
      '    + esc(cp["summary"] || "") + ", "',
      '    + esc(cp["target_word_count"] || 3000) + ", "',
      '    + esc("planned") + ") RETURNING id"',
      '  }});',
      '}',
      '',
      'if (items.length === 0) items.push({ json: { sql: "SELECT 1 AS note" } });',
      'return items;'
    ].join('\n');
    console.log('Updated Build Plan SQLs');
  }

  // Update Weekly Plan Summary
  const wps = wf.nodes.find(n => n.name === 'Weekly Plan Summary');
  if (wps) {
    wps.parameters.jsCode = [
      'var data = $("Parse Arc Plan").first().json;',
      'var plan = data.plan;',
      'var chapters = plan["chapter_plans"] || [];',
      'var plotT = plan["plot_thread_updates"] || [];',
      'var foresh = plan["foreshadowing_updates"] || [];',
      'var hr = plan["human_review_required"];',
      'var hrCount = typeof hr === "boolean" ? (hr ? 1 : 0) : (Array.isArray(hr) ? hr.length : 0);',
      '',
      'return [{ json: {',
      '  project_id: data.project_id,',
      '  chapters_planned: chapters.length,',
      '  plot_threads_updated: plotT.length,',
      '  foreshadowing_updated: foresh.length,',
      '  human_review_count: hrCount,',
      '  human_review_items: Array.isArray(hr) ? hr : [],',
      '  weekly_summary: plan["weekly_summary"] || "",',
      '  pacing_analysis: "",',
      '  status: "weekly_plan_complete"',
      '}}];'
    ].join('\n');
    console.log('Updated Weekly Plan Summary');
  }

  if (wf.settings) { for (const k of Object.keys(wf.settings)) { if (wf.settings[k] === null) delete wf.settings[k]; } }

  const r = await fetch(BASE + '/workflows/' + wf.id, {
    method: 'PUT', headers: H,
    body: JSON.stringify({ name: wf.name, nodes: wf.nodes, connections: wf.connections, settings: { executionOrder: 'v1' } })
  });
  console.log(r.ok ? 'Updated! Run WF05' : 'FAIL');
}
main().catch(e => console.error(e));
