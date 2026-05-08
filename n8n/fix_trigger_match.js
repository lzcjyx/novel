// Fix triggerWorkflow: match NEW execution by startedAt > trigger time
const fs = require("fs");
const code = fs.readFileSync("D:/novel/orchestrator/scheduler.js", "utf8");

// Replace the webhook trigger section
const oldBlock = `    if (webhook && webhook.parameters && webhook.parameters.path) {
      // Use webhook trigger
      const whPath = webhook.parameters.path;
      const whUrl = \`http://localhost:5678/webhook/\${whPath}\`;
      log(\`Triggering \${name} via webhook \${whPath}\`);
      // Fire webhook in background (may return 500 on timeout but workflow still runs)
      fetch(whUrl, { method: "POST", headers: { "Content-Type": "application/json" }, body: "{}" })
        .catch(e => log(\`Webhook trigger error: \${e.message}\`));
      // Wait then check for execution
      await sleep(3000);
      const listResp = await fetch(N8N_API_URL + \`/executions?workflowId=\${workflowId}&limit=1\`, { headers: HEADERS });
      const list = await listResp.json();
      const exec = (list.data && list.data[0]) ? list.data[0] : null;
      if (exec) log(\`\${name} execution: \${exec.id} status=\${exec.status}\`);
      return exec ? exec.id : "triggered";
    }`;

const newBlock = `    if (webhook && webhook.parameters && webhook.parameters.path) {
      const whPath = webhook.parameters.path;
      const whUrl = \`http://localhost:5678/webhook/\${whPath}\`;
      const triggerTime = new Date().toISOString();
      log(\`Triggering \${name} via webhook \${whPath}\`);
      fetch(whUrl, { method: "POST", headers: { "Content-Type": "application/json" }, body: "{}" })
        .catch(e => log(\`Webhook trigger error: \${e.message}\`));
      // Find NEW execution started AFTER trigger time
      let exec = null;
      for (let i = 0; i < 6; i++) {
        await sleep(3000);
        const listResp = await fetch(N8N_API_URL + \`/executions?workflowId=\${workflowId}&limit=3\`, { headers: HEADERS });
        const list = await listResp.json();
        exec = (list.data || []).find(e => e.startedAt > triggerTime) || null;
        if (exec) break;
      }
      if (exec) log(\`\${name} execution: \${exec.id} status=\${exec.status}\`);
      else log(\`\${name}: no new execution found\`);
      return exec ? exec.id : "triggered";
    }`;

if (code.includes(oldBlock)) {
  const updated = code.replace(oldBlock, newBlock);
  fs.writeFileSync("D:/novel/orchestrator/scheduler.js", updated);
  console.log("Fixed triggerWorkflow — matches NEW execution by startedAt");
} else {
  console.log("Block not found — file may have changed. Checking...");
  // Try to find and replace with more flexible approach
  const lines = code.split("\n");
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].includes("Fire webhook in background")) {
      console.log("Found at line", i + 1);
      break;
    }
  }
}
