/**
 * writer-service — HTTP wrapper around DeepSeek Anthropic-compatible API
 *
 * Endpoints:
 *   POST /generate-chapter  — generate initial chapter draft
 *   POST /revise-chapter    — revise chapter based on review reports
 *   GET  /health            — liveness check
 *   GET  /ready             — readiness check
 *
 * Calls DeepSeek's Anthropic-compatible Messages API:
 *   POST https://api.deepseek.com/anthropic/v1/messages
 *
 * All secrets are read from environment variables — never logged.
 */

require("dotenv").config({ path: require("path").join(__dirname, "..", ".env") });
require("dotenv").config(); // also load local .env as fallback

const express = require("express");
const fs = require("fs");
const path = require("path");
const crypto = require("crypto");

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const PORT = parseInt(process.env.WRITER_SERVICE_PORT || "8787", 10);
const TOKEN = process.env.WRITER_SERVICE_TOKEN || "change_me";
const ANTHROPIC_BASE_URL = process.env.ANTHROPIC_BASE_URL || "https://api.deepseek.com/anthropic";
const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY || "";
const API_URL = ANTHROPIC_BASE_URL.replace(/\/$/, "") + "/v1/messages";
const MODEL = process.env.WRITER_MODEL || "claude-sonnet-4-6";
const MAX_TOKENS = parseInt(process.env.WRITER_MAX_TOKENS || "8192", 10);
const TIMEOUT_MS = parseInt(process.env.CLAUDE_CODE_TIMEOUT_MS || "600000", 10);
const MAX_BODY_SIZE = process.env.WRITER_SERVICE_MAX_BODY || "2mb";
const MAX_CONCURRENT = parseInt(process.env.WRITER_SERVICE_MAX_CONCURRENT || "2", 10);
const PROMPTS_DIR = process.env.WRITER_SERVICE_PROMPTS_DIR ||
  path.join(__dirname, "..", "prompts");

// ---------------------------------------------------------------------------
// Secret redaction
// ---------------------------------------------------------------------------

const SENSITIVE_VALS = [
  ANTHROPIC_API_KEY, TOKEN,
  process.env.OPENAI_API_KEY || "",
  process.env.NEON_DATABASE_URL_POOLED || "",
  process.env.NEON_DATABASE_URL_DIRECT || "",
].filter(v => v && v.length > 4);

function redact(str) {
  let out = str;
  for (const val of SENSITIVE_VALS) {
    const esc = val.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    out = out.replace(new RegExp(esc, "g"), "***REDACTED***");
  }
  return out;
}

function safeLog(level, msg) {
  console[level](redact(msg));
}

// ---------------------------------------------------------------------------
// Concurrency limiter
// ---------------------------------------------------------------------------

let activeRequests = 0;

function concurrencyLimit(req, res, next) {
  if (activeRequests >= MAX_CONCURRENT) {
    return res.status(503).json({
      ok: false,
      error: `Max concurrency (${MAX_CONCURRENT}) reached.`,
    });
  }
  activeRequests++;
  res.on("finish", () => { activeRequests--; });
  next();
}

// ---------------------------------------------------------------------------
// Auth middleware
// ---------------------------------------------------------------------------

function auth(req, res, next) {
  const h = req.get("Authorization") || "";
  const token = h.startsWith("Bearer ") ? h.slice(7) : "";
  if (token !== TOKEN) {
    return res.status(401).json({ ok: false, error: "Unauthorized" });
  }
  next();
}

// ---------------------------------------------------------------------------
// Prompt builder — fills template with writing_brief data
// ---------------------------------------------------------------------------

function buildPrompt(writingBrief, promptType) {
  const promptsDir = path.resolve(PROMPTS_DIR);

  const templateMap = {
    "draft_writer": "draft_writer.md",
    "revision_writer": "revision_writer.md",
    "canon_extractor": "canon_extractor.md",
  };

  const templateFile = templateMap[promptType]
    ? path.join(promptsDir, templateMap[promptType])
    : null;

  if (!templateFile || !fs.existsSync(templateFile)) {
    return { systemPrompt: "", userPrompt: JSON.stringify(writingBrief, null, 2), usedTemplate: "raw" };
  }

  let template = fs.readFileSync(templateFile, "utf-8");

  // Split system prompt (first section before "输入") vs user content
  const parts = template.split(/\n(?=输入|writing_brief:)/);

  const replacements = {
    "{{WRITING_BRIEF_JSON}}": JSON.stringify(writingBrief, null, 2),
    "{{INITIAL_DRAFT_JSON}}": JSON.stringify(writingBrief.initial_draft || {}, null, 2),
    "{{REVIEW_REPORTS_JSON}}": JSON.stringify(writingBrief.review_reports || [], null, 2),
    "{{CHAPTER_JSON}}": JSON.stringify(writingBrief.chapter || {}, null, 2),
    "{{CANON_JSON}}": JSON.stringify(writingBrief.canon || {}, null, 2),
    "{{RECENT_SUMMARIES_JSON}}": JSON.stringify(writingBrief.recent_summaries || [], null, 2),
    "{{CHARACTERS_JSON}}": JSON.stringify(writingBrief.characters || [], null, 2),
    "{{CHARACTER_STATES_JSON}}": JSON.stringify(writingBrief.character_states || [], null, 2),
    "{{PLOT_THREADS_JSON}}": JSON.stringify(writingBrief.plot_threads || [], null, 2),
    "{{FORESHADOWING_JSON}}": JSON.stringify(writingBrief.foreshadowing || [], null, 2),
    "{{STYLE_GUIDE_JSON}}": JSON.stringify(writingBrief.style_guide || {}, null, 2),
    "{{BLOG_CONFIG_JSON}}": JSON.stringify(writingBrief.blog_config || {}, null, 2),
    "{{PROJECT_POLICY_JSON}}": JSON.stringify(writingBrief.project_policy || {}, null, 2),
    "{{PROJECT_ID}}": String(writingBrief.project_id || ""),
    "{{CHAPTER_ID}}": String(writingBrief.chapter_id || ""),
    "{{CHAPTER_TEXT}}": String(writingBrief.chapter_text || ""),
    "{{EXISTING_CANON_JSON}}": JSON.stringify(writingBrief.existing_canon || {}, null, 2),
    "{{QUALITY_THRESHOLD}}": String(writingBrief.quality_threshold || 85),
    "{{REVISE_COUNT}}": String(writingBrief.revise_count || 0),
    "{{PROJECT_JSON}}": JSON.stringify(writingBrief.project || {}, null, 2),
    "{{FINAL_CHAPTER_JSON}}": JSON.stringify(writingBrief.final_chapter || {}, null, 2),
  };

  for (const [ph, val] of Object.entries(replacements)) {
    template = template.split(ph).join(val);
  }

  // The entire filled template goes as the user message.
  // The system prompt is a short instruction extracted from the top of the template.
  const lines = template.split("\n");
  let systemPrompt = "你是一个专业的长篇小说写作AI。请严格按照要求输出合法JSON，不要输出解释或markdown代码块。";
  let userPrompt = template;

  // If the template starts with a clear instruction block, use it as system prompt
  if (lines[0] && lines[0].trim().length > 0 && !lines[0].startsWith("{{")) {
    let endIdx = 0;
    for (let i = 0; i < Math.min(lines.length, 15); i++) {
      if (lines[i].includes("输入") || lines[i].includes("writing_brief") || lines[i].startsWith("{{")) {
        endIdx = i;
        break;
      }
    }
    if (endIdx > 0) {
      systemPrompt = lines.slice(0, endIdx).join("\n").trim();
      userPrompt = lines.slice(endIdx).join("\n").trim();
    }
  }

  return { systemPrompt, userPrompt, usedTemplate: templateMap[promptType] || "raw" };
}

// ---------------------------------------------------------------------------
// Call DeepSeek Anthropic-compatible API
// ---------------------------------------------------------------------------

async function callDeepSeek(systemPrompt, userPrompt) {
  const startTime = Date.now();
  const body = {
    model: MODEL,
    max_tokens: MAX_TOKENS,
    temperature: 0.7,
    system: systemPrompt,
    messages: [
      { role: "user", content: userPrompt },
    ],
  };

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), TIMEOUT_MS);

  try {
    const response = await fetch(API_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "x-api-key": ANTHROPIC_API_KEY,
        "anthropic-version": "2023-06-01",
      },
      body: JSON.stringify(body),
      signal: controller.signal,
    });

    clearTimeout(timer);
    const durationMs = Date.now() - startTime;
    const text = await response.text();

    if (!response.ok) {
      return {
        ok: false,
        data: null,
        stderr: `HTTP ${response.status}: ${text.slice(0, 1000)}`,
        exitCode: response.status,
        durationMs,
      };
    }

    let parsed = null;
    try {
      const json = JSON.parse(text);
      // Anthropic Messages API response format
      const content = json.content?.[0]?.text || "";
      // Try to parse the content as JSON
      const trimmed = content.trim();
      try {
        parsed = JSON.parse(trimmed);
      } catch (_) {
        const m = trimmed.match(/\{[\s\S]*\}/);
        if (m) {
          try { parsed = JSON.parse(m[0]); } catch (_2) {}
        }
      }
      if (!parsed) {
        parsed = { raw_output: trimmed };
      }
    } catch (_) {
      return {
        ok: false,
        data: null,
        stderr: `Failed to parse API response: ${text.slice(0, 500)}`,
        exitCode: -1,
        durationMs,
      };
    }

    return { ok: true, data: parsed, stderr: "", exitCode: 0, durationMs };
  } catch (err) {
    clearTimeout(timer);
    const durationMs = Date.now() - startTime;
    const isTimeout = err.name === "AbortError";
    return {
      ok: false,
      data: null,
      stderr: isTimeout ? `Timed out after ${TIMEOUT_MS}ms` : err.message,
      exitCode: isTimeout ? -1 : -2,
      durationMs,
    };
  }
}

// ---------------------------------------------------------------------------
// Express app
// ---------------------------------------------------------------------------

const app = express();
app.use(express.json({ limit: MAX_BODY_SIZE }));

// ---------------------------------------------------------------------------
// GET /health
// ---------------------------------------------------------------------------

app.get("/health", (_req, res) => {
  res.json({
    ok: true,
    service: "novel-writer-service",
    version: "1.0.0",
    provider: "deepseek",
    model: MODEL,
    uptime_s: Math.floor(process.uptime()),
    active_requests: activeRequests,
    max_concurrent: MAX_CONCURRENT,
  });
});

// ---------------------------------------------------------------------------
// GET /ready
// ---------------------------------------------------------------------------

app.get("/ready", (_req, res) => {
  const draftExists = fs.existsSync(path.resolve(PROMPTS_DIR, "draft_writer.md"));
  res.json({
    ok: true,
    provider: "deepseek",
    api_url: API_URL.replace(/\/[^/]+$/, "/***"),
    model: MODEL,
    prompts_dir: path.resolve(PROMPTS_DIR),
    prompts_exist: draftExists,
    has_api_key: ANTHROPIC_API_KEY.length > 4,
  });
});

// ---------------------------------------------------------------------------
// POST /generate-chapter
// ---------------------------------------------------------------------------

app.post("/generate-chapter", auth, concurrencyLimit, async (req, res) => {
  const { job_id, project_id, chapter_plan_id, writing_brief, prompt_type } = req.body;

  if (!writing_brief) {
    return res.status(400).json({ ok: false, error: "writing_brief is required" });
  }

  const promptType = prompt_type || "draft_writer";

  safeLog("log",
    `[generate-chapter] job=${job_id} project=${project_id} plan=${chapter_plan_id} type=${promptType}`
  );

  try {
    const { systemPrompt, userPrompt, usedTemplate } = buildPrompt(writing_brief, promptType);
    safeLog("log",
      `[generate-chapter] job=${job_id} template=${usedTemplate} prompt_len=${userPrompt.length}`
    );

    const result = await callDeepSeek(systemPrompt, userPrompt);

    if (result.ok && result.data) {
      const wordCount = result.data.word_count ??
        (result.data.body_markdown || "").length;
      safeLog("log",
        `[generate-chapter] OK job=${job_id} words~${wordCount} dur=${result.durationMs}ms`
      );
      return res.json({
        ok: true,
        data: result.data,
        stderr: redact(result.stderr || ""),
        duration_ms: result.durationMs,
      });
    }

    safeLog("error",
      `[generate-chapter] FAIL job=${job_id} code=${result.exitCode} dur=${result.durationMs}ms`
    );
    return res.json({
      ok: false,
      error: result.stderr || `API call failed with code ${result.exitCode}`,
      stderr: redact(result.stderr || ""),
      exitCode: result.exitCode,
      duration_ms: result.durationMs,
    });
  } catch (err) {
    safeLog("error", `[generate-chapter] EXCEPTION job=${job_id}: ${err.message}`);
    return res.status(500).json({
      ok: false,
      error: err.message,
      stderr: "",
      exitCode: -1,
      duration_ms: 0,
    });
  }
});

// ---------------------------------------------------------------------------
// POST /revise-chapter
// ---------------------------------------------------------------------------

app.post("/revise-chapter", auth, concurrencyLimit, async (req, res) => {
  const { job_id, project_id, chapter_plan_id, writing_brief, prompt_type } = req.body;

  if (!writing_brief) {
    return res.status(400).json({ ok: false, error: "writing_brief is required" });
  }

  const promptType = prompt_type || "revision_writer";

  safeLog("log",
    `[revise-chapter] job=${job_id} project=${project_id} plan=${chapter_plan_id} type=${promptType}`
  );

  try {
    const { systemPrompt, userPrompt, usedTemplate } = buildPrompt(writing_brief, promptType);
    safeLog("log",
      `[revise-chapter] job=${job_id} template=${usedTemplate} prompt_len=${userPrompt.length}`
    );

    const result = await callDeepSeek(systemPrompt, userPrompt);

    if (result.ok && result.data) {
      safeLog("log", `[revise-chapter] OK job=${job_id} dur=${result.durationMs}ms`);
      return res.json({
        ok: true,
        data: result.data,
        stderr: redact(result.stderr || ""),
        duration_ms: result.durationMs,
      });
    }

    safeLog("error",
      `[revise-chapter] FAIL job=${job_id} code=${result.exitCode} dur=${result.durationMs}ms`
    );
    return res.json({
      ok: false,
      error: result.stderr || `API call failed with code ${result.exitCode}`,
      stderr: redact(result.stderr || ""),
      exitCode: result.exitCode,
      duration_ms: result.durationMs,
    });
  } catch (err) {
    safeLog("error", `[revise-chapter] EXCEPTION job=${job_id}: ${err.message}`);
    return res.status(500).json({
      ok: false,
      error: err.message,
      stderr: "",
      exitCode: -1,
      duration_ms: 0,
    });
  }
});

// ---------------------------------------------------------------------------
// 404 / error handlers
// ---------------------------------------------------------------------------

app.use((_req, res) => res.status(404).json({ ok: false, error: "Not found" }));

app.use((err, _req, res, _next) => {
  safeLog("error", `[writer-service] unhandled: ${err.message}`);
  res.status(500).json({ ok: false, error: "Internal server error" });
});

// ---------------------------------------------------------------------------
// Graceful shutdown
// ---------------------------------------------------------------------------

let server;

process.on("SIGTERM", () => {
  safeLog("log", "[writer-service] SIGTERM, shutting down");
  if (server) server.close(() => process.exit(0));
  setTimeout(() => process.exit(0), 5000);
});

process.on("SIGINT", () => {
  safeLog("log", "[writer-service] SIGINT, shutting down");
  if (server) server.close(() => process.exit(0));
  setTimeout(() => process.exit(0), 3000);
});

// ---------------------------------------------------------------------------
// Start
// ---------------------------------------------------------------------------

server = app.listen(PORT, () => {
  safeLog("log", `[writer-service] listening on :${PORT}`);
  safeLog("log", `[writer-service] provider: deepseek  model: ${MODEL}`);
  safeLog("log", `[writer-service] api: ${API_URL.replace(/\/[^/]+$/, "/***")}`);
  safeLog("log", `[writer-service] timeout: ${TIMEOUT_MS}ms  max_concurrent: ${MAX_CONCURRENT}`);
  safeLog("log", `[writer-service] prompts: ${path.resolve(PROMPTS_DIR)}`);
});
