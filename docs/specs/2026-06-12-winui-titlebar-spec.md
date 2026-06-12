# WinUI 3 Custom Title Bar Spec

## Problem

The desktop shell already uses Fluent/WinUI-like colors, navigation, and command bar styling, but the Tauri window still renders the native operating-system title bar because `decorations` is enabled. This leaves the top chrome visually inconsistent with the rest of the WinUI 3 shell and prevents the application from controlling title-bar drag regions and close-to-tray behavior from the same UI layer.

## Goals

- Replace the native decorated window frame with a webview-rendered title bar.
- Match WinUI 3 desktop chrome conventions: quiet Mica-like surface, compact app identity, draggable title region, and standard minimize/maximize/close controls aligned to the right.
- Preserve existing runtime semantics: closing the main window hides it to the tray, while the tray menu item `Quit Completely` performs the real application exit and interrupted-job recovery.
- Keep the layout stable and fast: adding the title bar must not create vertical overlap, horizontal scroll, layout shift, or expensive animation.
- Make the title bar testable through source-level regression tests and the normal frontend build.

## Non-Goals

- No native WinUI dependency is introduced.
- No new icon package is added for the three window buttons; the glyphs are drawn with CSS so bundle size and startup cost stay unchanged.
- No changes are made to the backend job transaction/recovery model beyond preserving the existing close-to-tray and quit semantics.

## User Experience Requirements

- The Tauri main window must have `decorations: false`.
- The app must render a 48px custom title bar at the top of the webview.
- The title bar must use `data-tauri-drag-region` on the non-interactive area so users can drag the window from the bar and app title.
- The title bar must show the product identity as "AI Novel Factory" with a compact app mark.
- Window controls must appear in this order: minimize, maximize/restore, close to tray.
- Window controls must be semantic `<button>` elements with accessible labels:
  - `Minimize window`
  - `Maximize or restore window`
  - `Close to tray`
- Window controls must use stable 46px-wide hit targets and CSS glyphs, not emoji or text characters.
- Hover, active, and focus-visible states must be clear. The close button hover state must use the Windows destructive red treatment.
- The close control must hide the Tauri window instead of directly exiting the app.
- The existing tray menu item `Quit Completely` remains the only explicit full-exit path.

## Layout Requirements

- The root shell must reserve space for the title bar using a grid or equivalent stable layout.
- The existing navigation view must fill the remaining height under the title bar and must not keep `height: 100vh` in a way that causes overflow.
- On narrow widths, the title bar remains fixed at the top of the shell and the navigation pane adapts below it as it does today.
- Text in the title bar must not overlap the window controls; long text must truncate or wrap safely.

## Performance Requirements

- No polling or resize listeners are required for the title bar.
- No layout-reading code is introduced.
- Transitions are limited to color/background changes and use existing micro-interaction timing.
- CSS glyphs avoid additional runtime dependencies and keep the frontend bundle small.

## Acceptance Criteria

- `tauri-app/src-tauri/tauri.conf.json` sets the main window `decorations` field to `false`.
- `tauri-app/src/App.tsx` imports `getCurrentWindow` from `@tauri-apps/api/window`.
- `tauri-app/src/App.tsx` renders `.app-titlebar` with `data-tauri-drag-region`.
- `tauri-app/src/App.tsx` renders three `.window-control` buttons with the required accessible labels.
- The close button calls Tauri `hide()` to preserve close-to-tray semantics.
- `tauri-app/src/index.css` defines `.app-shell`, `.app-titlebar`, `.window-controls`, `.window-control`, and CSS glyph classes.
- `.window-control` has a stable 46px width.
- `.window-control-close:hover` uses the destructive close treatment.
- Regression tests fail before implementation and pass after implementation.
- `node --test tauri-app/src/fluentTokens.test.mjs`, `npm run build`, `cargo test`, and `git diff --check` pass before commit.
