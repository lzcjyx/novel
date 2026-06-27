import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const source = readFileSync(new URL("./tauriClient.ts", import.meta.url), "utf8");

const expectedMethods = [
  "getContextRules",
  "generateDirectionCandidates",
  "listDirectionCandidates",
  "selectDirectionCandidate",
  "getDirectorBootstrapHandoff",
  "upsertHardFact",
  "listHardFacts",
  "upsertStyleAsset",
  "listStyleAssets",
  "getAuthorMemoryBanks",
  "upsertUserRecipe",
  "listUserRecipes",
  "createFeedbackRevisionCandidate",
  "listFeedbackDecisions",
  "decideFeedbackRevision",
  "writeRunArtifacts",
  "upsertContextRule",
  "importSillyTavernLorebook",
  "exportNovelBiblePackage",
  "importNovelBiblePackage",
  "exportProjectPackage",
  "importProjectPackage",
  "upsertPromptPreset",
  "upsertPromptUnit",
  "listPromptPresets",
  "getPromptPresetPackage",
  "importPromptPresetPackage",
  "upsertModelProfile",
  "getModelProfile",
  "listModelProfiles",
  "setWorkflowModelProfile",
  "validateModelProfile",
  "getBuiltinOperatorRecipes",
  "runOperatorRecipe",
  "getDraftCandidates",
  "selectDraftCandidate",
  "validateExtensionManifest",
  "importExtensionPackage",
  "listExtensionPackages",
  "setExtensionEnabled",
];

const expectedCommands = [
  "get_context_rules",
  "generate_direction_candidates",
  "list_direction_candidates",
  "select_direction_candidate",
  "get_director_bootstrap_handoff",
  "upsert_hard_fact",
  "list_hard_facts",
  "upsert_style_asset",
  "list_style_assets",
  "get_author_memory_banks",
  "upsert_user_recipe",
  "list_user_recipes",
  "create_feedback_revision_candidate",
  "list_feedback_decisions",
  "decide_feedback_revision",
  "write_run_artifacts",
  "export_audit_sidecar",
  "create_context_compression_summary",
  "set_context_compression_status",
  "list_context_compression_summaries",
  "upsert_context_rule",
  "import_sillytavern_lorebook",
  "export_novel_bible_package",
  "import_novel_bible_package",
  "export_project_package",
  "import_project_package",
  "upsert_prompt_preset",
  "upsert_prompt_unit",
  "list_prompt_presets",
  "get_prompt_preset_package",
  "import_prompt_preset_package",
  "create_prompt_preset_snapshot",
  "list_prompt_preset_snapshots",
  "clone_prompt_preset",
  "dry_run_prompt_preset",
  "upsert_model_profile",
  "get_model_profile",
  "list_model_profiles",
  "set_workflow_model_profile",
  "validate_model_profile",
  "get_builtin_operator_recipes",
  "run_operator_recipe",
  "get_draft_candidates",
  "select_draft_candidate",
  "validate_extension_manifest",
  "import_extension_package",
  "list_extension_packages",
  "set_extension_enabled",
];

test("runtime commands are exposed through the typed Tauri client", () => {
  for (const method of expectedMethods) {
    assert.match(source, new RegExp(`${method}(?:<[^>]+>)?\\s*\\(`), `missing method ${method}`);
  }
  for (const command of expectedCommands) {
    assert.ok(source.includes(`"${command}"`), `missing command ${command}`);
  }
});

test("tauri client exposes RAG health command", () => {
  assert.match(source, /getRagHealth<T>\(projectId: string\)/);
  assert.match(source, /invoke<T>\("get_rag_health"/);
});

test("tauri client exposes desktop pet window commands", () => {
  assert.match(source, /showPetWindow\(\): Promise<void>/);
  assert.match(source, /hidePetWindow\(\): Promise<void>/);
  assert.match(source, /showMainWindow\(\): Promise<void>/);
  assert.match(source, /invoke<void>\("show_pet_window"/);
  assert.match(source, /invoke<void>\("hide_pet_window"/);
  assert.match(source, /invoke<void>\("show_main_window"/);
});
