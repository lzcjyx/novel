import { useCallback, useEffect, useState } from "react";
import { tauriClient } from "../lib/tauriClient";

type NullableNumber = number | null | undefined;

interface AppSettings { provider: string; model: string; base_url: string; embedding_model: string; embedding_provider: string; embedding_base_url: string; embedding_dim: number; quality_threshold: number; auto_publish: boolean; max_revise_count: number; daily_target_words: number; data_dir: string; debug_mode: boolean; blog_provider: string; input_cost_per_million?: number | null; output_cost_per_million?: number | null; draft_model_profile_id?: string | null; review_model_profile_id?: string | null; repair_model_profile_id?: string | null; embedding_model_profile_id?: string | null; summarization_model_profile_id?: string | null; }
interface ChapterPlan { id: string; sequence: number; title?: string; outline?: string; target_word_count?: number; status: string; }
interface ContextRule { id: string; name: string; primary_keywords: string[]; secondary_keywords: string[]; priority: number; token_budget: number; content: string; source_type: string; enabled: boolean; }
interface OperatorRecipeAction { kind: string; label: string; parameters: any; }
interface OperatorRecipe { id: string; name: string; description: string; actions: OperatorRecipeAction[]; }
interface OperatorRecipeRunEvent { step: string; status: string; detail?: string; progress_pct: number; }
interface OperatorRecipeRunResult { ok: boolean; job_id: string; recipe_id: string; status: string; events: OperatorRecipeRunEvent[]; error_message?: string; }
interface DraftCandidate { id: string; candidate_number: number; title: string; body_markdown: string; summary?: string; word_count: number; status: string; selection_reason?: string; estimated_cost_usd?: NullableNumber; }
interface LorebookImportSummary { imported_count: number; skipped_count: number; }
interface NovelBibleImportSummary { imported_characters: number; imported_world_lore: number; }
interface PromptPreset { id: string; name: string; description?: string; scope: string; is_builtin: boolean; }
interface ModelProfile { id: string; name: string; provider: string; base_url: string; model: string; context_window: number; supports_json: boolean; supports_streaming: boolean; supports_embeddings: boolean; input_cost_per_million?: NullableNumber; output_cost_per_million?: NullableNumber; intended_use: string; }
interface ExtensionManifest { id: string; name: string; version: string; description?: string | null; enabled_by_default: boolean; permissions: string[]; hooks: string[]; package_kinds: string[]; metadata?: any; }
interface ExtensionPackage { manifest: ExtensionManifest; enabled: boolean; contributions: any[]; }
interface RagHealth { state: string; message: string; document_count: number; stale_count: number; embedding_provider: string; embedding_model: string; embedding_dim: number; last_indexed_at?: string | null; }
type RuntimePhaseId = "prepare" | "context" | "generate" | "review" | "deliver";

interface RuntimePageProps {
  selected: string;
  settings: AppSettings | null;
  refreshSettings: () => void;
}
// ---- RuntimePage ----
export function RuntimePage({ selected, settings, refreshSettings }: RuntimePageProps) {
  const [plans, setPlans] = useState<ChapterPlan[]>([]);
  const [selectedPlanId, setSelectedPlanId] = useState("");
  const [recipes, setRecipes] = useState<OperatorRecipe[]>([]);
  const [contextRules, setContextRules] = useState<ContextRule[]>([]);
  const [draftCandidates, setDraftCandidates] = useState<DraftCandidate[]>([]);
  const [promptPresets, setPromptPresets] = useState<PromptPreset[]>([]);
  const [runResult, setRunResult] = useState<OperatorRecipeRunResult | null>(null);
  const [runtimeMsg, setRuntimeMsg] = useState("");
  const [activePhase, setActivePhase] = useState<RuntimePhaseId>("prepare");
  const [ragHealth, setRagHealth] = useState<RagHealth | null>(null);
  const [lorebookJson, setLorebookJson] = useState("");
  const [manualRuleForm, setManualRuleForm] = useState({
    name: "手动上下文规则",
    primary_keywords: "",
    secondary_keywords: "",
    entity_refs: "",
    chapter_ranges: "",
    priority: 50,
    token_budget: 320,
    sticky_chapters: 0,
    cooldown_chapters: 0,
    content: "",
    enabled: true,
  });
  const [projectPackageJson, setProjectPackageJson] = useState("");
  const [biblePackageJson, setBiblePackageJson] = useState("");
  const [promptPackageJson, setPromptPackageJson] = useState("");
  const [promptPresetForm, setPromptPresetForm] = useState({ name: "草稿提示词预设", scope: "draft", description: "" });
  const [promptUnitForm, setPromptUnitForm] = useState({
    preset_id: "",
    identifier: "draft.body",
    role: "user",
    order: 100,
    enabled: true,
    injection_position: "main",
    generation_phase: "draft",
    content: "",
  });
  const [extensionJson, setExtensionJson] = useState(JSON.stringify({
    id: "local.prompt_pack.demo",
    name: "Local Prompt Pack Demo",
    version: "0.1.0",
    description: "Validated manifest example",
    enabled_by_default: false,
    permissions: ["project_read"],
    hooks: ["before_context_build"],
    package_kinds: ["prompt_pack"],
    metadata: {},
  }, null, 2));
  const [extensionPackages, setExtensionPackages] = useState<ExtensionPackage[]>([]);
  const [selectionReason, setSelectionReason] = useState("Best continuity and scene economy.");
  const [modelProfiles, setModelProfiles] = useState<ModelProfile[]>([]);
  const [profileId, setProfileId] = useState("");
  const [profileWorkflow, setProfileWorkflow] = useState("draft");
  const [profileWarnings, setProfileWarnings] = useState<any[]>([]);
  const [profileForm, setProfileForm] = useState({
    name: "草稿长上下文配置",
    provider: "openai_compat",
    base_url: "https://api.openai.com/v1",
    model: "gpt-4o",
    context_window: 128000,
    intended_use: "draft",
    supports_json: true,
    supports_streaming: true,
    supports_embeddings: false,
    input_cost_per_million: "",
    output_cost_per_million: "",
  });

  const loadRecipes = useCallback(async () => {
    try {
      setRecipes(await tauriClient.getBuiltinOperatorRecipes<OperatorRecipe[]>());
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  }, []);

  const loadPlans = useCallback(async () => {
    if (!selected) { setPlans([]); setSelectedPlanId(""); return; }
    try {
      const loaded = await tauriClient.getChapterPlans<ChapterPlan[]>(selected);
      setPlans(loaded);
      setSelectedPlanId((current) => {
        if (current && loaded.some(plan => plan.id === current)) return current;
        const planned = loaded.find(plan => plan.status === "planned");
        return planned?.id || loaded[0]?.id || "";
      });
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  }, [selected]);

  const loadContextRules = useCallback(async () => {
    if (!selected) { setContextRules([]); return; }
    try {
      setContextRules(await tauriClient.getContextRules<ContextRule[]>(selected));
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  }, [selected]);

  const loadRagHealth = useCallback(async () => {
    if (!selected) { setRagHealth(null); return; }
    try {
      setRagHealth(await tauriClient.getRagHealth<RagHealth>(selected));
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  }, [selected]);

  const loadDraftCandidates = useCallback(async () => {
    if (!selectedPlanId) { setDraftCandidates([]); return; }
    try {
      setDraftCandidates(await tauriClient.getDraftCandidates<DraftCandidate[]>(selectedPlanId));
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  }, [selectedPlanId]);

  const loadPromptPresets = useCallback(async () => {
    try {
      setPromptPresets(await tauriClient.listPromptPresets<PromptPreset[]>());
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  }, []);

  const loadModelProfiles = useCallback(async () => {
    try {
      setModelProfiles(await tauriClient.listModelProfiles<ModelProfile[]>());
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  }, []);

  const loadExtensionPackages = useCallback(async () => {
    try {
      setExtensionPackages(await tauriClient.listExtensionPackages<ExtensionPackage[]>());
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  }, []);

  useEffect(() => { loadRecipes(); }, [loadRecipes]);
  useEffect(() => { loadPromptPresets(); }, [loadPromptPresets]);
  useEffect(() => { loadModelProfiles(); }, [loadModelProfiles]);
  useEffect(() => { loadExtensionPackages(); }, [loadExtensionPackages]);
  useEffect(() => { loadPlans(); loadContextRules(); loadRagHealth(); }, [loadPlans, loadContextRules, loadRagHealth]);
  useEffect(() => { loadDraftCandidates(); }, [loadDraftCandidates]);

  const runRecipe = async (recipe: OperatorRecipe) => {
    if (!selected || !selectedPlanId) { setRuntimeMsg("错误：请先选择项目和章节计划。"); return; }
    setRuntimeMsg(`正在运行 ${recipe.name}...`);
    setRunResult(null);
    try {
      const result = await tauriClient.runOperatorRecipe<OperatorRecipeRunResult>({
        project_id: selected,
        chapter_plan_id: selectedPlanId,
        recipe_id: recipe.id,
      });
      setRunResult(result);
      setRuntimeMsg(result.ok ? `${recipe.name} 已完成` : `${recipe.name}：${result.error_message || result.status}`);
      loadDraftCandidates();
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const importLorebook = async () => {
    if (!selected) { setRuntimeMsg("错误：请先选择项目。"); return; }
    try {
      const summary = await tauriClient.importSillyTavernLorebook<LorebookImportSummary>(selected, lorebookJson);
      setRuntimeMsg(`已导入 ${summary.imported_count} 条 Lorebook，跳过 ${summary.skipped_count} 条`);
      setLorebookJson("");
      loadContextRules();
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const splitRuleList = (value: string) => value
    .split(/[,\n]/)
    .map(item => item.trim())
    .filter(Boolean);

  const saveManualContextRule = async () => {
    if (!selected) { setRuntimeMsg("错误：请先选择项目。"); return; }
    if (!manualRuleForm.name.trim() || !manualRuleForm.content.trim()) {
      setRuntimeMsg("错误：上下文规则名称和内容不能为空。");
      return;
    }
    try {
      const id = await tauriClient.upsertContextRule({
        id: null,
        project_id: selected,
        name: manualRuleForm.name.trim(),
        primary_keywords: splitRuleList(manualRuleForm.primary_keywords),
        secondary_keywords: splitRuleList(manualRuleForm.secondary_keywords),
        entity_refs: splitRuleList(manualRuleForm.entity_refs),
        chapter_ranges: splitRuleList(manualRuleForm.chapter_ranges),
        priority: Number(manualRuleForm.priority) || 0,
        token_budget: Math.max(0, Number(manualRuleForm.token_budget) || 0),
        sticky_chapters: Math.max(0, Number(manualRuleForm.sticky_chapters) || 0),
        cooldown_chapters: Math.max(0, Number(manualRuleForm.cooldown_chapters) || 0),
        content: manualRuleForm.content.trim(),
        source_type: "manual",
        source_id: null,
        enabled: manualRuleForm.enabled,
        metadata: { source: "runtime_page" },
      });
      setRuntimeMsg(`上下文规则已保存：${id.slice(0, 8)}`);
      setManualRuleForm(form => ({ ...form, content: "" }));
      loadContextRules();
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const exportNovelBible = async () => {
    if (!selected) { setRuntimeMsg("错误：请先选择项目。"); return; }
    try {
      const packagePayload = await tauriClient.exportNovelBiblePackage<any>(selected);
      setBiblePackageJson(JSON.stringify(packagePayload, null, 2));
      setRuntimeMsg("小说圣经包已导出");
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const importNovelBible = async () => {
    if (!selected) { setRuntimeMsg("错误：请先选择项目。"); return; }
    try {
      const packagePayload = JSON.parse(biblePackageJson);
      const summary = await tauriClient.importNovelBiblePackage<NovelBibleImportSummary>(selected, packagePayload);
      setRuntimeMsg(`已导入圣经包：${summary.imported_characters} 个人物，${summary.imported_world_lore} 条世界观`);
      loadContextRules();
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const exportProjectPackage = async () => {
    if (!selected) { setRuntimeMsg("错误：请先选择项目。"); return; }
    try {
      const packagePayload = await tauriClient.exportProjectPackage<any>(selected);
      setProjectPackageJson(JSON.stringify(packagePayload, null, 2));
      setRuntimeMsg("项目包已导出");
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const importProjectPackage = async () => {
    try {
      const importedId = await tauriClient.importProjectPackage(JSON.parse(projectPackageJson));
      setRuntimeMsg(`项目包已导入：${importedId.slice(0, 8)}`);
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const createPromptPreset = async () => {
    try {
      const id = await tauriClient.upsertPromptPreset({
        id: null,
        name: promptPresetForm.name,
        description: promptPresetForm.description.trim() || null,
        scope: promptPresetForm.scope,
        is_builtin: false,
        metadata: { source: "runtime_page" },
      });
      setRuntimeMsg(`提示词预设已保存：${id.slice(0, 8)}`);
      setPromptUnitForm(form => ({ ...form, preset_id: id }));
      loadPromptPresets();
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const savePromptUnit = async () => {
    if (!promptUnitForm.preset_id) {
      setRuntimeMsg("错误：请先选择提示词预设。");
      return;
    }
    if (!promptUnitForm.identifier.trim() || !promptUnitForm.content.trim()) {
      setRuntimeMsg("错误：提示词单元标识符和内容不能为空。");
      return;
    }
    try {
      const id = await tauriClient.upsertPromptUnit({
        preset_id: promptUnitForm.preset_id,
        identifier: promptUnitForm.identifier.trim(),
        role: promptUnitForm.role.trim() || "user",
        order: Number(promptUnitForm.order) || 0,
        enabled: promptUnitForm.enabled,
        injection_position: promptUnitForm.injection_position.trim() || "main",
        generation_phase: promptUnitForm.generation_phase.trim() || "draft",
        content: promptUnitForm.content,
        metadata: { source: "runtime_page" },
      });
      setRuntimeMsg(`提示词单元已保存：${id.slice(0, 8)}`);
      setPromptUnitForm(form => ({ ...form, content: "" }));
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const exportPromptPreset = async (presetId: string) => {
    try {
      const packagePayload = await tauriClient.getPromptPresetPackage<any>(presetId);
      setPromptPackageJson(JSON.stringify(packagePayload, null, 2));
      setRuntimeMsg("提示词预设包已导出");
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const importPromptPreset = async () => {
    try {
      const id = await tauriClient.importPromptPresetPackage(JSON.parse(promptPackageJson));
      setRuntimeMsg(`提示词预设包已导入：${id.slice(0, 8)}`);
      loadPromptPresets();
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const selectCandidate = async (candidateId: string) => {
    try {
      await tauriClient.selectDraftCandidate(candidateId, selectionReason.trim() || "由操作者选择。");
      setRuntimeMsg("草稿候选已选择");
      loadDraftCandidates();
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const validateExtension = async () => {
    try {
      const parsed = JSON.parse(extensionJson);
      await tauriClient.validateExtensionManifest(parsed.manifest || parsed);
      setRuntimeMsg("扩展 manifest 有效，默认保持关闭");
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const importExtensionPackage = async () => {
    try {
      const parsed = JSON.parse(extensionJson);
      const packagePayload = parsed.manifest
        ? parsed
        : { manifest: parsed, enabled: false, contributions: [] };
      const id = await tauriClient.importExtensionPackage(packagePayload);
      setRuntimeMsg(`扩展包已导入并保持关闭：${id}`);
      loadExtensionPackages();
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const toggleExtensionPackage = async (extensionId: string, enabled: boolean) => {
    try {
      await tauriClient.setExtensionEnabled(extensionId, enabled);
      setRuntimeMsg(`${enabled ? "已启用" : "已关闭"}扩展：${extensionId}`);
      loadExtensionPackages();
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const profileWorkflowToModelWorkflow = (workflow: string) => {
    switch (workflow) {
      case "draft": return "Draft";
      case "review": return "Review";
      case "repair": return "Repair";
      case "embedding": return "Embedding";
      case "summarization": return "Summarization";
      default: return "Draft";
    }
  };

  const saveModelProfile = async () => {
    const parseCost = (value: string) => {
      const trimmed = value.trim();
      if (!trimmed) return null;
      const parsed = Number(trimmed);
      return Number.isFinite(parsed) && parsed >= 0 ? parsed : null;
    };
    try {
      const id = await tauriClient.upsertModelProfile({
        id: profileId || null,
        name: profileForm.name,
        provider: profileForm.provider,
        base_url: profileForm.base_url,
        model: profileForm.model,
        context_window: profileForm.context_window,
        supports_json: profileForm.supports_json,
        supports_streaming: profileForm.supports_streaming,
        supports_embeddings: profileForm.supports_embeddings,
        input_cost_per_million: parseCost(profileForm.input_cost_per_million),
        output_cost_per_million: parseCost(profileForm.output_cost_per_million),
        intended_use: profileForm.intended_use,
        metadata: { source: "runtime_page" },
      });
      setProfileId(id);
      const warnings = await tauriClient.validateModelProfile<any[]>(id, profileWorkflowToModelWorkflow(profileWorkflow));
      setProfileWarnings(warnings);
      setRuntimeMsg(`模型配置已保存：${id.slice(0, 8)}`);
      loadModelProfiles();
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const loadModelProfileIntoForm = async (id: string) => {
    try {
      const profile = await tauriClient.getModelProfile<ModelProfile>(id);
      setProfileId(profile.id);
      setProfileForm({
        name: profile.name,
        provider: profile.provider,
        base_url: profile.base_url,
        model: profile.model,
        context_window: profile.context_window,
        intended_use: profile.intended_use,
        supports_json: profile.supports_json,
        supports_streaming: profile.supports_streaming,
        supports_embeddings: profile.supports_embeddings,
        input_cost_per_million: profile.input_cost_per_million == null ? "" : String(profile.input_cost_per_million),
        output_cost_per_million: profile.output_cost_per_million == null ? "" : String(profile.output_cost_per_million),
      });
      setProfileWarnings(await tauriClient.validateModelProfile<any[]>(profile.id, profileWorkflowToModelWorkflow(profileWorkflow)));
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const bindModelProfile = async () => {
    if (!profileId) { setRuntimeMsg("错误：请先选择或保存一个模型配置。"); return; }
    try {
      await tauriClient.setWorkflowModelProfile<AppSettings>(profileWorkflow, profileId);
      refreshSettings();
      setRuntimeMsg(`已将模型配置 ${profileId.slice(0, 8)} 绑定到 ${profileWorkflow}`);
    } catch (e: any) {
      setRuntimeMsg("错误：" + String(e));
    }
  };

  const currentProfileBinding = (workflow: string) => {
    switch (workflow) {
      case "draft": return settings?.draft_model_profile_id;
      case "review": return settings?.review_model_profile_id;
      case "repair": return settings?.repair_model_profile_id;
      case "embedding": return settings?.embedding_model_profile_id;
      case "summarization": return settings?.summarization_model_profile_id;
      default: return null;
    }
  };

  const selectedPlan = plans.find(plan => plan.id === selectedPlanId);
  const runtimePhases: { id: RuntimePhaseId; label: string; detail: string }[] = [
    { id: "prepare", label: "准备", detail: "章节、模型、RAG" },
    { id: "context", label: "上下文", detail: "规则、Lorebook、Trace" },
    { id: "generate", label: "生成", detail: "配方、提示词、候选" },
    { id: "review", label: "审阅", detail: "质量、反馈、修订" },
    { id: "deliver", label: "交付", detail: "包、扩展、导出" },
  ];
  const workflowBindings = [
    { id: "draft", label: "Draft", value: currentProfileBinding("draft") },
    { id: "review", label: "Review", value: currentProfileBinding("review") },
    { id: "repair", label: "Repair", value: currentProfileBinding("repair") },
    { id: "embedding", label: "Embedding", value: currentProfileBinding("embedding") },
    { id: "summarization", label: "Summarization", value: currentProfileBinding("summarization") },
  ];

  const renderModelProfileEditor = () => (
    <section className="card">
      <h3 className="section-title">模型配置</h3>
      <div style={{ marginBottom: 12 }}>
        {modelProfiles.length === 0 && <div className="text-meta">还没有保存模型配置。</div>}
        {modelProfiles.map(profile => (
          <button key={profile.id} className="btn btn-secondary btn-sm" style={{ margin: "0 8px 8px 0" }} onClick={() => loadModelProfileIntoForm(profile.id)}>
            {profile.name} ({profile.intended_use})
          </button>
        ))}
      </div>
      <div className="bible-edit-field">
        <label>工作流绑定</label>
        <select className="select" value={profileWorkflow} onChange={e => setProfileWorkflow(e.target.value)}>
          <option value="draft">Draft</option>
          <option value="review">Review</option>
          <option value="repair">Repair</option>
          <option value="embedding">Embedding</option>
          <option value="summarization">Summarization</option>
        </select>
        <div className="text-meta" style={{ marginTop: 6 }}>当前：{currentProfileBinding(profileWorkflow) || "未绑定"}</div>
      </div>
      <button className="btn btn-secondary btn-sm" onClick={bindModelProfile} disabled={!profileId}>绑定所选配置</button>
      <div className="bible-edit-field"><label>名称</label><input className="text-input" value={profileForm.name} onChange={e => setProfileForm(prev => ({ ...prev, name: e.target.value }))} /></div>
      <div className="bible-edit-field"><label>服务商</label><input className="text-input" value={profileForm.provider} onChange={e => setProfileForm(prev => ({ ...prev, provider: e.target.value }))} /></div>
      <div className="bible-edit-field"><label>Base URL</label><input className="text-input" value={profileForm.base_url} onChange={e => setProfileForm(prev => ({ ...prev, base_url: e.target.value }))} /></div>
      <div className="bible-edit-field"><label>模型</label><input className="text-input" value={profileForm.model} onChange={e => setProfileForm(prev => ({ ...prev, model: e.target.value }))} /></div>
      <div className="bible-edit-field"><label>上下文窗口</label><input className="text-input" type="number" value={profileForm.context_window} onChange={e => setProfileForm(prev => ({ ...prev, context_window: Number(e.target.value) || 0 }))} /></div>
      <div style={{ display: "flex", gap: 12, flexWrap: "wrap", marginBottom: 12 }}>
        <label className="text-meta"><input type="checkbox" checked={profileForm.supports_json} onChange={e => setProfileForm(prev => ({ ...prev, supports_json: e.target.checked }))} /> JSON</label>
        <label className="text-meta"><input type="checkbox" checked={profileForm.supports_streaming} onChange={e => setProfileForm(prev => ({ ...prev, supports_streaming: e.target.checked }))} /> Streaming</label>
        <label className="text-meta"><input type="checkbox" checked={profileForm.supports_embeddings} onChange={e => setProfileForm(prev => ({ ...prev, supports_embeddings: e.target.checked }))} /> Embeddings</label>
      </div>
      <button className="btn btn-primary btn-sm" onClick={saveModelProfile}>保存并验证</button>
      {profileWarnings.length > 0 && (
        <div className="content-preview" style={{ marginTop: 12 }}>
          {profileWarnings.map((warning, index) => <div key={index}>{warning.severity}: {warning.message}</div>)}
        </div>
      )}
    </section>
  );

  const renderContextPhase = () => (
    <section className="card">
      <h3 className="section-title">上下文规则</h3>
      <div style={{ borderBottom: "1px solid var(--hairline-dark)", paddingBottom: 12, marginBottom: 12 }}>
        <div className="chapter-title">手动上下文规则</div>
        <div className="bible-edit-field">
          <label>名称</label>
          <input className="text-input" value={manualRuleForm.name} onChange={e => setManualRuleForm(form => ({ ...form, name: e.target.value }))} />
        </div>
        <div className="bible-edit-field">
          <label>主关键词</label>
          <input className="text-input" value={manualRuleForm.primary_keywords} onChange={e => setManualRuleForm(form => ({ ...form, primary_keywords: e.target.value }))} />
        </div>
        <div className="bible-edit-field">
          <label>辅助关键词</label>
          <input className="text-input" value={manualRuleForm.secondary_keywords} onChange={e => setManualRuleForm(form => ({ ...form, secondary_keywords: e.target.value }))} />
        </div>
        <div className="bible-edit-field">
          <label>实体引用</label>
          <input className="text-input" value={manualRuleForm.entity_refs} onChange={e => setManualRuleForm(form => ({ ...form, entity_refs: e.target.value }))} />
        </div>
        <div className="bible-edit-field">
          <label>章节范围</label>
          <input className="text-input" value={manualRuleForm.chapter_ranges} onChange={e => setManualRuleForm(form => ({ ...form, chapter_ranges: e.target.value }))} />
        </div>
        <div className="mini-grid">
          <div className="bible-edit-field">
            <label>优先级</label>
            <input className="text-input" type="number" value={manualRuleForm.priority} onChange={e => setManualRuleForm(form => ({ ...form, priority: Number(e.target.value) }))} />
          </div>
          <div className="bible-edit-field">
            <label>Token 预算</label>
            <input className="text-input" type="number" min={0} value={manualRuleForm.token_budget} onChange={e => setManualRuleForm(form => ({ ...form, token_budget: Number(e.target.value) }))} />
          </div>
          <div className="bible-edit-field">
            <label>粘滞章节</label>
            <input className="text-input" type="number" min={0} value={manualRuleForm.sticky_chapters} onChange={e => setManualRuleForm(form => ({ ...form, sticky_chapters: Number(e.target.value) }))} />
          </div>
          <div className="bible-edit-field">
            <label>冷却章节</label>
            <input className="text-input" type="number" min={0} value={manualRuleForm.cooldown_chapters} onChange={e => setManualRuleForm(form => ({ ...form, cooldown_chapters: Number(e.target.value) }))} />
          </div>
        </div>
        <div className="bible-edit-field">
          <label>内容</label>
          <textarea value={manualRuleForm.content} onChange={e => setManualRuleForm(form => ({ ...form, content: e.target.value }))} />
        </div>
        <label className="checkbox-row" style={{ marginBottom: 10 }}>
          <input type="checkbox" checked={manualRuleForm.enabled} onChange={e => setManualRuleForm(form => ({ ...form, enabled: e.target.checked }))} />
          启用
        </label>
        <button className="btn btn-primary btn-sm" onClick={saveManualContextRule} disabled={!selected || !manualRuleForm.name.trim() || !manualRuleForm.content.trim()}>保存上下文规则</button>
      </div>
      <div className="bible-edit-field">
        <label>SillyTavern Lorebook JSON</label>
        <textarea value={lorebookJson} onChange={e => setLorebookJson(e.target.value)} placeholder='{"name":"World Info","entries":{}}' />
      </div>
      <button className="btn btn-secondary btn-sm" onClick={importLorebook} disabled={!selected || !lorebookJson.trim()}>导入 Lorebook</button>
      <div style={{ marginTop: 12 }}>
        {contextRules.length === 0 && <div className="text-meta">该项目还没有上下文规则。</div>}
        {contextRules.map(rule => (
          <div key={rule.id} style={{ borderBottom: "1px solid var(--hairline-dark)", padding: "8px 0" }}>
            <div className="chapter-title">{rule.name}</div>
            <div className="text-meta">优先级 {rule.priority}，预算 {rule.token_budget}，来源 {rule.source_type}，{rule.enabled ? "启用" : "关闭"}</div>
            <div className="text-meta">{rule.primary_keywords.join(", ")}</div>
          </div>
        ))}
      </div>
    </section>
  );

  const renderGeneratePhase = () => (
    <>
      <section className="card">
        <h3 className="section-title">操作配方</h3>
        {recipes.map(recipe => (
          <div key={recipe.id} style={{ borderBottom: "1px solid var(--hairline-dark)", padding: "10px 0" }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
              <div>
                <div className="chapter-title">{recipe.name}</div>
                <div className="text-meta">{recipe.description}</div>
              </div>
              <button className="btn btn-primary btn-sm" onClick={() => runRecipe(recipe)} disabled={!selectedPlanId}>运行</button>
            </div>
            <div className="text-meta" style={{ marginTop: 6 }}>{recipe.actions.map(action => action.kind).join(" -> ")}</div>
          </div>
        ))}
        {runResult && (
          <div className="content-preview" style={{ marginTop: 12 }}>
            <strong>{runResult.status}</strong> job {runResult.job_id.slice(0, 8)}
            {runResult.events.map((event, index) => (
              <div key={`${event.step}-${index}`}>{Math.round(event.progress_pct)}% {event.step}: {event.status}{event.detail ? ` - ${event.detail}` : ""}</div>
            ))}
          </div>
        )}
      </section>

      <section className="card">
        <h3 className="section-title">草稿候选</h3>
        <div className="bible-edit-field">
          <label>选择理由</label>
          <input className="text-input" value={selectionReason} onChange={e => setSelectionReason(e.target.value)} />
        </div>
        {draftCandidates.length === 0 && <div className="text-meta">所选计划还没有草稿候选。</div>}
        {draftCandidates.map(candidate => (
          <div key={candidate.id} style={{ borderBottom: "1px solid var(--hairline-dark)", padding: "10px 0" }}>
            <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
              <div>
                <div className="chapter-title">候选 {candidate.candidate_number}：{candidate.title}</div>
                <div className="text-meta">{candidate.word_count} 字，状态 {candidate.status}</div>
              </div>
              <button className="btn btn-secondary btn-sm" onClick={() => selectCandidate(candidate.id)} disabled={candidate.status === "selected"}>选择</button>
            </div>
            {candidate.summary && <div className="text-body" style={{ marginTop: 6 }}>{candidate.summary}</div>}
          </div>
        ))}
      </section>

      <section className="card">
        <h3 className="section-title">提示词预设</h3>
        <div className="bible-edit-field"><label>名称</label><input className="text-input" value={promptPresetForm.name} onChange={e => setPromptPresetForm(prev => ({ ...prev, name: e.target.value }))} /></div>
        <div className="bible-edit-field"><label>范围</label><input className="text-input" value={promptPresetForm.scope} onChange={e => setPromptPresetForm(prev => ({ ...prev, scope: e.target.value }))} /></div>
        <div className="bible-edit-field"><label>描述</label><input className="text-input" value={promptPresetForm.description} onChange={e => setPromptPresetForm(prev => ({ ...prev, description: e.target.value }))} /></div>
        <button className="btn btn-primary btn-sm" onClick={createPromptPreset} disabled={!promptPresetForm.name.trim()}>创建预设</button>
        <div style={{ borderTop: "1px solid var(--hairline-dark)", paddingTop: 12, marginTop: 12 }}>
          <div className="chapter-title">提示词单元</div>
          <div className="bible-edit-field">
            <label>预设</label>
            <select className="select" value={promptUnitForm.preset_id} onChange={e => setPromptUnitForm(prev => ({ ...prev, preset_id: e.target.value }))}>
              <option value="">选择预设</option>
              {promptPresets.map(preset => <option key={preset.id} value={preset.id}>{preset.name}</option>)}
            </select>
          </div>
          <div className="bible-edit-field"><label>标识符</label><input className="text-input" value={promptUnitForm.identifier} onChange={e => setPromptUnitForm(prev => ({ ...prev, identifier: e.target.value }))} /></div>
          <div className="mini-grid">
            <div className="bible-edit-field"><label>角色</label><input className="text-input" value={promptUnitForm.role} onChange={e => setPromptUnitForm(prev => ({ ...prev, role: e.target.value }))} /></div>
            <div className="bible-edit-field"><label>顺序</label><input className="text-input" type="number" value={promptUnitForm.order} onChange={e => setPromptUnitForm(prev => ({ ...prev, order: Number(e.target.value) }))} /></div>
            <div className="bible-edit-field"><label>注入位置</label><input className="text-input" value={promptUnitForm.injection_position} onChange={e => setPromptUnitForm(prev => ({ ...prev, injection_position: e.target.value }))} /></div>
            <div className="bible-edit-field"><label>生成阶段</label><input className="text-input" value={promptUnitForm.generation_phase} onChange={e => setPromptUnitForm(prev => ({ ...prev, generation_phase: e.target.value }))} /></div>
          </div>
          <div className="bible-edit-field">
            <label>内容</label>
            <textarea value={promptUnitForm.content} onChange={e => setPromptUnitForm(prev => ({ ...prev, content: e.target.value }))} />
          </div>
          <label className="checkbox-row" style={{ marginBottom: 10 }}>
            <input type="checkbox" checked={promptUnitForm.enabled} onChange={e => setPromptUnitForm(prev => ({ ...prev, enabled: e.target.checked }))} />
            启用
          </label>
          <button className="btn btn-secondary btn-sm" onClick={savePromptUnit} disabled={!promptUnitForm.preset_id || !promptUnitForm.identifier.trim() || !promptUnitForm.content.trim()}>保存提示词单元</button>
        </div>
        <div style={{ marginTop: 12 }}>
          {promptPresets.length === 0 && <div className="text-meta">还没有保存提示词预设。</div>}
          {promptPresets.map(preset => (
            <div key={preset.id} style={{ borderBottom: "1px solid var(--hairline-dark)", padding: "8px 0" }}>
              <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
                <div>
                  <div className="chapter-title">{preset.name}</div>
                  <div className="text-meta">{preset.scope}{preset.is_builtin ? "，内置" : ""}</div>
                </div>
                <button className="btn btn-secondary btn-sm" onClick={() => exportPromptPreset(preset.id)}>导出</button>
              </div>
            </div>
          ))}
        </div>
        <div className="bible-edit-field" style={{ marginTop: 12 }}>
          <label>预设包 JSON</label>
          <textarea value={promptPackageJson} onChange={e => setPromptPackageJson(e.target.value)} placeholder="导出预设或粘贴预设包。" />
        </div>
        <button className="btn btn-secondary btn-sm" onClick={importPromptPreset} disabled={!promptPackageJson.trim()}>导入预设包</button>
      </section>
    </>
  );

  const renderDeliverPhase = () => (
    <>
      <section className="card">
        <h3 className="section-title">项目包</h3>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12 }}>
          <button className="btn btn-secondary btn-sm" onClick={exportProjectPackage} disabled={!selected}>导出项目</button>
          <button className="btn btn-primary btn-sm" onClick={importProjectPackage} disabled={!projectPackageJson.trim()}>作为新项目导入</button>
        </div>
        <div className="bible-edit-field">
          <label>项目包 JSON</label>
          <textarea value={projectPackageJson} onChange={e => setProjectPackageJson(e.target.value)} placeholder="导出项目或粘贴项目包。" />
        </div>
      </section>

      <section className="card">
        <h3 className="section-title">小说圣经包</h3>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12 }}>
          <button className="btn btn-secondary btn-sm" onClick={exportNovelBible} disabled={!selected}>导出当前圣经</button>
          <button className="btn btn-primary btn-sm" onClick={importNovelBible} disabled={!selected || !biblePackageJson.trim()}>导入到当前项目</button>
        </div>
        <div className="bible-edit-field">
          <label>包 JSON</label>
          <textarea value={biblePackageJson} onChange={e => setBiblePackageJson(e.target.value)} placeholder="导出或粘贴一个包。" />
        </div>
      </section>

      <section className="card">
        <h3 className="section-title">扩展 Manifest</h3>
        <div className="bible-edit-field">
          <label>Manifest JSON</label>
          <textarea value={extensionJson} onChange={e => setExtensionJson(e.target.value)} />
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <button className="btn btn-secondary btn-sm" onClick={validateExtension}>验证 Manifest</button>
          <button className="btn btn-primary btn-sm" onClick={importExtensionPackage}>导入扩展包</button>
        </div>
        <div style={{ borderTop: "1px solid var(--hairline-dark)", paddingTop: 12, marginTop: 12 }}>
          <div className="chapter-title">已安装扩展</div>
          {extensionPackages.length === 0 && <div className="text-meta" style={{ marginTop: 8 }}>还没有安装扩展。</div>}
          {extensionPackages.map(pkg => (
            <div key={pkg.manifest.id} style={{ borderBottom: "1px solid var(--hairline-dark)", padding: "8px 0" }}>
              <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
                <div>
                  <div className="chapter-title">{pkg.manifest.name}</div>
                  <div className="text-meta">{pkg.manifest.id} v{pkg.manifest.version}，{pkg.enabled ? "启用" : "关闭"}</div>
                  <div className="text-meta">{pkg.manifest.package_kinds.join(", ")}</div>
                </div>
                <button className="btn btn-secondary btn-sm" onClick={() => toggleExtensionPackage(pkg.manifest.id, !pkg.enabled)}>
                  {pkg.enabled ? "关闭" : "启用"}
                </button>
              </div>
            </div>
          ))}
        </div>
      </section>
    </>
  );

  return (
    <section className="runtime-console">
      <header className="runtime-taskbar">
        <div>
          <h2 className="page-title">运行台</h2>
          <p className="text-meta">围绕当前章节计划检查上下文、模型、生成、审阅和交付。</p>
        </div>
        <div className="runtime-plan-picker">
          <label>章节计划</label>
          <select className="select" value={selectedPlanId} onChange={e => setSelectedPlanId(e.target.value)} disabled={!selected || plans.length === 0}>
            <option value="">选择计划</option>
            {plans.map(plan => <option key={plan.id} value={plan.id}>{plan.sequence}. {plan.title || "Untitled"} ({plan.status})</option>)}
          </select>
        </div>
        <div className={`runtime-rag-chip runtime-rag-${ragHealth?.state || "unknown"}`}>
          <strong>{ragHealth?.state || "unknown"}</strong>
          <span>{ragHealth?.message || "RAG 状态等待读取"}</span>
        </div>
      </header>

      {runtimeMsg && <div className={`msg-banner ${runtimeMsg.startsWith("错误") ? "msg-error" : "msg-success"}`}>{runtimeMsg}</div>}

      <nav className="runtime-phase-tabs" aria-label="运行台流程">
        {runtimePhases.map(phase => (
          <button
            key={phase.id}
            className={`runtime-phase-tab ${activePhase === phase.id ? "active" : ""}`}
            type="button"
            onClick={() => setActivePhase(phase.id)}
          >
            <strong>{phase.label}</strong>
            <span>{phase.detail}</span>
          </button>
        ))}
      </nav>

      {selectedPlan && <p className="runtime-plan-outline">{selectedPlan.outline || "暂无大纲"}</p>}

      {activePhase === "prepare" && (
        <section className="runtime-phase-panel">
          <div className="runtime-panel-main">
            <div className="runtime-panel-section">
              <h3 className="section-title">模型路由</h3>
              <div className="runtime-binding-table">
                {workflowBindings.map(binding => (
                  <div key={binding.id} className="runtime-binding-row">
                    <strong>{binding.label}</strong>
                    <span>{binding.value || "未绑定，使用全局默认"}</span>
                    <button className="btn btn-secondary btn-sm" onClick={() => { setProfileWorkflow(binding.id); setActivePhase("prepare"); }}>编辑</button>
                  </div>
                ))}
              </div>
            </div>
            {renderModelProfileEditor()}
          </div>
          <aside className="runtime-panel-side">
            <h3 className="section-title">RAG 健康</h3>
            <p className="text-body">{ragHealth?.message || "未读取 RAG 状态。"}</p>
            <div className="runtime-rag-stats">
              <span>文档 {ragHealth?.document_count ?? 0}</span>
              <span>过期 {ragHealth?.stale_count ?? 0}</span>
              <span>{ragHealth?.embedding_model || settings?.embedding_model || "未配置模型"}</span>
            </div>
            <button className="btn btn-secondary btn-sm" onClick={loadRagHealth} disabled={!selected}>刷新 RAG 状态</button>
          </aside>
        </section>
      )}

      {activePhase === "context" && (
        <section className="runtime-phase-panel runtime-phase-panel-stack">
          {renderContextPhase()}
        </section>
      )}

      {activePhase === "generate" && (
        <section className="runtime-phase-panel runtime-phase-panel-stack">
          {renderGeneratePhase()}
        </section>
      )}

      {activePhase === "review" && (
        <section className="runtime-phase-panel runtime-phase-panel-stack">
          <div className="card">
            <h3 className="section-title">审阅与修订</h3>
            <p className="text-meta">质量摘要、反馈决策和修订候选会在这里归位。</p>
          </div>
        </section>
      )}

      {activePhase === "deliver" && (
        <section className="runtime-phase-panel runtime-phase-panel-stack">
          {renderDeliverPhase()}
        </section>
      )}
    </section>
  );
}
