use crate::db::connection::Database;
use crate::models::GenerationJob;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};

const DEFAULT_SLOW_PHASE_THRESHOLD_MS: u64 = 30_000;

fn now_timestamp() -> String {
    chrono::Local::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

fn normalize_metadata(raw: &str) -> Value {
    let mut metadata = serde_json::from_str::<Value>(raw).unwrap_or_else(|_| json!({}));
    if !metadata.is_object() {
        metadata = json!({});
    }
    metadata
}

fn total_elapsed_ms(metadata: &Value) -> u64 {
    metadata
        .get("phase_events")
        .and_then(Value::as_array)
        .and_then(|events| events.last())
        .and_then(|event| event.get("elapsed_ms"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn event_elapsed_ms(event: &Value) -> u64 {
    event.get("elapsed_ms").and_then(Value::as_u64).unwrap_or(0)
}

fn event_duration_ms(event: &Value) -> u64 {
    event
        .get("duration_ms")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| event_elapsed_ms(event))
}

fn refresh_phase_summary(
    metadata: &mut Value,
    status_override: Option<&str>,
    failure_reason: Option<&str>,
    completed_at: Option<&str>,
) {
    let events = metadata
        .get("phase_events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let last_event = events.last();
    let last_step = last_event
        .and_then(|event| event.get("step"))
        .and_then(Value::as_str)
        .unwrap_or("job");
    let last_status = status_override
        .or_else(|| {
            last_event
                .and_then(|event| event.get("status"))
                .and_then(Value::as_str)
        })
        .unwrap_or("started");
    let last_detail = failure_reason.or_else(|| {
        last_event
            .and_then(|event| event.get("detail"))
            .and_then(Value::as_str)
    });
    let slow_phase_threshold_ms = metadata
        .get("slow_phase_threshold_ms")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_SLOW_PHASE_THRESHOLD_MS);
    let slowest_event = events.iter().max_by_key(|event| event_duration_ms(event));
    let slowest_step = slowest_event
        .and_then(|event| event.get("step"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let slowest_duration_ms = slowest_event.map(event_duration_ms).unwrap_or(0);
    let slow_steps = events
        .iter()
        .filter(|event| event_duration_ms(event) >= slow_phase_threshold_ms)
        .map(|event| {
            json!({
                "step": event.get("step").and_then(Value::as_str),
                "status": event.get("status").and_then(Value::as_str),
                "detail": event.get("detail").and_then(Value::as_str),
                "duration_ms": event_duration_ms(event),
                "elapsed_ms": event_elapsed_ms(event),
            })
        })
        .collect::<Vec<_>>();

    metadata["phase_summary"] = json!({
        "phase_count": events.len(),
        "last_step": last_step,
        "last_status": last_status,
        "last_detail": last_detail,
        "failure_reason": failure_reason,
        "completed_at": completed_at,
        "total_elapsed_ms": total_elapsed_ms(metadata),
        "slow_phase_threshold_ms": slow_phase_threshold_ms,
        "slowest_step": slowest_step,
        "slowest_duration_ms": slowest_duration_ms,
        "slow_step_count": slow_steps.len(),
        "slow_steps": slow_steps,
        "updated_at": now_timestamp(),
    });
}

fn summarize_usage(metadata: &mut Value) {
    let events = metadata
        .get("model_usage_events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let prompt_tokens = events
        .iter()
        .filter_map(|event| event.get("prompt_tokens").and_then(Value::as_i64))
        .sum::<i64>();
    let completion_tokens = events
        .iter()
        .filter_map(|event| event.get("completion_tokens").and_then(Value::as_i64))
        .sum::<i64>();
    let total_tokens = events
        .iter()
        .filter_map(|event| event.get("total_tokens").and_then(Value::as_i64))
        .sum::<i64>();
    let estimated_cost_usd = events
        .iter()
        .filter_map(|event| event.get("estimated_cost_usd").and_then(Value::as_f64))
        .sum::<f64>();
    let has_cost = events.iter().any(|event| {
        event
            .get("estimated_cost_usd")
            .and_then(Value::as_f64)
            .is_some()
    });
    let provider_reported_call_count = events
        .iter()
        .filter(|event| event.get("usage_source").and_then(Value::as_str) == Some("provider"))
        .count();
    let estimated_call_count = events
        .iter()
        .filter(|event| {
            event
                .get("usage_source")
                .and_then(Value::as_str)
                .unwrap_or("estimated")
                == "estimated"
        })
        .count();

    metadata["usage_summary"] = json!({
        "call_count": events.len(),
        "provider_reported_call_count": provider_reported_call_count,
        "estimated_call_count": estimated_call_count,
        "prompt_tokens": prompt_tokens,
        "completion_tokens": completion_tokens,
        "total_tokens": total_tokens,
        "estimated_cost_usd": if has_cost { json!(estimated_cost_usd) } else { Value::Null },
        "updated_at": now_timestamp(),
    });
}

pub fn create_generation_job(
    db: &Database,
    project_id: &str,
    chapter_plan_id: &str,
) -> Result<String, String> {
    let id = Database::new_uuid();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    conn.execute(
        "INSERT OR IGNORE INTO generation_jobs (id, project_id, chapter_plan_id, job_date, status)
        VALUES (?1, ?2, ?3, ?4, 'started')",
        params![id, project_id, chapter_plan_id, today],
    )
    .map_err(|e| format!("Create job: {}", e))?;

    conn.query_row(
        "SELECT id FROM generation_jobs
         WHERE project_id = ?1 AND chapter_plan_id = ?2 AND job_date = ?3
         ORDER BY created_at ASC LIMIT 1",
        params![project_id, chapter_plan_id, today],
        |row| row.get::<_, String>(0),
    )
    .map_err(|e| format!("Load generation job id: {}", e))
}

pub fn update_job_status(
    db: &Database,
    job_id: &str,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let completed = if status == "completed" || status == "failed" || status == "needs_human_review"
    {
        Some(now_timestamp())
    } else {
        None
    };

    if completed.is_some() || error_message.is_some() {
        let metadata_raw: String = conn
            .query_row(
                "SELECT metadata FROM generation_jobs WHERE id = ?1",
                params![job_id],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "{}".to_string());
        let mut metadata = normalize_metadata(&metadata_raw);
        refresh_phase_summary(
            &mut metadata,
            Some(status),
            error_message,
            completed.as_deref(),
        );
        let metadata_json = serde_json::to_string(&metadata)
            .map_err(|e| format!("Serialize job metadata: {}", e))?;

        conn.execute(
            "UPDATE generation_jobs SET status = ?1, completed_at = ?2, error_message = ?3, metadata = ?4, updated_at = datetime('now') WHERE id = ?5",
            params![status, completed, error_message, metadata_json, job_id],
        ).map_err(|e| format!("Update job: {}", e))?;
    } else {
        conn.execute(
            "UPDATE generation_jobs SET status = ?1, completed_at = ?2, error_message = ?3, updated_at = datetime('now') WHERE id = ?4",
            params![status, completed, error_message, job_id],
        ).map_err(|e| format!("Update job: {}", e))?;
    }
    Ok(())
}

pub fn record_job_phase_event(
    db: &Database,
    job_id: &str,
    step: &str,
    status: &str,
    detail: Option<&str>,
    progress_pct: f64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let metadata_raw: String = conn
        .query_row(
            "SELECT metadata FROM generation_jobs WHERE id = ?1",
            params![job_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load job metadata: {}", e))?;
    let mut metadata = normalize_metadata(&metadata_raw);
    let now_ms = chrono::Local::now().timestamp_millis();
    let started_at_ms = metadata
        .get("phase_started_at_ms")
        .and_then(Value::as_i64)
        .unwrap_or(now_ms);
    metadata["phase_started_at_ms"] = json!(started_at_ms);

    if !metadata.get("phase_events").is_some_and(Value::is_array) {
        metadata["phase_events"] = json!([]);
    }

    let previous_elapsed_ms = metadata
        .get("phase_events")
        .and_then(Value::as_array)
        .and_then(|events| events.last())
        .map(event_elapsed_ms)
        .unwrap_or(0);
    let elapsed_ms = now_ms.saturating_sub(started_at_ms).max(0) as u64;
    let duration_ms = elapsed_ms.saturating_sub(previous_elapsed_ms);
    let event = json!({
        "step": step,
        "status": status,
        "detail": detail,
        "progress_pct": progress_pct,
        "elapsed_ms": elapsed_ms,
        "duration_ms": duration_ms,
        "timestamp": now_timestamp(),
    });

    metadata["phase_events"]
        .as_array_mut()
        .ok_or_else(|| "Job metadata phase_events is not an array".to_string())?
        .push(event);
    let failure_reason = if status == "failed" { detail } else { None };
    refresh_phase_summary(&mut metadata, None, failure_reason, None);

    let metadata_json =
        serde_json::to_string(&metadata).map_err(|e| format!("Serialize job metadata: {}", e))?;
    conn.execute(
        "UPDATE generation_jobs SET metadata = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![metadata_json, job_id],
    )
    .map_err(|e| format!("Record job phase event: {}", e))?;

    Ok(())
}

pub fn estimate_tokens(text: &str) -> i32 {
    let chars = text.chars().count();
    if chars == 0 {
        0
    } else {
        ((chars as f64) / 3.6).ceil() as i32
    }
}

pub fn record_job_model_usage(
    db: &Database,
    job_id: &str,
    phase: &str,
    provider: &str,
    model: &str,
    prompt_tokens: i32,
    completion_tokens: i32,
    input_cost_per_million: Option<f64>,
    output_cost_per_million: Option<f64>,
) -> Result<(), String> {
    record_job_model_usage_with_source(
        db,
        job_id,
        phase,
        provider,
        model,
        prompt_tokens,
        completion_tokens,
        input_cost_per_million,
        output_cost_per_million,
        "estimated",
    )
}

pub fn record_job_model_usage_with_source(
    db: &Database,
    job_id: &str,
    phase: &str,
    provider: &str,
    model: &str,
    prompt_tokens: i32,
    completion_tokens: i32,
    input_cost_per_million: Option<f64>,
    output_cost_per_million: Option<f64>,
    usage_source: &str,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let metadata_raw: String = conn
        .query_row(
            "SELECT metadata FROM generation_jobs WHERE id = ?1",
            params![job_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Load job metadata: {}", e))?;
    let mut metadata = normalize_metadata(&metadata_raw);

    if !metadata
        .get("model_usage_events")
        .is_some_and(Value::is_array)
    {
        metadata["model_usage_events"] = json!([]);
    }

    let prompt_tokens = prompt_tokens.max(0);
    let completion_tokens = completion_tokens.max(0);
    let total_tokens = prompt_tokens + completion_tokens;
    let usage_source = if usage_source == "provider" {
        "provider"
    } else {
        "estimated"
    };
    let estimated_cost_usd = match (input_cost_per_million, output_cost_per_million) {
        (Some(input_rate), Some(output_rate)) => Some(
            (prompt_tokens as f64 / 1_000_000.0 * input_rate)
                + (completion_tokens as f64 / 1_000_000.0 * output_rate),
        ),
        _ => None,
    };

    metadata["model_usage_events"]
        .as_array_mut()
        .ok_or_else(|| "Job metadata model_usage_events is not an array".to_string())?
        .push(json!({
            "phase": phase,
            "provider": provider,
            "model": model,
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": total_tokens,
            "usage_source": usage_source,
            "input_cost_per_million": input_cost_per_million,
            "output_cost_per_million": output_cost_per_million,
            "estimated_cost_usd": estimated_cost_usd,
            "timestamp": now_timestamp(),
        }));
    summarize_usage(&mut metadata);

    let metadata_json =
        serde_json::to_string(&metadata).map_err(|e| format!("Serialize job metadata: {}", e))?;
    conn.execute(
        "UPDATE generation_jobs SET metadata = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![metadata_json, job_id],
    )
    .map_err(|e| format!("Record job model usage: {}", e))?;

    Ok(())
}

pub fn get_generation_jobs(db: &Database, project_id: &str) -> Result<Vec<GenerationJob>, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, chapter_plan_id, job_date, status, started_at, completed_at,
                error_message, retry_count, metadata, created_at, updated_at
         FROM generation_jobs WHERE project_id = ?1 ORDER BY created_at DESC LIMIT 100",
        )
        .map_err(|e| format!("Prepare: {}", e))?;

    let jobs = stmt
        .query_map(params![project_id], |row| {
            Ok(GenerationJob {
                id: row.get(0)?,
                project_id: row.get(1)?,
                chapter_plan_id: row.get(2)?,
                job_date: row.get(3)?,
                status: row.get(4)?,
                started_at: row.get(5)?,
                completed_at: row.get(6)?,
                error_message: row.get(7)?,
                retry_count: row.get(8)?,
                metadata: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })
        .map_err(|e| format!("Query: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect: {}", e))?;

    Ok(jobs)
}

pub fn get_today_chapter_count(db: &Database, project_id: &str) -> Result<i32, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM generation_jobs WHERE project_id = ?1 AND job_date = ?2 AND status = 'completed'",
        params![project_id, today],
        |r| r.get(0),
    ).unwrap_or(0);
    Ok(count)
}

pub fn is_job_running(db: &Database, project_id: &str) -> Result<bool, String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM generation_jobs WHERE project_id = ?1 AND status IN ('started','draft_created','reviewing','revising','publishing')",
        params![project_id],
        |r| r.get(0),
    ).unwrap_or(0);
    Ok(count > 0)
}

/// Mark the most recent non-completed/non-failed job for a project as failed.
/// Used for error recovery — ensures stuck jobs don't show "reviewing" forever.
pub fn mark_latest_job_failed(db: &Database, project_id: &str, error: &str) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let job_id = conn.query_row(
        "SELECT id FROM generation_jobs
         WHERE project_id = ?1 AND status NOT IN ('completed','failed','needs_human_review','skipped')
         ORDER BY started_at DESC LIMIT 1",
        params![project_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| format!("Find latest job: {}", e))?;
    drop(conn);

    if let Some(job_id) = job_id {
        update_job_status(db, &job_id, "failed", Some(error))?;
    }
    Ok(())
}

pub fn recover_stale_running_jobs(
    db: &Database,
    timeout_secs: i64,
    reason: &str,
) -> Result<usize, String> {
    let cutoff_modifier = format!("-{} seconds", timeout_secs.max(0));
    let conn = db.conn.lock().map_err(|e| format!("Lock: {}", e))?;
    let mut stmt = conn
        .prepare(
            "SELECT id FROM generation_jobs
             WHERE status IN ('started','draft_created','reviewing','revising','publishing')
               AND datetime(updated_at) <= datetime('now', ?1)
             ORDER BY updated_at ASC",
        )
        .map_err(|e| format!("Prepare stale job recovery: {}", e))?;
    let job_ids = stmt
        .query_map(params![cutoff_modifier], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Query stale jobs: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect stale jobs: {}", e))?;
    drop(stmt);
    drop(conn);

    for job_id in &job_ids {
        update_job_status(db, job_id, "failed", Some(reason))?;
    }

    Ok(job_ids.len())
}
