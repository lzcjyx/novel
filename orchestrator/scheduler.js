/**
 * AI Novel Factory — Smart Orchestrator
 *
 * Replaces manual n8n clicking with automated scheduling:
 * - Daily 09:00 JST: trigger WF03 (write chapter) → WF04 (revise)
 * - Every 10 min: check chapter_plans count; trigger WF05 if < 3 planned
 * - WF01/WF02: manual trigger via REST API (for new book bootstrap)
 * - Uses PostgreSQL advisory locks for re-entrancy protection
 */
require("dotenv").config({ path: require("path").join(__dirname, "..", ".env") });

const express = require("express");
const cron = require("node-cron");
const { Pool } = require("pg");
const fs = require("fs");
const path = require("path");

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------
const PORT = parseInt(process.env.ORCHESTRATOR_PORT || "3001", 10);
const N8N_API_URL = process.env.N8N_API_URL || "http://localhost:5678/api/v1";
const N8N_API_KEY = process.env.N8N_API_KEY || "";
const DB_URL = process.env.NEON_DATABASE_URL_POOLED || "";
const TZ = process.env.SCHEDULE_TIMEZONE || "Asia/Shanghai";
const DAILY_HOUR = parseInt(process.env.SCHEDULE_DAILY_HOUR || "18", 10);
const CHECK_INTERVAL_MIN = parseInt(process.env.SCHEDULE_CHECK_MIN || "10", 10);
const MIN_CHAPTER_PLANS = parseInt(process.env.MIN_CHAPTER_PLANS || "3", 10);
const WEEKLY_CHECK_DAY = parseInt(process.env.SCHEDULE_WEEKLY_DAY || "0", 10); // 0=Sunday

const LOG_FILE = process.env.ORCHESTRATOR_LOG_FILE || path.join(__dirname, "orchestrator.log");
const HEADERS = { "Content-Type": "application/json", "X-N8N-API-KEY": N8N_API_KEY };

// ---------------------------------------------------------------------------
// Database
// ---------------------------------------------------------------------------
const pool = new Pool({
  connectionString: DB_URL,
  max: 2,
  idleTimeoutMillis: 30000,
});

// ---------------------------------------------------------------------------
// n8n API helpers
// ---------------------------------------------------------------------------
let workflowCache = null; // { name: { id, name } }
let cacheTime = 0;

async function getWorkflowIds() {
  if (workflowCache && Date.now() - cacheTime < 600000) return workflowCache; // 10-min cache
  const resp = await fetch(N8N_API_URL + "/workflows?limit=20", { headers: HEADERS });
  const list = await resp.json();
  workflowCache = {};
  for (const w of list.data || []) {
    workflowCache[w.name] = { id: w.id, name: w.name };
  }
  cacheTime = Date.now();
  return workflowCache;
}

async function triggerWorkflow(workflowId, webhookBody = "{}") {
  // Get workflow name to find webhook path
  const wfResp = await fetch(N8N_API_URL + `/workflows/${workflowId}`, { headers: HEADERS });
  if (!wfResp.ok) { log(`Get workflow failed: ${wfResp.status}`); return null; }
  const wf = await wfResp.json();
  const webhook = wf.nodes.find(n => n.type === "n8n-nodes-base.webhook");
  const name = wf.name || "";

  if (webhook && webhook.parameters && webhook.parameters.path) {
    // Use webhook trigger
    const whPath = webhook.parameters.path;
    const whUrl = `http://localhost:5678/webhook/${whPath}`;
    log(`Triggering ${name} via webhook ${whPath}`);
    // Fire webhook in background
    fetch(whUrl, { method: "POST", headers: { "Content-Type": "application/json" }, body: webhookBody })
      .catch(e => log(`Webhook trigger error: ${e.message}`));
    // Wait 15s for execution to start, then take latest
    await sleep(15000);
    const listResp = await fetch(N8N_API_URL + `/executions?workflowId=${workflowId}&limit=1`, { headers: HEADERS });
    const list = await listResp.json();
    const exec = (list.data && list.data[0]) ? list.data[0] : null;
    if (exec) log(`${name} execution ${exec.id} (${exec.status})`);
    return exec ? exec.id : "triggered";
  }

  // Fallback: try direct execution
  const execResp = await fetch(N8N_API_URL + `/workflows/${workflowId}/executions`, {
    method: "POST", headers: HEADERS, body: JSON.stringify({}),
  });
  if (!execResp.ok) {
    log(`Trigger failed for ${workflowId.slice(0,8)}: ${await execResp.text()}`);
    return null;
  }
  const exec = await execResp.json();
  return exec.executionId || exec.id;
}

async function waitForExecution(executionId, timeoutMs = 600000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    await sleep(5000);
    const resp = await fetch(N8N_API_URL + `/executions/${executionId}`, { headers: HEADERS });
    const exec = await resp.json();
    if (exec.status === "success") return true;
    if (exec.status === "error" || exec.status === "crashed") {
      log(`Execution ${executionId.slice(0,8)} failed: ${exec.status}`);
      return false;
    }
  }
  log(`Execution ${executionId.slice(0,8)} timed out`);
  return false;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// ---------------------------------------------------------------------------
// Counting helper
// ---------------------------------------------------------------------------
async function getChapterPlansCount(novelId) {
  let q = `SELECT COUNT(*) AS cnt FROM chapter_plans cp
     JOIN projects p ON p.id = cp.project_id
     WHERE cp.status = 'planned'`;
  if (novelId) q += ` AND p.id = '${novelId}'`;
  else q += ` AND p.status = 'active'`;
  const result = await pool.query(q);
  return parseInt(result.rows[0].cnt, 10);
}

function getTodayChapterCount(slug) {
  try {
    const fs = require("fs");
    const path = require("path");
    const dir = slug ? `${PAPER_ROOT}/${slug}` : PAPER_ROOT;
    if (!fs.existsSync(dir)) return 0;
    const files = fs.readdirSync(dir).filter(f => f.endsWith(".md"));
    const today = new Date().toDateString();
    let count = 0;
    for (const f of files) {
      const stat = fs.statSync(path.join(dir, f));
      if (new Date(stat.mtime).toDateString() === today) count++;
    }
    return count;
  } catch (e) {
    return 0;
  }
}

async function getActiveNovelSlug() {
  const r = await pool.query("SELECT name FROM projects WHERE status = 'active' ORDER BY created_at DESC LIMIT 1");
  if (!r.rows.length) return null;
  return (r.rows[0].name || "novel").replace(/[^a-zA-Z0-9一-鿿]+/g, "-").toLowerCase();
}

async function isJobRunning() {
  const result = await pool.query(
    `SELECT COUNT(*) AS cnt FROM generation_jobs
     WHERE job_date = CURRENT_DATE AND status IN ('started', 'reviewing')`
  );
  return parseInt(result.rows[0].cnt, 10) > 0;
}

// ---------------------------------------------------------------------------
// Advisory lock for exclusive access
// ---------------------------------------------------------------------------
async function tryAcquireLock(lockId = 1) {
  const result = await pool.query("SELECT pg_try_advisory_lock($1) AS locked", [lockId]);
  return result.rows[0].locked;
}

async function releaseLock(lockId = 1) {
  await pool.query("SELECT pg_advisory_unlock($1)", [lockId]);
}

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------
function log(msg) {
  const ts = new Date().toISOString();
  const line = `[${ts}] ${msg}`;
  console.log(line);
  try {
    fs.appendFileSync(LOG_FILE, line + "\n");
  } catch (e) {}
}

// ---------------------------------------------------------------------------
// Core: Run daily pipeline (WF03 → WF04) for specified or active novel
// ---------------------------------------------------------------------------
async function runDailyPipeline(novelId, force = false) {
  log(`=== Starting daily pipeline${force ? ' (FORCED)' : ''} ===`);

  if (!novelId) {
    const r = await pool.query("SELECT id, name FROM projects WHERE status = 'active' ORDER BY created_at DESC LIMIT 1");
    if (!r.rows.length) { log("No active projects found"); return; }
    novelId = r.rows[0].id;
    log(`Auto-selected novel: ${r.rows[0].name} (${novelId.slice(0,8)})`);
  }

  if (!force) {
    if (await isJobRunning()) {
      log("Job already running — chapter being generated now, please wait");
      return;
    }
    const todayFiles = getTodayChapterCount();
    if (todayFiles > 0) {
      log(`Today's chapter already generated (${todayFiles} file(s)). Next chapter scheduled tomorrow ${DAILY_HOUR}:00. Manual trigger bypasses this limit.`);
      return;
    }
  }

  const locked = await tryAcquireLock(1);
  if (!locked) {
    log("Could not acquire advisory lock — another orchestrator is running");
    return;
  }

  try {
    const wfs = await getWorkflowIds();
    const wf03 = wfs["[NF] 03 — Daily Chapter Production"];
    const wf04 = wfs["[NF] 04 — Review and Repair"];

    if (!wf03) { log("WF03 not found in n8n!"); return; }
    if (!wf04) { log("WF04 not found in n8n!"); return; }

    // Step 1: Run WF03 (daily chapter) with novel_id
    log(`Triggering WF03 (${wf03.id.slice(0,8)})...`);
    const webhookBody = JSON.stringify({ project_id: novelId });
    const execId3 = await triggerWorkflow(wf03.id, webhookBody);
    if (!execId3) { log("WF03 trigger failed"); return; }
    log(`WF03 started: ${execId3.slice(0,8)}`);

    // Wait for completion
    const ok3 = await waitForExecution(execId3);
    if (!ok3) { log("WF03 did not complete successfully"); return; }
    log("WF03 completed successfully");

    // Step 2: Run WF04 (review & repair)
    log(`Triggering WF04 (${wf04.id.slice(0,8)})...`);
    const execId4 = await triggerWorkflow(wf04.id);
    if (!execId4) { log("WF04 trigger failed"); return; }
    log(`WF04 started: ${execId4.slice(0,8)}`);
    const ok4 = await waitForExecution(execId4);
    if (!ok4) { log("WF04 did not complete successfully"); return; }
    log("WF04 review completed");
  } catch (err) {
    log(`Daily pipeline error: ${err.message}`);
  } finally {
    await releaseLock(1);
  }
}

// ---------------------------------------------------------------------------
// Core: Check if weekly plan is needed
// ---------------------------------------------------------------------------
async function checkWeeklyPlan() {
  try {
    const count = await getChapterPlansCount();
    log(`Chapter plans check: ${count} planned (threshold: ${MIN_CHAPTER_PLANS})`);

    if (count < MIN_CHAPTER_PLANS) {
      const locked = await tryAcquireLock(2);
      if (!locked) return;

      try {
        const wfs = await getWorkflowIds();
        const wf05 = wfs["[NF] 05 — Weekly Arc Planner"];
        if (!wf05) { log("WF05 not found!"); return; }

        log(`Low plans (${count}) — triggering WF05 (${wf05.id.slice(0,8)})...`);
        const execId = await triggerWorkflow(wf05.id);
        if (execId) {
          log(`WF05 started: ${execId.slice(0,8)}`);
          await waitForExecution(execId, 900000); // 15-min timeout for planning
          const newCount = await getChapterPlansCount();
          log(`WF05 complete. Plans now: ${newCount}`);
        }
      } finally {
        await releaseLock(2);
      }
    }
  } catch (err) {
    log(`Weekly plan check error: ${err.message}`);
  }
}

// ===========================================================================
// HTTP API
// ===========================================================================
const app = express();
app.use(express.json());

// ----- Novel management -----

const PAPER_ROOT = process.env.PAPER_DIR || "D:/repo/daily-info/paper";

// GET /novels — list all novels with stats
app.get("/novels", async (req, res) => {
  try {
    const result = await pool.query(
      `SELECT p.id, p.name, p.genre, p.status, p.total_target_words, p.created_at,
        COALESCE((SELECT COUNT(*) FROM chapters WHERE project_id = p.id), 0)::int AS chapter_count,
        COALESCE((SELECT COALESCE(SUM(word_count),0) FROM chapters WHERE project_id = p.id), 0)::bigint AS total_words,
        COALESCE((SELECT COUNT(*) FROM chapter_plans WHERE project_id = p.id AND status = 'planned'), 0)::int AS plans_left
       FROM projects p ORDER BY p.created_at DESC`
    );
    const novels = result.rows.map(r => {
      const slug = "novel-" + r.id.slice(0,8);
      const dir = `${PAPER_ROOT}/${slug}`;
      let files = [];
      try { files = require("fs").readdirSync(dir).filter(f => f.endsWith(".md")).sort(); } catch(e) {}
      const today = new Date().toDateString();
      let todayCount = 0; let totalWords = 0;
      for (const f of files) {
        try {
          const fullPath = `${dir}/${f}`;
          const st = require("fs").statSync(fullPath);
          if (new Date(st.mtime).toDateString() === today) todayCount++;
          // Count words from file content (approx: char count / 2 for Chinese)
          const content = require("fs").readFileSync(fullPath, "utf8");
          totalWords += Math.round(content.replace(/[#>\-\*\s\n]/g, "").length / 2);
        } catch(e) {}
      }
      return {
        id: r.id, name: r.name, slug, genre: r.genre, status: r.status,
        target_words: r.total_target_words, chapter_count: files.length,
        total_words: totalWords, plans_left: r.plans_left,
        chapters_today: todayCount, created_at: r.created_at,
      };
    });
    res.json({ ok: true, novels });
  } catch(e) { res.status(500).json({ ok: false, error: e.message }); }
});

// POST /novels — create new novel (DB insert + dir create atomic, then bootstrap)
app.post("/novels", async (req, res) => {
  const client = await pool.connect();
  try {
    const name = req.body?.name || `My Novel ${new Date().toISOString().slice(0,10)}`;

    // Step 1: Create project record (immediate, atomic)
    await client.query("BEGIN");
    const result = await client.query(
      `INSERT INTO projects (name, genre, target_audience, style_profile, total_target_words, daily_target_words, status, metadata)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id, name, status, created_at`,
      [name, "fantasy", "18-35", JSON.stringify({tone:"热血",narrative_style:""}), 500000, 5000, "active", JSON.stringify({})]
    );
    await client.query("COMMIT");
    const project = result.rows[0];
    const slug = "novel-" + project.id.slice(0,8);

    // Step 2: Create paper directory
    const dir = `${PAPER_ROOT}/${slug}`;
    try { require("fs").mkdirSync(dir, { recursive: true }); } catch(e) {}

    log(`Novel created: ${name} (${project.id.slice(0,8)}) dir: ${dir}`);

    // Done — user can now activate and generate plans/write chapters
    res.json({ ok: true, project: { id: project.id, name: project.name, slug, status: project.status }, message: "Novel created. Activate and generate plans to start writing." });
  } catch(e) {
    await client.query("ROLLBACK").catch(()=>{});
    res.status(500).json({ ok: false, error: e.message });
  } finally {
    client.release();
  }
});

// DELETE /novels/:id — delete novel and all its files
app.delete("/novels/:id", async (req, res) => {
  const { id } = req.params;
  try {
    // Get novel info before deleting
    const info = await pool.query("SELECT name FROM projects WHERE id = $1", [id]);
    if (!info.rows.length) return res.status(404).json({ ok: false, error: "Novel not found" });
    const slug = "novel-" + id.slice(0,8);

    // Delete DB records (order matters for FK constraints)
    await pool.query("DELETE FROM generation_jobs WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM chapter_versions WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM chapters WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM chapter_plans WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM agent_reviews WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM vector_documents WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM canon_rules WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM characters WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM plot_threads WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM world_lore WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM organizations WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM items WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM locations WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM magic_or_power_systems WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM style_guides WHERE project_id = $1", [id]);
    await pool.query("DELETE FROM projects WHERE id = $1", [id]);

    // Delete files
    const dir = `${PAPER_ROOT}/${slug}`;
    try {
      const fs = require("fs");
      if (fs.existsSync(dir)) {
        fs.readdirSync(dir).forEach(f => fs.unlinkSync(`${dir}/${f}`));
        fs.rmdirSync(dir);
      }
    } catch(e) { log(`File cleanup warning: ${e.message}`); }

    log(`Novel "${name}" (${id.slice(0,8)}) deleted`);
    res.json({ ok: true, message: `Novel "${name}" deleted` });
  } catch(e) { res.status(500).json({ ok: false, error: e.message }); }
});

// ----- Status & triggers (per-novel) -----

// GET /status — current state for specified novel (or default)
app.get("/status", async (req, res) => {
  const novelId = req.query.novel_id;
  try {
    // If specific novel requested, return per-novel stats
    if (novelId) {
      const result = await pool.query(
        `SELECT p.id, p.name, p.genre, p.status, p.total_target_words,
          COALESCE((SELECT COUNT(*) FROM chapter_plans WHERE project_id = p.id AND status = 'planned'), 0)::int AS plans_left
         FROM projects p WHERE p.id = $1`, [novelId]);
      if (!result.rows.length) return res.status(404).json({ ok: false, error: "Novel not found" });
      const r = result.rows[0];
      const slug = "novel-" + r.id.slice(0,8);
      const dir = `${PAPER_ROOT}/${slug}`;
      let todayCount = 0; let fileChapterCount = 0;
      try {
        const fs = require("fs"); const today = new Date().toDateString();
        const files = fs.readdirSync(dir).filter(f => f.endsWith(".md"));
        fileChapterCount = files.length;
        files.forEach(f => {
          try { if (new Date(fs.statSync(`${dir}/${f}`).mtime).toDateString() === today) todayCount++; } catch(e) {}
        });
      } catch(e) {}
      return res.json({
        ok: true, novel: r, slug, chapters_today: todayCount,
        chapter_count: fileChapterCount,
        plans_left: parseInt(r.plans_left),
        is_running: await isJobRunning() || !!activeJob,
        active_job: activeJob,
        daily_schedule: `${DAILY_HOUR}:00 ${TZ}`,
      });
    }
    // Default: return first active novel
    const wfs = await getWorkflowIds();
    const plansCount = await getChapterPlansCount();
    const todayCount = await getTodayChapterCount();
    const running = await isJobRunning();
    res.json({
      ok: true,
      active_job: activeJob,
      chapter_plans_remaining: plansCount,
      chapters_today: todayCount,
      is_running: running || !!activeJob,
      workflows: Object.fromEntries(
        Object.entries(wfs).map(([name, w]) => [name, w.id.slice(0, 8)])
      ),
      daily_schedule: `${DAILY_HOUR}:00 ${TZ}`,
      weekly_check_day: WEEKLY_CHECK_DAY === 0 ? "Sunday" : `Day ${WEEKLY_CHECK_DAY}`,
    });
  } catch (err) {
    res.status(500).json({ ok: false, error: err.message });
  }
});

// POST /trigger/:name — manually trigger a workflow
app.post("/trigger/:name", async (req, res) => {
  const wfs = await getWorkflowIds();
  const search = req.params.name.toLowerCase().replace(/^wf0?/,"");
  const key = Object.keys(wfs).find((k) => k.toLowerCase().includes(search));
  if (!key) return res.status(404).json({ ok: false, error: `Workflow ${req.params.name} not found` });

  try {
    const execId = await triggerWorkflow(wfs[key].id);
    res.json({ ok: true, workflow: key, execution_id: execId });
  } catch (err) {
    res.status(500).json({ ok: false, error: err.message });
  }
});

// Track running execution for progress reporting
let activeJob = null; // { execId, workflowId, startedAt, stage }

// POST /daily — run daily pipeline (force=true bypasses one-per-day limit)
app.post("/daily", async (req, res) => {
  const novelId = req.body?.novel_id || req.query.novel_id;
  const force = req.body?.force || req.query.force;
  activeJob = { stage: "starting", startedAt: new Date().toISOString(), novel_id: novelId, force: !!force };
  res.json({ ok: true, message: force ? "Forced pipeline started" : "Daily pipeline started", novel_id: novelId, stage: "starting", force: !!force });
  try {
    activeJob.stage = "wf03_running";
    await runDailyPipeline(novelId, !!force);
    activeJob.stage = "complete";
  } catch(e) {
    activeJob.stage = "error"; activeJob.error = e.message;
    log(`Manual daily error: ${e.message}`);
  }
  setTimeout(() => { activeJob = null; }, 60000);
});

// GET /logs — recent logs
app.get("/logs", (req, res) => {
  try {
    const data = fs.readFileSync(LOG_FILE, "utf8");
    const lines = data.split("\n").filter(Boolean).slice(-50);
    res.json({ ok: true, lines });
  } catch (e) {
    res.json({ ok: true, lines: [] });
  }
});

// ===========================================================================
// Cron schedules
// ===========================================================================

// Daily pipeline: 09:00 JST (UTC+9)
cron.schedule(`0 ${DAILY_HOUR} * * *`, () => {
  log("Cron: daily pipeline triggered");
  runDailyPipeline().catch((e) => log(`Cron daily error: ${e.message}`));
}, { timezone: TZ });

// Periodic check: every N minutes
cron.schedule(`*/${CHECK_INTERVAL_MIN} * * * *`, () => {
  checkWeeklyPlan().catch((e) => log(`Periodic check error: ${e.message}`));
  syncDBToFiles().catch((e) => log(`Sync error: ${e.message}`));
});

// Sync: remove DB chapters whose .md files don't exist
async function syncDBToFiles() {
  try {
    const novels = await pool.query("SELECT id, name FROM projects");
    for (const n of novels.rows) {
      const slug = "novel-" + n.id.slice(0,8);
      const dir = `${PAPER_ROOT}/${slug}`;
      let fileSeqs = [];
      try { fileSeqs = require("fs").readdirSync(dir).filter(f => f.endsWith(".md")).map(f => parseInt(f.match(/ch(\d+)/)?.[1]) || 0).filter(s => s > 0); } catch(e) {}
      if (fileSeqs.length === 0) continue;
      // Delete DB chapters not in file list
      const r = await pool.query(
        `DELETE FROM chapters WHERE project_id = $1 AND sequence NOT IN (${fileSeqs.join(",")})`,
        [n.id]
      );
      if (r.rowCount > 0) log(`Synced "${n.name}": removed ${r.rowCount} orphan chapters`);
    }
  } catch(e) {}
}

// ===========================================================================
// Startup catch-up: run daily pipeline if we missed the scheduled time
// ===========================================================================
async function startupCatchUp() {
  // Check if any chapters exist at all (new project, don't catch up)
  const totalChapters = await pool.query("SELECT COUNT(*) as c FROM chapters");
  if (parseInt(totalChapters.rows[0].c) === 0) {
    log("Startup: new project, no chapters yet — skipping catch-up");
    return;
  }
  const todayCount = await getTodayChapterCount();
  if (todayCount > 0) {
    log(`Startup: ${todayCount} chapters already generated today — skipping catch-up`);
    return;
  }

  // Check if scheduled time has passed
  const now = new Date();
  const scheduledTime = new Date(now);
  scheduledTime.setHours(DAILY_HOUR, 0, 0, 0);

  if (now >= scheduledTime) {
    log(`Startup: Scheduled time ${DAILY_HOUR}:00 passed, no chapters today — running catch-up pipeline now`);
    // Delay 30s to let n8n fully initialize
    setTimeout(() => {
      runDailyPipeline().catch((e) => log(`Catch-up error: ${e.message}`));
    }, 30000);
  } else {
    log(`Startup: Scheduled time ${DAILY_HOUR}:00 not yet reached — waiting`);
  }
}

// ===========================================================================
// Startup
// ===========================================================================
app.listen(PORT, () => {
  log(`Orchestrator started on port ${PORT}`);
  log(`Daily schedule: ${DAILY_HOUR}:00 ${TZ}`);
  log(`Check interval: every ${CHECK_INTERVAL_MIN} min`);
  log(`Weekly plan threshold: < ${MIN_CHAPTER_PLANS} chapters`);
  log("Services: n8n=" + N8N_API_URL + " db=" + (DB_URL ? "connected" : "MISSING"));

  // Run catch-up check after a short delay
  setTimeout(() => {
    startupCatchUp().catch((e) => log(`Catch-up check error: ${e.message}`));
  }, 5000);
});
