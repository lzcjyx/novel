const fs = require('fs');
const j = JSON.parse(fs.readFileSync('D:/novel/n8n/wf05_latest.json', 'utf8'));
const e = (j.data || [])[0];
if (!e) { console.log('No execution'); process.exit(); }

console.log('Status:', e.status);
console.log('Workflow ID:', e.workflowId);

// Check data field
if (e.data) {
  const rd = e.data.resultData;
  if (rd && rd.runData) {
    for (const [name, runs] of Object.entries(rd.runData)) {
      const r = runs[0];
      const items = r?.data?.main?.[0] || [];
      if (name === 'Parse Arc Plan' || name === 'Build Plan SQLs' || name === 'Weekly Plan Summary') {
        console.log('===', name, '(', items.length, 'items) ===');
        if (items.length > 0) {
          const json = items[0].json;
          const keys = Object.keys(json);
          console.log('Keys:', keys.join(', '));
          if (json.plan) {
            console.log('Plan keys:', Object.keys(json.plan).join(', '));
            console.log('Sample:', JSON.stringify(json.plan).substring(0, 600));
          } else {
            console.log('First item:', JSON.stringify(json).substring(0, 400));
          }
        }
      }
    }
  } else {
    console.log('No runData in resultData');
    console.log('data keys:', Object.keys(e.data));
  }
} else {
  console.log('No data field. Top keys:', Object.keys(e));
}
