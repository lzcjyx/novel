import {
  useEffect,
  useMemo,
  useState,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { emitTo, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { tauriClient } from "../lib/tauriClient";

interface PetStatus {
  selected: string;
  projectName?: string | null;
  loading: boolean;
  running: boolean;
  message: string;
  ragState: string;
  animationLevel: string;
  compact: boolean;
}

const defaultStatus: PetStatus = {
  selected: "",
  projectName: null,
  loading: false,
  running: false,
  message: "",
  ragState: "unknown",
  animationLevel: "subtle",
  compact: false,
};

const stateCopy: Record<string, [string, string]> = {
  waiting: ["等待项目", "选一个项目开始写作"],
  working: ["正在工作", "生成流程运行中"],
  idle: ["待命", "可以继续推进章节"],
  attention: ["需要查看", "刚刚有错误或提示"],
  context: ["上下文受限", "RAG 向量检索未开启"],
};

export function PetWindow() {
  const [status, setStatus] = useState<PetStatus>(defaultStatus);
  const [expanded, setExpanded] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    document.body.classList.add("pet-body");
    return () => document.body.classList.remove("pet-body");
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listen<PetStatus>("pet-status", (event) => {
      setStatus({ ...defaultStatus, ...event.payload });
    }).then((cleanup) => {
      unlisten = cleanup;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  useEffect(() => {
    if (!expanded && !menuOpen) return;
    const timeout = window.setTimeout(() => {
      setExpanded(false);
      setMenuOpen(false);
    }, 6500);
    return () => window.clearTimeout(timeout);
  }, [expanded, menuOpen]);

  const petState = useMemo(() => {
    const hasError = status.message.includes("错误") || status.message.toLowerCase().includes("error");
    if (!status.selected) return "waiting";
    if (hasError) return "attention";
    if (status.loading || status.running) return "working";
    if (status.ragState === "disabled" || status.ragState === "empty" || status.ragState === "stale") return "context";
    return "idle";
  }, [status]);

  const animationLevel = ["static", "subtle", "lively"].includes(status.animationLevel)
    ? status.animationLevel
    : "subtle";
  const copy = stateCopy[petState] || stateCopy.idle;

  const persistPosition = async () => {
    try {
      const position = await getCurrentWindow().outerPosition();
      await tauriClient.savePetPosition(position.x, position.y);
    } catch {
      // Position persistence is best-effort; dragging must never block the pet.
    }
  };

  const handlePointerDown = async (event: ReactPointerEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    setMenuOpen(false);
    await getCurrentWindow().startDragging();
  };

  const handleDoubleClick = async () => {
    await tauriClient.showMainWindow();
  };

  const handleContextMenu = (event: ReactMouseEvent) => {
    event.preventDefault();
    setExpanded(true);
    setMenuOpen(true);
  };

  const openSettings = async () => {
    await emitTo("main", "open-settings", {});
    await tauriClient.showMainWindow();
  };

  return (
    <main
      className={`pet-window pet-window-${petState} pet-window-${animationLevel} ${status.compact ? "pet-window-compact" : ""}`}
      aria-label={`桌面宠物：${copy[0]}`}
      onContextMenu={handleContextMenu}
      onPointerUp={persistPosition}
    >
      <button
        type="button"
        className="pet-face"
        aria-label={copy[0]}
        onPointerDown={handlePointerDown}
        onClick={() => setExpanded((value) => !value)}
        onDoubleClick={handleDoubleClick}
      >
        <span className="pet-ear pet-ear-left" aria-hidden="true" />
        <span className="pet-ear pet-ear-right" aria-hidden="true" />
        <span className="pet-eye pet-eye-left" aria-hidden="true" />
        <span className="pet-eye pet-eye-right" aria-hidden="true" />
        <span className="pet-mouth" aria-hidden="true" />
      </button>

      {expanded && !status.compact && (
        <section className="pet-bubble" aria-live="polite">
          <strong>{copy[0]}</strong>
          <span>{status.projectName || "未选择项目"}</span>
          <span>{status.message || copy[1]}</span>
          <span>RAG: {status.ragState}</span>
        </section>
      )}

      {menuOpen && (
        <nav className="pet-menu" aria-label="桌宠菜单">
          <button type="button" onClick={handleDoubleClick}>打开主窗口</button>
          <button type="button" onClick={openSettings}>打开设置</button>
          <button type="button" onClick={() => tauriClient.hidePetWindow()}>隐藏宠物</button>
        </nav>
      )}
    </main>
  );
}
