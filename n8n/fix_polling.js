// Fix polling: more retries, longer wait between
const fs = require("fs");
let code = fs.readFileSync("D:/novel/orchestrator/scheduler.js", "utf8");

// Replace the loop (lines with sleep + find new execution)
code = code.replace(
  /\/\/ Find NEW execution[\s\S]*?return exec \? exec\.id : "triggered";/m,
  `// Find NEW execution (startedAt > triggerTime)
    let exec = null;
    for (let i = 0; i < 10; i++) {
      await sleep(5000);
      const listResp = await fetch(N8N_API_URL + \`/executions?workflowId=\${workflowId}&limit=5\`, { headers: HEADERS });
      const list = await listResp.json();
      exec = (list.data || []).find(e => e.startedAt > triggerTime) || null;
      if (exec) { log(\`\${name} execution \${exec.id} (\${exec.status})\`); break; }
    }
    if (!exec) log(\`\${name}: no execution after 50s\`);
    return exec ? exec.id : "triggered";`
);

fs.writeFileSync("D:/novel/orchestrator/scheduler.js", code);
console.log("Fixed polling (10 retries × 5s = 50s max)");
