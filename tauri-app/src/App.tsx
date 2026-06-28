import { useState, useEffect, useCallback, createContext, useContext, useRef, useMemo, type PointerEvent as ReactPointerEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { clampGraphPosition, createGraphBasePositions, flowGraphPosition, graphNodeBadge, graphNodeDisplayLabel, graphTypeLabel, positionFromClientPoint, type GraphPosition } from "./graphLayout.js";
import { tauriClient } from "./lib/tauriClient";
import { AuthorControlPage } from "./pages/AuthorControlPage";
import { PetWindow } from "./pages/PetWindow";
import { RuntimePage } from "./pages/RuntimePage";

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
interface AppSettings { provider: string; model: string; base_url: string; embedding_model: string; embedding_provider: string; embedding_base_url: string; embedding_dim: number; quality_threshold: number; auto_publish: boolean; max_revise_count: number; daily_target_words: number; data_dir: string; debug_mode: boolean; blog_provider: string; publish_schedule_enabled: boolean; publication_target_provider: string; publication_target_path: string; publication_posts_dir: string; publication_remote_name: string; publication_branch?: string | null; publication_build_command: string; publication_commit_template: string; publication_push_enabled: boolean; publication_dry_run: boolean; publication_validate_build: boolean; input_cost_per_million?: number | null; output_cost_per_million?: number | null; draft_model_profile_id?: string | null; review_model_profile_id?: string | null; repair_model_profile_id?: string | null; embedding_model_profile_id?: string | null; summarization_model_profile_id?: string | null; pet_enabled: boolean; pet_animation_level: string; pet_compact_mode: boolean; pet_position_x: number; pet_position_y: number; }
interface Project { id: string; name: string; }
interface OperatorControls { generation_mode?: string; chapter_intent?: string; must_include_beats?: string; forbidden_moves?: string; style_emphasis?: string; pinned_source_keys?: string[]; unpinned_source_keys?: string[]; }
type NullableNumber = number | null | undefined;

interface RetrievalSource { rank: number; document_id: string; source_type: string; source_id?: string; title?: string; excerpt: string; similarity?: NullableNumber; relevance_label: string; metadata: string; }
interface RetrievalTrace { source_count: number; best_similarity?: NullableNumber; sources: RetrievalSource[]; }
interface GraphContextNode { id: string; node_type: string; label: string; }
interface GraphContextNeighbor { id: string; node_type: string; label: string; edge_type: string; direction: string; depth?: number; via_id: string; via_type: string; via_label: string; description?: string; }
interface GraphContext { seeds: GraphContextNode[]; neighbors: GraphContextNeighbor[]; source_keys: string[]; summary: string; }
interface ContextRuleActivation { rule_id: string; name: string; source_key: string; priority: number; token_estimate: number; content: string; matched_keywords: string[]; matched_secondary_keywords: string[]; activation_reason: string; }
interface ContextActivationTrace { activated_rules: ContextRuleActivation[]; source_keys: string[]; }
interface PromptUnitTrace { identifier: string; role: string; order: number; injection_position: string; generation_phase: string; token_estimate: number; }
interface PromptRuntimePreview { prompt_name: string; generation_phase: string; system_prompt: string; user_prompt: string; token_estimate: number; unit_traces: PromptUnitTrace[]; }
interface WritingContextPreview { project?: any; chapter_plan?: any; continuity?: any; canon?: any; graph_context?: GraphContext; context_activation?: ContextActivationTrace; retrieval?: any[]; retrieval_trace?: RetrievalTrace; style?: any; learned_patterns?: any[]; operator_controls?: OperatorControls; prompt_runtime?: PromptRuntimePreview; }
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

const agentSurfaceMap: Record<string, { label: string; icon: string; detail: string }> = {
  agent: { label: "Agent 总控", icon: "A", detail: "任务、阶段、预览" },
  orchestrate: { label: "流程编排", icon: "F", detail: "上下文与工具链" },
  memory: { label: "记忆中枢", icon: "M", detail: "RAG、圣经、图谱" },
  quality: { label: "质量审稿", icon: "Q", detail: "多 Agent 审稿" },
  ops: { label: "发布运维", icon: "O", detail: "任务与发布" },
  settings: { label: "项目设置", icon: "S", detail: "项目与模型" },
};

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
  const isPetWindow = new URLSearchParams(window.location.search).get("window") === "pet";
  if (isPetWindow) return <PetWindow />;

  const [page, setPage] = useState("agent");
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
    let unlisten: (() => void) | null = null;
    listen("open-settings", () => setPage("settings")).then((u) => { unlisten = u; });
    return () => {
      if (unlisten) unlisten();
    };
  }, []);
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

  const selectedProject = projects.find(project => project.id === selected);
  const togglePublishSchedule = async () => {
    if (!settings) return;
    await invoke("update_settings", {
      settings: { ...settings, publish_schedule_enabled: !settings.publish_schedule_enabled },
    });
    refreshSettings();
  };

  useEffect(() => {
    const ragState = settings?.embedding_provider && settings.embedding_provider !== "none"
      ? "usable"
      : "disabled";
    void emitTo("pet", "pet-status", {
      selected,
      projectName: selectedProject?.name,
      loading,
      running: Boolean(status?.is_running),
      message: msg,
      ragState,
      animationLevel: settings?.pet_animation_level || "subtle",
      compact: Boolean(settings?.pet_compact_mode),
    }).catch(() => {});
  }, [
    selected,
    selectedProject?.name,
    loading,
    status?.is_running,
    msg,
    settings?.embedding_provider,
    settings?.pet_animation_level,
    settings?.pet_compact_mode,
  ]);

  const renderPage = () => {
    switch (page) {
      case "agent": return <Dashboard />;
      case "orchestrate": return <OrchestrateAgentPage selected={selected} settings={settings} refreshSettings={refreshSettings} />;
      case "memory": return <MemoryAgentPage />;
      case "quality": return <ReviewPage />;
      case "ops": return <OpsAgentPage />;
      case "settings": return <ProjectSettingsAgentPage refreshSettings={refreshSettings} refreshProjects={loadProjects} />;
      case "dashboard": return <Dashboard />;
      case "projects": return <ProjectList refresh={loadProjects} />;
      case "chapters": return <Chapters />;
      case "plans": return <ChapterPlans />;
      case "reviews": return <ReviewPage />;
      case "jobs": return <JobsPage />;
      case "bible": return <BiblePage />;
      case "graph": return <KnowledgeGraphPage />;
      case "runtime": return <RuntimePage selected={selected} settings={settings} refreshSettings={refreshSettings} />;
      case "authorControl": return <AuthorControlPage selected={selected} />;
      case "learn": return <LearnPage />;
      default: return <Dashboard />;
    }
  };

  const handleMinimizeWindow = async () => {
    await getCurrentWindow().minimize();
  };

  const handleToggleMaximizeWindow = async () => {
    const currentWindow = getCurrentWindow();
    if (await currentWindow.isMaximized()) {
      await currentWindow.unmaximize();
    } else {
      await currentWindow.maximize();
    }
  };

  const handleCloseToTray = async () => {
    await getCurrentWindow().hide();
  };

  const handleTitlebarDrag = async (event: ReactPointerEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    await getCurrentWindow().startDragging();
  };

  return (
    <Ctx.Provider value={ctx}>
      <div className="app-shell">
        <header className="app-titlebar">
          <div className="titlebar-drag-zone" data-tauri-drag-region onPointerDown={handleTitlebarDrag}>
            <div className="titlebar-brand" data-tauri-drag-region>
              <span className="titlebar-app-mark" aria-hidden="true">A</span>
              <span className="titlebar-title" data-tauri-drag-region>AI Novel Factory</span>
            </div>
          </div>
          <div className="window-controls">
            <button type="button" className="window-control" aria-label="最小化窗口" onClick={handleMinimizeWindow}>
              <span className="window-glyph window-glyph-minimize" aria-hidden="true" />
            </button>
            <button type="button" className="window-control" aria-label="最大化或还原窗口" onClick={handleToggleMaximizeWindow}>
              <span className="window-glyph window-glyph-maximize" aria-hidden="true" />
            </button>
            <button type="button" className="window-control window-control-close" aria-label="隐藏到托盘" onClick={handleCloseToTray}>
              <span className="window-glyph window-glyph-close" aria-hidden="true" />
            </button>
          </div>
        </header>
        <div className="app-navigation-view">
          <aside className="navigation-pane">
            <div className="navigation-brand">Agent Novel OS</div>
            {Object.keys(agentSurfaceMap).map(p => (
              <button key={p} className={`navigation-item ${page === p ? "active" : ""}`} onClick={() => setPage(p)} aria-current={page === p ? "page" : undefined}>
                <span className="nav-icon" aria-hidden="true">{agentSurfaceMap[p].icon}</span>
                <span>{agentSurfaceMap[p].label}</span>
                <span className="text-meta">{agentSurfaceMap[p].detail}</span>
              </button>
            ))}
            <select className="sidebar-select navigation-project-select" value={selected} onChange={e => setSelected(e.target.value)} aria-label="当前项目">
              <option value="">-- 选择项目 --</option>
              {projects.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
            </select>
            <button
              type="button"
              className={`navigation-item scheduler-toggle ${settings?.publish_schedule_enabled ? "active" : ""}`}
              onClick={togglePublishSchedule}
              aria-pressed={Boolean(settings?.publish_schedule_enabled)}
            >
              <span className="nav-icon" aria-hidden="true">W</span>
              <span>定时写作</span>
              <span className="text-meta">{settings?.publish_schedule_enabled ? "开启" : "关闭"}</span>
            </button>
          </aside>
          <section className="app-frame">
            <div className="app-command-bar">
              <div className="command-context">
                <span className="command-kicker">{agentSurfaceMap[page]?.label || "Agent 总控"}</span>
                <strong>{selectedProject?.name || "未选择项目"}</strong>
              </div>
              <div className="info-bar" data-state={status?.is_running ? "running" : "ready"} role="status">
                <span className="info-dot" aria-hidden="true" />
                <span>{status?.is_running ? "生成中" : "就绪"}</span>
              </div>
            </div>
            <main className="app-main">
              {renderPage()}
            </main>
          </section>
        </div>
      </div>
    </Ctx.Provider>
  );
}

function AgentSectionTabs({
  tabs,
  active,
  onChange,
}: {
  tabs: Array<{ id: string; label: string }>;
  active: string;
  onChange: (id: string) => void;
}) {
  return (
    <div className="agent-section-tabs" role="tablist" aria-label="Agent surface modes">
      {tabs.map(tab => (
        <button
          key={tab.id}
          type="button"
          className={`agent-section-tab ${active === tab.id ? "active" : ""}`}
          onClick={() => onChange(tab.id)}
          role="tab"
          aria-selected={active === tab.id}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}

function OrchestrateAgentPage({
  selected,
  settings,
  refreshSettings,
}: {
  selected: string;
  settings: AppSettings | null;
  refreshSettings: () => void;
}) {
  const [mode, setMode] = useState("runtime");
  return (
    <div className="agent-surface">
      <header className="agent-surface-header">
        <div>
          <span className="command-kicker">Orchestrator Agent</span>
          <h2 className="page-title">流程编排</h2>
        </div>
        <p>把运行台和作者控制收束到一个编排面板，保留深度控制但减少顶层入口冲突。</p>
      </header>
      <AgentSectionTabs
        active={mode}
        onChange={setMode}
        tabs={[
          { id: "runtime", label: "运行台" },
          { id: "author", label: "作者控制" },
        ]}
      />
      {mode === "runtime" && <RuntimePage selected={selected} settings={settings} refreshSettings={refreshSettings} />}
      {mode === "author" && <AuthorControlPage selected={selected} />}
    </div>
  );
}

function MemoryAgentPage() {
  const [mode, setMode] = useState("learn");
  return (
    <div className="agent-surface">
      <header className="agent-surface-header">
        <div>
          <span className="command-kicker">Memory Agent</span>
          <h2 className="page-title">记忆中枢</h2>
        </div>
        <p>把学习库、小说圣经和关系图谱收束为同一个连续性记忆工作区。</p>
      </header>
      <AgentSectionTabs
        active={mode}
        onChange={setMode}
        tabs={[
          { id: "learn", label: "学习条目" },
          { id: "bible", label: "小说圣经" },
          { id: "graph", label: "关系图谱" },
        ]}
      />
      {mode === "learn" && <LearnPage />}
      {mode === "bible" && <BiblePage />}
      {mode === "graph" && <KnowledgeGraphPage />}
    </div>
  );
}

function OpsAgentPage() {
  return (
    <div className="agent-surface">
      <header className="agent-surface-header">
        <div>
          <span className="command-kicker">Ops Agent</span>
          <h2 className="page-title">发布运维</h2>
        </div>
        <p>集中查看生成任务、失败恢复和发布前状态。</p>
      </header>
      <JobsPage />
    </div>
  );
}

function ProjectSettingsAgentPage({
  refreshSettings,
  refreshProjects,
}: {
  refreshSettings: () => void;
  refreshProjects: () => void;
}) {
  const [mode, setMode] = useState("settings");
  return (
    <div className="agent-surface">
      <header className="agent-surface-header">
        <div>
          <span className="command-kicker">Project Agent</span>
          <h2 className="page-title">项目设置</h2>
        </div>
        <p>项目、章节计划和模型参数留在一个配置面板里，减少顶层跳转。</p>
      </header>
      <AgentSectionTabs
        active={mode}
        onChange={setMode}
        tabs={[
          { id: "settings", label: "模型与发布" },
          { id: "projects", label: "项目" },
          { id: "chapters", label: "章节" },
          { id: "plans", label: "章节计划" },
        ]}
      />
      {mode === "settings" && <SettingsPage refreshSettings={refreshSettings} />}
      {mode === "projects" && <ProjectList refresh={refreshProjects} />}
      {mode === "chapters" && <Chapters />}
      {mode === "plans" && <ChapterPlans />}
    </div>
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
  const lastPetEmitRef = useRef(0);

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
  const emitPetPipelineStatus = useCallback((ev: PipelineStep) => {
    const now = Date.now();
    const failed = ev.status.toLowerCase().includes("fail");
    const complete = ev.progress_pct >= 100;
    if (!failed && !complete && now - lastPetEmitRef.current < 250) return;
    lastPetEmitRef.current = now;
    void emitTo("pet", "pet-status", {
      loading: !complete && !failed,
      running: !complete && !failed,
      phaseLabel: pipelineLabels[ev.step] || ev.step,
      progressPct: Math.max(0, Math.min(100, Math.round(ev.progress_pct))),
      statusText: ev.detail || ev.status,
      message: ev.detail || ev.status,
    }).catch(() => {});
  }, []);

  // Listen for Tauri pipeline events
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listen("pipeline-step", (event: any) => {
      const ev = event.payload as PipelineStep;
      emitPetPipelineStatus(ev);
      if (ev.preview_text) {
        setLivePreview({
          title: ev.preview_title || "未命名",
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
    setProgress("启动中...");
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
      const preview = await tauriClient.getNextChapterContextPreview<WritingContextPreview>(
        selected,
        controlsPayload(),
      );
      setContextPreview(preview);
    } catch (e) {
      setContextPreview(null);
      setPreviewMsg("错误：" + e);
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
      const r = await tauriClient.generateNextChapter<GenerationResult>(
        selected,
        true,
        controlsPayload(),
      );
      setMsg(r.message);
      await loadPreview();
    } catch (e) { setMsg("错误：" + e); }
    setLoading(false);
  };

  const handleWeekly = async () => {
    setLoading(true); setMsg("");
    try {
      const r = await tauriClient.runWeeklyArcPlanner<GenerationResult>(selected);
      setMsg(r.message);
      await loadPreview();
    } catch (e) { setMsg("错误：" + e); }
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
  const activatedRules = contextPreview?.context_activation?.activated_rules || [];
  const promptRuntime = contextPreview?.prompt_runtime;
  const promptUnits = promptRuntime?.unit_traces || [];
  const latestPhase = [...pipelineSteps].reverse().find(step => !step.preview_kind);

  return (
    <>
      {selNovel && (
        <div className="hero-band">
          {selNovel.name}
          <div style={{ fontSize: 14, fontWeight: 400, marginTop: 8, opacity: 0.8 }}>{selNovel.genre}</div>
        </div>
      )}
      <h2 className="page-title" style={selNovel ? undefined : {}}>Agent 总控台</h2>

      <div className="status-grid">
        {[
          {l:"章节",v:status?.chapter_count ?? "?"},
          {l:"今日",v:status?.chapters_today ?? "?"},
          {l:"剩余计划",v:status?.plans_left ?? "?"},
          {l:"总字数",v:(status?.total_words||0).toLocaleString()},
          {l:"状态",v: loading ? "生成中" : (status?.is_running ? "生成中" : "待命"), badge: true},
          {l:"RAG",v: settings?.embedding_provider && settings.embedding_provider !== "none" ? "开启" : "关闭"},
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
            <h3 className="section-title">章节控制</h3>
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
            <button className="btn btn-primary" onClick={handleWrite} disabled={loading || !selected}>按控制写作</button>
            <button className="btn btn-secondary" onClick={handleWeekly} disabled={loading || !selected}>生成周计划</button>
            {(status?.is_running && !loading) && <button className="btn btn-reset" onClick={async () => { await tauriClient.resetRunning(selected); }}>重置卡住的任务</button>}
          </div>
        </section>

        <section className="writer-panel context-panel">
          <div className="panel-heading">
            <h3 className="section-title">上下文预览</h3>
            <button className="btn btn-sm btn-secondary" onClick={loadPreview} disabled={!selected || previewLoading}>{previewLoading ? "刷新中" : "刷新"}</button>
          </div>
          {contextPreview ? (
            <>
              <div className="next-plan">
                <div className="status-label">下一章</div>
                <div className="chapter-title">第 {plan.sequence ?? "?"} 章 — {plan.title || "未命名"}</div>
                <div className="text-body">{plan.outline || "暂无大纲"}</div>
              </div>
              <div className="context-stats">
                <div><strong>{canon.characters?.length || 0}</strong><span>人物</span></div>
                <div><strong>{canon.active_plot_threads?.length || 0}</strong><span>剧情线</span></div>
                <div><strong>{canon.unresolved_foreshadowing?.length || 0}</strong><span>伏笔</span></div>
                <div><strong>{graphNeighbors.length}</strong><span>图谱连接</span></div>
                <div><strong>{retrievalTrace?.source_count || 0}</strong><span>RAG 来源</span></div>
                <div><strong>{activatedRules.length}</strong><span>规则</span></div>
              </div>
              {promptRuntime && (
                <div className="prompt-runtime-preview">
                  <div className="prompt-runtime-head">
                    <div>
                      <div className="status-label">提示词运行时</div>
                      <strong>{promptRuntime.prompt_name} / {promptRuntime.generation_phase}</strong>
                    </div>
                    <span>{promptRuntime.token_estimate.toLocaleString()} tokens</span>
                  </div>
                  <div className="prompt-unit-strip">
                    {promptUnits.map(unit => (
                      <span key={unit.identifier}>
                        {unit.order}. {unit.role} / {unit.identifier} / {unit.token_estimate}
                      </span>
                    ))}
                  </div>
                  <details className="prompt-runtime-details">
                    <summary>系统提示词</summary>
                    <pre>{promptRuntime.system_prompt}</pre>
                  </details>
                  <details className="prompt-runtime-details">
                    <summary>用户提示词</summary>
                    <pre>{promptRuntime.user_prompt}</pre>
                  </details>
                </div>
              )}
              {continuity.previous_ending_hook && (
                <div className="context-slice">
                  <div className="status-label">上一章钩子</div>
                  <p>{continuity.previous_ending_hook}</p>
                </div>
              )}
              {activatedRules.length > 0 && (
                <div className="activation-rule-list">
                  <div className="status-label">已激活圣经规则</div>
                  {activatedRules.slice(0, 5).map(rule => (
                    <div className="activation-rule-row" key={rule.rule_id}>
                      <div>
                        <strong>{rule.name}</strong>
                        <span>{rule.source_key} / p{rule.priority} / {rule.token_estimate} tokens</span>
                      </div>
                      <p>{rule.content}</p>
                      <em>{[...rule.matched_keywords, ...rule.matched_secondary_keywords].join(" / ")}</em>
                    </div>
                  ))}
                </div>
              )}
              {graphNeighbors.length > 0 && (
                <div className="graph-context-strip">
                  <div className="status-label">图谱上下文</div>
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
                  <div className="status-label">RAG 来源</div>
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
            <div className={`msg-banner ${previewMsg.startsWith("错误") ? "msg-error" : "msg-success"}`}>{previewMsg || "尚未加载上下文"}</div>
          )}
        </section>
      </div>

      {(loading || pipelineSteps.length > 0 || livePreview) && (
        <section className="live-workbench">
          <div className="live-status">
            <div>
              <div className="status-label">实时写作</div>
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
          <div className="live-progress">{progress || (loading ? "启动中..." : "待命")}</div>
          <div className="live-progress-feed">
            <div className="live-feed-head">
              <span>实时进度</span>
              <strong>{progressFeed.length ? `${progressFeed[progressFeed.length - 1].percent}%` : "待命"}</strong>
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
                <div className="live-feed-empty">开始章节生成后，这里会显示实时进度。</div>
              )}
            </div>
          </div>
          <div className="live-writer-preview">
            <div className="live-preview-head">
              <span>{livePreview ? `${livePreview.kind} 预览` : "草稿预览"}</span>
              <strong>{livePreview?.title || "暂无预览"}</strong>
            </div>
            <div className="live-preview-body">
              {visiblePreview || (loading ? "等待模型返回正文预览..." : "开始写作后这里会显示本章正文预览。")}
              {loading && <span className="live-cursor" />}
            </div>
          </div>
        </section>
      )}
      {msg && <div className={`msg-banner ${msg.toLowerCase().includes("error") || msg.includes("错误") ? "msg-error" : "msg-success"}`}>{msg}</div>}

      <div style={{ marginTop: 24 }}>
        <h3 className="section-title">最近日志</h3>
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
  const [form, setForm] = useState({ name: "我的小说", genre: "fantasy", targetAudience: "", tone: "热血", description: "" });

  const handleCreate = async () => {
    setLoading(true); setMsg("正在通过 AI 生成小说圣经...（可能需要 30-60 秒）");
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
      setMsg(`小说《${result.name}》已创建。可以切到“小说圣经”或“章节计划”继续。`);
    } catch (e) { setMsg("错误：" + e); }
    setLoading(false);
  };

  const handleDelete = async (id: string) => {
    const p = projects.find(x => x.id === id);
    if (!confirm(`确定删除《${p?.name}》？`)) return;
    try { await invoke("delete_project", { id }); refresh(); } catch (e) { alert("错误：" + e); }
  };

  return (
    <>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <h2 className="page-title" style={{ marginBottom: 0 }}>项目（{projects.length}）</h2>
        <button className="btn btn-primary" onClick={() => setShowCreate(!showCreate)}>+ 新建小说</button>
      </div>

      {showCreate && (
        <div className="card-feature" style={{ marginBottom: 24 }}>
          <h3 className="section-title">创建新小说</h3>
          <p className="text-meta" style={{ marginBottom: 16 }}>填写世界观偏好，AI 会根据输入生成完整小说圣经。</p>
          <div className="bible-edit-field">
            <label>小说名</label>
            <input value={form.name} onChange={e => setForm({...form, name: e.target.value})} />
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div className="bible-edit-field">
              <label>题材（可输入或选择）</label>
              <input value={form.genre} onChange={e => setForm({...form, genre: e.target.value})} list="genres" placeholder="e.g. 修仙, 科幻, 都市异能..." />
              <datalist id="genres">
                <option value="修仙" /><option value="武侠" /><option value="科幻" /><option value="都市" /><option value="历史" /><option value="悬疑" /><option value="言情" /><option value="无限流" /><option value="末世" /><option value="游戏异界" />
              </datalist>
            </div>
            <div className="bible-edit-field">
              <label>基调（可输入或选择）</label>
              <input value={form.tone} onChange={e => setForm({...form, tone: e.target.value})} list="tones" placeholder="e.g. 热血, 轻松, 暗黑..." />
              <datalist id="tones">
                <option value="热血" /><option value="轻松" /><option value="悬疑" /><option value="暗黑" /><option value="史诗" /><option value="幽默" /><option value="温馨" /><option value="冷酷" /><option value="烧脑" />
              </datalist>
            </div>
          </div>
          <div className="bible-edit-field">
            <label>目标读者</label>
            <input value={form.targetAudience} onChange={e => setForm({...form, targetAudience: e.target.value})} placeholder="e.g. 18-35岁男性读者" />
          </div>
          <div className="bible-edit-field">
            <label>世界观简述</label>
            <textarea value={form.description} onChange={e => setForm({...form, description: e.target.value})} placeholder="简要描述世界设定、时代背景或核心概念..." />
          </div>
          <div style={{ marginTop: 16, display: "flex", gap: 8 }}>
            <button className="btn btn-primary" onClick={handleCreate}>生成小说圣经</button>
            <button className="btn btn-secondary" onClick={() => setShowCreate(false)}>取消</button>
          </div>
        </div>
      )}

      {projects.map(p => (
        <div key={p.id} className="card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div style={{ cursor: "pointer" }} onClick={() => setSelected(p.id)}>
            <div style={{ fontFamily: "var(--font-display)", fontSize: 16, fontWeight: 400, color: "var(--on-dark)" }}>{p.name}</div>
            <div className="text-meta">{p.genre || "未填写"} · {p.chapter_count} 章 · {(p.total_words||0).toLocaleString()} 字 · {p.plans_left} 个计划 · 今日 {p.chapters_today} 章</div>
          </div>
          <button className="btn btn-sm btn-danger" onClick={() => handleDelete(p.id)}>删除</button>
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
    } catch (_) { setContent("加载失败"); }
  };

  const saveEdit = async () => {
    if (!openId) return;
    setSaving(true);
    try {
      const ch = chapters.find(c => c.id === openId);
      await invoke("save_edited_chapter", { chapterId: openId, title: ch?.title || "修订版", bodyMarkdown: editContent });
      setContent(editContent); setEditing(false);
      // Reload versions
      const vers = await invoke<ChapterVersion[]>("get_chapter_versions", { chapterId: openId });
      setVersions(vers);
    } catch (e) { alert("错误：" + e); }
    setSaving(false);
  };

  return (
    <>
      <h2 className="page-title">章节</h2>
      {chapters.map(ch => (
        <div key={ch.id} className="chapter-item" onClick={() => open(ch)}>
          <div className="chapter-title">第 {ch.sequence} 章 — {ch.title || "未命名"} <span className="badge badge-active">{ch.status}</span></div>
          <div className="chapter-meta">{ch.word_count || 0} words</div>
          {openId === ch.id && (
            <div style={{ marginTop: 8 }} onClick={e => e.stopPropagation()}>
              {!editing ? (
                <>
                  <div className="content-preview">{content || "（暂无内容）"}</div>
                  <div style={{ marginTop: 8, display: "flex", gap: 8 }}>
                    <button className="btn btn-sm btn-primary" onClick={() => setEditing(true)}>编辑</button>
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
                    <button className="btn btn-sm btn-primary" onClick={saveEdit} disabled={saving}>{saving ? "保存中..." : "保存修改"}</button>
                    <button className="btn btn-sm btn-secondary" onClick={() => setEditing(false)}>取消</button>
                  </div>
                </>
              )}
            </div>
          )}
        </div>
      ))}
      {chapters.length === 0 && <div className="text-meta">还没有章节。创建小说后可以在仪表盘开始写作。</div>}
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
    } catch (e) { alert("错误：" + e); }
  };

  return (
    <>
      <h2 className="page-title">章节计划</h2>
      {plans.map(p => (
        <div key={p.id} className="card" onClick={() => { setEditId(p.id); setEditTitle(p.title || ""); setEditOutline(p.outline || ""); }} style={{ cursor: "pointer" }}>
          <div className="chapter-title" style={{ color: p.status === "planned" ? "var(--primary)" : "var(--on-dark)" }}>
            第 {p.sequence} 章 — {p.title || "未命名"} <span className="badge badge-active">{p.status}</span>
          </div>
          {editId === p.id ? (
            <div onClick={e => e.stopPropagation()} style={{ marginTop: 8 }}>
              <div className="bible-edit-field"><label>标题</label><input value={editTitle} onChange={e => setEditTitle(e.target.value)} /></div>
              <div className="bible-edit-field"><label>大纲</label><textarea value={editOutline} onChange={e => setEditOutline(e.target.value)} /></div>
              <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
                <button className="btn btn-sm btn-primary" onClick={() => savePlan(p.id)}>保存</button>
                <button className="btn btn-sm btn-secondary" onClick={() => setEditId(null)}>取消</button>
              </div>
            </div>
          ) : (
            <div className="text-meta" style={{ marginTop: 4 }}>{p.outline?.slice(0, 200) || "暂无大纲"} · {p.target_word_count || 0} 字</div>
          )}
        </div>
      ))}
      {plans.length === 0 && <div className="text-meta">还没有章节计划。请先生成周计划。</div>}
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
    continuity_reviewer: "连贯性", character_reviewer: "人物", plot_logic_reviewer: "剧情逻辑",
    pacing_reviewer: "节奏", style_reviewer: "风格", safety_reviewer: "安全", publication_reviewer: "发布",
    canon_consistency_precheck: "圣经预检",
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
      <h2 className="page-title">审稿报告</h2>
      {qualitySummary && (
        <section className="quality-dashboard">
          <div className="quality-stat-grid">
            <div><strong>{qualitySummary.reviewed_chapter_count}</strong><span>已审</span></div>
            <div><strong>{formatNumber(qualitySummary.average_final_score, 1)}</strong><span>平均终分</span></div>
            <div><strong>{qualitySummary.publish_ready_count}</strong><span>可发布</span></div>
            <div><strong>{qualitySummary.revise_count}</strong><span>需修订</span></div>
            <div><strong>{qualitySummary.needs_human_review_count}</strong><span>人工复核</span></div>
            <div><strong>{qualitySummary.total_blocking_issues}</strong><span>阻塞问题</span></div>
          </div>
          <div className="quality-summary-row">
            <div>
              <div className="status-label">最新决策</div>
              <div className="quality-latest">{qualitySummary.latest_decision || "暂无审稿"}</div>
            </div>
            <div>
              <div className="status-label">最新分数</div>
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
        {chapters.map(c => <option key={c.id} value={c.id}>第 {c.sequence} 章 — {c.title}</option>)}
      </select>
      {scores && (
        <div className="card-feature" style={{ marginBottom: 16 }}>
          <div style={{ fontFamily: "var(--font-display)", fontSize: 22, fontWeight: 300, color: "var(--on-dark)" }}>
            Final Score: <span style={{ color: scoreColor(scores.final_score) }}>{formatNumber(scores.final_score, 0)}</span>
          </div>
          <div className="text-meta" style={{ marginTop: 4 }}>决策：{scores.decision} · 平均：{formatNumber(scores.average_score, 1)} · 阻塞：{scores.blocking_issue_count} · 发布：{scores.publish_allowed ? "是" : "否"}</div>
        </div>
      )}
      {canonIssues.length > 0 && (
        <section className="card-feature canon-precheck-panel" style={{ marginBottom: 16 }}>
          <h3 className="section-title" style={{ color: "var(--warning)" }}>圣经预检问题</h3>
          <div className="canon-issue-list">
            {canonIssues.map((issue, idx) => (
              <div className="canon-issue-row" key={`${issue.rule_type || "issue"}-${idx}`}>
                <div className="canon-issue-head">
                  <span className={`badge ${issue.severity === "blocking" ? "badge-warning" : "badge-active"}`}>{issue.severity || "warning"}</span>
                  <strong>{issue.rule_type || "canon_issue"}</strong>
                </div>
                <div>{issue.message || "检测到圣经一致性问题。"}</div>
                {issue.evidence && <div className="text-meta">证据：{issue.evidence}</div>}
              </div>
            ))}
          </div>
        </section>
      )}
      {reviews.map(r => (
        <div key={r.id} className="card" style={{ borderLeft: r.pass ? "3px solid var(--success)" : "3px solid var(--warning)" }}>
          <div className="chapter-title">{agentNames[r.agent_name] || r.agent_name} <span className={`badge ${(r.score || 0) >= 80 ? "badge-success" : (r.score || 0) >= 60 ? "badge-active" : "badge-warning"}`}>{r.score}</span></div>
          <div className="text-meta" style={{ marginTop: 4 }}>{r.pass ? "通过" : "未通过"}</div>
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
    acquire_lock: "锁定",
    load_canon: "圣经",
    retrieve_context: "RAG",
    generate_draft: "草稿",
    aggregate_reviews: "审稿",
    revise: "修订",
    export: "导出",
    update_canon: "更新圣经",
    complete: "完成",
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
      <h2 className="page-title">生成任务</h2>
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
                <div><strong>{summary.phase_count ?? events.length}</strong><span>阶段</span></div>
                <div><strong>{formatDuration(summary.total_elapsed_ms)}</strong><span>耗时</span></div>
                <div><strong>{slowest}</strong><span>最慢</span></div>
                <div><strong>{j.retry_count || 0}</strong><span>重试</span></div>
                <div><strong>{formatTokens(usage.total_tokens)}</strong><span>Tokens</span></div>
                <div><strong>{formatCost(usage.estimated_cost_usd)}</strong><span>费用</span></div>
                <div><strong>{summary.last_step ? (phaseLabels[summary.last_step] || summary.last_step) : "n/a"}</strong><span>最后步骤</span></div>
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
                <div className="text-meta">这个任务还没有阶段时间线。</div>
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

  if (!bible) return <div className="text-meta">选择项目后查看小说圣经数据。</div>;

  const tabs: Record<string, any[]> = {
    characters: bible.characters, locations: bible.locations, organizations: bible.organizations,
    items: bible.items, world_lore: bible.world_lore, magic_systems: bible.magic_systems,
    canon_rules: bible.canon_rules, plot_threads: bible.plot_threads, foreshadowing: bible.foreshadowing,
    style_guides: bible.style_guides, timeline_events: bible.timeline_events,
  };
  const tabNames: Record<string, string> = {
    characters: "人物", locations: "地点", organizations: "组织", items: "物品",
    world_lore: "世界观", magic_systems: "力量体系", canon_rules: "圣经规则",
    plot_threads: "剧情线", foreshadowing: "伏笔", style_guides: "风格指南", timeline_events: "时间线",
  };

  const saveBibleEdit = async () => {
    if (!editingItem) return;
    try {
      await invoke("update_bible_entry", { table: tab, id: editingItem.id, data: JSON.stringify(editingItem) });
      setBible({...bible, [tab]: tabs[tab].map(item => item.id === editingItem.id ? editingItem : item)});
      setEditingItem(null);
    } catch (e) { alert("错误：" + e); }
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
      <h2 className="page-title">小说圣经</h2>
      <div className="tab-bar">
        {Object.keys(tabs).map(k => (
          <button key={k} className={`tab-btn ${tab === k ? "active" : ""}`} onClick={() => { setTab(k); setEditingItem(null); }}>{tabNames[k]}</button>
        ))}
      </div>

      {editingItem && (
        <div className="bible-edit-panel">
          <h3 style={{ fontFamily: "var(--font-display)", fontSize: 18, fontWeight: 300, color: "var(--on-dark)", marginBottom: 16 }}>编辑{tabNames[tab]}</h3>
          {(editableFields[tab] || []).map(field => (
            <div className="bible-edit-field" key={field}>
              <label>{field}</label>
              <input value={(editingItem[field] || "") as string} onChange={e => setEditingItem({...editingItem, [field]: e.target.value})} />
            </div>
          ))}
          <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
            <button className="btn btn-sm btn-primary" onClick={saveBibleEdit}>保存</button>
            <button className="btn btn-sm btn-secondary" onClick={() => setEditingItem(null)}>取消</button>
            <button className="btn btn-sm btn-commerce" style={{marginLeft:8}} onClick={async () => { await saveBibleEdit(); await invoke("rebuild_vector_index", { projectId: selected }); alert("已应用到全部，向量索引已重建。"); }}>应用到全部并重建索引</button>
          </div>
        </div>
      )}

      <div className="card">
        {(tabs[tab] || []).length === 0 && (
          <div className="text-meta">{tab === "foreshadowing" || tab === "timeline_events" ? "写作章节后会逐步填充这里。" : "暂无数据。"}</div>
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
  const [manualPositions, setManualPositions] = useState<Record<string, GraphPosition>>({});
  const [flowTick, setFlowTick] = useState(0);
  const [draggingNodeId, setDraggingNodeId] = useState("");
  const graphCanvasRef = useRef<HTMLElement | null>(null);
  const draggingNodeRef = useRef<{ id: string; pointerId: number } | null>(null);

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
      setMessage("错误：" + e);
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
  const typeLabel = graphTypeLabel;
  const query = search.trim().toLowerCase();
  const visibleNodes = nodes.filter(node => {
    const matchesType = typeFilter === "all" || node.node_type === typeFilter;
    const text = `${node.label} ${node.subtitle || ""} ${node.description || ""}`.toLowerCase();
    return matchesType && (!query || text.includes(query));
  });
  const visibleIds = new Set(visibleNodes.map(node => node.id));
  const visibleEdges = edges.filter(edge => visibleIds.has(edge.source_node_id) && visibleIds.has(edge.target_node_id));
  const visibleNodeKey = visibleNodes.map(node => `${node.node_type}:${node.id}:${node.degree}`).sort().join("|");
  const selectedNode = nodeById.get(selectedNodeId) || visibleNodes[0];
  const selectedNodeKey = selectedNode ? `${selectedNode.node_type}:${selectedNode.id}` : "";
  const connectedEdges = selectedNode
    ? neighborhood?.center.id === selectedNode.id && neighborhood.center.node_type === selectedNode.node_type
      ? neighborhood.edges
      : edges.filter(edge => edge.source_node_id === selectedNode.id || edge.target_node_id === selectedNode.id)
    : [];
  const selectedDegree = selectedNode ? connectedEdges.length : 0;
  const graphEmptyMessage = nodes.length === 0
    ? "还没有图谱节点。生成圣经或写作章节后会自动建立关系图谱。"
    : visibleNodes.length === 0
      ? "没有匹配的节点。"
      : visibleEdges.length === 0
        ? "已有节点，但还没有关系。可以生成章节或手动添加关系。"
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
    setNeighborhoodStatus("正在加载图谱上下文...");
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
          setNeighborhoodStatus("错误：" + e);
        }
      });
    return () => { cancelled = true; };
  }, [selected, selectedNodeKey]);

  const basePositions = useMemo(
    () => createGraphBasePositions(visibleNodes),
    [visibleNodeKey],
  );
  const positions = useMemo(() => {
    const map = new Map<string, GraphPosition>();
    for (const node of visibleNodes) {
      const base = manualPositions[node.id] || basePositions[node.id] || { x: 50, y: 50 };
      const position = manualPositions[node.id] || draggingNodeId === node.id
        ? clampGraphPosition(base)
        : flowGraphPosition(node, base, flowTick);
      map.set(node.id, position);
    }
    return map;
  }, [visibleNodes, visibleNodeKey, manualPositions, basePositions, draggingNodeId, flowTick]);

  useEffect(() => {
    setManualPositions(prev => {
      const next: Record<string, GraphPosition> = {};
      for (const node of visibleNodes) {
        if (prev[node.id]) next[node.id] = prev[node.id];
      }
      return next;
    });
  }, [visibleNodeKey]);

  useEffect(() => {
    if (!selected || !visibleNodes.length) return;
    const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduceMotion) return;
    let frame = 0;
    const startedAt = performance.now();
    const animate = (timestamp: number) => {
      setFlowTick((timestamp - startedAt) / 1000);
      frame = window.requestAnimationFrame(animate);
    };
    frame = window.requestAnimationFrame(animate);
    return () => window.cancelAnimationFrame(frame);
  }, [selected, visibleNodeKey]);

  const updateDragPosition = useCallback((event: { clientX: number; clientY: number }) => {
    const dragging = draggingNodeRef.current;
    const canvas = graphCanvasRef.current;
    if (!dragging || !canvas) return;
    const next = positionFromClientPoint(canvas.getBoundingClientRect(), event.clientX, event.clientY);
    setManualPositions(prev => ({ ...prev, [dragging.id]: next }));
  }, []);

  const handleNodePointerDown = (event: ReactPointerEvent<HTMLButtonElement>, node: KnowledgeGraphNode) => {
    event.preventDefault();
    event.stopPropagation();
    setSelectedNodeId(node.id);
    draggingNodeRef.current = { id: node.id, pointerId: event.pointerId };
    setDraggingNodeId(node.id);
    event.currentTarget.setPointerCapture(event.pointerId);
    updateDragPosition(event);
  };

  const stopDragging = useCallback((pointerId?: number) => {
    if (pointerId !== undefined && draggingNodeRef.current?.pointerId !== pointerId) return;
    draggingNodeRef.current = null;
    setDraggingNodeId("");
  }, []);

  const handleCanvasPointerMove = (event: ReactPointerEvent<HTMLElement>) => {
    if (!draggingNodeRef.current) return;
    event.preventDefault();
    updateDragPosition(event);
  };

  const createEdge = async () => {
    if (!selected) return;
    const source = nodeById.get(edgeForm.sourceId);
    const target = nodeById.get(edgeForm.targetId);
    if (!source || !target) {
      setMessage("错误：请选择来源和目标节点");
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
      setMessage("关系已创建。");
    } catch (e) {
      setMessage("错误：" + e);
    }
  };

  const deleteEdge = async (edgeId: string) => {
    try {
      await invoke("delete_knowledge_graph_edge", { edgeId });
      await loadGraph();
      setMessage("关系已删除。");
    } catch (e) {
      setMessage("错误：" + e);
    }
  };

  if (!selected) {
    return (
      <>
        <h2 className="page-title">关系图谱</h2>
        <div className="text-meta">选择项目后查看关系图谱。</div>
      </>
    );
  }

  return (
    <>
      <h2 className="page-title">关系图谱</h2>
      <div className="graph-workbench">
        <div className="graph-toolbar">
          <div className="graph-stat"><strong>{nodes.length}</strong><span>节点</span></div>
          <div className="graph-stat"><strong>{edges.length}</strong><span>关系</span></div>
          <div className="graph-stat"><strong>{snapshot?.orphan_count ?? 0}</strong><span>孤立节点</span></div>
          <div className="graph-stat"><strong>{selectedDegree}</strong><span>当前关系数</span></div>
          <input className="text-input graph-search" value={search} onChange={e => setSearch(e.target.value)} placeholder="搜索设定、人物或地点..." />
        </div>

        <div className="graph-type-filter">
          <button className={`tab-btn ${typeFilter === "all" ? "active" : ""}`} onClick={() => setTypeFilter("all")}>全部</button>
          {types.map(type => (
            <button key={type} className={`tab-btn ${typeFilter === type ? "active" : ""}`} onClick={() => setTypeFilter(type)}>
              {typeLabel(type)}
            </button>
          ))}
        </div>

        <div className="graph-layout">
          <section
            ref={graphCanvasRef}
            className="graph-canvas"
            onPointerMove={handleCanvasPointerMove}
            onPointerUp={event => stopDragging(event.pointerId)}
            onPointerCancel={event => stopDragging(event.pointerId)}
            onPointerLeave={() => stopDragging()}
          >
            <svg className="graph-edge-layer" viewBox="0 0 100 100" preserveAspectRatio="none" role="img" aria-label="关系图谱连线">
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
                  className={`graph-node graph-node-${node.node_type} ${selectedNode?.id === node.id ? "active" : ""} ${draggingNodeId === node.id ? "dragging" : ""}`}
                  style={{ left: `${pos.x}%`, top: `${pos.y}%` }}
                  onPointerDown={event => handleNodePointerDown(event, node)}
                  onClick={event => { event.stopPropagation(); setSelectedNodeId(node.id); }}
                  title={`${node.label} (${typeLabel(node.node_type)})`}
                  aria-label={`${node.label}，${typeLabel(node.node_type)}，${node.degree} 条关系`}
                  aria-pressed={selectedNode?.id === node.id}
                  type="button"
                >
                  <span className="graph-node-type" aria-hidden="true">{graphNodeBadge(node.node_type)}</span>
                  <span className="graph-node-label">{graphNodeDisplayLabel(node.label)}</span>
                  <span className="graph-node-degree" aria-hidden="true">{node.degree}</span>
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
                <div className="text-meta">{[selectedNode.subtitle, selectedNode.status, `${selectedNode.degree} 条关系`].filter(Boolean).join(" · ")}</div>
                {selectedNode.description && <p className="graph-description">{selectedNode.description}</p>}
                <div className="graph-rag-panel">
                  <h4>检索提示</h4>
                  {neighborhoodStatus ? (
                    <div className="text-meta">{neighborhoodStatus}</div>
                  ) : neighborhood ? (
                    <>
                      <div className="graph-hint-row">
                        <span>来源</span>
                        <strong>{neighborhood.retrieval_hints.source_key}</strong>
                      </div>
                      <div className="graph-hint-row">
                        <span>连接</span>
                        <strong>{neighborhood.retrieval_hints.connected_source_keys.length}</strong>
                      </div>
                      <div className="graph-hint-chips">
                        {neighborhood.retrieval_hints.connected_source_keys.slice(0, 8).map(key => <span key={key}>{key}</span>)}
                        {neighborhood.retrieval_hints.connected_source_keys.length === 0 && <em>暂无连接来源</em>}
                      </div>
                      <div className="graph-query-terms">
                        {neighborhood.retrieval_hints.query_terms.slice(0, 8).map(term => <span key={term}>{term}</span>)}
                      </div>
                    </>
                  ) : (
                    <div className="text-meta">暂无检索提示。</div>
                  )}
                </div>
                <h4>关系</h4>
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
                        {!edge.auto_inferred && <button className="btn btn-sm btn-danger" onClick={() => deleteEdge(edge.id)}>删除</button>}
                      </div>
                    );
                  })}
                  {connectedEdges.length === 0 && <div className="text-meta">暂无关系。</div>}
                </div>
              </>
            ) : (
              <div className="text-meta">未选择节点。</div>
            )}

            <div className="edge-form">
              <h4>添加关系</h4>
              <label>
                <span>来源</span>
                <select value={edgeForm.sourceId} onChange={e => setEdgeForm(prev => ({ ...prev, sourceId: e.target.value }))}>
                  {nodes.map(node => <option key={node.id} value={node.id}>{node.label} · {typeLabel(node.node_type)}</option>)}
                </select>
              </label>
              <label>
                <span>目标</span>
                <select value={edgeForm.targetId} onChange={e => setEdgeForm(prev => ({ ...prev, targetId: e.target.value }))}>
                  {nodes.map(node => <option key={node.id} value={node.id}>{node.label} · {typeLabel(node.node_type)}</option>)}
                </select>
              </label>
              <label>
                <span>关系类型</span>
                <input value={edgeForm.edgeType} onChange={e => setEdgeForm(prev => ({ ...prev, edgeType: e.target.value }))} />
              </label>
              <label>
                <span>描述</span>
                <textarea value={edgeForm.description} onChange={e => setEdgeForm(prev => ({ ...prev, description: e.target.value }))} />
              </label>
              <button className="btn btn-primary" onClick={createEdge} disabled={nodes.length < 2}>创建关系</button>
            </div>
          </aside>
        </div>
        {message && <div className={`msg-banner ${message.startsWith("错误") ? "msg-error" : "msg-success"}`}>{message}</div>}
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
    if (!apiKey.trim()) { setTestResult("错误：请先输入 API Key。"); return; }
    setTesting(true); setTestResult("连接中...");
    try {
      const r = await invoke<{ ok: boolean; message: string; latency_ms?: number }>("test_model_provider", { provider, apiKey: apiKey.trim(), baseUrl: null, model: settings?.model || null });
      setTestResult(r.ok ? `已连接（${r.latency_ms ?? "?"}ms）— ${r.message}` : `失败：${r.message}`);
      if (r.ok) setApiKey("");
    } catch (e) { setTestResult("错误：" + e); }
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
      <h2 className="page-title">设置</h2>
      <div className="card-feature" style={{ marginBottom: 24 }}>
        <h3 className="section-title" style={{ color: "var(--primary)" }}>模型服务商</h3>
        <div className="bible-edit-field">
          <label>服务商</label>
          <select className="select" value={provider} onChange={e => { setProvider(e.target.value); }}>
            <option value="deepseek">DeepSeek（推荐）</option>
            <option value="kimi">Kimi (月之暗面)</option>
            <option value="zhipu">智谱 GLM</option>
            <option value="openai">OpenAI</option>
            <option value="anthropic">Anthropic / Claude</option>
            <option value="gemini">Google Gemini</option>
            <option value="openai_compat">OpenAI 兼容接口</option>
            <option value="custom">自定义...</option>
          </select>
        </div>
        <div className="bible-edit-field">
          <label>模型 <span className="text-meta">（可编辑，支持任意模型名）</span></label>
          <input className="text-input" value={settings?.model || ""} onChange={e => invoke("update_settings", { settings: { ...settings, model: e.target.value } }).then(refreshSettings)} placeholder={provider==="deepseek"?"deepseek-v4-pro":provider==="kimi"?"moonshot-v1-8k":provider==="zhipu"?"glm-4-flash":"模型名称"} />
        </div>
        <div className="bible-edit-field">
          <label>API Key {savedProvider && savedProvider !== provider ? "（已保存给 " + savedProvider + "）" : savedProvider ? "（已保存）" : "（尚未保存）"}</label>
          <input className="text-input" type="password" value={apiKey} onChange={e => setApiKey(e.target.value)} placeholder="sk-..." />
        </div>
        <button className="btn btn-primary" onClick={saveAndTest} disabled={testing || !apiKey.trim()}>{testing ? "测试中..." : "保存并测试连接"}</button>
        {testResult && (
          <div className={`msg-banner ${testResult.startsWith("已连接") ? "msg-success" : "msg-error"}`} style={{ marginTop: 12 }}>
            {testResult}
          </div>
        )}
      </div>
      {settings && (
        <div className="card">
          <h3 className="section-title">当前配置（点击编辑）</h3>
          <SaveField label="模型" value={settings.model} onSave={v => invoke("update_settings", { settings: { ...settings, model: v } }).then(refreshSettings)} />
          <SaveField label="Base URL" value={settings.base_url} onSave={v => invoke("update_settings", { settings: { ...settings, base_url: v } }).then(refreshSettings)} />
          <SaveField label="数据目录" value={settings.data_dir} onSave={v => invoke("update_settings", { settings: { ...settings, data_dir: v } }).then(refreshSettings)} />
          <SaveField label="质量阈值" value={String(settings.quality_threshold)} onSave={v => invoke("update_settings", { settings: { ...settings, quality_threshold: parseInt(v) || 85 } }).then(refreshSettings)} />
          <SaveField label="最大修订次数" value={String(settings.max_revise_count)} onSave={v => invoke("update_settings", { settings: { ...settings, max_revise_count: parseInt(v) || 2 } }).then(refreshSettings)} />
          <SaveField label="每日目标字数" value={String(settings.daily_target_words)} onSave={v => invoke("update_settings", { settings: { ...settings, daily_target_words: parseInt(v) || 3000 } }).then(refreshSettings)} />
          <SaveField label="输入成本 / 100 万 Tokens" value={formatOptionalCost(settings.input_cost_per_million)} type="number" onSave={v => invoke("update_settings", { settings: { ...settings, input_cost_per_million: parseOptionalCost(v) } }).then(refreshSettings)} />
          <SaveField label="输出成本 / 100 万 Tokens" value={formatOptionalCost(settings.output_cost_per_million)} type="number" onSave={v => invoke("update_settings", { settings: { ...settings, output_cost_per_million: parseOptionalCost(v) } }).then(refreshSettings)} />
          <div className="bible-edit-field">
            <label>Embedding 服务商</label>
            <select className="select" value={settings.embedding_provider || "none"} onChange={e => {
              invoke("update_settings", { settings: { ...settings, embedding_provider: e.target.value } }).then(refreshSettings);
            }}>
              <option value="none">无（关闭）</option>
              <option value="openai">OpenAI</option>
              <option value="zhipu">智谱 GLM</option>
              <option value="openai_compat">自定义</option>
            </select>
            {(!settings.embedding_provider || settings.embedding_provider === "none") && (
              <div className="msg-banner msg-error" style={{ marginTop: 8 }}>RAG 向量检索已关闭。配置 Embedding 服务商后可启用；未启用时，AI 的连续性上下文会更少。</div>
            )}
          </div>
          {settings.embedding_provider && settings.embedding_provider !== "none" && (
            <EmbeddingSettings settings={settings} refreshSettings={refreshSettings} />
          )}
          <div className="bible-edit-field" style={{ marginTop: 12 }}>
            <label>自动发布</label>
            <input type="checkbox" checked={settings.auto_publish} onChange={e => {
              invoke("update_settings", { settings: { ...settings, auto_publish: e.target.checked } }).then(refreshSettings);
            }} />
          </div>
          <div className="pet-settings-panel">
            <h3 className="section-title">定时写作与自动发布</h3>
            <label className="checkbox-row">
              <input type="checkbox" checked={settings.publish_schedule_enabled} onChange={e => {
                invoke("update_settings", { settings: { ...settings, publish_schedule_enabled: e.target.checked } }).then(refreshSettings);
              }} />
              开启定时写作
            </label>
            <div className="bible-edit-field">
              <label>发布适配器</label>
              <select className="select" value={settings.publication_target_provider || "firefly_git"} onChange={e => {
                invoke("update_settings", { settings: { ...settings, publication_target_provider: e.target.value } }).then(refreshSettings);
              }}>
                <option value="firefly_git">Firefly / Git 静态站点</option>
              </select>
            </div>
            <SaveField label="网站仓库路径" value={settings.publication_target_path || ""} onSave={v => invoke("update_settings", { settings: { ...settings, publication_target_path: v } }).then(refreshSettings)} />
            <SaveField label="文章目录" value={settings.publication_posts_dir || "src/content/posts"} onSave={v => invoke("update_settings", { settings: { ...settings, publication_posts_dir: v || "src/content/posts" } }).then(refreshSettings)} />
            <SaveField label="远端名称" value={settings.publication_remote_name || "origin"} onSave={v => invoke("update_settings", { settings: { ...settings, publication_remote_name: v || "origin" } }).then(refreshSettings)} />
            <SaveField label="分支" value={settings.publication_branch || ""} onSave={v => invoke("update_settings", { settings: { ...settings, publication_branch: v.trim() || null } }).then(refreshSettings)} />
            <SaveField label="构建命令" value={settings.publication_build_command || "pnpm build"} onSave={v => invoke("update_settings", { settings: { ...settings, publication_build_command: v || "pnpm build" } }).then(refreshSettings)} />
            <SaveField label="提交模板" value={settings.publication_commit_template || "publish: add {title}"} onSave={v => invoke("update_settings", { settings: { ...settings, publication_commit_template: v || "publish: add {title}" } }).then(refreshSettings)} />
            <label className="checkbox-row">
              <input type="checkbox" checked={settings.publication_validate_build} onChange={e => {
                invoke("update_settings", { settings: { ...settings, publication_validate_build: e.target.checked } }).then(refreshSettings);
              }} />
              发布前运行构建验证
            </label>
            <label className="checkbox-row">
              <input type="checkbox" checked={settings.publication_push_enabled} onChange={e => {
                invoke("update_settings", { settings: { ...settings, publication_push_enabled: e.target.checked } }).then(refreshSettings);
              }} />
              发布后推送远端
            </label>
            <label className="checkbox-row">
              <input type="checkbox" checked={settings.publication_dry_run} onChange={e => {
                invoke("update_settings", { settings: { ...settings, publication_dry_run: e.target.checked } }).then(refreshSettings);
              }} />
              仅演练，不写入外部仓库
            </label>
            <div className="text-meta">自动发布只会暂存生成的文章文件，不会暂存网站仓库中的其它改动。</div>
          </div>
          <div className="bible-edit-field">
            <label>调试模式</label>
            <input type="checkbox" checked={settings.debug_mode} onChange={e => {
              invoke("update_settings", { settings: { ...settings, debug_mode: e.target.checked } }).then(refreshSettings);
            }} />
          </div>
          <div className="pet-settings-panel">
            <h3 className="section-title">桌面宠物</h3>
            <label className="checkbox-row">
              <input type="checkbox" checked={settings.pet_enabled} onChange={e => {
                const enabled = e.target.checked;
                invoke("update_settings", { settings: { ...settings, pet_enabled: enabled } })
                  .then(() => enabled ? tauriClient.showPetWindow() : tauriClient.hidePetWindow())
                  .then(refreshSettings);
              }} />
              开启宠物
            </label>
            <div className="bible-edit-field">
              <label>动画级别</label>
              <select className="select" value={settings.pet_animation_level || "subtle"} onChange={e => {
                invoke("update_settings", { settings: { ...settings, pet_animation_level: e.target.value } }).then(refreshSettings);
              }}>
                <option value="static">静态</option>
                <option value="subtle">轻微</option>
                <option value="lively">活跃</option>
              </select>
            </div>
            <label className="checkbox-row">
              <input type="checkbox" checked={settings.pet_compact_mode} onChange={e => {
                invoke("update_settings", { settings: { ...settings, pet_compact_mode: e.target.checked } }).then(refreshSettings);
              }} />
              紧凑显示
            </label>
            <div className="text-meta">宠物只读取现有运行状态，不会增加额外后台任务。</div>
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
          {value || "（点击设置）"}
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
    if (!selected) { setResult("错误：请先选择项目。"); return; }
    if (file.size > maxLearningFileBytes) { setResult("错误：文件过大（超过 1 MiB）。"); return; }
    setLearning(true); setResult("正在读取文件...");
    try {
      const text = await file.text();
      setResult("正在提取模式...");
      const r = await invoke<any[]>("learn_from_file_text", {
        projectId: selected,
        fileName: file.name,
        byteLen: file.size,
        text,
        sourceTitle: sourceTitle.trim() || null,
      });
      setResult(`已学习 ${r.length} 个模式`);
      setSelectedFile(null); setSourceTitle("");
      if (fileInputRef.current) fileInputRef.current.value = "";
      loadEntries();
    } catch (e: any) { setResult("错误：" + (e.message || String(e))); }
    finally { setLearning(false); }
  };

  const handleLearnUrl = async () => {
    if (!url.trim()) return;
    if (!selected) { setResult("错误：请先选择项目。"); return; }
    setLearning(true); setResult("正在抓取并提取页面...");
    try {
      setResult("正在提取模式...");
      const r = await invoke<any[]>("learn_from_url", { projectId: selected, url: url.trim() });
      setResult(`已学习 ${r.length} 个模式`);
      setUrl("");
      loadEntries();
    } catch (e: any) { setResult("错误：" + (e.message || String(e))); }
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
  const sourceMarkerLabel = (sourceType?: string) =>
    sourceType === "web" ? "网页" : sourceType === "self_reflection" ? "复盘" : "文件";

  return (
    <>
      <h2 className="page-title">学习库</h2>
      <div className="tab-bar">
        <button className={`tab-btn ${tab==="file"?"active":""}`} onClick={()=>setTab("file")}>文件学习</button>
        <button className={`tab-btn ${tab==="web"?"active":""}`} onClick={()=>setTab("web")}>网页学习</button>
        <button className={`tab-btn ${tab==="library"?"active":""}`} onClick={()=>{setTab("library");loadEntries();}}>知识库（{entries.length}）</button>
      </div>

      {tab === "file" && (
        <div className="card-feature">
          <div className="bible-edit-field"><label>来源标题</label><input className="text-input" value={sourceTitle} onChange={e=>setSourceTitle(e.target.value)} placeholder={selectedFile?.name || "来源名称..."} /></div>
          <input ref={fileInputRef} type="file" accept={learningFileAccept} onChange={handleFileSelected} style={{display:"none"}} />
          <div className="bible-edit-field">
            <label>来源文件</label>
            <div style={{display:"flex",gap:8,alignItems:"center",flexWrap:"wrap"}}>
              <button className="btn btn-secondary" onClick={()=>fileInputRef.current?.click()} disabled={learning}>选择文件</button>
              <span className="text-meta">{selectedFile ? `${selectedFile.name} (${Math.ceil(selectedFile.size / 1024)} KB)` : "未选择文件"}</span>
            </div>
          </div>
          <button className="btn btn-primary" onClick={handleLearnFile} disabled={learning||!selectedFile}>{learning?"提取中...":"提取知识"}</button>
          {result && <div className={`msg-banner ${result.includes("错误")?"msg-error":"msg-success"}`} style={{marginTop:8}}>{result}</div>}
        </div>
      )}

      {tab === "web" && (
        <div className="card-feature">
          <div className="bible-edit-field"><label>学习 URL</label><input className="text-input" value={url} onChange={e=>setUrl(e.target.value)} placeholder="https://..." /></div>
          <button className="btn btn-primary" onClick={handleLearnUrl} disabled={learning||!url.trim()}>{learning?"抓取中...":"抓取并学习"}</button>
          {result && <div className={`msg-banner ${result.includes("错误")?"msg-error":"msg-success"}`} style={{marginTop:8}}>{result}</div>}
        </div>
      )}

      {tab === "library" && (
        <div className="status-grid" style={{gridTemplateColumns:"1fr"}}>
          {entries.length === 0 && <div className="text-meta">还没有学习模式。导入文件或使用网页学习来建立知识库。</div>}
          {entries.map((e: any) => (
            <div key={e.id} className="card" style={{cursor:"pointer", borderLeft:`3px solid ${catColors[e.category]||"var(--hairline-dark)"}`}}>
              <div style={{display:"flex",justifyContent:"space-between",alignItems:"center"}} onClick={()=>setExpandedId(expandedId===e.id?null:e.id)}>
                <div>
                  <span className="chapter-title">{e.pattern_name}</span>
                  <span className="badge badge-active" style={{marginLeft:8}}>{e.category}</span>
                  <span className="text-meta source-meta" style={{marginLeft:8}}>
                    <span className={`source-marker source-marker-${e.source_type || "file"}`} aria-hidden="true">
                      {sourceMarkerLabel(e.source_type)}
                    </span>
                    {e.source_title}
                  </span>
                </div>
                <button className="btn btn-sm btn-danger" onClick={(ev)=>{ev.stopPropagation();handleDelete(e.id);}}>删除</button>
              </div>
              {expandedId === e.id && (
                <div style={{marginTop:8,padding:8,background:"var(--surface-dark-elevated)",borderRadius:"var(--radius-sm)"}}>
                  <div className="text-body">{e.pattern_description}</div>
                  {e.example_text && <div className="content-preview" style={{marginTop:8}}>{e.example_text.slice(0,500)}</div>}
                  {e.application_notes && <div className="text-meta" style={{marginTop:4}}>应用：{e.application_notes}</div>}
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
    if (!embKey.trim()) { setResult("错误：请输入 Embedding API Key"); return; }
    setTesting(true); setResult("测试中...");
    try {
      const r = await invoke<{ok:boolean;message:string;latency_ms?:number}>("test_embedding_provider", { provider: settings.embedding_provider, apiKey: embKey.trim(), baseUrl, model });
      setResult(r.ok ? `已连接（${r.latency_ms ?? "?"}ms，${r.message}）` : `失败：${r.message}`);
      if (r.ok) { setEmbKey(""); refreshSettings(); }
    } catch (e) { setResult("错误：" + e); }
    setTesting(false);
  };

  return (
    <div className="card" style={{ marginTop: 12, borderLeft: "3px solid var(--primary)" }}>
      <h4 style={{ fontFamily:"var(--font-display)",fontSize:16,fontWeight:300,color:"var(--on-dark)",marginBottom:12 }}>Embedding API 设置</h4>
      <div className="bible-edit-field"><label>Base URL</label><input className="text-input" value={baseUrl} onChange={e=>{setBaseUrl(e.target.value); invoke("update_settings",{settings:{...settings,embedding_base_url:e.target.value}}).then(refreshSettings);}} placeholder="https://..." /></div>
      <div className="bible-edit-field"><label>模型</label><input className="text-input" value={model} onChange={e=>{setModel(e.target.value); invoke("update_settings",{settings:{...settings,embedding_model:e.target.value}}).then(refreshSettings);}} /></div>
      <div className="bible-edit-field"><label>API Key</label><input className="text-input" type="password" value={embKey} onChange={e=>setEmbKey(e.target.value)} placeholder="输入 Embedding API Key..." /></div>
      <button className="btn btn-primary" onClick={saveAndTest} disabled={testing||!embKey.trim()} style={{marginTop:8}}>{testing?"测试中...":"保存并测试 Embedding"}</button>
      {result && <div className={`msg-banner ${result.startsWith("已连接")?"msg-success":"msg-error"}`} style={{marginTop:8}}>{result}</div>}
    </div>
  );
}

export default App;
