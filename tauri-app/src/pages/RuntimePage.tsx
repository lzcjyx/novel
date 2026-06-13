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
  const [lorebookJson, setLorebookJson] = useState("");
  const [manualRuleForm, setManualRuleForm] = useState({
    name: "Manual Context Rule",
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
  const [promptPresetForm, setPromptPresetForm] = useState({ name: "Draft Prompt Preset", scope: "draft", description: "" });
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
    name: "Draft long-context profile",
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
      setRuntimeMsg("Error: " + String(e));
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
      setRuntimeMsg("Error: " + String(e));
    }
  }, [selected]);

  const loadContextRules = useCallback(async () => {
    if (!selected) { setContextRules([]); return; }
    try {
      setContextRules(await tauriClient.getContextRules<ContextRule[]>(selected));
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  }, [selected]);

  const loadDraftCandidates = useCallback(async () => {
    if (!selectedPlanId) { setDraftCandidates([]); return; }
    try {
      setDraftCandidates(await tauriClient.getDraftCandidates<DraftCandidate[]>(selectedPlanId));
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  }, [selectedPlanId]);

  const loadPromptPresets = useCallback(async () => {
    try {
      setPromptPresets(await tauriClient.listPromptPresets<PromptPreset[]>());
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  }, []);

  const loadModelProfiles = useCallback(async () => {
    try {
      setModelProfiles(await tauriClient.listModelProfiles<ModelProfile[]>());
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  }, []);

  const loadExtensionPackages = useCallback(async () => {
    try {
      setExtensionPackages(await tauriClient.listExtensionPackages<ExtensionPackage[]>());
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  }, []);

  useEffect(() => { loadRecipes(); }, [loadRecipes]);
  useEffect(() => { loadPromptPresets(); }, [loadPromptPresets]);
  useEffect(() => { loadModelProfiles(); }, [loadModelProfiles]);
  useEffect(() => { loadExtensionPackages(); }, [loadExtensionPackages]);
  useEffect(() => { loadPlans(); loadContextRules(); }, [loadPlans, loadContextRules]);
  useEffect(() => { loadDraftCandidates(); }, [loadDraftCandidates]);

  const runRecipe = async (recipe: OperatorRecipe) => {
    if (!selected || !selectedPlanId) { setRuntimeMsg("Error: Select a project and chapter plan first."); return; }
    setRuntimeMsg(`Running ${recipe.name}...`);
    setRunResult(null);
    try {
      const result = await tauriClient.runOperatorRecipe<OperatorRecipeRunResult>({
        project_id: selected,
        chapter_plan_id: selectedPlanId,
        recipe_id: recipe.id,
      });
      setRunResult(result);
      setRuntimeMsg(result.ok ? `${recipe.name} completed` : `${recipe.name}: ${result.error_message || result.status}`);
      loadDraftCandidates();
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const importLorebook = async () => {
    if (!selected) { setRuntimeMsg("Error: Select a project first."); return; }
    try {
      const summary = await tauriClient.importSillyTavernLorebook<LorebookImportSummary>(selected, lorebookJson);
      setRuntimeMsg(`Imported ${summary.imported_count} lorebook entries, skipped ${summary.skipped_count}`);
      setLorebookJson("");
      loadContextRules();
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const splitRuleList = (value: string) => value
    .split(/[,\n]/)
    .map(item => item.trim())
    .filter(Boolean);

  const saveManualContextRule = async () => {
    if (!selected) { setRuntimeMsg("Error: Select a project first."); return; }
    if (!manualRuleForm.name.trim() || !manualRuleForm.content.trim()) {
      setRuntimeMsg("Error: Context rule name and content are required.");
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
      setRuntimeMsg(`Context rule saved: ${id.slice(0, 8)}`);
      setManualRuleForm(form => ({ ...form, content: "" }));
      loadContextRules();
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const exportNovelBible = async () => {
    if (!selected) { setRuntimeMsg("Error: Select a project first."); return; }
    try {
      const packagePayload = await tauriClient.exportNovelBiblePackage<any>(selected);
      setBiblePackageJson(JSON.stringify(packagePayload, null, 2));
      setRuntimeMsg("Novel bible package exported");
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const importNovelBible = async () => {
    if (!selected) { setRuntimeMsg("Error: Select a project first."); return; }
    try {
      const packagePayload = JSON.parse(biblePackageJson);
      const summary = await tauriClient.importNovelBiblePackage<NovelBibleImportSummary>(selected, packagePayload);
      setRuntimeMsg(`Imported bible package: ${summary.imported_characters} characters, ${summary.imported_world_lore} lore entries`);
      loadContextRules();
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const exportProjectPackage = async () => {
    if (!selected) { setRuntimeMsg("Error: Select a project first."); return; }
    try {
      const packagePayload = await tauriClient.exportProjectPackage<any>(selected);
      setProjectPackageJson(JSON.stringify(packagePayload, null, 2));
      setRuntimeMsg("Project package exported");
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const importProjectPackage = async () => {
    try {
      const importedId = await tauriClient.importProjectPackage(JSON.parse(projectPackageJson));
      setRuntimeMsg(`Project package imported: ${importedId.slice(0, 8)}`);
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
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
      setRuntimeMsg(`Prompt preset saved: ${id.slice(0, 8)}`);
      setPromptUnitForm(form => ({ ...form, preset_id: id }));
      loadPromptPresets();
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const savePromptUnit = async () => {
    if (!promptUnitForm.preset_id) {
      setRuntimeMsg("Error: Select a prompt preset first.");
      return;
    }
    if (!promptUnitForm.identifier.trim() || !promptUnitForm.content.trim()) {
      setRuntimeMsg("Error: Prompt unit identifier and content are required.");
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
      setRuntimeMsg(`Prompt unit saved: ${id.slice(0, 8)}`);
      setPromptUnitForm(form => ({ ...form, content: "" }));
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const exportPromptPreset = async (presetId: string) => {
    try {
      const packagePayload = await tauriClient.getPromptPresetPackage<any>(presetId);
      setPromptPackageJson(JSON.stringify(packagePayload, null, 2));
      setRuntimeMsg("Prompt preset package exported");
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const importPromptPreset = async () => {
    try {
      const id = await tauriClient.importPromptPresetPackage(JSON.parse(promptPackageJson));
      setRuntimeMsg(`Prompt preset package imported: ${id.slice(0, 8)}`);
      loadPromptPresets();
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const selectCandidate = async (candidateId: string) => {
    try {
      await tauriClient.selectDraftCandidate(candidateId, selectionReason.trim() || "Selected by operator.");
      setRuntimeMsg("Draft candidate selected");
      loadDraftCandidates();
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const validateExtension = async () => {
    try {
      const parsed = JSON.parse(extensionJson);
      await tauriClient.validateExtensionManifest(parsed.manifest || parsed);
      setRuntimeMsg("Extension manifest is valid and disabled by default");
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const importExtensionPackage = async () => {
    try {
      const parsed = JSON.parse(extensionJson);
      const packagePayload = parsed.manifest
        ? parsed
        : { manifest: parsed, enabled: false, contributions: [] };
      const id = await tauriClient.importExtensionPackage(packagePayload);
      setRuntimeMsg(`Extension package imported disabled: ${id}`);
      loadExtensionPackages();
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const toggleExtensionPackage = async (extensionId: string, enabled: boolean) => {
    try {
      await tauriClient.setExtensionEnabled(extensionId, enabled);
      setRuntimeMsg(`${enabled ? "Enabled" : "Disabled"} extension: ${extensionId}`);
      loadExtensionPackages();
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
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
      setRuntimeMsg(`Model profile saved: ${id.slice(0, 8)}`);
      loadModelProfiles();
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
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
      setRuntimeMsg("Error: " + String(e));
    }
  };

  const bindModelProfile = async () => {
    if (!profileId) { setRuntimeMsg("Error: Select or save a profile first."); return; }
    try {
      await tauriClient.setWorkflowModelProfile<AppSettings>(profileWorkflow, profileId);
      refreshSettings();
      setRuntimeMsg(`Bound profile ${profileId.slice(0, 8)} to ${profileWorkflow}`);
    } catch (e: any) {
      setRuntimeMsg("Error: " + String(e));
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

  return (
    <>
      <h2 className="page-title">Runtime</h2>
      <div className="card-feature" style={{ marginBottom: 16 }}>
        <div className="bible-edit-field">
          <label>Chapter Plan</label>
          <select className="select" value={selectedPlanId} onChange={e => setSelectedPlanId(e.target.value)} disabled={!selected || plans.length === 0}>
            <option value="">Select a plan</option>
            {plans.map(plan => <option key={plan.id} value={plan.id}>{plan.sequence}. {plan.title || "Untitled"} ({plan.status})</option>)}
          </select>
          {selectedPlan && <div className="text-meta" style={{ marginTop: 6 }}>{selectedPlan.outline || "No outline"}</div>}
        </div>
        {runtimeMsg && <div className={`msg-banner ${runtimeMsg.startsWith("Error") ? "msg-error" : "msg-success"}`}>{runtimeMsg}</div>}
      </div>

      <div className="status-grid" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))", alignItems: "start" }}>
        <section className="card">
          <h3 className="section-title">Operator Recipes</h3>
          {recipes.map(recipe => (
            <div key={recipe.id} style={{ borderBottom: "1px solid var(--hairline-dark)", padding: "10px 0" }}>
              <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
                <div>
                  <div className="chapter-title">{recipe.name}</div>
                  <div className="text-meta">{recipe.description}</div>
                </div>
                <button className="btn btn-primary btn-sm" onClick={() => runRecipe(recipe)} disabled={!selectedPlanId}>Run</button>
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
          <h3 className="section-title">Context Rules</h3>
          <div style={{ borderBottom: "1px solid var(--hairline-dark)", paddingBottom: 12, marginBottom: 12 }}>
            <div className="chapter-title">Manual Context Rule</div>
            <div className="bible-edit-field">
              <label>Name</label>
              <input className="text-input" value={manualRuleForm.name} onChange={e => setManualRuleForm(form => ({ ...form, name: e.target.value }))} />
            </div>
            <div className="bible-edit-field">
              <label>Primary Keywords</label>
              <input className="text-input" value={manualRuleForm.primary_keywords} onChange={e => setManualRuleForm(form => ({ ...form, primary_keywords: e.target.value }))} />
            </div>
            <div className="bible-edit-field">
              <label>Secondary Keywords</label>
              <input className="text-input" value={manualRuleForm.secondary_keywords} onChange={e => setManualRuleForm(form => ({ ...form, secondary_keywords: e.target.value }))} />
            </div>
            <div className="bible-edit-field">
              <label>Entity Refs</label>
              <input className="text-input" value={manualRuleForm.entity_refs} onChange={e => setManualRuleForm(form => ({ ...form, entity_refs: e.target.value }))} />
            </div>
            <div className="bible-edit-field">
              <label>Chapter Ranges</label>
              <input className="text-input" value={manualRuleForm.chapter_ranges} onChange={e => setManualRuleForm(form => ({ ...form, chapter_ranges: e.target.value }))} />
            </div>
            <div className="mini-grid">
              <div className="bible-edit-field">
                <label>Priority</label>
                <input className="text-input" type="number" value={manualRuleForm.priority} onChange={e => setManualRuleForm(form => ({ ...form, priority: Number(e.target.value) }))} />
              </div>
              <div className="bible-edit-field">
                <label>Token Budget</label>
                <input className="text-input" type="number" min={0} value={manualRuleForm.token_budget} onChange={e => setManualRuleForm(form => ({ ...form, token_budget: Number(e.target.value) }))} />
              </div>
              <div className="bible-edit-field">
                <label>Sticky Chapters</label>
                <input className="text-input" type="number" min={0} value={manualRuleForm.sticky_chapters} onChange={e => setManualRuleForm(form => ({ ...form, sticky_chapters: Number(e.target.value) }))} />
              </div>
              <div className="bible-edit-field">
                <label>Cooldown Chapters</label>
                <input className="text-input" type="number" min={0} value={manualRuleForm.cooldown_chapters} onChange={e => setManualRuleForm(form => ({ ...form, cooldown_chapters: Number(e.target.value) }))} />
              </div>
            </div>
            <div className="bible-edit-field">
              <label>Content</label>
              <textarea value={manualRuleForm.content} onChange={e => setManualRuleForm(form => ({ ...form, content: e.target.value }))} />
            </div>
            <label className="checkbox-row" style={{ marginBottom: 10 }}>
              <input type="checkbox" checked={manualRuleForm.enabled} onChange={e => setManualRuleForm(form => ({ ...form, enabled: e.target.checked }))} />
              Enabled
            </label>
            <button className="btn btn-primary btn-sm" onClick={saveManualContextRule} disabled={!selected || !manualRuleForm.name.trim() || !manualRuleForm.content.trim()}>Save Context Rule</button>
          </div>
          <div className="bible-edit-field">
            <label>SillyTavern Lorebook JSON</label>
            <textarea value={lorebookJson} onChange={e => setLorebookJson(e.target.value)} placeholder='{"name":"World Info","entries":{}}' />
          </div>
          <button className="btn btn-secondary btn-sm" onClick={importLorebook} disabled={!selected || !lorebookJson.trim()}>Import Lorebook</button>
          <div style={{ marginTop: 12 }}>
            {contextRules.length === 0 && <div className="text-meta">No context rules for this project.</div>}
            {contextRules.map(rule => (
              <div key={rule.id} style={{ borderBottom: "1px solid var(--hairline-dark)", padding: "8px 0" }}>
                <div className="chapter-title">{rule.name}</div>
                <div className="text-meta">priority {rule.priority}, budget {rule.token_budget}, source {rule.source_type}, {rule.enabled ? "enabled" : "disabled"}</div>
                <div className="text-meta">{rule.primary_keywords.join(", ")}</div>
              </div>
            ))}
          </div>
        </section>

        <section className="card">
          <h3 className="section-title">Draft Alternatives</h3>
          <div className="bible-edit-field">
            <label>Selection Reason</label>
            <input className="text-input" value={selectionReason} onChange={e => setSelectionReason(e.target.value)} />
          </div>
          {draftCandidates.length === 0 && <div className="text-meta">No draft candidates for the selected plan.</div>}
          {draftCandidates.map(candidate => (
            <div key={candidate.id} style={{ borderBottom: "1px solid var(--hairline-dark)", padding: "10px 0" }}>
              <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
                <div>
                  <div className="chapter-title">Candidate {candidate.candidate_number}: {candidate.title}</div>
                  <div className="text-meta">{candidate.word_count} words, status {candidate.status}</div>
                </div>
                <button className="btn btn-secondary btn-sm" onClick={() => selectCandidate(candidate.id)} disabled={candidate.status === "selected"}>Select</button>
              </div>
              {candidate.summary && <div className="text-body" style={{ marginTop: 6 }}>{candidate.summary}</div>}
            </div>
          ))}
        </section>

        <section className="card">
          <h3 className="section-title">Project Package</h3>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12 }}>
            <button className="btn btn-secondary btn-sm" onClick={exportProjectPackage} disabled={!selected}>Export Project</button>
            <button className="btn btn-primary btn-sm" onClick={importProjectPackage} disabled={!projectPackageJson.trim()}>Import As New Project</button>
          </div>
          <div className="bible-edit-field">
            <label>Project Package JSON</label>
            <textarea value={projectPackageJson} onChange={e => setProjectPackageJson(e.target.value)} placeholder="Export a project or paste a package here." />
          </div>
        </section>

        <section className="card">
          <h3 className="section-title">Novel Bible Package</h3>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12 }}>
            <button className="btn btn-secondary btn-sm" onClick={exportNovelBible} disabled={!selected}>Export Current Bible</button>
            <button className="btn btn-primary btn-sm" onClick={importNovelBible} disabled={!selected || !biblePackageJson.trim()}>Import Into Current Project</button>
          </div>
          <div className="bible-edit-field">
            <label>Package JSON</label>
            <textarea value={biblePackageJson} onChange={e => setBiblePackageJson(e.target.value)} placeholder="Export a package or paste one here." />
          </div>
        </section>

        <section className="card">
          <h3 className="section-title">Prompt Presets</h3>
          <div className="bible-edit-field"><label>Name</label><input className="text-input" value={promptPresetForm.name} onChange={e => setPromptPresetForm(prev => ({ ...prev, name: e.target.value }))} /></div>
          <div className="bible-edit-field"><label>Scope</label><input className="text-input" value={promptPresetForm.scope} onChange={e => setPromptPresetForm(prev => ({ ...prev, scope: e.target.value }))} /></div>
          <div className="bible-edit-field"><label>Description</label><input className="text-input" value={promptPresetForm.description} onChange={e => setPromptPresetForm(prev => ({ ...prev, description: e.target.value }))} /></div>
          <button className="btn btn-primary btn-sm" onClick={createPromptPreset} disabled={!promptPresetForm.name.trim()}>Create Preset</button>
          <div style={{ borderTop: "1px solid var(--hairline-dark)", paddingTop: 12, marginTop: 12 }}>
            <div className="chapter-title">Prompt Unit</div>
            <div className="bible-edit-field">
              <label>Preset</label>
              <select className="select" value={promptUnitForm.preset_id} onChange={e => setPromptUnitForm(prev => ({ ...prev, preset_id: e.target.value }))}>
                <option value="">Select a preset</option>
                {promptPresets.map(preset => <option key={preset.id} value={preset.id}>{preset.name}</option>)}
              </select>
            </div>
            <div className="bible-edit-field"><label>Identifier</label><input className="text-input" value={promptUnitForm.identifier} onChange={e => setPromptUnitForm(prev => ({ ...prev, identifier: e.target.value }))} /></div>
            <div className="mini-grid">
              <div className="bible-edit-field"><label>Role</label><input className="text-input" value={promptUnitForm.role} onChange={e => setPromptUnitForm(prev => ({ ...prev, role: e.target.value }))} /></div>
              <div className="bible-edit-field"><label>Order</label><input className="text-input" type="number" value={promptUnitForm.order} onChange={e => setPromptUnitForm(prev => ({ ...prev, order: Number(e.target.value) }))} /></div>
              <div className="bible-edit-field"><label>Injection Position</label><input className="text-input" value={promptUnitForm.injection_position} onChange={e => setPromptUnitForm(prev => ({ ...prev, injection_position: e.target.value }))} /></div>
              <div className="bible-edit-field"><label>Generation Phase</label><input className="text-input" value={promptUnitForm.generation_phase} onChange={e => setPromptUnitForm(prev => ({ ...prev, generation_phase: e.target.value }))} /></div>
            </div>
            <div className="bible-edit-field">
              <label>Content</label>
              <textarea value={promptUnitForm.content} onChange={e => setPromptUnitForm(prev => ({ ...prev, content: e.target.value }))} />
            </div>
            <label className="checkbox-row" style={{ marginBottom: 10 }}>
              <input type="checkbox" checked={promptUnitForm.enabled} onChange={e => setPromptUnitForm(prev => ({ ...prev, enabled: e.target.checked }))} />
              Enabled
            </label>
            <button className="btn btn-secondary btn-sm" onClick={savePromptUnit} disabled={!promptUnitForm.preset_id || !promptUnitForm.identifier.trim() || !promptUnitForm.content.trim()}>Save Prompt Unit</button>
          </div>
          <div style={{ marginTop: 12 }}>
            {promptPresets.length === 0 && <div className="text-meta">No prompt presets saved.</div>}
            {promptPresets.map(preset => (
              <div key={preset.id} style={{ borderBottom: "1px solid var(--hairline-dark)", padding: "8px 0" }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
                  <div>
                    <div className="chapter-title">{preset.name}</div>
                    <div className="text-meta">{preset.scope}{preset.is_builtin ? ", built in" : ""}</div>
                  </div>
                  <button className="btn btn-secondary btn-sm" onClick={() => exportPromptPreset(preset.id)}>Export</button>
                </div>
              </div>
            ))}
          </div>
          <div className="bible-edit-field" style={{ marginTop: 12 }}>
            <label>Preset Package JSON</label>
            <textarea value={promptPackageJson} onChange={e => setPromptPackageJson(e.target.value)} placeholder="Export a preset or paste a package here." />
          </div>
          <button className="btn btn-secondary btn-sm" onClick={importPromptPreset} disabled={!promptPackageJson.trim()}>Import Preset Package</button>
        </section>

        <section className="card">
          <h3 className="section-title">Provider Profile</h3>
          <div style={{ marginBottom: 12 }}>
            {modelProfiles.length === 0 && <div className="text-meta">No saved model profiles.</div>}
            {modelProfiles.map(profile => (
              <button key={profile.id} className="btn btn-secondary btn-sm" style={{ margin: "0 8px 8px 0" }} onClick={() => loadModelProfileIntoForm(profile.id)}>
                {profile.name} ({profile.intended_use})
              </button>
            ))}
          </div>
          <div className="bible-edit-field">
            <label>Workflow Binding</label>
            <select className="select" value={profileWorkflow} onChange={e => setProfileWorkflow(e.target.value)}>
              <option value="draft">Draft</option>
              <option value="review">Review</option>
              <option value="repair">Repair</option>
              <option value="embedding">Embedding</option>
              <option value="summarization">Summarization</option>
            </select>
            <div className="text-meta" style={{ marginTop: 6 }}>Current: {currentProfileBinding(profileWorkflow) || "none"}</div>
          </div>
          <button className="btn btn-secondary btn-sm" onClick={bindModelProfile} disabled={!profileId}>Bind Selected Profile</button>
          <div className="bible-edit-field"><label>Name</label><input className="text-input" value={profileForm.name} onChange={e => setProfileForm(prev => ({ ...prev, name: e.target.value }))} /></div>
          <div className="bible-edit-field"><label>Provider</label><input className="text-input" value={profileForm.provider} onChange={e => setProfileForm(prev => ({ ...prev, provider: e.target.value }))} /></div>
          <div className="bible-edit-field"><label>Base URL</label><input className="text-input" value={profileForm.base_url} onChange={e => setProfileForm(prev => ({ ...prev, base_url: e.target.value }))} /></div>
          <div className="bible-edit-field"><label>Model</label><input className="text-input" value={profileForm.model} onChange={e => setProfileForm(prev => ({ ...prev, model: e.target.value }))} /></div>
          <div className="bible-edit-field"><label>Context Window</label><input className="text-input" type="number" value={profileForm.context_window} onChange={e => setProfileForm(prev => ({ ...prev, context_window: Number(e.target.value) || 0 }))} /></div>
          <div style={{ display: "flex", gap: 12, flexWrap: "wrap", marginBottom: 12 }}>
            <label className="text-meta"><input type="checkbox" checked={profileForm.supports_json} onChange={e => setProfileForm(prev => ({ ...prev, supports_json: e.target.checked }))} /> JSON</label>
            <label className="text-meta"><input type="checkbox" checked={profileForm.supports_streaming} onChange={e => setProfileForm(prev => ({ ...prev, supports_streaming: e.target.checked }))} /> Streaming</label>
            <label className="text-meta"><input type="checkbox" checked={profileForm.supports_embeddings} onChange={e => setProfileForm(prev => ({ ...prev, supports_embeddings: e.target.checked }))} /> Embeddings</label>
          </div>
          <button className="btn btn-primary btn-sm" onClick={saveModelProfile}>Save & Validate</button>
          {profileWarnings.length > 0 && (
            <div className="content-preview" style={{ marginTop: 12 }}>
              {profileWarnings.map((warning, index) => <div key={index}>{warning.severity}: {warning.message}</div>)}
            </div>
          )}
        </section>

        <section className="card">
          <h3 className="section-title">Extension Manifest</h3>
          <div className="bible-edit-field">
            <label>Manifest JSON</label>
            <textarea value={extensionJson} onChange={e => setExtensionJson(e.target.value)} />
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <button className="btn btn-secondary btn-sm" onClick={validateExtension}>Validate Manifest</button>
            <button className="btn btn-primary btn-sm" onClick={importExtensionPackage}>Import Extension Package</button>
          </div>
          <div style={{ borderTop: "1px solid var(--hairline-dark)", paddingTop: 12, marginTop: 12 }}>
            <div className="chapter-title">Installed Extensions</div>
            {extensionPackages.length === 0 && <div className="text-meta" style={{ marginTop: 8 }}>No extensions installed.</div>}
            {extensionPackages.map(pkg => (
              <div key={pkg.manifest.id} style={{ borderBottom: "1px solid var(--hairline-dark)", padding: "8px 0" }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
                  <div>
                    <div className="chapter-title">{pkg.manifest.name}</div>
                    <div className="text-meta">{pkg.manifest.id} v{pkg.manifest.version}, {pkg.enabled ? "enabled" : "disabled"}</div>
                    <div className="text-meta">{pkg.manifest.package_kinds.join(", ")}</div>
                  </div>
                  <button className="btn btn-secondary btn-sm" onClick={() => toggleExtensionPackage(pkg.manifest.id, !pkg.enabled)}>
                    {pkg.enabled ? "Disable" : "Enable"}
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>
      </div>
    </>
  );
}
