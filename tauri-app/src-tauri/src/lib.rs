use serde::{Deserialize, Serialize};
use std::fs;
use std::process::{Child, Command};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::sync::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

struct ServiceHandles(Mutex<Vec<Child>>);

fn orch_url() -> String { std::env::var("ORCH_URL").unwrap_or_else(|_| "http://localhost:3001".into()) }
fn paper_root() -> String { std::env::var("PAPER_DIR").unwrap_or_else(|_| "D:/repo/daily-info/paper".into()) }

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Status {
    pub ok: bool,
    pub novel: Option<NovelInfo>,
    pub slug: Option<String>,
    pub chapter_plans_remaining: Option<u32>,
    pub chapters_today: Option<u32>,
    pub chapter_count: Option<i32>,
    pub plans_left: Option<i32>,
    pub is_running: Option<bool>,
    pub daily_schedule: Option<String>,
    pub weekly_check_day: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogsResponse {
    pub ok: bool,
    pub lines: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TriggerResponse {
    pub ok: bool,
    pub message: Option<String>,
    pub workflow: Option<String>,
    pub execution_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChapterFile {
    pub filename: String,
    pub title: String,
    pub sequence: u32,
    pub size: u64,
    pub modified: u64,
}

// ---------------------------------------------------------------------------
// API helpers
// ---------------------------------------------------------------------------

async fn api_get(path: &str) -> Result<String, String> {
    let url = format!("{}{}", orch_url(), path);
    reqwest::get(&url).await
        .map_err(|e| format!("HTTP error: {}", e))?
        .text().await
        .map_err(|e| format!("Read error: {}", e))
}

async fn api_post(path: &str) -> Result<String, String> {
    let url = format!("{}{}", orch_url(), path);
    reqwest::Client::new().post(&url).send().await
        .map_err(|e| format!("HTTP error: {}", e))?
        .text().await
        .map_err(|e| format!("Read error: {}", e))
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_status(novel_id: Option<String>) -> Result<Status, String> {
    let id = novel_id.filter(|s| !s.is_empty());
    let path = if let Some(ref id) = id { format!("/status?novel_id={}", id) } else { "/status".to_string() };
    let body = api_get(&path).await?;
    serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {} (body: {})", e, &body[..body.len().min(200)]))
}

#[tauri::command]
async fn get_logs() -> Result<LogsResponse, String> {
    let body = api_get("/logs").await?;
    serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {}", e))
}

#[tauri::command]
async fn trigger_daily(novel_id: Option<String>) -> Result<TriggerResponse, String> {
    let path = if let Some(id) = novel_id { format!("/daily?novel_id={}&force=true", id) } else { "/daily?force=true".to_string() };
    let body = api_post(&path).await?;
    serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {}", e))
}

#[tauri::command]
async fn trigger_workflow(name: String) -> Result<TriggerResponse, String> {
    let body = api_post(&format!("/trigger/{}", name)).await?;
    serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {}", e))
}

#[tauri::command]
async fn list_chapters(novel_slug: String) -> Result<Vec<ChapterFile>, String> {
    let mut chapters = Vec::new();
    let dir_path = format!("{}/{}", paper_root(), novel_slug);
    let dir = match fs::read_dir(&dir_path) {
        Ok(d) => d,
        Err(_) => return Ok(chapters),
    };

    for entry in dir.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "md") { continue; }
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let metadata = match fs::metadata(&path) { Ok(m) => m, Err(_) => continue };

        let seq: u32 = filename
            .split("ch").last()
            .and_then(|s| s.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
            .unwrap_or(0);

        let title = fs::read_to_string(&path).ok()
            .and_then(|c| c.lines().find(|l| l.starts_with("# ")).map(|l| l.trim_start_matches("# ").to_string()))
            .unwrap_or_else(|| filename.clone());

        let modified = metadata.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs()).unwrap_or(0);

        chapters.push(ChapterFile { filename, title, sequence: seq, size: metadata.len(), modified });
    }
    chapters.sort_by_key(|c| c.sequence);
    Ok(chapters)
}

#[tauri::command]
async fn read_chapter(novel_slug: String, filename: String) -> Result<String, String> {
    let path = format!("{}/{}/{}", paper_root(), novel_slug, filename);
    fs::read_to_string(&path).map_err(|e| format!("Read error: {}", e))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NovelInfo {
    #[serde(default)]
    pub id: String,
    pub name: Option<String>,
    #[serde(default)]
    pub slug: String,
    pub genre: Option<String>,
    pub status: Option<String>,
    pub target_words: Option<i32>,
    pub chapter_count: Option<i32>,
    pub total_words: Option<i64>,
    pub plans_left: Option<i32>,
    pub chapters_today: Option<i32>,
    pub created_at: Option<String>,
}

#[tauri::command]
async fn list_novels() -> Result<Vec<NovelInfo>, String> {
    let body = api_get("/novels").await?;
    let resp: serde_json::Value = serde_json::from_str(&body).map_err(|e| format!("JSON error: {}", e))?;
    let novels = resp["novels"].as_array().unwrap_or(&vec![]).iter()
        .map(|n| NovelInfo {
            id: n["id"].as_str().map(|s| s.to_string()).unwrap_or_default(),
            name: n["name"].as_str().map(|s| s.to_string()),
            slug: n["slug"].as_str().map(|s| s.to_string()).unwrap_or_default(),
            genre: n["genre"].as_str().map(|s| s.to_string()),
            status: n["status"].as_str().map(|s| s.to_string()),
            target_words: n["target_words"].as_i64().map(|v| v as i32),
            chapter_count: n["chapter_count"].as_i64().map(|v| v as i32),
            total_words: n["total_words"].as_i64(),
            plans_left: n["plans_left"].as_i64().map(|v| v as i32),
            chapters_today: n["chapters_today"].as_i64().map(|v| v as i32),
            created_at: n["created_at"].as_str().map(|s| s.to_string()),
        }).collect();
    Ok(novels)
}

#[tauri::command]
async fn create_novel(name: String) -> Result<TriggerResponse, String> {
    let url = format!("{}/novels", orch_url());
    let body = reqwest::Client::new().post(&url)
        .header("Content-Type", "application/json")
        .body(format!("{{\"name\":\"{}\"}}", name))
        .send().await.map_err(|e| format!("HTTP: {}", e))?
        .text().await.map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&body).map_err(|e| format!("JSON: {}", e))
}

#[tauri::command]
async fn delete_novel(id: String) -> Result<TriggerResponse, String> {
    let url = format!("{}/novels/{}", orch_url(), id);
    let body = reqwest::Client::new().delete(&url).send().await.map_err(|e| format!("HTTP error: {}", e))?.text().await.map_err(|e| format!("Read error: {}", e))?;
    serde_json::from_str(&body).map_err(|e| format!("JSON error: {}", e))
}


// ---------------------------------------------------------------------------
// App Entry Point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // Auto-start writer-service + orchestrator
            let mut children = Vec::new();
            #[cfg(target_os = "windows")]
            {
                const NO_WINDOW: u32 = 0x08000000;
                let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_default();
                let base = if exe_dir.to_string_lossy().contains("target") { std::env::current_dir().unwrap_or_default() } else { exe_dir };
                let mut c1 = Command::new("node"); c1.args(["writer-service/server.js"]).current_dir(&base);
                #[cfg(target_os = "windows")] { c1.creation_flags(NO_WINDOW); }
                if let Ok(c) = c1.spawn() { children.push(c); }
                let mut c2 = Command::new("node"); c2.args(["orchestrator/scheduler.js"]).current_dir(&base);
                #[cfg(target_os = "windows")] { c2.creation_flags(NO_WINDOW); }
                if let Ok(c) = c2.spawn() { children.push(c); }
            }
            app.manage(ServiceHandles(Mutex::new(children)));

            // Build tray menu
            let open = MenuItemBuilder::with_id("open", "Open Panel").build(app)?;
            let write = MenuItemBuilder::with_id("write", "Write Chapter Now").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&open)
                .item(&write)
                .separator()
                .item(&quit)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("AI Novel Factory")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "write" => {
                        let _ = api_post("/daily");
                    }
                    "quit" => {
                        if let Some(h) = app.try_state::<ServiceHandles>() {
                            if let Ok(mut ch) = h.0.lock() {
                                for c in ch.iter_mut() { let _ = c.kill(); let _ = c.wait(); }
                            }
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_logs,
            trigger_daily,
            trigger_workflow,
            list_chapters,
            read_chapter,
            list_novels,
            create_novel,
            delete_novel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
