import { existsSync, readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const source = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
const runtimePagePath = new URL("./pages/RuntimePage.tsx", import.meta.url);
const runtimeSource = existsSync(runtimePagePath) ? readFileSync(runtimePagePath, "utf8") : "";

test("runtime page is localized in a page module", () => {
  assert.ok(existsSync(runtimePagePath), "RuntimePage should live in src/pages/RuntimePage.tsx");
  assert.ok(source.includes('import { RuntimePage } from "./pages/RuntimePage";'), "App should import RuntimePage page module");
  assert.ok(!source.includes("function RuntimePage()"), "RuntimePage implementation should not remain in App.tsx");
});

test("runtime page exposes manual context rule creation", () => {
  assert.ok(runtimeSource.includes("manualRuleForm"), "missing manual context rule form state");
  assert.ok(runtimeSource.includes("upsertContextRule"), "missing upsert context rule action");
  assert.ok(runtimeSource.includes("Manual Context Rule"), "missing manual context rule UI heading");
  assert.ok(runtimeSource.includes("Primary Keywords"), "missing primary keywords input");
  assert.ok(runtimeSource.includes("Secondary Keywords"), "missing secondary keywords input");
  assert.ok(runtimeSource.includes("Token Budget"), "missing token budget input");
});

test("runtime page exposes prompt unit editing", () => {
  assert.ok(runtimeSource.includes("promptUnitForm"), "missing prompt unit form state");
  assert.ok(runtimeSource.includes("upsertPromptUnit"), "missing prompt unit save action");
  assert.ok(runtimeSource.includes("Prompt Unit"), "missing prompt unit UI heading");
  assert.ok(runtimeSource.includes("Identifier"), "missing prompt unit identifier input");
  assert.ok(runtimeSource.includes("Generation Phase"), "missing prompt unit generation phase input");
  assert.ok(runtimeSource.includes("Injection Position"), "missing prompt unit injection position input");
});

test("runtime page exposes extension package management", () => {
  assert.ok(runtimeSource.includes("extensionPackages"), "missing extension package list state");
  assert.ok(runtimeSource.includes("importExtensionPackage"), "missing extension import action");
  assert.ok(runtimeSource.includes("setExtensionEnabled"), "missing extension enable action");
  assert.ok(runtimeSource.includes("Import Extension Package"), "missing extension import button");
  assert.ok(runtimeSource.includes("Installed Extensions"), "missing installed extensions list");
});

test("runtime page validates model profiles against selected workflow", () => {
  assert.ok(runtimeSource.includes("profileWorkflowToModelWorkflow"), "missing workflow validation mapper");
  assert.ok(runtimeSource.includes("validateModelProfile<any[]>(id, profileWorkflowToModelWorkflow(profileWorkflow))"), "save flow should validate selected workflow");
  assert.ok(runtimeSource.includes("validateModelProfile<any[]>(profile.id, profileWorkflowToModelWorkflow(profileWorkflow))"), "load flow should validate selected workflow");
  assert.ok(!runtimeSource.includes('validateModelProfile<any[]>(id, "Draft")'), "save flow must not hard-code Draft validation");
  assert.ok(!runtimeSource.includes('validateModelProfile<any[]>(profile.id, "Draft")'), "load flow must not hard-code Draft validation");
});
