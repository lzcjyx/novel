// AI Novel Factory — First-time setup
// Usage: node scripts/setup.js
// 1. Checks .env exists
// 2. Initializes database schema
// 3. Creates paper directory
// 4. Verifies connectivity

const fs = require("fs");
const path = require("path");
const { Pool } = require("pg");
require("dotenv").config({ path: path.join(__dirname, "..", ".env") });

const ROOT = path.join(__dirname, "..");
const PAPER_DIR = process.env.PAPER_DIR || path.join(ROOT, "data", "paper");
const DB_URL = process.env.NEON_DATABASE_URL_POOLED;

async function main() {
  console.log("=== AI Novel Factory Setup ===\n");

  // 1. Check .env
  if (!fs.existsSync(path.join(ROOT, ".env"))) {
    console.error("ERROR: .env not found. Copy .env.example to .env and fill in your values.");
    process.exit(1);
  }
  console.log("[1/4] .env found ✓");

  // 2. Check DB connection
  if (!DB_URL || DB_URL.includes("CHANGE_ME")) {
    console.error("ERROR: Database URL not configured in .env");
    process.exit(1);
  }
  const pool = new Pool({ connectionString: DB_URL, max: 1 });
  try {
    await pool.query("SELECT 1");
    console.log("[2/4] Database connection OK ✓");
  } catch (e) {
    console.error("ERROR: Cannot connect to database:", e.message);
    process.exit(1);
  }

  // 3. Run migrations
  const sqlDir = path.join(ROOT, "sql");
  const files = fs.readdirSync(sqlDir).filter(f => f.endsWith(".sql")).sort();
  try {
    await pool.query("SELECT 1 FROM schema_migrations");
    console.log("[3/4] Schema already initialized ✓");
  } catch (e) {
    console.log("[3/4] Initializing database schema...");
    for (const f of files) {
      const sql = fs.readFileSync(path.join(sqlDir, f), "utf8");
      await pool.query(sql);
      console.log("  - " + f + " ✓");
    }
    console.log("[3/4] Schema created ✓");
  }

  // 4. Create paper directory
  if (!fs.existsSync(PAPER_DIR)) {
    fs.mkdirSync(PAPER_DIR, { recursive: true });
    console.log("[4/4] Created " + PAPER_DIR + " ✓");
  } else {
    console.log("[4/4] Paper directory exists ✓");
  }

  await pool.end();
  console.log("\n=== Setup complete! Run the Tauri app or start.bat. ===");
}

main().catch(e => { console.error("Setup failed:", e.message); process.exit(1); });
