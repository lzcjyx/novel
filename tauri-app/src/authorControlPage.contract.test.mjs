import { existsSync, readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const appSource = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
const pagePath = new URL("./pages/AuthorControlPage.tsx", import.meta.url);
const pageSource = existsSync(pagePath) ? readFileSync(pagePath, "utf8") : "";

test("author control page is localized in a page module", () => {
  assert.ok(existsSync(pagePath), "AuthorControlPage should live in src/pages/AuthorControlPage.tsx");
  assert.ok(
    appSource.includes('import { AuthorControlPage } from "./pages/AuthorControlPage";'),
    "App should import AuthorControlPage page module",
  );
  assert.ok(!appSource.includes("function AuthorControlPage("), "AuthorControlPage implementation should not live in App.tsx");
});

test("author control navigation is wired through App", () => {
  assert.ok(appSource.includes("OrchestrateAgentPage"), "missing orchestrator surface");
  assert.ok(appSource.includes('{ id: "author", label: "作者控制" }'), "missing internal Author Control tab");
  assert.ok(appSource.includes('mode === "author" && <AuthorControlPage selected={selected} />'), "missing Author Control surface switch");
});

test("author control page exposes core author-control surfaces", () => {
  for (const label of ["总导演", "硬事实", "风格资产", "记忆库"]) {
    assert.ok(pageSource.includes(label), `missing ${label} section`);
  }
  for (const method of [
    "generateDirectionCandidates",
    "listDirectionCandidates",
    "selectDirectionCandidate",
    "getDirectorBootstrapHandoff",
    "getAuthorMemoryBanks",
  ]) {
    assert.ok(pageSource.includes(method), `missing ${method} usage`);
  }
});
