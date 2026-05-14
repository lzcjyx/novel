import { useState, useEffect, useCallback, createContext, useContext } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// ---- Types ----
interface ProjectStats { id: string; name: string; slug: string; genre?: string; status: string; target_words?: number; chapter_count: number; total_words: number; plans_left: number; chapters_today: number; created_at: string; }
interface Chapter { id: string; project_id: string; chapter_plan_id?: string; sequence: number; title?: string; status: string; word_count?: number; summary?: string; published_at?: string; }
interface ChapterPlan { id: string; sequence: number; title?: string; outline?: string; target_word_count?: number; status: string; }
interface ChapterVersion { id: string; chapter_id: string; version_number: number; version_type: string; title?: string; body_markdown?: string; word_count?: number; }
interface AgentReview { id: string; agent_name: string; score?: number; pass?: boolean; blocking_issues: string; minor_issues: string; recommendations: string; }
interface ReviewScores { average_score?: number; final_score?: number; decision?: string; publish_allowed: boolean; blocking_issue_count: number; }
interface GenerationJob { id: string; chapter_plan_id: string; job_date: string; status: string; started_at: string; completed_at?: string; error_message?: string; }
interface GenerationResult { ok: boolean; message: string; chapter_id?: string; chapter_title?: string; sequence?: number; word_count?: number; final_score?: number; decision?: string; }
interface StatusResponse { ok: boolean; novel?: { name: string; genre?: string; }; slug?: string; chapter_count?: number; chapters_today?: number; plans_left?: number; total_words?: number; is_running: boolean; }
interface BibleData { characters: any[]; locations: any[]; organizations: any[]; items: any[]; world_lore: any[]; magic_systems: any[]; canon_rules: any[]; plot_threads: any[]; foreshadowing: any[]; style_guides: any[]; timeline_events: any[]; }
interface AppSettings { provider: string; model: string; base_url: string; embedding_model: string; embedding_provider: string; embedding_base_url: string; embedding_dim: number; quality_threshold: number; auto_publish: boolean; max_revise_count: number; daily_target_words: number; data_dir: string; debug_mode: boolean; blog_provider: string; }
interface Project { id: string; name: string; }

// ---- Context ----
interface AppContextType {
  projects: ProjectStats[];
  selected: string; setSelected: (id: string) => void;
  settings: AppSettings | null;
  refreshSettings: () => void;
  status: StatusResponse | null;
  logs: string[];
  loading: boolean; setLoading: (v: boolean) => void;
  msg: string; setMsg: (m: string) => void;
}
const Ctx = createContext<AppContextType>(null!);
const useApp = () => useContext(Ctx);

// ---- Main App ----
function App() {
  const [page, setPage] = useState("dashboard");
  const [projects, setProjects] = useState<ProjectStats[]>([]);
  const [selected, setSelected] = useState("");
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [msg, setMsg] = useState("");

  const loadProjects = useCallback(async () => {
    try { const p = await invoke<ProjectStats[]>("get_projects"); setProjects(p); if (p.length && !selected) setSelected(p[0].id); } catch (e) { console.error(e); }
  }, [selected]);

  const refreshSettings = useCallback(async () => {
    try { setSettings(await invoke<AppSettings>("get_settings")); } catch (e) { console.error(e); }
  }, []);

  const loadStatus = useCallback(async () => {
    try { setStatus(await invoke<StatusResponse>("get_status", { projectId: selected || null })); } catch (e) { console.error(e); }
  }, [selected]);

  const loadLogs = useCallback(async () => {
    try { const l = await invoke<string[]>("get_logs"); setLogs(l); } catch (e) { console.error(e); }
  }, []);

  useEffect(() => { loadProjects(); refreshSettings(); loadLogs(); }, []);
  useEffect(() => { if (selected) loadStatus(); const t = setInterval(() => { if (selected) loadStatus(); loadLogs(); }, 10000); return () => clearInterval(t); }, [selected]);

  const ctx = { projects, selected, setSelected, settings, refreshSettings, status, logs, loading, setLoading, msg, setMsg };

  // Pre-render all pages, show/hide with CSS to preserve state across tab switches
  const pageComponents: Record<string, React.ReactNode> = {
    dashboard: <Dashboard />,
    projects: <ProjectList refresh={loadProjects} />,
    chapters: <Chapters />,
    plans: <ChapterPlans />,
    reviews: <ReviewPage />,
    jobs: <JobsPage />,
    bible: <BiblePage />,
    settings: <SettingsPage refreshSettings={refreshSettings} />,
    learn: <LearnPage />,
  };

  const navLabels: Record<string, string> = {
    dashboard: "Dashboard", projects: "Projects", chapters: "Chapters",
    plans: "Plans", reviews: "Reviews", jobs: "Jobs", bible: "Bible", learn: "Learn", settings: "Settings",
  };

  return (
    <Ctx.Provider value={ctx}>
      <div className="app-layout">
        <div className="app-sidebar">
          <div className="sidebar-brand">AI Novel Factory</div>
          {Object.keys(navLabels).map(p => (
            <button key={p} className={`sidebar-nav-btn ${page === p ? "active" : ""}`} onClick={() => setPage(p)}>
              {navLabels[p]}
            </button>
          ))}
          <select className="sidebar-select" value={selected} onChange={e => setSelected(e.target.value)}>
            <option value="">-- Select Project --</option>
            {projects.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
          </select>
        </div>
        <main className="app-main">
          {Object.entries(pageComponents).map(([key, component]) => (
            <div key={key} style={{ display: page === key ? "block" : "none" }}>{component}</div>
          ))}
        </main>
      </div>
    </Ctx.Provider>
  );
}

// ---- Dashboard ----
function Dashboard() {
  const { status, loading, selected, settings, logs, msg, setLoading, setMsg } = useApp();
  const [pipelineSteps, setPipelineSteps] = useState<Array<{step:string,status:string,detail?:string,progress_pct:number,timestamp:string}>>([]);
  const [progress, setProgress] = useState("");

  // Listen for Tauri pipeline events
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listen("pipeline-step", (event: any) => {
      const ev = event.payload;
      setPipelineSteps((prev: any) => {
        const existing = prev.findIndex((s: any) => s.step === ev.step);
        if (existing >= 0) {
          const next = [...prev];
          next[existing] = ev;
          return next;
        }
        return [...prev, ev];
      });
      setProgress(`${Math.round(ev.progress_pct)}%`);
    }).then((u) => { unlisten = u; });
    return () => { if (unlisten) unlisten(); };
  }, []);

  useEffect(() => {
    if (!loading) { setProgress(""); return; }
    setPipelineSteps([]); // Clear on new run
    setProgress("Starting...");
  }, [loading]);

  const handleWrite = async () => {
    setLoading(true); setMsg("");
    try {
      const r = await invoke<GenerationResult>("generate_next_chapter", { projectId: selected, force: true });
      setMsg(r.message);
    } catch (e) { setMsg("Error: " + e); }
    setLoading(false);
  };

  const handleWeekly = async () => {
    setLoading(true); setMsg("");
    try {
      const r = await invoke<GenerationResult>("run_weekly_arc_planner", { projectId: selected });
      setMsg(r.message);
    } catch (e) { setMsg("Error: " + e); }
    setLoading(false);
  };

  const selNovel = status?.novel;

  return (
    <>
      {selNovel && (
        <div className="hero-band">
          {selNovel.name}
          <div style={{ fontSize: 14, fontWeight: 400, marginTop: 8, opacity: 0.8 }}>{selNovel.genre}</div>
        </div>
      )}
      <h2 className="page-title" style={selNovel ? undefined : {}}>{selNovel ? "" : "Dashboard"}</h2>

      <div className="status-grid">
        {[
          {l:"Chapters",v:status?.chapter_count ?? "?"},
          {l:"Today",v:status?.chapters_today ?? "?"},
          {l:"Plans Left",v:status?.plans_left ?? "?"},
          {l:"Total Words",v:(status?.total_words||0).toLocaleString()},
          {l:"Status",v: loading ? "RUNNING" : (status?.is_running ? "RUNNING" : "IDLE"), badge: true},
          {l:"RAG",v: settings?.embedding_provider && settings.embedding_provider !== "none" ? "ON" : "OFF"},
        ].map((s,i) => (
          <div key={i} className="status-card">
            <div className="status-label">{s.l}</div>
            <div className="status-value">{s.v}</div>
          </div>
        ))}
      </div>

      <div style={{ marginBottom: 16 }}>
        <button className="btn btn-primary" onClick={handleWrite} disabled={loading || !selected}>Write Chapter Now</button>
        <button className="btn btn-primary" onClick={handleWeekly} disabled={loading || !selected} style={{ marginLeft: 8 }}>Generate Weekly Plan</button>
        {(status?.is_running && !loading) && <button className="btn btn-reset" style={{ marginLeft: 8 }} onClick={async () => { await invoke("reset_running"); }}>Reset Stuck Job</button>}
      </div>

      {loading && pipelineSteps.length > 0 && (
        <div className="card" style={{ marginTop: 16 }}>
          <h3 className="section-title">Pipeline Progress</h3>
          <div style={{ fontFamily: "monospace", fontSize: 12 }}>
            {["acquire_lock","load_canon","retrieve_context","generate_draft","aggregate_reviews","revise","export","update_canon","complete"].map(step => {
              const s = pipelineSteps.find(p => p.step === step);
              const isReview = step.startsWith("review_");
              if (isReview) return null;
              const icons: Record<string,string> = {running:"◌",done:"✓",failed:"✗"};
              const colors: Record<string,string> = {running:"var(--primary)",done:"var(--success)",failed:"var(--warning)"};
              const label: Record<string,string> = {
                acquire_lock:"获取生成锁",load_canon:"加载圣经数据",retrieve_context:"向量检索上下文",
                generate_draft:"AI生成初稿",aggregate_reviews:"汇总审稿意见",
                revise:"AI修订",export:"导出Markdown",update_canon:"更新圣经",complete:"完成"
              };
              return (
                <div key={step} style={{ padding:"3px 0", color: s ? colors[s.status]||"var(--on-dark-mute)" : "var(--on-dark-mute)" }}>
                  {s ? (icons[s.status]||"○") : "○"} {label[step]||step}
                  {s?.detail ? <span style={{fontSize:11,marginLeft:8,opacity:0.7}}>{s.detail}</span> : null}
                </div>
              );
            })}
          </div>
          <div style={{ marginTop: 8, fontSize: 12, color: "var(--primary)" }}>{progress}</div>
        </div>
      )}
      {msg && <div className={`msg-banner ${msg.toLowerCase().includes("error") ? "msg-error" : "msg-success"}`}>{msg}</div>}

      <div style={{ marginTop: 24 }}>
        <h3 className="section-title">Recent Logs</h3>
        <div className="card">
          {logs.slice(-20).map((l,i) => <div key={i} className="log-line">{l}</div>)}
        </div>
      </div>
    </>
  );
}

// ---- ProjectList ----
function ProjectList({ refresh }: { refresh: () => void }) {
  const { projects, setSelected, setLoading, setMsg } = useApp();
  const [showCreate, setShowCreate] = useState(false);
  const [form, setForm] = useState({ name: "My Novel", genre: "fantasy", targetAudience: "", tone: "热血", description: "" });

  const handleCreate = async () => {
    setLoading(true); setMsg("Generating novel bible via AI... (may take 30-60s)");
    setShowCreate(false);
    try {
      const result = await invoke<Project>("create_project", {
        name: form.name,
        genre: form.genre || "fantasy",
        targetAudience: form.targetAudience || "general",
        tone: form.tone || "neutral",
        description: form.description || "",
        targetTotalWords: 500000,
        dailyTargetWords: 3000,
      });
      await refresh();
      setSelected(result.id);
      setMsg(`Novel "${result.name}" created! Switch to Bible/Plans tabs.`);
    } catch (e) { setMsg("Error: " + e); }
    setLoading(false);
  };

  const handleDelete = async (id: string) => {
    const p = projects.find(x => x.id === id);
    if (!confirm(`Delete "${p?.name}"?`)) return;
    try { await invoke("delete_project", { id }); refresh(); } catch (e) { alert("Error: " + e); }
  };

  return (
    <>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <h2 className="page-title" style={{ marginBottom: 0 }}>Projects ({projects.length})</h2>
        <button className="btn btn-primary" onClick={() => setShowCreate(!showCreate)}>+ New Novel</button>
      </div>

      {showCreate && (
        <div className="card-feature" style={{ marginBottom: 24 }}>
          <h3 className="section-title">Create New Novel</h3>
          <p className="text-meta" style={{ marginBottom: 16 }}>Fill in your world-building preferences. The AI will generate a complete bible based on your input.</p>
          <div className="bible-edit-field">
            <label>Novel Name</label>
            <input value={form.name} onChange={e => setForm({...form, name: e.target.value})} />
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div className="bible-edit-field">
              <label>Genre (type or select)</label>
              <input value={form.genre} onChange={e => setForm({...form, genre: e.target.value})} list="genres" placeholder="e.g. 修仙, 科幻, 都市异能..." />
              <datalist id="genres">
                <option value="修仙" /><option value="武侠" /><option value="科幻" /><option value="都市" /><option value="历史" /><option value="悬疑" /><option value="言情" /><option value="无限流" /><option value="末世" /><option value="游戏异界" />
              </datalist>
            </div>
            <div className="bible-edit-field">
              <label>Tone (type or select)</label>
              <input value={form.tone} onChange={e => setForm({...form, tone: e.target.value})} list="tones" placeholder="e.g. 热血, 轻松, 暗黑..." />
              <datalist id="tones">
                <option value="热血" /><option value="轻松" /><option value="悬疑" /><option value="暗黑" /><option value="史诗" /><option value="幽默" /><option value="温馨" /><option value="冷酷" /><option value="烧脑" />
              </datalist>
            </div>
          </div>
          <div className="bible-edit-field">
            <label>Target Audience</label>
            <input value={form.targetAudience} onChange={e => setForm({...form, targetAudience: e.target.value})} placeholder="e.g. 18-35岁男性读者" />
          </div>
          <div className="bible-edit-field">
            <label>World-Building Description (short)</label>
            <textarea value={form.description} onChange={e => setForm({...form, description: e.target.value})} placeholder="Brief description of the world setting, era, or central concept..." />
          </div>
          <div style={{ marginTop: 16, display: "flex", gap: 8 }}>
            <button className="btn btn-primary" onClick={handleCreate}>Generate Novel Bible</button>
            <button className="btn btn-secondary" onClick={() => setShowCreate(false)}>Cancel</button>
          </div>
        </div>
      )}

      {projects.map(p => (
        <div key={p.id} className="card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div style={{ cursor: "pointer" }} onClick={() => setSelected(p.id)}>
            <div style={{ fontFamily: "var(--font-display)", fontSize: 16, fontWeight: 400, color: "var(--on-dark)" }}>{p.name}</div>
            <div className="text-meta">{p.genre || "N/A"} · {p.chapter_count} ch · {(p.total_words||0).toLocaleString()} words · {p.plans_left} plans · {p.chapters_today} today</div>
          </div>
          <button className="btn btn-sm btn-danger" onClick={() => handleDelete(p.id)}>Delete</button>
        </div>
      ))}
    </>
  );
}

// ---- Chapters ----
function Chapters() {
  const { selected } = useApp();
  const [chapters, setChapters] = useState<Chapter[]>([]);
  const [openId, setOpenId] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [editing, setEditing] = useState(false);
  const [editContent, setEditContent] = useState("");
  const [versions, setVersions] = useState<ChapterVersion[]>([]);
  const [saving, setSaving] = useState(false);

  useEffect(() => { if (selected) invoke<Chapter[]>("get_chapters", { projectId: selected }).then(setChapters).catch(e => console.error(e)); }, [selected]);

  const open = async (ch: Chapter) => {
    if (openId === ch.id) { setOpenId(null); setEditing(false); return; }
    setOpenId(ch.id); setEditing(false);
    try {
      const vers = await invoke<ChapterVersion[]>("get_chapter_versions", { chapterId: ch.id });
      setVersions(vers);
      if (vers.length) { setContent(vers[0].body_markdown || ""); setEditContent(vers[0].body_markdown || ""); }
      else setContent("");
    } catch (_) { setContent("Error loading"); }
  };

  const saveEdit = async () => {
    if (!openId) return;
    setSaving(true);
    try {
      const ch = chapters.find(c => c.id === openId);
      await invoke("save_edited_chapter", { chapterId: openId, title: ch?.title || "Revised", bodyMarkdown: editContent });
      setContent(editContent); setEditing(false);
      // Reload versions
      const vers = await invoke<ChapterVersion[]>("get_chapter_versions", { chapterId: openId });
      setVersions(vers);
    } catch (e) { alert("Error: " + e); }
    setSaving(false);
  };

  return (
    <>
      <h2 className="page-title">Chapters</h2>
      {chapters.map(ch => (
        <div key={ch.id} className="chapter-item" onClick={() => open(ch)}>
          <div className="chapter-title">Ch.{ch.sequence} — {ch.title || "Untitled"} <span className="badge badge-active">{ch.status}</span></div>
          <div className="chapter-meta">{ch.word_count || 0} words</div>
          {openId === ch.id && (
            <div style={{ marginTop: 8 }} onClick={e => e.stopPropagation()}>
              {!editing ? (
                <>
                  <div className="content-preview">{content || "(No content)"}</div>
                  <div style={{ marginTop: 8, display: "flex", gap: 8 }}>
                    <button className="btn btn-sm btn-primary" onClick={() => setEditing(true)}>Edit</button>
                    {versions.length > 1 && (
                      <select className="select" style={{ height: 36, fontSize: 12 }} onChange={e => {
                        const v = versions.find(v => v.id === e.target.value);
                        if (v) { setContent(v.body_markdown || ""); setEditContent(v.body_markdown || ""); }
                      }}>
                        {versions.map(v => <option key={v.id} value={v.id}>v{v.version_number} ({v.version_type})</option>)}
                      </select>
                    )}
                  </div>
                </>
              ) : (
                <>
                  <textarea className="content-editor" value={editContent} onChange={e => setEditContent(e.target.value)} />
                  <div style={{ marginTop: 8, display: "flex", gap: 8 }}>
                    <button className="btn btn-sm btn-primary" onClick={saveEdit} disabled={saving}>{saving ? "Saving..." : "Save Changes"}</button>
                    <button className="btn btn-sm btn-secondary" onClick={() => setEditing(false)}>Cancel</button>
                  </div>
                </>
              )}
            </div>
          )}
        </div>
      ))}
      {chapters.length === 0 && <div className="text-meta">No chapters yet. Create a novel and click Write Chapter Now.</div>}
    </>
  );
}

// ---- ChapterPlans ----
function ChapterPlans() {
  const { selected } = useApp();
  const [plans, setPlans] = useState<ChapterPlan[]>([]);
  const [editId, setEditId] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState("");
  const [editOutline, setEditOutline] = useState("");

  useEffect(() => { if (selected) invoke<ChapterPlan[]>("get_chapter_plans", { projectId: selected }).then(setPlans).catch(e => console.error(e)); }, [selected]);

  const savePlan = async (id: string) => {
    try {
      await invoke("update_chapter_plan", { id, title: editTitle, outline: editOutline });
      setPlans(plans.map(p => p.id === id ? {...p, title: editTitle, outline: editOutline} : p));
      setEditId(null);
    } catch (e) { alert("Error: " + e); }
  };

  return (
    <>
      <h2 className="page-title">Chapter Plans</h2>
      {plans.map(p => (
        <div key={p.id} className="card" onClick={() => { setEditId(p.id); setEditTitle(p.title || ""); setEditOutline(p.outline || ""); }} style={{ cursor: "pointer" }}>
          <div className="chapter-title" style={{ color: p.status === "planned" ? "var(--primary)" : "var(--on-dark)" }}>
            Ch.{p.sequence} — {p.title || "Untitled"} <span className="badge badge-active">{p.status}</span>
          </div>
          {editId === p.id ? (
            <div onClick={e => e.stopPropagation()} style={{ marginTop: 8 }}>
              <div className="bible-edit-field"><label>Title</label><input value={editTitle} onChange={e => setEditTitle(e.target.value)} /></div>
              <div className="bible-edit-field"><label>Outline</label><textarea value={editOutline} onChange={e => setEditOutline(e.target.value)} /></div>
              <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
                <button className="btn btn-sm btn-primary" onClick={() => savePlan(p.id)}>Save</button>
                <button className="btn btn-sm btn-secondary" onClick={() => setEditId(null)}>Cancel</button>
              </div>
            </div>
          ) : (
            <div className="text-meta" style={{ marginTop: 4 }}>{p.outline?.slice(0, 200) || "No outline"} · {p.target_word_count || 0} words</div>
          )}
        </div>
      ))}
      {plans.length === 0 && <div className="text-meta">No chapter plans. Run Generate Weekly Plan first.</div>}
    </>
  );
}

// ---- ReviewPage ----
function ReviewPage() {
  const { selected } = useApp();
  const [chapters, setChapters] = useState<Chapter[]>([]);
  const [selCh, setSelCh] = useState("");
  const [reviews, setReviews] = useState<AgentReview[]>([]);
  const [scores, setScores] = useState<ReviewScores | null>(null);

  useEffect(() => { if (selected) invoke<Chapter[]>("get_chapters", { projectId: selected }).then(setChapters).catch(e => console.error(e)); }, [selected]);
  useEffect(() => {
    if (!selCh) { setReviews([]); setScores(null); return; }
    invoke<AgentReview[]>("get_agent_reviews", { chapterId: selCh }).then(setReviews).catch(e => console.error(e));
    invoke<ReviewScores>("get_review_scores", { chapterId: selCh }).then(setScores).catch(e => console.error(e));
  }, [selCh]);

  const agentNames: Record<string, string> = {
    continuity_reviewer: "Continuity", character_reviewer: "Character", plot_logic_reviewer: "Plot Logic",
    pacing_reviewer: "Pacing", style_reviewer: "Style", safety_reviewer: "Safety", publication_reviewer: "Publication",
  };

  return (
    <>
      <h2 className="page-title">Review Reports</h2>
      <select className="select" value={selCh} onChange={e => setSelCh(e.target.value)} style={{ marginBottom: 16 }}>
        <option value="">-- Select Chapter --</option>
        {chapters.map(c => <option key={c.id} value={c.id}>Ch.{c.sequence} — {c.title}</option>)}
      </select>
      {scores && (
        <div className="card-feature" style={{ marginBottom: 16 }}>
          <div style={{ fontFamily: "var(--font-display)", fontSize: 22, fontWeight: 300, color: "var(--on-dark)" }}>
            Final Score: <span style={{ color: scores.final_score! >= 85 ? "var(--success)" : "var(--primary)" }}>{scores.final_score?.toFixed(0)}</span>
          </div>
          <div className="text-meta" style={{ marginTop: 4 }}>Decision: {scores.decision} · Avg: {scores.average_score?.toFixed(1)} · Blocking: {scores.blocking_issue_count} · Publish: {scores.publish_allowed ? "Yes" : "No"}</div>
        </div>
      )}
      {reviews.map(r => (
        <div key={r.id} className="card" style={{ borderLeft: r.pass ? "3px solid var(--success)" : "3px solid var(--warning)" }}>
          <div className="chapter-title">{agentNames[r.agent_name] || r.agent_name} <span className={`badge ${(r.score || 0) >= 80 ? "badge-success" : (r.score || 0) >= 60 ? "badge-active" : "badge-warning"}`}>{r.score}</span></div>
          <div className="text-meta" style={{ marginTop: 4 }}>{r.pass ? "Pass" : "Fail"}</div>
          {r.blocking_issues && r.blocking_issues !== "[]" && <div className="msg-banner msg-error">{r.blocking_issues.slice(0, 300)}</div>}
        </div>
      ))}
    </>
  );
}

// ---- JobsPage ----
function JobsPage() {
  const { selected } = useApp();
  const [jobs, setJobs] = useState<GenerationJob[]>([]);
  useEffect(() => { if (selected) invoke<GenerationJob[]>("get_generation_jobs", { projectId: selected }).then(setJobs).catch(e => console.error(e)); }, [selected]);

  const color = (s: string) => s === "completed" ? "var(--success)" : s === "failed" ? "var(--warning)" : s === "needs_human_review" ? "#f39c12" : "var(--primary)";

  return (
    <>
      <h2 className="page-title">Generation Jobs</h2>
      {jobs.map(j => (
        <div key={j.id} className="card">
          <div className="chapter-title">{j.job_date} — <span style={{ color: color(j.status) }}>{j.status}</span></div>
          <div className="text-meta">{j.started_at} · {j.completed_at ? `Done: ${j.completed_at}` : ""} · {j.error_message || ""}</div>
        </div>
      ))}
    </>
  );
}

// ---- BiblePage ----
function BiblePage() {
  const { selected } = useApp();
  const [bible, setBible] = useState<BibleData | null>(null);
  const [tab, setTab] = useState("characters");
  const [editingItem, setEditingItem] = useState<any>(null);

  useEffect(() => { if (selected) invoke<BibleData>("get_bible", { projectId: selected }).then(setBible).catch(e => console.error(e)); }, [selected]);

  if (!bible) return <div className="text-meta">Select a project to view Bible data.</div>;

  const tabs: Record<string, any[]> = {
    characters: bible.characters, locations: bible.locations, organizations: bible.organizations,
    items: bible.items, world_lore: bible.world_lore, magic_systems: bible.magic_systems,
    canon_rules: bible.canon_rules, plot_threads: bible.plot_threads, foreshadowing: bible.foreshadowing,
    style_guides: bible.style_guides, timeline_events: bible.timeline_events,
  };
  const tabNames: Record<string, string> = {
    characters: "Characters", locations: "Locations", organizations: "Organizations", items: "Items",
    world_lore: "World Lore", magic_systems: "Magic", canon_rules: "Canon Rules",
    plot_threads: "Plot Threads", foreshadowing: "Foreshadowing", style_guides: "Style Guide", timeline_events: "Timeline",
  };

  const saveBibleEdit = async () => {
    if (!editingItem) return;
    try {
      await invoke("update_bible_entry", { table: tab, id: editingItem.id, data: JSON.stringify(editingItem) });
      setBible({...bible, [tab]: tabs[tab].map(item => item.id === editingItem.id ? editingItem : item)});
      setEditingItem(null);
    } catch (e) { alert("Error: " + e); }
  };

  const editableFields: Record<string, string[]> = {
    characters: ["name","role","personality","motivation","speech_style","appearance","backstory"],
    locations: ["name","type","description"],
    organizations: ["name","description","goals"],
    items: ["name","description","abilities","limitations"],
    world_lore: ["title","lore_type","content"],
    magic_systems: ["name","description","rules","limitations"],
    canon_rules: ["rule_type","rule_text","severity"],
    plot_threads: ["name","description","priority"],
    foreshadowing: ["clue_text","intended_payoff"],
    style_guides: ["name","style_text"],
    timeline_events: [],
  };

  return (
    <>
      <h2 className="page-title">Novel Bible</h2>
      <div className="tab-bar">
        {Object.keys(tabs).map(k => (
          <button key={k} className={`tab-btn ${tab === k ? "active" : ""}`} onClick={() => { setTab(k); setEditingItem(null); }}>{tabNames[k]}</button>
        ))}
      </div>

      {editingItem && (
        <div className="bible-edit-panel">
          <h3 style={{ fontFamily: "var(--font-display)", fontSize: 18, fontWeight: 300, color: "var(--on-dark)", marginBottom: 16 }}>Edit {tabNames[tab]}</h3>
          {(editableFields[tab] || []).map(field => (
            <div className="bible-edit-field" key={field}>
              <label>{field}</label>
              <input value={(editingItem[field] || "") as string} onChange={e => setEditingItem({...editingItem, [field]: e.target.value})} />
            </div>
          ))}
          <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
            <button className="btn btn-sm btn-primary" onClick={saveBibleEdit}>Save</button>
            <button className="btn btn-sm btn-secondary" onClick={() => setEditingItem(null)}>Cancel</button>
            <button className="btn btn-sm btn-commerce" style={{marginLeft:8}} onClick={async () => { await saveBibleEdit(); await invoke("rebuild_vector_index", { projectId: selected }); alert("Applied to all — vector index rebuilt."); }}>Apply to All & Rebuild Index</button>
          </div>
        </div>
      )}

      <div className="card">
        {(tabs[tab] || []).length === 0 && (
          <div className="text-meta">{tab === "foreshadowing" || tab === "timeline_events" ? "These will populate after writing chapters." : "No data."}</div>
        )}
        {(tabs[tab] || []).map((item: any, i: number) => (
          <div key={i} className="bible-item" style={{ cursor: "pointer" }} onClick={() => setEditingItem({...item})}>
            <div className="bible-item-name">
              {item.name || item.title || item.event_summary || item.clue_text?.slice(0, 60) || item.rule_text?.slice(0, 60) || `#${i+1}`}
              <span className="text-meta" style={{ marginLeft: 8, fontWeight: 400 }}>
                {[item.role, item.lore_type, item.item_type, item.severity, item.arc_status, item.importance ? `importance:${item.importance}` : ""].filter(Boolean).join(" · ")}
              </span>
            </div>
            {(item.personality || item.description || item.content || item.motivation || item.clue_text) && (
              <div className="bible-item-desc">{(item.personality || item.description || item.content || item.motivation || item.clue_text || "").slice(0, 200)}</div>
            )}
          </div>
        ))}
      </div>
    </>
  );
}

// ---- SettingsPage ----
function SettingsPage({ refreshSettings }: { refreshSettings: () => void }) {
  const { settings } = useApp();
  const [provider, setProvider] = useState(settings?.provider || "deepseek");
  const [apiKey, setApiKey] = useState("");
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState("");
  const [savedProvider, setSavedProvider] = useState("");

  useEffect(() => {
    if (settings) { setProvider(settings.provider); setSavedProvider(settings.provider); }
  }, [settings]);

  const saveAndTest = async () => {
    if (!apiKey.trim()) { setTestResult("Error: Please enter an API key first."); return; }
    setTesting(true); setTestResult("Connecting...");
    try {
      const r = await invoke<{ ok: boolean; message: string; latency_ms?: number }>("test_model_provider", { provider, apiKey: apiKey.trim(), baseUrl: null, model: settings?.model || null });
      setTestResult(r.ok ? `Connected! (${r.latency_ms ?? "?"}ms) — ${r.message}` : `FAIL: ${r.message}`);
      if (r.ok) setApiKey("");
    } catch (e) { setTestResult("Error: " + e); }
    setTesting(false);
    refreshSettings();
  };

  return (
    <>
      <h2 className="page-title">Settings</h2>
      <div className="card-feature" style={{ marginBottom: 24 }}>
        <h3 className="section-title" style={{ color: "var(--primary)" }}>Model Provider</h3>
        <div className="bible-edit-field">
          <label>Provider</label>
          <select className="select" value={provider} onChange={e => { setProvider(e.target.value); }}>
            <option value="deepseek">DeepSeek (Recommended)</option>
            <option value="kimi">Kimi (月之暗面)</option>
            <option value="zhipu">智谱 GLM</option>
            <option value="openai">OpenAI</option>
            <option value="anthropic">Anthropic / Claude</option>
            <option value="gemini">Google Gemini</option>
            <option value="openai_compat">OpenAI Compatible</option>
            <option value="custom">Custom...</option>
          </select>
        </div>
        <div className="bible-edit-field">
          <label>Model <span className="text-meta">(editable — type any model name)</span></label>
          <input className="text-input" value={settings?.model || ""} onChange={e => invoke("update_settings", { settings: { ...settings, model: e.target.value } }).then(refreshSettings)} placeholder={provider==="deepseek"?"deepseek-v4-pro":provider==="kimi"?"moonshot-v1-8k":provider==="zhipu"?"glm-4-flash":"model name"} />
        </div>
        <div className="bible-edit-field">
          <label>API Key {savedProvider && savedProvider !== provider ? "(saved for " + savedProvider + ")" : savedProvider ? "(saved)" : "(not yet saved)"}</label>
          <input className="text-input" type="password" value={apiKey} onChange={e => setApiKey(e.target.value)} placeholder="sk-..." />
        </div>
        <button className="btn btn-primary" onClick={saveAndTest} disabled={testing || !apiKey.trim()}>{testing ? "Testing..." : "Save & Test Connection"}</button>
        {testResult && (
          <div className={`msg-banner ${testResult.startsWith("Connected") ? "msg-success" : "msg-error"}`} style={{ marginTop: 12 }}>
            {testResult}
          </div>
        )}
      </div>
      {settings && (
        <div className="card">
          <h3 className="section-title">Current Config (click to edit)</h3>
          <SaveField label="Model" value={settings.model} onSave={v => invoke("update_settings", { settings: { ...settings, model: v } }).then(refreshSettings)} />
          <SaveField label="Base URL" value={settings.base_url} onSave={v => invoke("update_settings", { settings: { ...settings, base_url: v } }).then(refreshSettings)} />
          <SaveField label="Data Directory" value={settings.data_dir} onSave={v => invoke("update_settings", { settings: { ...settings, data_dir: v } }).then(refreshSettings)} />
          <SaveField label="Quality Threshold" value={String(settings.quality_threshold)} onSave={v => invoke("update_settings", { settings: { ...settings, quality_threshold: parseInt(v) || 85 } }).then(refreshSettings)} />
          <SaveField label="Max Revise Count" value={String(settings.max_revise_count)} onSave={v => invoke("update_settings", { settings: { ...settings, max_revise_count: parseInt(v) || 2 } }).then(refreshSettings)} />
          <SaveField label="Daily Target Words" value={String(settings.daily_target_words)} onSave={v => invoke("update_settings", { settings: { ...settings, daily_target_words: parseInt(v) || 3000 } }).then(refreshSettings)} />
          <div className="bible-edit-field">
            <label>Embedding Provider</label>
            <select className="select" value={settings.embedding_provider || "none"} onChange={e => {
              invoke("update_settings", { settings: { ...settings, embedding_provider: e.target.value } }).then(refreshSettings);
            }}>
              <option value="none">None (disabled)</option>
              <option value="openai">OpenAI</option>
              <option value="zhipu">智谱 GLM</option>
              <option value="openai_compat">Custom</option>
            </select>
            {(!settings.embedding_provider || settings.embedding_provider === "none") && (
              <div className="msg-banner msg-error" style={{ marginTop: 8 }}>RAG vector search is disabled. Configure an embedding provider to enable. Without RAG, the AI has less continuity context.</div>
            )}
          </div>
          {settings.embedding_provider && settings.embedding_provider !== "none" && (
            <EmbeddingSettings settings={settings} refreshSettings={refreshSettings} />
          )}
          <div className="bible-edit-field" style={{ marginTop: 12 }}>
            <label>Auto Publish</label>
            <input type="checkbox" checked={settings.auto_publish} onChange={e => {
              invoke("update_settings", { settings: { ...settings, auto_publish: e.target.checked } }).then(refreshSettings);
            }} />
          </div>
          <div className="bible-edit-field">
            <label>Debug Mode</label>
            <input type="checkbox" checked={settings.debug_mode} onChange={e => {
              invoke("update_settings", { settings: { ...settings, debug_mode: e.target.checked } }).then(refreshSettings);
            }} />
          </div>
        </div>
      )}
    </>
  );
}

// Inline-edit component: click to edit, blur/Enter to save
function SaveField({ label, value, onSave }: { label: string; value: string; type?: string; onSave: (v: string) => void }) {
  const [editing, setEditing] = useState(false);
  const [val, setVal] = useState(value);
  useEffect(() => { setVal(value); }, [value]);
  const save = () => { setEditing(false); if (val !== value) onSave(val); };
  return (
    <div className="bible-edit-field">
      <label>{label}</label>
      {editing ? (
        <input autoFocus value={val} onChange={e => setVal(e.target.value)} onBlur={save} onKeyDown={e => { if (e.key === "Enter") save(); if (e.key === "Escape") { setVal(value); setEditing(false); } }}
          style={{ padding: "6px 10px", background: "var(--surface-dark-card)", color: "var(--on-dark)", border: "1px solid var(--primary)", borderRadius: "var(--radius-sm)", fontSize: 13, width: "100%", maxWidth: 400 }} />
      ) : (
        <div onClick={() => setEditing(true)} style={{ padding: "6px 10px", cursor: "pointer", color: "var(--on-dark-body)", borderBottom: "1px dashed var(--hairline-dark)" }}>
          {value || "(click to set)"}
        </div>
      )}
    </div>
  );
}

// ---- LearnPage ----
function LearnPage() {
  const { selected } = useApp();
  const [tab, setTab] = useState<"input"|"web"|"library">("library");
  const [text, setText] = useState("");
  const [sourceTitle, setSourceTitle] = useState("");
  const [url, setUrl] = useState("");
  const [learning, setLearning] = useState(false);
  const [result, setResult] = useState("");
  const [entries, setEntries] = useState<any[]>([]);
  const [expandedId, setExpandedId] = useState<string|null>(null);

  const loadEntries = async () => {
    if (!selected) return;
    try { setEntries(await invoke<any[]>("get_learning_entries", { projectId: selected })); } catch (e) { console.error(e); }
  };

  useEffect(() => { loadEntries(); }, [selected]);

  const handleLearnText = async () => {
    if (!text.trim()) return;
    setLearning(true); setResult("Extracting patterns...");
    try {
      const r = await invoke<any[]>("learn_from_text", { projectId: selected, text, sourceTitle: sourceTitle || "User Input" });
      setResult(`Learned ${r.length} patterns`);
      setText(""); setSourceTitle("");
      loadEntries();
    } catch (e) { setResult("Error: " + e); }
    setLearning(false);
  };

  const handleLearnUrl = async () => {
    if (!url.trim()) return;
    // Normalize URL (add https:// if no protocol)
    let fetchUrl = url.trim();
    if (!/^https?:\/\//i.test(fetchUrl)) fetchUrl = "https://" + fetchUrl;
    setLearning(true); setResult("Fetching page...");
    try {
      let html: string;
      try {
        const controller = new AbortController();
        const timer = setTimeout(() => controller.abort(), 30000);
        const resp = await fetch(fetchUrl, { signal: controller.signal });
        clearTimeout(timer);
        if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
        html = await resp.text();
      } catch {
        setResult("Frontend blocked, trying backend...");
        html = await invoke<string>("fetch_url_text", { url: fetchUrl });
      }
      if (html.length > 1_000_000) throw new Error("Page too large (>1MB)");

      // Strip HTML tags in JS (same logic as Rust, no crash risk)
      let raw = html.replace(/<br\s*\/?>/gi, "\n").replace(/<\/?p[^>]*>/gi, "\n");
      raw = raw.replace(/<[^>]*>/g, "");
      raw = raw.replace(/&nbsp;/g, " ").replace(/&amp;/g, "&").replace(/&lt;/g, "<").replace(/&gt;/g, ">");
      raw = raw.replace(/&quot;/g, "\"").replace(/&#39;/g, "'");
      const lines = raw.split("\n").map(l => l.trim()).filter(l =>
        l.length > 40 && !l.startsWith("function") && !l.startsWith("var ") &&
        !l.toLowerCase().includes("cookie") && !l.toLowerCase().includes("subscribe")
      );
      const text = lines.join("\n").slice(0, 15000);
      if (text.length < 200) throw new Error("No meaningful content found. Try a page with article/novel text.");

      setResult("Extracting patterns...");
      const sourceTitle = url.split("/").pop() || "web source";
      const r = await invoke<any[]>("learn_from_text", { projectId: selected, text, sourceTitle, sourceType: "web" });
      setResult(`Learned ${r.length} patterns`);
      setUrl("");
      loadEntries();
    } catch (e: any) { setResult("Error: " + (e.message || String(e))); }
    setLearning(false);
  };

  const handleDelete = async (id: string) => {
    await invoke("delete_learning_entry", { id });
    loadEntries();
  };

  const catColors: Record<string,string> = {
    plot_pattern:"var(--primary)", character_archetype:"var(--commerce)", dialogue_style:"#f39c12",
    sentence_structure:"var(--success)", pacing_technique:"#2196f3", description_method:"#9c27b0",
    narrative_device:"#00bcd4", improvement_note:"var(--warning)",
  };

  return (
    <>
      <h2 className="page-title">Learn & Evolve</h2>
      <div className="tab-bar">
        <button className={`tab-btn ${tab==="input"?"active":""}`} onClick={()=>setTab("input")}>Manual Input</button>
        <button className={`tab-btn ${tab==="web"?"active":""}`} onClick={()=>setTab("web")}>Web Learn</button>
        <button className={`tab-btn ${tab==="library"?"active":""}`} onClick={()=>{setTab("library");loadEntries();}}>Knowledge Library ({entries.length})</button>
      </div>

      {tab === "input" && (
        <div className="card-feature">
          <div className="bible-edit-field"><label>Source Title (e.g. 鲁迅《狂人日记》)</label><input className="text-input" value={sourceTitle} onChange={e=>setSourceTitle(e.target.value)} placeholder="Source name..." /></div>
          <div className="bible-edit-field"><label>Paste sample novel text</label><textarea className="content-editor" value={text} onChange={e=>setText(e.target.value)} placeholder="Paste up to 10000 characters of sample text..."/></div>
          <button className="btn btn-primary" onClick={handleLearnText} disabled={learning||!text.trim()}>{learning?"Extracting...":"Extract Knowledge"}</button>
          {result && <div className={`msg-banner ${result.includes("Error")?"msg-error":"msg-success"}`} style={{marginTop:8}}>{result}</div>}
        </div>
      )}

      {tab === "web" && (
        <div className="card-feature">
          <div className="bible-edit-field"><label>URL to learn from</label><input className="text-input" value={url} onChange={e=>setUrl(e.target.value)} placeholder="https://..." /></div>
          <button className="btn btn-primary" onClick={handleLearnUrl} disabled={learning||!url.trim()}>{learning?"Fetching...":"Fetch & Learn"}</button>
          {result && <div className={`msg-banner ${result.includes("Error")?"msg-error":"msg-success"}`} style={{marginTop:8}}>{result}</div>}
        </div>
      )}

      {tab === "library" && (
        <div className="status-grid" style={{gridTemplateColumns:"1fr"}}>
          {entries.length === 0 && <div className="text-meta">No learned patterns yet. Paste sample text or use Web Learn to build your knowledge base.</div>}
          {entries.map((e: any) => (
            <div key={e.id} className="card" style={{cursor:"pointer", borderLeft:`3px solid ${catColors[e.category]||"var(--hairline-dark)"}`}}>
              <div style={{display:"flex",justifyContent:"space-between",alignItems:"center"}} onClick={()=>setExpandedId(expandedId===e.id?null:e.id)}>
                <div>
                  <span className="chapter-title">{e.pattern_name}</span>
                  <span className="badge badge-active" style={{marginLeft:8}}>{e.category}</span>
                  <span className="text-meta" style={{marginLeft:8}}>{e.source_type === "web" ? "🌐" : e.source_type === "self_reflection" ? "🔄" : "📝"} {e.source_title}</span>
                </div>
                <button className="btn btn-sm btn-danger" onClick={(ev)=>{ev.stopPropagation();handleDelete(e.id);}}>Delete</button>
              </div>
              {expandedId === e.id && (
                <div style={{marginTop:8,padding:8,background:"var(--surface-dark-elevated)",borderRadius:"var(--radius-sm)"}}>
                  <div className="text-body">{e.pattern_description}</div>
                  {e.example_text && <div className="content-preview" style={{marginTop:8}}>{e.example_text.slice(0,500)}</div>}
                  {e.application_notes && <div className="text-meta" style={{marginTop:4}}>Apply: {e.application_notes}</div>}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </>
  );
}

function EmbeddingSettings({ settings, refreshSettings }: { settings: AppSettings; refreshSettings: () => void }) {
  const [embKey, setEmbKey] = useState("");
  const [testing, setTesting] = useState(false);
  const [result, setResult] = useState("");
  const [baseUrl, setBaseUrl] = useState(settings.embedding_base_url || "");
  const [model, setModel] = useState(settings.embedding_model || "text-embedding-3-small");

  useEffect(() => {
    if (settings.embedding_provider === "zhipu" && !baseUrl) setBaseUrl("https://open.bigmodel.cn/api/paas/v4");
    else if (settings.embedding_provider === "openai" && !baseUrl) setBaseUrl("https://api.openai.com/v1");
  }, [settings.embedding_provider]);

  const saveAndTest = async () => {
    if (!embKey.trim()) { setResult("Error: Enter embedding API key"); return; }
    setTesting(true); setResult("Testing...");
    try {
      const r = await invoke<{ok:boolean;message:string;latency_ms?:number}>("test_embedding_provider", { provider: settings.embedding_provider, apiKey: embKey.trim(), baseUrl, model });
      setResult(r.ok ? `Connected! (${r.latency_ms ?? "?"}ms, ${r.message})` : `FAIL: ${r.message}`);
      if (r.ok) { setEmbKey(""); refreshSettings(); }
    } catch (e) { setResult("Error: " + e); }
    setTesting(false);
  };

  return (
    <div className="card" style={{ marginTop: 12, borderLeft: "3px solid var(--primary)" }}>
      <h4 style={{ fontFamily:"var(--font-display)",fontSize:16,fontWeight:300,color:"var(--on-dark)",marginBottom:12 }}>Embedding API Settings</h4>
      <div className="bible-edit-field"><label>Base URL</label><input className="text-input" value={baseUrl} onChange={e=>{setBaseUrl(e.target.value); invoke("update_settings",{settings:{...settings,embedding_base_url:e.target.value}}).then(refreshSettings);}} placeholder="https://..." /></div>
      <div className="bible-edit-field"><label>Model</label><input className="text-input" value={model} onChange={e=>{setModel(e.target.value); invoke("update_settings",{settings:{...settings,embedding_model:e.target.value}}).then(refreshSettings);}} /></div>
      <div className="bible-edit-field"><label>API Key</label><input className="text-input" type="password" value={embKey} onChange={e=>setEmbKey(e.target.value)} placeholder="Enter embedding API key..." /></div>
      <button className="btn btn-primary" onClick={saveAndTest} disabled={testing||!embKey.trim()} style={{marginTop:8}}>{testing?"Testing...":"Save & Test Embedding"}</button>
      {result && <div className={`msg-banner ${result.startsWith("Connected")?"msg-success":"msg-error"}`} style={{marginTop:8}}>{result}</div>}
    </div>
  );
}

export default App;
