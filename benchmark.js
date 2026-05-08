// Tauri App Benchmark v2 — verify real outcomes, not just API connectivity
// Tests: Dashboard data matches DB+files, buttons complete tasks, logs clean
const ORCH = "http://localhost:3001";
const { execSync } = require("child_process");
const fs = require("fs");
let passed = 0, failed = 0;

async function test(name, fn) {
  process.stdout.write(`  ${name}... `);
  try { await fn(); console.log("✓"); passed++; }
  catch (e) { console.log(`✗ ${e.message}`); failed++; }
}

async function api(method, path, body) {
  const opts = { method, headers: { "Content-Type": "application/json" } };
  if (body) opts.body = JSON.stringify(body);
  const resp = await fetch(`${ORCH}${path}`, opts);
  const data = await resp.json();
  if (!resp.ok) throw new Error(`HTTP ${resp.status}: ${JSON.stringify(data).substring(0,100)}`);
  return data;
}

const PAPER_DIR = "D:/repo/daily-info/paper";

function getDB() {
  const { Pool } = require("pg");
  return new Pool({ connectionString: "postgresql://neondb_owner:npg_9s0iqbRjPALB@ep-floral-glitter-ap7osh22-pooler.c-7.us-east-1.aws.neon.tech/neondb?sslmode=require", max: 1 });
}

async function main() {
  console.log("=== AI Novel Factory Benchmark v2 ===\n");
  const pool = getDB();
  let novelId = "", novelSlug = "";
  let chapterCountBefore = 0, novelCountBefore = 0;

  // === 1. DATA INTEGRITY: Dashboard shows real data ===
  console.log("[1] Dashboard Data Integrity");
  await test("Orchestrator + n8n + writer all up", async () => {
    await api("GET", "/status");
    const r = await fetch("http://localhost:5678/healthz"); if (!r.ok) throw new Error("n8n down");
    const r2 = await fetch("http://localhost:8787/health"); if (!r2.ok) throw new Error("writer down");
  });
  await test("GET /novels returns active novels", async () => {
    const data = await api("GET", "/novels");
    if (!data.novels?.length) throw new Error("No novels");
    novelId = data.novels[0].id;
    novelSlug = data.novels[0].slug;
    novelCountBefore = data.novels.length;
    chapterCountBefore = data.novels[0].chapter_count || 0;
  });
  await test("Dashboard Chapter count matches files", async () => {
    const dir = `${PAPER_DIR}/${novelSlug}`;
    const fileCount = fs.existsSync(dir) ? fs.readdirSync(dir).filter(f => f.endsWith(".md")).length : 0;
    const data = await api("GET", `/status?novel_id=${novelId}`);
    if (data.chapter_count !== fileCount) throw new Error(`API=${data.chapter_count} files=${fileCount}`);
    if (data.plans_left === undefined) throw new Error("Missing plans_left");
    if (data.daily_schedule === undefined) throw new Error("Missing schedule");
  });
  await test("Total Words > 0 (real chapters exist)", async () => {
    const data = await api("GET", "/novels");
    const novel = data.novels.find(n => n.id === novelId);
    if (!novel || !novel.total_words || novel.total_words < 100) throw new Error(`Words too low: ${novel?.total_words}`);
  });

  // === 2. WRITE CHAPTER NOW: completes full pipeline ===
  console.log("\n[2] Write Chapter Now — Complete Pipeline");
  const chBefore = chapterCountBefore;
  let dailyOk = false;
  await test("POST /daily triggers pipeline", async () => {
    const data = await api("POST", `/daily?novel_id=${novelId}`);
    dailyOk = data.ok;
  });
  await test("Pipeline runs (wait 90s max for completion)", async () => {
    for (let i = 0; i < 18; i++) {
      await new Promise(r => setTimeout(r, 5000));
      const d = await api("GET", `/status?novel_id=${novelId}`);
      if (!d.is_running && d.chapter_count > 0) break;
    }
  });
  await test("New chapter file created", async () => {
    const dir = `${PAPER_DIR}/${novelSlug}`;
    const count = fs.existsSync(dir) ? fs.readdirSync(dir).filter(f => f.endsWith(".md")).length : 0;
    if (count <= chBefore) throw new Error(`Before=${chBefore} After=${count} — no new file`);
    console.log(` (${count} files)`);
  });
  await test("Chapter in database", async () => {
    const r = await pool.query("SELECT COUNT(*) as c FROM chapters WHERE project_id=$1", [novelId]);
    if (parseInt(r.rows[0].c) <= chBefore) throw new Error("No new DB record");
  });
  await test("Chapter plan status updated (planned→in_progress)", async () => {
    const r = await pool.query("SELECT COUNT(*) as c FROM chapter_plans WHERE project_id=$1 AND status='in_progress'", [novelId]);
    if (parseInt(r.rows[0].c) < 1) throw new Error("No plans set to in_progress");
  });

  // === 3. GENERATE WEEKLY PLAN: completes planning ===
  console.log("\n[3] Generate Weekly Plan — Complete");
  let plansBefore = 0;
  await test("Count plans before trigger", async () => {
    const r = await pool.query("SELECT COUNT(*) as c FROM chapter_plans WHERE project_id=$1 AND status='planned'", [novelId]);
    plansBefore = parseInt(r.rows[0].c);
  });
  await test("POST /trigger/wf05 starts planner", async () => {
    await api("POST", "/trigger/wf05");
  });
  await test("New plans generated (wait 120s max)", async () => {
    for (let i = 0; i < 24; i++) {
      await new Promise(r => setTimeout(r, 5000));
      const r = await pool.query("SELECT COUNT(*) as c FROM chapter_plans WHERE project_id=$1 AND status='planned'", [novelId]);
      if (parseInt(r.rows[0].c) > plansBefore) break;
    }
    const r = await pool.query("SELECT COUNT(*) as c FROM chapter_plans WHERE project_id=$1 AND status='planned'", [novelId]);
    const after = parseInt(r.rows[0].c);
    if (after <= plansBefore) throw new Error(`Before=${plansBefore} After=${after}`);
    console.log(` (${after} plans)`);
  });

  // === 4. +NEW NOVEL: creates novel with DB + file structure ===
  console.log("\n[4] +New Novel — Complete Bootstrap");
  await test("POST /novels triggers bootstrap", async () => {
    await api("POST", "/novels");
  });
  await test("Novel count increased (wait 180s max for bootstrap)", async () => {
    for (let i = 0; i < 36; i++) {
      await new Promise(r => setTimeout(r, 5000));
      const r = await pool.query("SELECT COUNT(*) as c FROM projects");
      if (parseInt(r.rows[0].c) > novelCountBefore) break;
    }
    const r = await pool.query("SELECT COUNT(*) as c FROM projects");
    if (parseInt(r.rows[0].c) <= novelCountBefore) throw new Error("No new project");
  });
  await test("New novel has bible data (characters etc)", async () => {
    const r = await pool.query("SELECT p.id FROM projects p ORDER BY p.created_at DESC LIMIT 1");
    const newId = r.rows[0].id;
    const c = await pool.query("SELECT COUNT(*) as cnt FROM characters WHERE project_id=$1", [newId]);
    if (parseInt(c.rows[0].cnt) < 1) throw new Error("No characters — bible not generated");
    console.log(` (${c.rows[0].cnt} characters)`);
  });

  // === 5. LOGS: no errors/failures ===
  console.log("\n[5] Logs — Clean Execution");
  await test("Logs contain no error/fail/skip lines", async () => {
    const data = await api("GET", "/logs");
    if (!data.lines?.length) throw new Error("Logs empty — orchestrator not logging to file");
    const badLines = data.lines.filter(l =>
      l.toLowerCase().includes("error") ||
      l.toLowerCase().includes("fail") ||
      l.toLowerCase().includes("skip")
    );
    if (badLines.length > 0) throw new Error(`${badLines.length} error/fail/skip lines found:\n${badLines.slice(-3).join("\n")}`);
    console.log(` (${data.lines.length} lines, all clean)`);
  });
  await test("No 'Activate failed' or 'Trigger failed'", async () => {
    const data = await api("GET", "/logs");
    const bad = data.lines.filter(l => l.includes("Activate failed") || l.includes("Trigger failed"));
    if (bad.length > 0) throw new Error(`${bad.length} trigger failures`);
  });

  await pool.end();

  // Summary
  const total = passed + failed;
  console.log(`\n=== Results: ${passed}/${total} passed ===`);
  if (failed > 0) {
    console.log(`  ${failed} FAILED:`);
    console.log("  Check: orchestrator log, n8n executions, writer-service log");
  }
  process.exit(failed > 0 ? 1 : 0);
}

main().catch(e => { console.error("CRASH:", e.message); process.exit(1); });
