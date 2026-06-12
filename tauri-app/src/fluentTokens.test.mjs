import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const css = readFileSync(new URL("./index.css", import.meta.url), "utf8");
const app = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
const tauriConfig = readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8");

test("fluent shell tokens replace the old dark gold shell", () => {
  assert.match(css, /--accent:\s*#0067c0/i);
  assert.match(css, /--mica-bg:/);
  assert.match(css, /--control-fill:/);
  assert.match(css, /font-family:\s*var\(--font-system\)/);
  assert.doesNotMatch(css, /--primary:\s*#d0a85c/i);
  assert.doesNotMatch(css, /--canvas-dark:\s*#111417/i);
});

test("app shell uses WinUI-like navigation and command bar classes", () => {
  assert.match(app, /className="app-navigation-view"/);
  assert.match(app, /className="app-command-bar"/);
  assert.match(app, /className="nav-icon"/);
  assert.match(app, /className="info-bar/);
});

test("learn source markers do not use emoji as structural icons", () => {
  assert.doesNotMatch(app, /🌐|🔄|📝/);
  assert.match(app, /source-marker/);
});

test("tauri window uses a custom Fluent titlebar instead of native decorations", () => {
  const config = JSON.parse(tauriConfig);
  assert.equal(config.app.windows[0].decorations, false);
  assert.match(app, /@tauri-apps\/api\/window/);
  assert.match(app, /className="app-shell"/);
  assert.match(app, /className="app-titlebar"/);
  assert.match(app, /data-tauri-drag-region/);
  assert.match(app, /aria-label="Minimize window"/);
  assert.match(app, /aria-label="Maximize or restore window"/);
  assert.match(app, /aria-label="Close to tray"/);
  assert.match(app, /\.hide\(\)/);
  assert.doesNotMatch(app, /[\u{1F300}-\u{1FAFF}]/u);
});

test("fluent titlebar CSS defines stable WinUI-style window controls", () => {
  assert.match(css, /\.app-shell/);
  assert.match(css, /grid-template-rows:\s*48px\s+minmax\(0,\s*1fr\)/);
  assert.match(css, /\.app-titlebar/);
  assert.match(css, /height:\s*48px/);
  assert.match(css, /\.window-controls/);
  assert.match(css, /\.window-control/);
  assert.match(css, /width:\s*46px/);
  assert.match(css, /\.window-control-close:hover/);
});
