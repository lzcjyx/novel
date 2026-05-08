const fs = require('fs');
const j = JSON.parse(fs.readFileSync('D:/novel/n8n/wf05_exec34.json', 'utf8'));
const rd = j.data?.resultData;
if (!rd || !rd.runData) { console.log('No runData'); process.exit(); }

// Show Weekly Plan Summary
const wps = (rd.runData['Weekly Plan Summary'] || [])[0];
if (wps && wps.data) {
  const items = wps.data.main[0] || [];
  console.log('=== Weekly Plan Summary (' + items.length + ' items) ===');
  items.forEach(i => console.log(JSON.stringify(i.json, null, 2)));
}

// Also show Parse Arc Plan keys to verify
const pap = (rd.runData['Parse Arc Plan'] || [])[0];
if (pap && pap.data) {
  const items = pap.data.main[0] || [];
  if (items.length > 0) {
    const plan = items[0].json.plan;
    console.log('\n=== Plan keys ===');
    for (const [k, v] of Object.entries(plan)) {
      if (Array.isArray(v)) console.log(k + ': ' + v.length + ' items');
      else console.log(k + ': ' + typeof v);
    }
  }
}
