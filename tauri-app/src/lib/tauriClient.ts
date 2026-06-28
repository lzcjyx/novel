import { invoke } from "@tauri-apps/api/core";

export interface OperatorRecipeRunRequest {
  project_id: string;
  chapter_plan_id: string;
  recipe_id: string;
}

export const tauriClient = {
  getNextChapterContextPreview<T>(projectId: string, operatorControls: unknown): Promise<T> {
    return invoke<T>("get_next_chapter_context_preview", {
      projectId,
      operatorControls,
    });
  },

  getRagHealth<T>(projectId: string): Promise<T> {
    return invoke<T>("get_rag_health", { projectId });
  },

  generateNextChapter<T>(
    projectId: string,
    force: boolean,
    operatorControls: unknown,
  ): Promise<T> {
    return invoke<T>("generate_next_chapter", {
      projectId,
      force,
      operatorControls,
    });
  },

  runWeeklyArcPlanner<T>(projectId: string): Promise<T> {
    return invoke<T>("run_weekly_arc_planner", { projectId });
  },

  getChapterPlans<T>(projectId: string): Promise<T> {
    return invoke<T>("get_chapter_plans", { projectId });
  },

  resetRunning(projectId: string): Promise<void> {
    return invoke<void>("reset_running", { projectId });
  },

  showPetWindow(): Promise<void> {
    return invoke<void>("show_pet_window");
  },

  hidePetWindow(): Promise<void> {
    return invoke<void>("hide_pet_window");
  },

  showMainWindow(): Promise<void> {
    return invoke<void>("show_main_window");
  },

  savePetPosition(x: number, y: number): Promise<void> {
    return invoke<void>("save_pet_position", { x, y });
  },

  listDuePublications<T>(): Promise<T> {
    return invoke<T>("list_due_publications");
  },

  processDuePublications<T>(): Promise<T> {
    return invoke<T>("process_due_publications");
  },

  retryPublication(publicationId: string): Promise<void> {
    return invoke<void>("retry_publication", { publicationId });
  },

  getContextRules<T>(projectId: string): Promise<T> {
    return invoke<T>("get_context_rules", { projectId });
  },

  generateDirectionCandidates<T>(
    projectId: string | null,
    inspiration: string,
    candidateCount: number,
  ): Promise<T> {
    return invoke<T>("generate_direction_candidates", {
      projectId,
      inspiration,
      candidateCount,
    });
  },

  listDirectionCandidates<T>(projectId: string | null): Promise<T> {
    return invoke<T>("list_direction_candidates", { projectId });
  },

  selectDirectionCandidate<T>(candidateId: string, revisionNote: string | null): Promise<T> {
    return invoke<T>("select_direction_candidate", { candidateId, revisionNote });
  },

  getDirectorBootstrapHandoff<T>(candidateId: string): Promise<T> {
    return invoke<T>("get_director_bootstrap_handoff", { candidateId });
  },

  upsertHardFact(input: unknown): Promise<string> {
    return invoke<string>("upsert_hard_fact", { input });
  },

  listHardFacts<T>(projectId: string, activeOnly: boolean): Promise<T> {
    return invoke<T>("list_hard_facts", { projectId, activeOnly });
  },

  upsertStyleAsset(input: unknown): Promise<string> {
    return invoke<string>("upsert_style_asset", { input });
  },

  listStyleAssets<T>(projectId: string, enabledOnly: boolean): Promise<T> {
    return invoke<T>("list_style_assets", { projectId, enabledOnly });
  },

  getAuthorMemoryBanks<T>(projectId: string): Promise<T> {
    return invoke<T>("get_author_memory_banks", { projectId });
  },

  upsertUserRecipe(input: unknown): Promise<string> {
    return invoke<string>("upsert_user_recipe", { input });
  },

  listUserRecipes<T>(projectId: string, enabledOnly: boolean): Promise<T> {
    return invoke<T>("list_user_recipes", { projectId, enabledOnly });
  },

  createFeedbackRevisionCandidate(input: unknown): Promise<string> {
    return invoke<string>("create_feedback_revision_candidate", { input });
  },

  listFeedbackDecisions<T>(projectId: string): Promise<T> {
    return invoke<T>("list_feedback_decisions", { projectId });
  },

  decideFeedbackRevision<T>(
    decisionId: string,
    action: string,
    decisionNote: string | null,
  ): Promise<T> {
    return invoke<T>("decide_feedback_revision", { decisionId, action, decisionNote });
  },

  writeRunArtifacts<T>(jobId: string, baseDir: string, payload: unknown): Promise<T> {
    return invoke<T>("write_run_artifacts", { jobId, baseDir, payload });
  },

  exportAuditSidecar<T>(projectId: string, baseDir: string): Promise<T> {
    return invoke<T>("export_audit_sidecar", { projectId, baseDir });
  },

  createContextCompressionSummary(input: unknown): Promise<string> {
    return invoke<string>("create_context_compression_summary", { input });
  },

  setContextCompressionStatus(summaryId: string, status: string): Promise<void> {
    return invoke<void>("set_context_compression_status", { summaryId, status });
  },

  listContextCompressionSummaries<T>(projectId: string, approvedOnly: boolean): Promise<T> {
    return invoke<T>("list_context_compression_summaries", { projectId, approvedOnly });
  },

  upsertContextRule(input: unknown): Promise<string> {
    return invoke<string>("upsert_context_rule", { input });
  },

  importSillyTavernLorebook<T>(projectId: string, lorebookJson: string): Promise<T> {
    return invoke<T>("import_sillytavern_lorebook", { projectId, lorebookJson });
  },

  exportNovelBiblePackage<T>(projectId: string): Promise<T> {
    return invoke<T>("export_novel_bible_package", { projectId });
  },

  importNovelBiblePackage<T>(projectId: string, packagePayload: unknown): Promise<T> {
    return invoke<T>("import_novel_bible_package", { projectId, package: packagePayload });
  },

  exportProjectPackage<T>(projectId: string): Promise<T> {
    return invoke<T>("export_project_package", { projectId });
  },

  importProjectPackage(packagePayload: unknown): Promise<string> {
    return invoke<string>("import_project_package", { package: packagePayload });
  },

  upsertPromptPreset(input: unknown): Promise<string> {
    return invoke<string>("upsert_prompt_preset", { input });
  },

  upsertPromptUnit(input: unknown): Promise<string> {
    return invoke<string>("upsert_prompt_unit", { input });
  },

  listPromptPresets<T>(): Promise<T> {
    return invoke<T>("list_prompt_presets");
  },

  getPromptPresetPackage<T>(presetId: string): Promise<T> {
    return invoke<T>("get_prompt_preset_package", { presetId });
  },

  importPromptPresetPackage(packagePayload: unknown): Promise<string> {
    return invoke<string>("import_prompt_preset_package", { package: packagePayload });
  },

  createPromptPresetSnapshot<T>(presetId: string, note?: string | null): Promise<T> {
    return invoke<T>("create_prompt_preset_snapshot", { presetId, note });
  },

  listPromptPresetSnapshots<T>(presetId: string): Promise<T> {
    return invoke<T>("list_prompt_preset_snapshots", { presetId });
  },

  clonePromptPreset(sourcePresetId: string, newName: string, newId?: string | null): Promise<string> {
    return invoke<string>("clone_prompt_preset", { sourcePresetId, newId, newName });
  },

  dryRunPromptPreset<T>(
    presetId: string,
    generationPhase: string,
    temporaryOverrides: Record<string, string>,
  ): Promise<T> {
    return invoke<T>("dry_run_prompt_preset", { presetId, generationPhase, temporaryOverrides });
  },

  upsertModelProfile(input: unknown): Promise<string> {
    return invoke<string>("upsert_model_profile", { input });
  },

  getModelProfile<T>(profileId: string): Promise<T> {
    return invoke<T>("get_model_profile", { profileId });
  },

  listModelProfiles<T>(): Promise<T> {
    return invoke<T>("list_model_profiles");
  },

  setWorkflowModelProfile<T>(workflow: string, profileId: string | null): Promise<T> {
    return invoke<T>("set_workflow_model_profile", { workflow, profileId });
  },

  validateModelProfile<T>(profileId: string, workflow: string): Promise<T> {
    return invoke<T>("validate_model_profile", { profileId, workflow });
  },

  getBuiltinOperatorRecipes<T>(): Promise<T> {
    return invoke<T>("get_builtin_operator_recipes");
  },

  runOperatorRecipe<T>(request: OperatorRecipeRunRequest): Promise<T> {
    return invoke<T>("run_operator_recipe", { request });
  },

  getDraftCandidates<T>(chapterPlanId: string): Promise<T> {
    return invoke<T>("get_draft_candidates", { chapterPlanId });
  },

  selectDraftCandidate(candidateId: string, selectionReason: string): Promise<void> {
    return invoke<void>("select_draft_candidate", { candidateId, selectionReason });
  },

  validateExtensionManifest(manifest: unknown): Promise<void> {
    return invoke<void>("validate_extension_manifest", { manifest });
  },

  importExtensionPackage(packagePayload: unknown): Promise<string> {
    return invoke<string>("import_extension_package", { package: packagePayload });
  },

  listExtensionPackages<T>(): Promise<T> {
    return invoke<T>("list_extension_packages");
  },

  setExtensionEnabled(extensionId: string, enabled: boolean): Promise<void> {
    return invoke<void>("set_extension_enabled", { extensionId, enabled });
  },
};
