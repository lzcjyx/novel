import { useState, useEffect, useCallback, createContext, useContext, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// ---- Types ----
interface ProjectStats { id: string; name: string; slug: string; genre?: string; status: string; target_words?: number; chapter_count: number; total_words: number; plans_left: number; chapters_today: number; created_at: string; }
interface Chapter { id: string; project_id: string; chapter_plan_id?: string; sequence: number; title?: string; status: string; word_count?: number; summary?: string; published_at?: string; }
interface ChapterPlan { id: string; sequence: number; title?: string; outline?: string; target_word_count?: number; status: string; }
interface ChapterVersion { id: string; chapter_id: string; version_number: number; version_type: string; title?: string; body_markdown?: string; word_count?: number; }
interface AgentReview { id: string; agent_name: string; score?: number; pass?: boolean; blocking_issues: string; minor_issues: string; recommendations: string; }
interface CanonPrecheckIssue { rule_type?: string; severity?: string; message?: string; evidence?: string; }
interface ReviewScores { average_score?: NullableNumber; final_score?: NullableNumber; decision?: string; publish_allowed: boolean; blocking_issue_count: number; }
interface AgentQualityScore { agent_name: string; review_count: number; average_score?: NullableNumber; pass_rate?: NullableNumber; blocking_issue_count: number; }
interface ProjectQualitySummary { project_id: string; reviewed_chapter_count: number; publish_ready_count: number; revise_count: number; needs_human_review_count: number; average_score?: NullableNumber; average_final_score?: NullableNumber; total_blocking_issues: number; latest_decision?: string; latest_final_score?: NullableNumber; agent_scores: AgentQualityScore[]; }
interface GenerationJob { id: string; chapter_plan_id: string; job_date: string; status: string; started_at: string; completed_at?: string; error_message?: string; retry_count: number; metadata: string; }
interface JobPhaseEvent { step: string; status: string; detail?: string; progress_pct: number; elapsed_ms: number; duration_ms?: number; timestamp: string; }
interface PipelineStep { step: string; status: string; detail?: string; progress_pct: number; timestamp: string; preview_title?: string; preview_text?: string; preview_kind?: string; }
interface LiveProgressEntry { id: string; step: string; phase: string; status: string; percent: number; detail: string; timestamp: string; preview: boolean; }
interface JobPhaseSummary { phase_count?: number; last_step?: string; last_status?: string; last_detail?: string; failure_reason?: string; completed_at?: string; total_elapsed_ms?: number; slow_phase_threshold_ms?: number; slowest_step?: string; slowest_duration_ms?: number; slow_step_count?: number; slow_steps?: JobPhaseEvent[]; updated_at?: string; }
interface JobModelUsageEvent { phase: string; provider: string; model: string; prompt_tokens: number; completion_tokens: number; total_tokens: number; estimated_cost_usd?: number | null; timestamp: string; }
interface JobUsageSummary { call_count?: number; prompt_tokens?: number; completion_tokens?: number; total_tokens?: number; estimated_cost_usd?: number | null; updated_at?: string; }
interface JobMetadata { phase_events?: JobPhaseEvent[]; phase_summary?: JobPhaseSummary; model_usage_events?: JobModelUsageEvent[]; usage_summary?: JobUsageSummary; }
interface GenerationResult { ok: boolean; message: string; chapter_id?: string; chapter_title?: string; sequence?: number; word_count?: number; final_score?: number; decision?: string; }
interface StatusResponse { ok: boolean; novel?: { name: string; genre?: string; }; slug?: string; chapter_count?: number; chapters_today?: number; plans_left?: number; total_words?: number; is_running: boolean; }
interface BibleData { characters: any[]; locations: any[]; organizations: any[]; items: any[]; world_lore: any[]; magic_systems: any[]; canon_rules: any[]; plot_threads: any[]; foreshadowing: any[]; style_guides: any[]; timeline_events: any[]; }
interface AppSettings { provider: string; model: string; base_url: string; embedding_model: string; embedding_provider: string; embedding_base_url: string; embedding_dim: number; quality_threshold: number; auto_publish: boolean; max_revise_count: number; daily_target_words: number; data_dir: string; debug_mode: boolean; blog_provider: string; input_cost_per_million?: number | null; output_cost_per_million?: number | null; }
interface Project { id: string; name: string; }
interface OperatorControls { generation_mode?: string; chapter_intent?: string; must_include_beats?: string; forbidden_moves?: string; style_emphasis?: string; }
type NullableNumber = number | null | undefined;

interface RetrievalSource { rank: number; document_id: string; source_type: string; source_id?: string; title?: string; excerpt: string; similarity?: NullableNumber; relevance_label: string; metadata: string; }
interface RetrievalTrace { source_count: number; best_similarity?: NullableNumber; sources: RetrievalSource[]; }
interface GraphContextNode { id: string; node_type: string; label: string; }
interface GraphContextNeighbor { id: string; node_type: string; label: string; edge_type: string; direction: string; depth?: number; via_id: string; via_type: string; via_label: string; description?: string; }
interface GraphContext { seeds: GraphContextNode[]; neighbors: GraphContextNeighbor[]; source_keys: string[]; summary: string; }
interface WritingContextPreview { project?: any; chapter_plan?: any; continuity?: any; canon?: any; graph_context?: GraphContext; retrieval?: any[]; retrieval_trace?: RetrievalTrace; style?: any; learned_patterns?: any[]; operator_controls?: OperatorControls; }
interface KnowledgeGraphNode { id: string; node_type: string; label: string; subtitle?: string; description?: string; status: string; degree: number; }
interface KnowledgeGraphEdge { id: string; source_node_id: string; source_node_type: string; target_node_id: string; target_node_type: string; edge_type: string; description?: string; auto_inferred: boolean; confidence: number; }
interface KnowledgeGraphSnapshot { nodes: KnowledgeGraphNode[]; edges: KnowledgeGraphEdge[]; orphan_count: number; }
interface KnowledgeGraphRetrievalHints { source_key: string; connected_source_keys: string[]; query_terms: string[]; }
interface KnowledgeGraphNeighborhood { center: KnowledgeGraphNode; neighbors: KnowledgeGraphNode[]; edges: KnowledgeGraphEdge[]; retrieval_hints: KnowledgeGraphRetrievalHints; }

const isFiniteNumber = (value: NullableNumber): value is number =>
  typeof value === "number" && Number.isFinite(value);

const formatNumber = (value: NullableNumber, digits = 1) =>
  isFiniteNumber(value) ? value.toFixed(digits) : "n/a";

const formatPercent = (value: NullableNumber) =>
  isFiniteNumber(value) ? `${Math.round(value * 100)}%` : "n/a";

const formatPassRate = (value: NullableNumber) =>
  isFiniteNumber(value) ? `${formatPercent(value)} pass` : "n/a";

const scoreColor = (value: NullableNumber) =>
  isFiniteNumber(value) && value >= 85 ? "var(--success)" : "var(--primary)";

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
  useEffect(() => {
    const refreshAfterResume = () => {
      loadProjects();
      if (selected) loadStatus();
      loadLogs();
    };
    const handleVisibility = () => {
      if (!document.hidden) refreshAfterResume();
    };
    let unlisten: (() => void) | null = null;
    listen("app-resume", refreshAfterResume).then((u) => { unlisten = u; });
    window.addEventListener("focus", refreshAfterResume);
    document.addEventListener("visibilitychange", handleVisibility);
    return () => {
      if (unlisten) unlisten();
      window.removeEventListener("focus", refreshAfterResume);
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, [selected, loadProjects, loadStatus, loadLogs]);

  const ctx = { projects, selected, setSelected, settings, refreshSettings, status, logs, loading, setLoading, msg, setMsg };

  const navLabels: Record<string, string> = {
    dashboard: "Dashboard", projects: "Projects", chapters: "Chapters",
    plans: "Plans", reviews: "Reviews", jobs: "Jobs", bible: "Bible", graph: "Graph", learn: "Learn", settings: "Settings",
  };

  const renderPage = () => {
    switch (page) {
      case "dashboard": return <Dashboard />;
      case "projects": return <ProjectList refresh={loadProjects} />;
      case "chapters": return <Chapters />;
      case "plans": return <ChapterPlans />;
      case "reviews": return <ReviewPage />;
      case "jobs": return <JobsPage />;
      case "bible": return <BiblePage />;
      case "graph": return <KnowledgeGraphPage />;
      case "learn": return <LearnPage />;
      case "settings": return <SettingsPage refreshSettings={refreshSettings} />;
      default: return <Dashboard />;
    }
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
          {renderPage()}
        </main>
      </div>
    </Ctx.Provider>
  );
}

// ---- Dashboard ----
function Dashboard() {
  const { status, loading, selected, settings, logs, msg, setLoading, setMsg } = useApp();
  const [pipelineSteps, setPipelineSteps] = useState<PipelineStep[]>([]);
  const [progress, setProgress] = useState("");
  const [livePreview, setLivePreview] = useState<{title:string;text:string;kind:string} | null>(null);
  const [visiblePreview, setVisiblePreview] = useState("");
  const [operatorControls, setOperatorControls] = useState<OperatorControls>({
    generation_mode: "continuity_first",
    forbidden_moves: "套路打脸、反派降智、无代价胜利、连续三段同句式开头",
    style_emphasis: "克制、具体、少解释，用动作和物件承载情绪",
  });
  const [contextPreview, setContextPreview] = useState<WritingContextPreview | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewMsg, setPreviewMsg] = useState("");
  const [progressFeed, setProgressFeed] = useState<LiveProgressEntry[]>([]);
  const liveProgressRef = useRef<HTMLDivElement | null>(null);

  const pipelineOrder = ["acquire_lock","load_canon","retrieve_context","generate_draft","aggregate_reviews","revise","export","update_canon","complete"];
  const pipelineLabels: Record<string,string> = {
    acquire_lock:"获取生成锁",load_canon:"加载圣经数据",retrieve_context:"向量检索上下文",
    generate_draft:"AI生成初稿",aggregate_reviews:"汇总审稿意见",
    revise:"AI修订",export:"导出Markdown",update_canon:"更新圣经",complete:"完成"
  };
  const feedStatusClass = (status: string) => status.toLowerCase().replace(/[^a-z0-9_-]/g, "-");
  const feedTime = (timestamp: string) => {
    const parsed = new Date(timestamp);
    return Number.isNaN(parsed.getTime()) ? timestamp.slice(0, 8) : parsed.toLocaleTimeString();
  };

  // Listen for Tauri pipeline events
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listen("pipeline-step", (event: any) => {
      const ev = event.payload as PipelineStep;
      if (ev.preview_text) {
        setLivePreview({
          title: ev.preview_title || "Untitled",
          text: ev.preview_text,
          kind: ev.preview_kind || "preview",
        });
      }
      setPipelineSteps((prev) => {
        const existing = prev.findIndex((s) => s.step === ev.step);
        if (existing >= 0) {
          const next = [...prev];
          next[existing] = ev;
          return next;
        }
        return [...prev, ev];
      });
      setProgress(`${Math.round(ev.progress_pct)}%`);
      const previewDetail = ev.preview_text
        ? `${ev.preview_kind || "preview"} received ${ev.preview_text.length.toLocaleString()} chars`
        : "";
      setProgressFeed((prev) => {
        const entry: LiveProgressEntry = {
          id: `${ev.timestamp || Date.now()}-${ev.step}-${ev.status}-${prev.length}`,
          step: ev.step,
          phase: pipelineLabels[ev.step] || ev.step,
          status: ev.status,
          percent: Math.round(ev.progress_pct),
          detail: ev.detail || previewDetail || "step update",
          timestamp: ev.timestamp || new Date().toISOString(),
          preview: Boolean(ev.preview_text || ev.preview_kind),
        };
        return [...prev.slice(-99), entry];
      });
    }).then((u) => { unlisten = u; });
    return () => { if (unlisten) unlisten(); };
  }, []);

  useEffect(() => {
    if (!loading) { setProgress(""); return; }
    setPipelineSteps([]); // Clear on new run
    setLivePreview(null);
    setVisiblePreview("");
    setProgressFeed([{
      id: `start-${Date.now()}`,
      step: "start",
      phase: "准备生成",
      status: "running",
      percent: 0,
      detail: "等待生成 pipeline 返回进度",
      timestamp: new Date().toISOString(),
      preview: false,
    }]);
    setProgress("Starting...");
  }, [loading]);

  useEffect(() => {
    const el = liveProgressRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [progressFeed]);

  useEffect(() => {
    if (!livePreview?.text) {
      setVisiblePreview("");
      return;
    }
    let cursor = 0;
    const chunkSize = Math.max(18, Math.ceil(livePreview.text.length / 150));
    setVisiblePreview("");
    const timer = window.setInterval(() => {
      cursor = Math.min(livePreview.text.length, cursor + chunkSize);
      setVisiblePreview(livePreview.text.slice(0, cursor));
      if (cursor >= livePreview.text.length) {
        window.clearInterval(timer);
      }
    }, 28);
    return () => window.clearInterval(timer);
  }, [livePreview]);

  const modes = [
    { value: "continuity_first", label: "连贯优先" },
    { value: "suspense_push", label: "悬疑推进" },
    { value: "character_arc", label: "人物弧光" },
    { value: "high_pressure", label: "高压冲突" },
  ];

  const updateControl = (key: keyof OperatorControls, value: string) => {
    setOperatorControls(prev => ({ ...prev, [key]: value }));
  };

  const controlsPayload = (): OperatorControls => ({
    generation_mode: operatorControls.generation_mode || "continuity_first",
    chapter_intent: operatorControls.chapter_intent?.trim() || undefined,
    must_include_beats: operatorControls.must_include_beats?.trim() || undefined,
    forbidden_moves: operatorControls.forbidden_moves?.trim() || undefined,
    style_emphasis: operatorControls.style_emphasis?.trim() || undefined,
  });

  const loadPreview = async () => {
    if (!selected) return;
    setPreviewLoading(true);
    setPreviewMsg("");
    try {
      const preview = await invoke<WritingContextPreview>("get_next_chapter_context_preview", {
        projectId: selected,
        operatorControls: controlsPayload(),
      });
      setContextPreview(preview);
    } catch (e) {
      setContextPreview(null);
      setPreviewMsg("Error: " + e);
    }
    setPreviewLoading(false);
  };

  useEffect(() => {
    if (selected) loadPreview();
  }, [selected]);

  useEffect(() => {
    const refreshPreview = () => {
      if (selected) loadPreview();
    };
    const handleVisibility = () => {
      if (!document.hidden) refreshPreview();
    };
    let unlisten: (() => void) | null = null;
    listen("app-resume", refreshPreview).then((u) => { unlisten = u; });
    window.addEventListener("focus", refreshPreview);
    document.addEventListener("visibilitychange", handleVisibility);
    return () => {
      if (unlisten) unlisten();
      window.removeEventListener("focus", refreshPreview);
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, [selected]);

  const handleWrite = async () => {
    setLoading(true); setMsg("");
    try {
      const r = await invoke<GenerationResult>("generate_next_chapter", {
        projectId: selected,
        force: true,
        operatorControls: controlsPayload(),
      });
      setMsg(r.message);
      await loadPreview();
    } catch (e) { setMsg("Error: " + e); }
    setLoading(false);
  };

  const handleWeekly = async () => {
    setLoading(true); setMsg("");
    try {
      const r = await invoke<GenerationResult>("run_weekly_arc_planner", { projectId: selected });
      setMsg(r.message);
      await loadPreview();
    } catch (e) { setMsg("Error: " + e); }
    setLoading(false);
  };

  const selNovel = status?.novel;
  const plan = contextPreview?.chapter_plan || {};
  const continuity = contextPreview?.continuity || {};
  const canon = contextPreview?.canon || {};
  const learned = contextPreview?.learned_patterns || [];
  const retrievalTrace = contextPreview?.retrieval_trace;
  const ragSources = retrievalTrace?.sources || [];
  const graphContext = contextPreview?.graph_context;
  const graphNeighbors = graphContext?.neighbors || [];
  const latestPhase = [...pipelineSteps].reverse().find(step => !step.preview_kind);

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

      <div className="writer-console">
        <section className="writer-panel">
          <div className="panel-heading">
            <h3 className="section-title">Chapter Controls</h3>
          </div>
          <div className="mode-segments">
            {modes.map(mode => (
              <button
                key={mode.value}
                className={`mode-btn ${operatorControls.generation_mode === mode.value ? "active" : ""}`}
                onClick={() => updateControl("generation_mode", mode.value)}
                type="button"
              >
                {mode.label}
              </button>
            ))}
          </div>
          <div className="control-grid">
            <label className="control-field control-field-wide">
              <span>本章意图</span>
              <textarea value={operatorControls.chapter_intent || ""} onChange={e => updateControl("chapter_intent", e.target.value)} placeholder="例：让主角发现线索，但付出关系破裂的代价" />
            </label>
            <label className="control-field">
              <span>必写节拍</span>
              <textarea value={operatorControls.must_include_beats || ""} onChange={e => updateControl("must_include_beats", e.target.value)} placeholder="例：旧名再次出现；盟友隐瞒关键信息" />
            </label>
            <label className="control-field">
              <span>禁止动作</span>
              <textarea value={operatorControls.forbidden_moves || ""} onChange={e => updateControl("forbidden_moves", e.target.value)} />
            </label>
            <label className="control-field control-field-wide">
              <span>风格重点</span>
              <textarea value={operatorControls.style_emphasis || ""} onChange={e => updateControl("style_emphasis", e.target.value)} />
            </label>
          </div>
          <div className="action-row">
            <button className="btn btn-primary" onClick={handleWrite} disabled={loading || !selected}>Write With Controls</button>
            <button className="btn btn-secondary" onClick={handleWeekly} disabled={loading || !selected}>Generate Weekly Plan</button>
            {(status?.is_running && !loading) && <button className="btn btn-reset" onClick={async () => { await invoke("reset_running"); }}>Reset Stuck Job</button>}
          </div>
        </section>

        <section className="writer-panel context-panel">
          <div className="panel-heading">
            <h3 className="section-title">Context Preview</h3>
            <button className="btn btn-sm btn-secondary" onClick={loadPreview} disabled={!selected || previewLoading}>{previewLoading ? "Refreshing" : "Refresh"}</button>
          </div>
          {contextPreview ? (
            <>
              <div className="next-plan">
                <div className="status-label">Next Chapter</div>
                <div className="chapter-title">Ch.{plan.sequence ?? "?"} — {plan.title || "Untitled"}</div>
                <div className="text-body">{plan.outline || "No outline"}</div>
              </div>
              <div className="context-stats">
                <div><strong>{canon.characters?.length || 0}</strong><span>Characters</span></div>
                <div><strong>{canon.active_plot_threads?.length || 0}</strong><span>Threads</span></div>
                <div><strong>{canon.unresolved_foreshadowing?.length || 0}</strong><span>Foreshadowing</span></div>
                <div><strong>{graphNeighbors.length}</strong><span>Graph Links</span></div>
                <div><strong>{retrievalTrace?.source_count || 0}</strong><span>RAG Sources</span></div>
                <div><strong>{learned.length}</strong><span>Learned</span></div>
              </div>
              {continuity.previous_ending_hook && (
                <div className="context-slice">
                  <div className="status-label">Previous Hook</div>
                  <p>{continuity.previous_ending_hook}</p>
                </div>
              )}
              {graphNeighbors.length > 0 && (
                <div className="graph-context-strip">
                  <div className="status-label">Graph Context</div>
                  {graphContext?.summary && <p className="graph-context-summary">{graphContext.summary}</p>}
                  <div className="graph-context-links">
                    {graphNeighbors.slice(0, 6).map((neighbor, idx) => (
                      <span key={`${neighbor.via_id}-${neighbor.id}-${idx}`}>
                        {neighbor.via_label} / {neighbor.edge_type} / {neighbor.label}
                      </span>
                    ))}
                  </div>
                </div>
              )}
              {ragSources.length > 0 && (
                <div className="rag-source-list">
                  <div className="status-label">RAG Sources</div>
                  {ragSources.slice(0, 5).map(source => (
                    <div className="rag-source-row" key={source.document_id}>
                      <div className="rag-source-rank">{source.rank}</div>
                      <div>
                        <div className="rag-source-title">{source.title || source.source_id || source.source_type}</div>
                        <div className="rag-source-meta">{source.source_type}{source.source_id ? ` · ${source.source_id}` : ""}</div>
                      </div>
                      <div className={`rag-source-score rag-score-${source.relevance_label}`}>
                        {formatNumber(source.similarity, 2)}
                      </div>
                      <p className="rag-source-excerpt">{source.excerpt}</p>
                    </div>
                  ))}
                </div>
              )}
              {learned.length > 0 && (
                <div className="learned-strip">
                  {learned.slice(0, 6).map((entry: any) => <span key={entry.id}>{entry.pattern_name}</span>)}
                </div>
              )}
            </>
          ) : (
            <div className={`msg-banner ${previewMsg.startsWith("Error") ? "msg-error" : "msg-success"}`}>{previewMsg || "No context loaded"}</div>
          )}
        </section>
      </div>

      {(loading || pipelineSteps.length > 0 || livePreview) && (
        <section className="live-workbench">
          <div className="live-status">
            <div>
              <div className="status-label">Live Writer</div>
              <div className="live-phase">{latestPhase ? (pipelineLabels[latestPhase.step] || latestPhase.step) : (loading ? "准备生成" : "等待任务")}</div>
            </div>
            <div className={`live-pulse ${loading ? "active" : ""}`} />
          </div>
          <div className="pipeline-timeline">
            {pipelineOrder.map(step => {
              const s = pipelineSteps.find(p => p.step === step);
              const isReview = step.startsWith("review_");
              if (isReview) return null;
              return (
                <div key={step} className={`pipeline-tick ${s?.status || "pending"}`}>
                  <span className="pipeline-dot" />
                  <span>{pipelineLabels[step] || step}</span>
                  {s?.detail ? <em>{s.detail}</em> : null}
                </div>
              );
            })}
          </div>
          <div className="live-progress">{progress || (loading ? "Starting..." : "Idle")}</div>
          <div className="live-progress-feed">
            <div className="live-feed-head">
              <span>Live Progress Feed</span>
              <strong>{progressFeed.length ? `${progressFeed[progressFeed.length - 1].percent}%` : "Idle"}</strong>
            </div>
            <div className="live-feed-lines" ref={liveProgressRef} role="log" aria-live="polite">
              {progressFeed.length > 0 ? progressFeed.map(entry => (
                <div key={entry.id} className={`live-feed-line live-feed-${feedStatusClass(entry.status)} ${entry.preview ? "preview" : ""}`}>
                  <span className="live-feed-time">{feedTime(entry.timestamp)}</span>
                  <strong>{entry.phase}</strong>
                  <em>{entry.status} · {entry.percent}%</em>
                  <p>{entry.detail}</p>
                </div>
              )) : (
                <div className="live-feed-empty">Start chapter generation to stream progress here.</div>
              )}
            </div>
          </div>
          <div className="live-writer-preview">
            <div className="live-preview-head">
              <span>{livePreview ? `${livePreview.kind} preview` : "Draft preview"}</span>
              <strong>{livePreview?.title || "No preview yet"}</strong>
            </div>
            <div className="live-preview-body">
              {visiblePreview || (loading ? "等待模型返回正文预览..." : "开始写作后这里会显示本章正文预览。")}
              {loading && <span className="live-cursor" />}
            </div>
          </div>
        </section>
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
  const [qualitySummary, setQualitySummary] = useState<ProjectQualitySummary | null>(null);

  useEffect(() => {
    if (!selected) {
      setChapters([]);
      setQualitySummary(null);
      return;
    }
    invoke<Chapter[]>("get_chapters", { projectId: selected }).then(setChapters).catch(e => console.error(e));
    invoke<ProjectQualitySummary>("get_project_quality_summary", { projectId: selected }).then(setQualitySummary).catch(e => console.error(e));
  }, [selected]);
  useEffect(() => {
    if (!selCh) { setReviews([]); setScores(null); return; }
    invoke<AgentReview[]>("get_agent_reviews", { chapterId: selCh }).then(setReviews).catch(e => console.error(e));
    invoke<ReviewScores | null>("get_review_scores", { chapterId: selCh }).then(setScores).catch(e => console.error(e));
  }, [selCh]);

  const agentNames: Record<string, string> = {
    continuity_reviewer: "Continuity", character_reviewer: "Character", plot_logic_reviewer: "Plot Logic",
    pacing_reviewer: "Pacing", style_reviewer: "Style", safety_reviewer: "Safety", publication_reviewer: "Publication",
    canon_consistency_precheck: "Canon Precheck",
  };
  const parseCanonIssues = (raw: string): CanonPrecheckIssue[] => {
    try {
      const parsed = JSON.parse(raw || "[]");
      return Array.isArray(parsed) ? parsed.filter(issue => issue && typeof issue === "object") : [];
    } catch {
      return [];
    }
  };
  const canonIssues = reviews
    .filter(review => review.agent_name === "canon_consistency_precheck")
    .flatMap(review => parseCanonIssues(review.blocking_issues));

  return (
    <>
      <h2 className="page-title">Review Reports</h2>
      {qualitySummary && (
        <section className="quality-dashboard">
          <div className="quality-stat-grid">
            <div><strong>{qualitySummary.reviewed_chapter_count}</strong><span>Reviewed</span></div>
            <div><strong>{formatNumber(qualitySummary.average_final_score, 1)}</strong><span>Avg Final</span></div>
            <div><strong>{qualitySummary.publish_ready_count}</strong><span>Ready</span></div>
            <div><strong>{qualitySummary.revise_count}</strong><span>Revise</span></div>
            <div><strong>{qualitySummary.needs_human_review_count}</strong><span>Human Review</span></div>
            <div><strong>{qualitySummary.total_blocking_issues}</strong><span>Blocking</span></div>
          </div>
          <div className="quality-summary-row">
            <div>
              <div className="status-label">Latest Decision</div>
              <div className="quality-latest">{qualitySummary.latest_decision || "No reviews"}</div>
            </div>
            <div>
              <div className="status-label">Latest Score</div>
              <div className="quality-latest">{formatNumber(qualitySummary.latest_final_score, 0)}</div>
            </div>
          </div>
          {qualitySummary.agent_scores.length > 0 && (
            <div className="agent-quality-list">
              {qualitySummary.agent_scores.map(agent => (
                <div className="agent-quality-row" key={agent.agent_name}>
                  <div>
                    <div className="agent-quality-name">{agentNames[agent.agent_name] || agent.agent_name}</div>
                    <div className="text-meta">{agent.review_count} reviews · {agent.blocking_issue_count} blocking</div>
                  </div>
                  <div className="agent-quality-score">{formatNumber(agent.average_score, 1)}</div>
                  <div className="agent-quality-pass">{formatPassRate(agent.pass_rate)}</div>
                </div>
              ))}
            </div>
          )}
        </section>
      )}
      <select className="select" value={selCh} onChange={e => setSelCh(e.target.value)} style={{ marginBottom: 16 }}>
        <option value="">-- Select Chapter --</option>
        {chapters.map(c => <option key={c.id} value={c.id}>Ch.{c.sequence} — {c.title}</option>)}
      </select>
      {scores && (
        <div className="card-feature" style={{ marginBottom: 16 }}>
          <div style={{ fontFamily: "var(--font-display)", fontSize: 22, fontWeight: 300, color: "var(--on-dark)" }}>
            Final Score: <span style={{ color: scoreColor(scores.final_score) }}>{formatNumber(scores.final_score, 0)}</span>
          </div>
          <div className="text-meta" style={{ marginTop: 4 }}>Decision: {scores.decision} · Avg: {formatNumber(scores.average_score, 1)} · Blocking: {scores.blocking_issue_count} · Publish: {scores.publish_allowed ? "Yes" : "No"}</div>
        </div>
      )}
      {canonIssues.length > 0 && (
        <section className="card-feature canon-precheck-panel" style={{ marginBottom: 16 }}>
          <h3 className="section-title" style={{ color: "var(--warning)" }}>Canon Precheck Issues</h3>
          <div className="canon-issue-list">
            {canonIssues.map((issue, idx) => (
              <div className="canon-issue-row" key={`${issue.rule_type || "issue"}-${idx}`}>
                <div className="canon-issue-head">
                  <span className={`badge ${issue.severity === "blocking" ? "badge-warning" : "badge-active"}`}>{issue.severity || "warning"}</span>
                  <strong>{issue.rule_type || "canon_issue"}</strong>
                </div>
                <div>{issue.message || "Canon consistency issue detected."}</div>
                {issue.evidence && <div className="text-meta">Evidence: {issue.evidence}</div>}
              </div>
            ))}
          </div>
        </section>
      )}
      {reviews.map(r => (
        <div key={r.id} className="card" style={{ borderLeft: r.pass ? "3px solid var(--success)" : "3px solid var(--warning)" }}>
          <div className="chapter-title">{agentNames[r.agent_name] || r.agent_name} <span className={`badge ${(r.score || 0) >= 80 ? "badge-success" : (r.score || 0) >= 60 ? "badge-active" : "badge-warning"}`}>{r.score}</span></div>
          <div className="text-meta" style={{ marginTop: 4 }}>{r.pass ? "Pass" : "Fail"}</div>
          {r.agent_name !== "canon_consistency_precheck" && r.blocking_issues && r.blocking_issues !== "[]" && <div className="msg-banner msg-error">{r.blocking_issues.slice(0, 300)}</div>}
        </div>
      ))}
    </>
  );
}

// ---- JobsPage ----
function JobsPage() {
  const { selected } = useApp();
  const [jobs, setJobs] = useState<GenerationJob[]>([]);
  const loadJobs = useCallback(() => {
    if (!selected) {
      setJobs([]);
      return;
    }
    invoke<GenerationJob[]>("get_generation_jobs", { projectId: selected }).then(setJobs).catch(e => console.error(e));
  }, [selected]);

  useEffect(() => {
    loadJobs();
    const timer = window.setInterval(loadJobs, 5000);
    const handleVisibility = () => {
      if (!document.hidden) loadJobs();
    };
    let unlisten: (() => void) | null = null;
    listen("app-resume", loadJobs).then((u) => { unlisten = u; });
    window.addEventListener("focus", loadJobs);
    document.addEventListener("visibilitychange", handleVisibility);
    return () => {
      window.clearInterval(timer);
      if (unlisten) unlisten();
      window.removeEventListener("focus", loadJobs);
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, [loadJobs]);

  const color = (s: string) => s === "completed" ? "var(--success)" : s === "failed" ? "var(--warning)" : s === "needs_human_review" ? "#f39c12" : "var(--primary)";
  const phaseLabels: Record<string, string> = {
    acquire_lock: "Lock",
    load_canon: "Canon",
    retrieve_context: "RAG",
    generate_draft: "Draft",
    aggregate_reviews: "Reviews",
    revise: "Revise",
    export: "Export",
    update_canon: "Update Canon",
    complete: "Complete",
  };
  const parseMetadata = (metadata: string): JobMetadata => {
    try {
      const parsed = JSON.parse(metadata || "{}");
      return parsed && typeof parsed === "object" ? parsed : {};
    } catch {
      return {};
    }
  };
  const formatDuration = (ms?: number) => {
    if (ms === undefined || Number.isNaN(ms)) return "n/a";
    if (ms < 1000) return `${ms}ms`;
    const seconds = ms / 1000;
    if (seconds < 60) return `${seconds.toFixed(1)}s`;
    const minutes = Math.floor(seconds / 60);
    return `${minutes}m ${(seconds % 60).toFixed(0)}s`;
  };
  const formatTokens = (tokens?: number) => {
    if (tokens === undefined || Number.isNaN(tokens)) return "n/a";
    if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}k`;
    return `${tokens}`;
  };
  const formatCost = (cost?: number | null) => {
    if (cost === undefined || cost === null || Number.isNaN(cost)) return "n/a";
    return `$${cost.toFixed(4)}`;
  };

  return (
    <>
      <h2 className="page-title">Generation Jobs</h2>
      <div className="jobs-workbench">
        {jobs.map(j => {
          const metadata = parseMetadata(j.metadata);
          const events = metadata.phase_events || [];
          const summary = metadata.phase_summary || {};
          const usage = metadata.usage_summary || {};
          const failure = summary.failure_reason || j.error_message;
          const slowest = summary.slowest_step
            ? `${phaseLabels[summary.slowest_step] || summary.slowest_step} ${formatDuration(summary.slowest_duration_ms)}`
            : "n/a";
          return (
            <div key={j.id} className="card job-card">
              <div className="job-header">
                <div>
                  <div className="chapter-title">{j.job_date} — <span style={{ color: color(j.status) }}>{j.status}</span></div>
                  <div className="text-meta">{j.started_at}{j.completed_at ? ` · Done: ${j.completed_at}` : ""}</div>
                </div>
                <div className="job-status-pill" style={{ borderColor: color(j.status), color: color(j.status) }}>{j.status}</div>
              </div>
              <div className="job-metric-row">
                <div><strong>{summary.phase_count ?? events.length}</strong><span>Phases</span></div>
                <div><strong>{formatDuration(summary.total_elapsed_ms)}</strong><span>Elapsed</span></div>
                <div><strong>{slowest}</strong><span>Slowest</span></div>
                <div><strong>{j.retry_count || 0}</strong><span>Retries</span></div>
                <div><strong>{formatTokens(usage.total_tokens)}</strong><span>Tokens</span></div>
                <div><strong>{formatCost(usage.estimated_cost_usd)}</strong><span>Cost</span></div>
                <div><strong>{summary.last_step ? (phaseLabels[summary.last_step] || summary.last_step) : "n/a"}</strong><span>Last Step</span></div>
              </div>
              {failure && <div className="msg-banner msg-error job-failure">{failure}</div>}
              {events.length > 0 ? (
                <div className="job-timeline">
                  {events.map((event, idx) => (
                    <div key={`${event.step}-${idx}`} className={`job-phase job-phase-${event.status}`}>
                      <div className="job-phase-marker">{idx + 1}</div>
                      <div className="job-phase-main">
                        <div className="job-phase-title">
                          <span>{phaseLabels[event.step] || event.step}</span>
                          <strong>{formatDuration(event.duration_ms ?? event.elapsed_ms)}</strong>
                        </div>
                        {event.detail && <div className="job-phase-detail">{event.detail}</div>}
                      </div>
                      <div className="job-phase-progress">{Math.round(event.progress_pct)}%</div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-meta">No phase timeline recorded for this job.</div>
              )}
            </div>
          );
        })}
      </div>
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

// ---- KnowledgeGraphPage ----
function KnowledgeGraphPage() {
  const { selected } = useApp();
  const [snapshot, setSnapshot] = useState<KnowledgeGraphSnapshot | null>(null);
  const [search, setSearch] = useState("");
  const [typeFilter, setTypeFilter] = useState("all");
  const [selectedNodeId, setSelectedNodeId] = useState("");
  const [neighborhood, setNeighborhood] = useState<KnowledgeGraphNeighborhood | null>(null);
  const [neighborhoodStatus, setNeighborhoodStatus] = useState("");
  const [edgeForm, setEdgeForm] = useState({ sourceId: "", targetId: "", edgeType: "related_to", description: "" });
  const [message, setMessage] = useState("");

  const loadGraph = useCallback(async () => {
    if (!selected) {
      setSnapshot(null);
      return;
    }
    try {
      const data = await invoke<KnowledgeGraphSnapshot>("get_knowledge_graph", { projectId: selected });
      setSnapshot(data);
      setNeighborhood(null);
      setMessage("");
    } catch (e) {
      setSnapshot(null);
      setMessage("Error: " + e);
    }
  }, [selected]);

  useEffect(() => { loadGraph(); }, [loadGraph]);

  useEffect(() => {
    if (!selected) return;
    const refreshGraph = () => { loadGraph(); };
    const handleVisibility = () => {
      if (!document.hidden) refreshGraph();
    };
    let unlistenResume: (() => void) | null = null;
    let unlistenPipeline: (() => void) | null = null;
    listen("app-resume", refreshGraph).then((u) => { unlistenResume = u; });
    listen("pipeline-step", (event: any) => {
      const ev = event.payload as PipelineStep;
      if (ev.step === "update_canon" || ev.step === "complete") refreshGraph();
    }).then((u) => { unlistenPipeline = u; });
    window.addEventListener("focus", refreshGraph);
    document.addEventListener("visibilitychange", handleVisibility);
    return () => {
      if (unlistenResume) unlistenResume();
      if (unlistenPipeline) unlistenPipeline();
      window.removeEventListener("focus", refreshGraph);
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, [selected, loadGraph]);

  const nodes = snapshot?.nodes || [];
  const edges = snapshot?.edges || [];
  const nodeById = new Map(nodes.map(node => [node.id, node]));
  const types = Array.from(new Set(nodes.map(node => node.node_type))).sort();
  const typeLabel = (type: string) => type.replace(/_/g, " ");
  const query = search.trim().toLowerCase();
  const visibleNodes = nodes.filter(node => {
    const matchesType = typeFilter === "all" || node.node_type === typeFilter;
    const text = `${node.label} ${node.subtitle || ""} ${node.description || ""}`.toLowerCase();
    return matchesType && (!query || text.includes(query));
  });
  const visibleIds = new Set(visibleNodes.map(node => node.id));
  const visibleEdges = edges.filter(edge => visibleIds.has(edge.source_node_id) && visibleIds.has(edge.target_node_id));
  const selectedNode = nodeById.get(selectedNodeId) || visibleNodes[0];
  const selectedNodeKey = selectedNode ? `${selectedNode.node_type}:${selectedNode.id}` : "";
  const connectedEdges = selectedNode
    ? neighborhood?.center.id === selectedNode.id && neighborhood.center.node_type === selectedNode.node_type
      ? neighborhood.edges
      : edges.filter(edge => edge.source_node_id === selectedNode.id || edge.target_node_id === selectedNode.id)
    : [];
  const selectedDegree = selectedNode ? connectedEdges.length : 0;
  const graphEmptyMessage = nodes.length === 0
    ? "No graph nodes yet. Generate canon or write a chapter to build the graph."
    : visibleNodes.length === 0
      ? "No matching nodes."
      : visibleEdges.length === 0
        ? "Nodes are available, but no relationships yet. Generate a chapter or add an edge."
        : "";

  useEffect(() => {
    if (nodes.length >= 2 && (!edgeForm.sourceId || !nodeById.has(edgeForm.sourceId))) {
      setEdgeForm(prev => ({ ...prev, sourceId: nodes[0].id, targetId: nodes[1].id }));
    }
  }, [nodes.length]);

  useEffect(() => {
    if (selectedNode && selectedNode.id !== selectedNodeId) setSelectedNodeId(selectedNode.id);
  }, [selectedNode?.id]);

  useEffect(() => {
    if (!selected || !selectedNode) {
      setNeighborhood(null);
      setNeighborhoodStatus("");
      return;
    }
    let cancelled = false;
    setNeighborhoodStatus("Loading graph context...");
    invoke<KnowledgeGraphNeighborhood>("get_knowledge_graph_neighborhood", {
      projectId: selected,
      nodeId: selectedNode.id,
      nodeType: selectedNode.node_type,
    })
      .then(data => {
        if (!cancelled) {
          setNeighborhood(data);
          setNeighborhoodStatus("");
        }
      })
      .catch(e => {
        if (!cancelled) {
          setNeighborhood(null);
          setNeighborhoodStatus("Error: " + e);
        }
      });
    return () => { cancelled = true; };
  }, [selected, selectedNodeKey]);

  const positions = (() => {
    const byType = new Map<string, KnowledgeGraphNode[]>();
    for (const node of visibleNodes) {
      byType.set(node.node_type, [...(byType.get(node.node_type) || []), node]);
    }
    const map = new Map<string, { x: number; y: number }>();
    const orderedTypes = Array.from(byType.keys()).sort();
    orderedTypes.forEach((type, typeIndex) => {
      const group = byType.get(type) || [];
      const radius = 18 + Math.min(typeIndex, 5) * 7;
      group.forEach((node, index) => {
        const angle = ((index / Math.max(group.length, 1)) * Math.PI * 2) + (typeIndex * 0.58);
        map.set(node.id, {
          x: 50 + Math.cos(angle) * radius,
          y: 50 + Math.sin(angle) * Math.min(radius * 0.82, 35),
        });
      });
    });
    return map;
  })();

  const createEdge = async () => {
    if (!selected) return;
    const source = nodeById.get(edgeForm.sourceId);
    const target = nodeById.get(edgeForm.targetId);
    if (!source || !target) {
      setMessage("Error: choose source and target nodes");
      return;
    }
    try {
      await invoke<KnowledgeGraphEdge>("create_knowledge_graph_edge", {
        projectId: selected,
        sourceId: source.id,
        sourceType: source.node_type,
        targetId: target.id,
        targetType: target.node_type,
        edgeType: edgeForm.edgeType.trim() || "related_to",
        description: edgeForm.description.trim() || null,
      });
      setEdgeForm(prev => ({ ...prev, description: "" }));
      await loadGraph();
      setMessage("Edge created.");
    } catch (e) {
      setMessage("Error: " + e);
    }
  };

  const deleteEdge = async (edgeId: string) => {
    try {
      await invoke("delete_knowledge_graph_edge", { edgeId });
      await loadGraph();
      setMessage("Edge deleted.");
    } catch (e) {
      setMessage("Error: " + e);
    }
  };

  if (!selected) {
    return (
      <>
        <h2 className="page-title">Knowledge Graph</h2>
        <div className="text-meta">Select a project to view the knowledge graph.</div>
      </>
    );
  }

  return (
    <>
      <h2 className="page-title">Knowledge Graph</h2>
      <div className="graph-workbench">
        <div className="graph-toolbar">
          <div className="graph-stat"><strong>{nodes.length}</strong><span>Nodes</span></div>
          <div className="graph-stat"><strong>{edges.length}</strong><span>Edges</span></div>
          <div className="graph-stat"><strong>{snapshot?.orphan_count ?? 0}</strong><span>Orphans</span></div>
          <div className="graph-stat"><strong>{selectedDegree}</strong><span>Selected Degree</span></div>
          <input className="text-input graph-search" value={search} onChange={e => setSearch(e.target.value)} placeholder="Search canon..." />
        </div>

        <div className="graph-type-filter">
          <button className={`tab-btn ${typeFilter === "all" ? "active" : ""}`} onClick={() => setTypeFilter("all")}>All</button>
          {types.map(type => (
            <button key={type} className={`tab-btn ${typeFilter === type ? "active" : ""}`} onClick={() => setTypeFilter(type)}>
              {typeLabel(type)}
            </button>
          ))}
        </div>

        <div className="graph-layout">
          <section className="graph-canvas">
            <svg className="graph-edge-layer" viewBox="0 0 100 100" preserveAspectRatio="none" role="img" aria-label="Knowledge graph relationships">
              {visibleEdges.map((edge, edgeIndex) => {
                const source = positions.get(edge.source_node_id);
                const target = positions.get(edge.target_node_id);
                const sourceNode = nodeById.get(edge.source_node_id);
                const targetNode = nodeById.get(edge.target_node_id);
                if (!source || !target) return null;
                const isSelectedEdge = Boolean(selectedNode && (
                  (edge.source_node_id === selectedNode.id && edge.source_node_type === selectedNode.node_type) ||
                  (edge.target_node_id === selectedNode.id && edge.target_node_type === selectedNode.node_type)
                ));
                const label = `${sourceNode?.label || edge.source_node_id} ${edge.edge_type} ${targetNode?.label || edge.target_node_id}`;
                return (
                  <g key={edge.id} className={`graph-edge ${isSelectedEdge ? "active" : ""}`}>
                    <title>{edge.description ? `${label}: ${edge.description}` : label}</title>
                    <line className="graph-edge-base" x1={source.x} y1={source.y} x2={target.x} y2={target.y} />
                    <line className="graph-edge-flow" x1={source.x} y1={source.y} x2={target.x} y2={target.y} style={{ animationDelay: `${(edgeIndex % 6) * -0.22}s` }} />
                  </g>
                );
              })}
            </svg>
            {visibleNodes.map(node => {
              const pos = positions.get(node.id) || { x: 50, y: 50 };
              return (
                <button
                  key={node.id}
                  className={`graph-node graph-node-${node.node_type} ${selectedNode?.id === node.id ? "active" : ""}`}
                  style={{ left: `${pos.x}%`, top: `${pos.y}%` }}
                  onClick={() => setSelectedNodeId(node.id)}
                  title={`${node.label} (${typeLabel(node.node_type)})`}
                  aria-label={`${node.label}, ${typeLabel(node.node_type)}, ${node.degree} relationships`}
                  aria-pressed={selectedNode?.id === node.id}
                  type="button"
                >
                  <span>{node.label}</span>
                  <em>{node.degree}</em>
                </button>
              );
            })}
            {graphEmptyMessage && <div className={`graph-empty ${visibleNodes.length > 0 ? "graph-empty-compact" : ""}`}>{graphEmptyMessage}</div>}
          </section>

          <aside className="graph-inspector">
            {selectedNode ? (
              <>
                <div className="status-label">{typeLabel(selectedNode.node_type)}</div>
                <h3>{selectedNode.label}</h3>
                <div className="text-meta">{[selectedNode.subtitle, selectedNode.status, `${selectedNode.degree} edges`].filter(Boolean).join(" · ")}</div>
                {selectedNode.description && <p className="graph-description">{selectedNode.description}</p>}
                <div className="graph-rag-panel">
                  <h4>Retrieval Hints</h4>
                  {neighborhoodStatus ? (
                    <div className="text-meta">{neighborhoodStatus}</div>
                  ) : neighborhood ? (
                    <>
                      <div className="graph-hint-row">
                        <span>Source</span>
                        <strong>{neighborhood.retrieval_hints.source_key}</strong>
                      </div>
                      <div className="graph-hint-row">
                        <span>Connected</span>
                        <strong>{neighborhood.retrieval_hints.connected_source_keys.length}</strong>
                      </div>
                      <div className="graph-hint-chips">
                        {neighborhood.retrieval_hints.connected_source_keys.slice(0, 8).map(key => <span key={key}>{key}</span>)}
                        {neighborhood.retrieval_hints.connected_source_keys.length === 0 && <em>No connected sources</em>}
                      </div>
                      <div className="graph-query-terms">
                        {neighborhood.retrieval_hints.query_terms.slice(0, 8).map(term => <span key={term}>{term}</span>)}
                      </div>
                    </>
                  ) : (
                    <div className="text-meta">No retrieval hints available.</div>
                  )}
                </div>
                <h4>Relationships</h4>
                <div className="edge-list">
                  {connectedEdges.map(edge => {
                    const otherId = edge.source_node_id === selectedNode.id ? edge.target_node_id : edge.source_node_id;
                    const other = nodeById.get(otherId);
                    return (
                      <div key={edge.id} className="edge-row">
                        <div>
                          <strong>{edge.edge_type}</strong>
                          <span>{other?.label || otherId}</span>
                          {edge.description && <p>{edge.description}</p>}
                        </div>
                        {!edge.auto_inferred && <button className="btn btn-sm btn-danger" onClick={() => deleteEdge(edge.id)}>Delete</button>}
                      </div>
                    );
                  })}
                  {connectedEdges.length === 0 && <div className="text-meta">No relationships yet.</div>}
                </div>
              </>
            ) : (
              <div className="text-meta">No node selected.</div>
            )}

            <div className="edge-form">
              <h4>Add Relationship</h4>
              <label>
                <span>Source</span>
                <select value={edgeForm.sourceId} onChange={e => setEdgeForm(prev => ({ ...prev, sourceId: e.target.value }))}>
                  {nodes.map(node => <option key={node.id} value={node.id}>{node.label} · {typeLabel(node.node_type)}</option>)}
                </select>
              </label>
              <label>
                <span>Target</span>
                <select value={edgeForm.targetId} onChange={e => setEdgeForm(prev => ({ ...prev, targetId: e.target.value }))}>
                  {nodes.map(node => <option key={node.id} value={node.id}>{node.label} · {typeLabel(node.node_type)}</option>)}
                </select>
              </label>
              <label>
                <span>Type</span>
                <input value={edgeForm.edgeType} onChange={e => setEdgeForm(prev => ({ ...prev, edgeType: e.target.value }))} />
              </label>
              <label>
                <span>Description</span>
                <textarea value={edgeForm.description} onChange={e => setEdgeForm(prev => ({ ...prev, description: e.target.value }))} />
              </label>
              <button className="btn btn-primary" onClick={createEdge} disabled={nodes.length < 2}>Create Edge</button>
            </div>
          </aside>
        </div>
        {message && <div className={`msg-banner ${message.startsWith("Error") ? "msg-error" : "msg-success"}`}>{message}</div>}
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

  const formatOptionalCost = (value?: number | null) => value === null || value === undefined ? "" : String(value);
  const parseOptionalCost = (value: string) => {
    const trimmed = value.trim();
    if (!trimmed) return null;
    const parsed = Number(trimmed);
    return Number.isFinite(parsed) && parsed >= 0 ? parsed : null;
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
          <SaveField label="Input Cost / 1M Tokens" value={formatOptionalCost(settings.input_cost_per_million)} type="number" onSave={v => invoke("update_settings", { settings: { ...settings, input_cost_per_million: parseOptionalCost(v) } }).then(refreshSettings)} />
          <SaveField label="Output Cost / 1M Tokens" value={formatOptionalCost(settings.output_cost_per_million)} type="number" onSave={v => invoke("update_settings", { settings: { ...settings, output_cost_per_million: parseOptionalCost(v) } }).then(refreshSettings)} />
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
function SaveField({ label, value, type = "text", onSave }: { label: string; value: string; type?: string; onSave: (v: string) => void }) {
  const [editing, setEditing] = useState(false);
  const [val, setVal] = useState(value);
  useEffect(() => { setVal(value); }, [value]);
  const save = () => { setEditing(false); if (val !== value) onSave(val); };
  return (
    <div className="bible-edit-field">
      <label>{label}</label>
      {editing ? (
        <input autoFocus type={type} step={type === "number" ? "0.0001" : undefined} min={type === "number" ? "0" : undefined} value={val} onChange={e => setVal(e.target.value)} onBlur={save} onKeyDown={e => { if (e.key === "Enter") save(); if (e.key === "Escape") { setVal(value); setEditing(false); } }}
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
  const [tab, setTab] = useState<"file"|"web"|"library">("library");
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [sourceTitle, setSourceTitle] = useState("");
  const [url, setUrl] = useState("");
  const [learning, setLearning] = useState(false);
  const [result, setResult] = useState("");
  const [entries, setEntries] = useState<any[]>([]);
  const [expandedId, setExpandedId] = useState<string|null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const learningFileAccept = ".txt,.md,.markdown,.text,.log,.csv,.json,.html";
  const maxLearningFileBytes = 1_048_576;

  const loadEntries = async () => {
    if (!selected) return;
    try { setEntries(await invoke<any[]>("get_learning_entries", { projectId: selected })); } catch (e) { console.error(e); }
  };

  useEffect(() => { loadEntries(); }, [selected]);

  const handleFileSelected = (event: { currentTarget: HTMLInputElement }) => {
    const file = event.currentTarget.files?.[0] || null;
    setSelectedFile(file);
    setResult("");
    if (file && !sourceTitle.trim()) setSourceTitle(file.name);
  };

  const handleLearnFile = async () => {
    const file = selectedFile;
    if (!file) return;
    if (!selected) { setResult("Error: Select a project first."); return; }
    if (file.size > maxLearningFileBytes) { setResult("Error: File too large (>1 MiB)."); return; }
    setLearning(true); setResult("Reading file...");
    try {
      const text = await file.text();
      setResult("Extracting patterns...");
      const r = await invoke<any[]>("learn_from_file_text", {
        projectId: selected,
        fileName: file.name,
        byteLen: file.size,
        text,
        sourceTitle: sourceTitle.trim() || null,
      });
      setResult(`Learned ${r.length} patterns`);
      setSelectedFile(null); setSourceTitle("");
      if (fileInputRef.current) fileInputRef.current.value = "";
      loadEntries();
    } catch (e: any) { setResult("Error: " + (e.message || String(e))); }
    finally { setLearning(false); }
  };

  const handleLearnUrl = async () => {
    if (!url.trim()) return;
    if (!selected) { setResult("Error: Select a project first."); return; }
    setLearning(true); setResult("Fetching and extracting page...");
    try {
      setResult("Extracting patterns...");
      const r = await invoke<any[]>("learn_from_url", { projectId: selected, url: url.trim() });
      setResult(`Learned ${r.length} patterns`);
      setUrl("");
      loadEntries();
    } catch (e: any) { setResult("Error: " + (e.message || String(e))); }
    finally { setLearning(false); }
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
        <button className={`tab-btn ${tab==="file"?"active":""}`} onClick={()=>setTab("file")}>File Learn</button>
        <button className={`tab-btn ${tab==="web"?"active":""}`} onClick={()=>setTab("web")}>Web Learn</button>
        <button className={`tab-btn ${tab==="library"?"active":""}`} onClick={()=>{setTab("library");loadEntries();}}>Knowledge Library ({entries.length})</button>
      </div>

      {tab === "file" && (
        <div className="card-feature">
          <div className="bible-edit-field"><label>Source Title</label><input className="text-input" value={sourceTitle} onChange={e=>setSourceTitle(e.target.value)} placeholder={selectedFile?.name || "Source name..."} /></div>
          <input ref={fileInputRef} type="file" accept={learningFileAccept} onChange={handleFileSelected} style={{display:"none"}} />
          <div className="bible-edit-field">
            <label>Source File</label>
            <div style={{display:"flex",gap:8,alignItems:"center",flexWrap:"wrap"}}>
              <button className="btn btn-secondary" onClick={()=>fileInputRef.current?.click()} disabled={learning}>Choose File</button>
              <span className="text-meta">{selectedFile ? `${selectedFile.name} (${Math.ceil(selectedFile.size / 1024)} KB)` : "No file selected"}</span>
            </div>
          </div>
          <button className="btn btn-primary" onClick={handleLearnFile} disabled={learning||!selectedFile}>{learning?"Extracting...":"Extract Knowledge"}</button>
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
          {entries.length === 0 && <div className="text-meta">No learned patterns yet. Import a file or use Web Learn to build your knowledge base.</div>}
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
