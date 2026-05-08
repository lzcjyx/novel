import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Novel { id: string; name?: string; slug: string; genre?: string; status?: string; target_words?: number; chapter_count?: number; total_words?: number; plans_left?: number; chapters_today?: number; created_at?: string; }
interface Chapter { filename: string; title: string; sequence: number; size: number; modified: number; }
interface Status { ok: boolean; novel?: { name: string; genre: string; }; slug?: string; chapters_today?: number; plans_left?: number; chapter_count?: number; is_running?: boolean; daily_schedule?: string; }

const navBtn = (a: boolean): React.CSSProperties => ({ padding: "12px 20px", border: "none", background: a ? "#2a2a2a" : "transparent", color: a ? "#fff" : "#888", cursor: "pointer", fontSize: "14px", fontWeight: a ? 600 : 400, borderBottom: a ? "2px solid #6c5ce7" : "2px solid transparent" });
const btn = (bg: string): React.CSSProperties => ({ padding: "8px 16px", background: bg, color: "#fff", border: "none", borderRadius: "6px", cursor: "pointer", fontSize: "13px", fontWeight: 600, marginRight: "6px", marginBottom: "6px" });
const badge = (on: boolean): React.CSSProperties => ({ display: "inline-block", padding: "3px 8px", borderRadius: "10px", fontSize: "11px", fontWeight: 600, background: on ? "#1a3a1a" : "#2a2a2a", color: on ? "#4caf50" : "#888" });

const S: Record<string, React.CSSProperties> = {
  body: { fontFamily: "system-ui", background: "#0f0f0f", color: "#e0e0e0", minHeight: "100vh", display: "flex", flexDirection: "column" },
  nav: { display: "flex", background: "#1a1a1a", borderBottom: "1px solid #2a2a2a", padding: "0 20px", gap: 4 },
  content: { flex: 1, padding: "20px", maxWidth: "960px", margin: "0 auto", width: "100%", boxSizing: "border-box" as const, overflowY: "auto" as const, maxHeight: "calc(100vh - 50px)" },
  card: { background: "#1a1a1a", borderRadius: "8px", padding: "16px", marginBottom: "12px", border: "1px solid #2a2a2a" },
  cardSel: { background: "#1e1a30", borderRadius: "8px", padding: "16px", marginBottom: "12px", border: "1px solid #6c5ce7" },
  label: { fontSize: "11px", fontWeight: 600, color: "#888", textTransform: "uppercase" as const, letterSpacing: "0.5px", marginBottom: "2px" },
  val: { fontSize: "28px", fontWeight: 700, color: "#fff" },
  grid: { display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))", gap: "10px", marginBottom: "16px" },
  dangerBtn: { padding: "6px 12px", background: "transparent", color: "#e74c3c", border: "1px solid #e74c3c", borderRadius: "4px", cursor: "pointer", fontSize: "11px" },
  chItem: { padding: "10px 0", borderBottom: "1px solid #2a2a2a", cursor: "pointer" },
  chTitle: { fontSize: "14px", fontWeight: 600, color: "#fff" },
  chMeta: { fontSize: "11px", color: "#888", marginTop: "3px" },
  preview: { marginTop: "8px", padding: "12px", background: "#0a0a0a", borderRadius: "4px", fontSize: "13px", color: "#ccc", maxHeight: "250px", overflow: "auto", whiteSpace: "pre-wrap", fontFamily: "Georgia,serif", lineHeight: 1.7 },
  logLine: { padding: "3px 0", fontSize: "11px", color: "#aaa", fontFamily: "monospace", borderBottom: "1px solid #1a1a1a" },
  novelName: { fontSize: "16px", fontWeight: 600, color: "#fff", cursor: "pointer" },
  novelMeta: { fontSize: "12px", color: "#888" },
  progress: { marginTop: "8px", padding: "12px", borderRadius: "6px", background: "#1a2a30", border: "1px solid #2a4a6a", color: "#6cb4e4", fontSize: "13px", fontWeight: 600, textAlign: "center" as const },
};

function App() {
  const [page, setPage] = useState<"dashboard" | "novels" | "logs">("dashboard");
  const [novels, setNovels] = useState<Novel[]>([]);
  const [selected, setSelected] = useState<string>("");
  const [status, setStatus] = useState<Status | null>(null);
  const [chapters, setChapters] = useState<Chapter[]>([]);
  const [logs, setLogs] = useState<string[]>([]);
  const [openCh, setOpenCh] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [msg, setMsg] = useState("");
  const [loading, setLoading] = useState(false);
  const [progress, setProgress] = useState("");

  const loadNovels = useCallback(async () => {
    try { const n = await invoke<Novel[]>("list_novels"); setNovels(n); if (n.length && !selected) setSelected(n[0].id); } catch (e) { console.error("list_novels:", e); }
  }, [selected]);

  const loadStatus = useCallback(async () => {
    if (!selected) return;
    try { setStatus(await invoke<Status>("get_status", { novelId: selected })); } catch (e) { console.error("get_status:", e); }
  }, [selected]);

  const loadChapters = useCallback(async () => {
    const novel = novels.find(n => n.id === selected);
    if (!novel) return;
    try { setChapters(await invoke<Chapter[]>("list_chapters", { novelSlug: novel.slug })); } catch (e) { console.error("list_chapters:", e); setChapters([]); }
  }, [selected, novels]);

  const refreshLogs = useCallback(async () => {
    try { const l = await invoke<{ ok: boolean; lines: string[] }>("get_logs"); setLogs(l.lines || []); } catch (e) { console.error("get_logs:", e); }
  }, []);

  // Poll: status every 10s, novels every 30s, logs every 30s
  useEffect(() => { loadNovels(); refreshLogs(); }, []);
  useEffect(() => { if (selected) { loadStatus(); loadChapters(); } const t1 = setInterval(() => { if (selected) loadStatus(); }, 10000); const t2 = setInterval(refreshLogs, 30000); const t3 = setInterval(loadNovels, 30000); return () => { clearInterval(t1); clearInterval(t2); clearInterval(t3); }; }, [selected]);

  // Poll progress when loading
  useEffect(() => {
    if (!loading) { setProgress(""); return; }
    setProgress("Starting pipeline...");
    const dots = ["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"];
    let i = 0;
    const t = setInterval(() => { setProgress(dots[i] + " Pipeline running... (check Logs for details)"); i = (i+1)%dots.length; }, 500);
    return () => clearInterval(t);
  }, [loading]);

  const handleDaily = async () => {
    setLoading(true); setMsg("");
    try {
      const r = await invoke<{ ok: boolean; message?: string }>("trigger_daily", { novelId: selected });
      setMsg(r.message || "Success!");
      setTimeout(() => { loadStatus(); setLoading(false); }, 5000);
    } catch (e) { setMsg("Error: " + e); setLoading(false); }
  };
  const handleWeekly = async () => {
    setLoading(true); setMsg("");
    try {
      const r = await invoke<{ ok: boolean; message?: string }>("trigger_workflow", { name: "wf05" });
      setMsg(r.message || "Done!");
      setTimeout(() => { loadStatus(); setLoading(false); }, 5000);
    } catch (e) { setMsg("Error: " + e); setLoading(false); }
  };
  const handleCreate = async () => {
    const name = prompt("Enter novel name:", "My Novel");
    if (!name) return;
    setLoading(true); setMsg("");
    try {
      const r = await invoke<{ ok: boolean; message?: string }>("create_novel", { name });
      setMsg(r.message || "Created: " + name);
      setTimeout(() => { loadNovels(); setLoading(false); }, 5000);
    } catch (e) { setMsg("Error: " + e); setLoading(false); }
  };
  const handleDelete = async (id: string) => {
    const n = novels.find(x => x.id === id);
    if (!confirm("Delete \"" + (n?.name || id) + "\" and ALL its chapters?")) return;
    try { await invoke("delete_novel", { id }); setMsg("Deleted!"); if (selected === id) setSelected(""); loadNovels(); } catch (e) { setMsg("Error: " + e); }
  };
  const openChapter = async (slug: string, fn: string) => {
    setOpenCh(fn === openCh ? null : fn);
    if (fn !== openCh) try { setContent(await invoke<string>("read_chapter", { novelSlug: slug, filename: fn })); } catch (_) { setContent("Read error"); }
  };

  const selNovel = novels.find(n => n.id === selected);

  return (
    <div style={S.body}>
      <div style={S.nav}>
        {(["dashboard","novels","logs"] as const).map(p => (
          <button key={p} style={navBtn(page === p)} onClick={() => { setPage(p); if (p === "logs") refreshLogs(); }}>
            {p === "dashboard" ? "Dashboard" : p === "novels" ? "Novels" : "Logs"}
          </button>
        ))}
        <select value={selected} onChange={e => setSelected(e.target.value)}
          style={{ marginLeft: "auto", alignSelf: "center", padding: "6px 10px", background: "#1a1a1a", color: "#fff", border: "1px solid #2a2a2a", borderRadius: "4px", fontSize: "13px" }}>
          {novels.map(n => <option key={n.id} value={n.id}>{n.name}</option>)}
        </select>
      </div>

      <div style={S.content}>
        {page === "dashboard" && (
          <>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
              <h2 style={{ fontSize: 18, color: "#fff", margin: 0 }}>{selNovel?.name || "Select a novel"}</h2>
              <div><button style={btn("#6c5ce7")} onClick={handleCreate} disabled={loading}>+ New Novel</button></div>
            </div>
            {selNovel && (
              <>
                <div style={S.grid}>
                  {[{l:"Chapters",v:status?.chapter_count ?? selNovel.chapter_count ?? "?"},
                    {l:"Today",v:status?.chapters_today ?? selNovel.chapters_today ?? "?"},
                    {l:"Plans Left",v:status?.plans_left ?? selNovel.plans_left ?? "?"},
                    {l:"Total Words",v:(selNovel.total_words||0).toLocaleString()},
                    {l:"Status",v:<span style={badge(loading||(status?.is_running??false))}>{loading?"RUNNING":(status?.is_running?"RUNNING":"IDLE")}</span>},
                    {l:"Schedule",v:<span style={{fontSize:"13px"}}>{status?.daily_schedule||"?"}</span>},
                  ].map((s,i) => (
                    <div key={i} style={S.card}><div style={S.label}>{s.l}</div><div style={S.val}>{s.v}</div></div>
                  ))}
                </div>
                <button style={btn("#6c5ce7")} onClick={handleDaily} disabled={loading}>Write Chapter Now</button>
                <button style={btn("#00b894")} onClick={handleWeekly} disabled={loading}>Generate Weekly Plan</button>
                {progress && <div style={S.progress}>{progress}</div>}
                {msg && <div style={{...S.card, marginTop: 8, background: msg.startsWith("Error") ? "#2a1a1a" : "#1a2a1a", borderColor: msg.startsWith("Error") ? "#4a2a2a" : "#2a4a2a"}}>{msg}</div>}
              </>
            )}
          </>
        )}

        {page === "novels" && (
          <>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
              <h2 style={{ fontSize: 18, color: "#fff", margin: 0 }}>Novels ({novels.length})</h2>
              <div>
                <button style={btn("#555")} onClick={loadNovels}>Refresh</button>
                <button style={btn("#6c5ce7")} onClick={handleCreate} disabled={loading}>+ New Novel</button>
              </div>
            </div>
            {novels.map(n => {
              const isOpen = selected === n.id;
              return (
                <div key={n.id} style={isOpen ? S.cardSel : S.card}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <div style={{ flex: 1, cursor: "pointer" }} onClick={() => { if (isOpen) { setSelected(""); } else { setSelected(n.id); loadChapters(); } }}>
                      <div style={S.novelName}>{n.name}</div>
                      <div style={S.novelMeta}>
                        {n.genre || "No genre"} · {n.chapter_count || 0} chapters · {n.plans_left || 0} plans left · {(n.total_words||0).toLocaleString()} words · Created {n.created_at ? new Date(n.created_at).toLocaleDateString() : "?"}
                      </div>
                    </div>
                    <button style={S.dangerBtn} onClick={() => handleDelete(n.id)}>Delete</button>
                  </div>
                  {isOpen && (
                    <div style={{ marginTop: 10 }}>
                      <div style={S.grid}>
                        {[{l:"Chapters",v:n.chapter_count},{l:"Plans",v:n.plans_left},{l:"Words",v:(n.total_words||0).toLocaleString()},{l:"Today",v:n.chapters_today}].map((s,i) => (
                          <div key={i} style={{...S.card, marginBottom: 0}}><div style={S.label}>{s.l}</div><div style={{...S.val,fontSize:"22px"}}>{s.v??"?"}</div></div>
                        ))}
                      </div>
                      <div style={{ marginTop: 8, color: "#888", fontSize: 12 }}>Click a chapter to read:</div>
                      {chapters.length === 0 && <div style={{color:"#888", padding: 8}}>No chapters yet. Write one from Dashboard.</div>}
                      {chapters.map(ch => (
                        <div key={ch.filename} style={S.chItem} onClick={() => openChapter(n.slug, ch.filename)}>
                          <div style={S.chTitle}>Ch.{ch.sequence} — {ch.title}</div>
                          <div style={S.chMeta}>{ch.filename} · {(ch.size/1024).toFixed(1)}KB · {new Date(ch.modified*1000).toLocaleString()}</div>
                          {openCh === ch.filename && <div style={S.preview}>{content || "Loading..."}</div>}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </>
        )}

        {page === "logs" && (
          <>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
              <h2 style={{ fontSize: 18, color: "#fff", margin: 0 }}>Execution Logs</h2>
              <button style={btn("#6c5ce7")} onClick={refreshLogs}>Refresh</button>
            </div>
            <div style={S.card}>
              {logs.length === 0
                ? <div style={{color:"#888"}}>No execution logs yet. Run "Write Chapter Now" to see logs here.</div>
                : logs.map((l,i) => <div key={i} style={S.logLine}>{l}</div>)}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
export default App;
