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
  assert.ok(runtimeSource.includes("手动上下文规则"), "missing manual context rule UI heading");
  assert.ok(runtimeSource.includes("主关键词"), "missing primary keywords input");
  assert.ok(runtimeSource.includes("辅助关键词"), "missing secondary keywords input");
  assert.ok(runtimeSource.includes("Token 预算"), "missing token budget input");
});

test("runtime page exposes prompt unit editing", () => {
  assert.ok(runtimeSource.includes("promptUnitForm"), "missing prompt unit form state");
  assert.ok(runtimeSource.includes("upsertPromptUnit"), "missing prompt unit save action");
  assert.ok(runtimeSource.includes("提示词单元"), "missing prompt unit UI heading");
  assert.ok(runtimeSource.includes("标识符"), "missing prompt unit identifier input");
  assert.ok(runtimeSource.includes("生成阶段"), "missing prompt unit generation phase input");
  assert.ok(runtimeSource.includes("注入位置"), "missing prompt unit injection position input");
});

test("runtime page exposes extension package management", () => {
  assert.ok(runtimeSource.includes("extensionPackages"), "missing extension package list state");
  assert.ok(runtimeSource.includes("importExtensionPackage"), "missing extension import action");
  assert.ok(runtimeSource.includes("setExtensionEnabled"), "missing extension enable action");
  assert.ok(runtimeSource.includes("导入扩展包"), "missing extension import button");
  assert.ok(runtimeSource.includes("已安装扩展"), "missing installed extensions list");
});

test("runtime page validates model profiles against selected workflow", () => {
  assert.ok(runtimeSource.includes("profileWorkflowToModelWorkflow"), "missing workflow validation mapper");
  assert.ok(runtimeSource.includes("validateModelProfile<any[]>(id, profileWorkflowToModelWorkflow(profileWorkflow))"), "save flow should validate selected workflow");
  assert.ok(runtimeSource.includes("validateModelProfile<any[]>(profile.id, profileWorkflowToModelWorkflow(profileWorkflow))"), "load flow should validate selected workflow");
  assert.ok(!runtimeSource.includes('validateModelProfile<any[]>(id, "Draft")'), "save flow must not hard-code Draft validation");
  assert.ok(!runtimeSource.includes('validateModelProfile<any[]>(profile.id, "Draft")'), "load flow must not hard-code Draft validation");
});

test("runtime page is organized as a five phase workflow console", () => {
  for (const phase of ["准备", "上下文", "生成", "审阅", "交付"]) {
    assert.ok(runtimeSource.includes(phase), `missing workflow phase ${phase}`);
  }
  assert.match(runtimeSource, /runtimePhases/);
  assert.match(runtimeSource, /activePhase/);
  assert.match(runtimeSource, /className="runtime-console"/);
  assert.match(runtimeSource, /className="runtime-taskbar"/);
  assert.match(runtimeSource, /getRagHealth/);
  assert.match(runtimeSource, /workflowBindings/);
  assert.doesNotMatch(runtimeSource, /className="status-grid" style=\{\{ gridTemplateColumns: "repeat\(auto-fit, minmax\(320px, 1fr\)\)"/);
});

test("runtime feature entry points are grouped by workflow responsibility", () => {
  const expectedGroups = [
    "模型路由",
    "RAG 健康",
    "手动上下文规则",
    "SillyTavern Lorebook JSON",
    "操作配方",
    "提示词预设",
    "草稿候选",
    "项目包",
    "小说圣经包",
    "扩展 Manifest",
  ];
  for (const group of expectedGroups) {
    assert.ok(runtimeSource.includes(group), `missing grouped capability ${group}`);
  }
  assert.equal((runtimeSource.match(/扩展 Manifest/g) || []).length, 1);
  assert.equal((runtimeSource.match(/项目包/g) || []).length, 1);
});
