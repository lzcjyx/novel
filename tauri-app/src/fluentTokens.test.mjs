import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const css = readFileSync(new URL("./index.css", import.meta.url), "utf8");
const app = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
const tauriConfig = readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8");
const capabilities = JSON.parse(readFileSync(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"));
const requiredWindowPermissions = [
  "core:window:allow-start-dragging",
  "core:window:allow-minimize",
  "core:window:allow-is-maximized",
  "core:window:allow-maximize",
  "core:window:allow-unmaximize",
  "core:window:allow-hide",
];

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

test("custom titlebar has Tauri window permissions and does not make controls draggable", () => {
  const config = JSON.parse(tauriConfig);
  assert.equal(config.app.windows[0].decorations, false);
  for (const permission of requiredWindowPermissions) {
    assert.ok(capabilities.permissions.includes(permission), `${permission} permission missing`);
  }
  assert.doesNotMatch(app, /<header className="app-titlebar" data-tauri-drag-region>/);
  assert.match(app, /handleTitlebarDrag/);
  assert.match(app, /\.startDragging\(\)/);
  assert.match(app, /className="titlebar-drag-zone"[\s\S]*data-tauri-drag-region/);
  assert.doesNotMatch(app, /className="window-controls" data-tauri-drag-region/);
});

test("graph nodes render compact canvas content while preserving labels off-canvas", () => {
  assert.match(app, /const graphNodeInitial/);
  assert.match(app, /className="graph-node-initial"/);
  assert.match(app, /className="graph-node-degree"/);
  assert.doesNotMatch(app, /<span>\{node\.label\}<\/span>/);
  assert.match(app, /title=\{\`\$\{node\.label\}/);
  assert.match(app, /aria-label=\{\`\$\{node\.label\}/);
});

test("graph workbench uses light Fluent graph tokens", () => {
  assert.match(css, /\.graph-canvas\s*\{[\s\S]*var\(--surface-solid\)/);
  assert.match(css, /\.graph-edge-base\s*\{[\s\S]*rgba\(0,\s*0,\s*0,\s*0\.\d+\)/);
  assert.match(css, /\.graph-edge\.active \.graph-edge-base\s*\{[\s\S]*var\(--accent\)/);
  assert.match(css, /\.graph-node\s*\{[\s\S]*var\(--control-fill\)/);
  assert.match(css, /\.graph-node-initial/);
  assert.match(css, /\.graph-node-degree/);
  assert.doesNotMatch(css, /\.graph-edge\.active \.graph-edge-base\s*\{[\s\S]*208,\s*168,\s*92/);
});

test("desktop pet is configurable and status aware without canvas animation", () => {
  assert.doesNotMatch(app, /function AppPet/);
  assert.match(app, /PetWindow/);
  assert.match(app, /settings\.pet_enabled/);
  assert.match(app, /pet_animation_level/);
  assert.match(app, /pet_compact_mode/);
  assert.match(css, /\.pet-window/);
  assert.match(css, /\.pet-window-static/);
  assert.match(css, /prefers-reduced-motion:\s*reduce/);
  assert.doesNotMatch(app, /requestAnimationFrame\(.*pet/i);
});

test("desktop pet runs in an independent transparent Tauri window", () => {
  const config = JSON.parse(tauriConfig);
  const petWindow = config.app.windows.find((window) => window.label === "pet");
  assert.ok(petWindow, "missing pet window");
  assert.equal(petWindow.transparent, true);
  assert.equal(petWindow.decorations, false);
  assert.equal(petWindow.alwaysOnTop, true);
  assert.equal(petWindow.skipTaskbar, true);
  assert.ok(capabilities.windows.includes("pet"));
  assert.match(app, /new URLSearchParams\(window\.location\.search\)\.get\("window"\) === "pet"/);
  assert.match(app, /<PetWindow \/>/);
  assert.match(app, /emitTo\("pet", "pet-status"/);
  assert.doesNotMatch(app, /function AppPet/);
  assert.match(css, /\.pet-window/);
  assert.match(css, /\.pet-face/);
  assert.match(css, /\.pet-bubble/);
});
