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

  getContextRules<T>(projectId: string): Promise<T> {
    return invoke<T>("get_context_rules", { projectId });
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
