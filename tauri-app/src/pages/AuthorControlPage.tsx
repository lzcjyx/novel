import { useEffect, useState } from "react";
import { tauriClient } from "../lib/tauriClient";

interface DirectionCandidate {
  id: string;
  project_id?: string | null;
  inspiration: string;
  title_options: string[];
  positioning: string;
  target_reader: string;
  core_hook: string;
  series_promise: string;
  first_30_chapter_promise: string;
  checkpoint_status: string;
  revision_note?: string | null;
  selected: boolean;
  metadata: Record<string, unknown>;
}

interface DirectorBootstrapHandoff {
  candidate_id: string;
  suggested_title: string;
  positioning: string;
  core_hook: string;
  requires_human_review: boolean;
}

interface MemoryBankEntry {
  id: string;
  source_key: string;
  title: string;
  summary: string;
  edit_command: string;
}

interface MemoryBank {
  id: string;
  label: string;
  entries: MemoryBankEntry[];
}

interface MemoryBanksSnapshot {
  banks: MemoryBank[];
}

export function AuthorControlPage({ selected }: { selected: string }) {
  const [inspiration, setInspiration] = useState("");
  const [candidateCount, setCandidateCount] = useState(2);
  const [candidates, setCandidates] = useState<DirectionCandidate[]>([]);
  const [memoryBanks, setMemoryBanks] = useState<MemoryBank[]>([]);
  const [handoff, setHandoff] = useState<DirectorBootstrapHandoff | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");

  const loadCandidates = async () => {
    if (!selected) {
      setCandidates([]);
      return;
    }
    const rows = await tauriClient.listDirectionCandidates<DirectionCandidate[]>(selected);
    setCandidates(rows);
  };

  const loadMemoryBanks = async () => {
    if (!selected) {
      setMemoryBanks([]);
      return;
    }
    const snapshot = await tauriClient.getAuthorMemoryBanks<MemoryBanksSnapshot>(selected);
    setMemoryBanks(snapshot.banks);
  };

  useEffect(() => {
    Promise.all([loadCandidates(), loadMemoryBanks()]).catch((error) =>
      setMessage(`错误：${String(error)}`),
    );
  }, [selected]);

  const generateCandidates = async () => {
    if (!selected || !inspiration.trim()) return;
    setBusy(true);
    setMessage("");
    setHandoff(null);
    try {
      const rows = await tauriClient.generateDirectionCandidates<DirectionCandidate[]>(
        selected,
        inspiration.trim(),
        candidateCount,
      );
      setCandidates(rows);
      setMessage(`已生成 ${rows.length} 个方向候选。`);
    } catch (error) {
      setMessage(`错误：${String(error)}`);
    } finally {
      setBusy(false);
    }
  };

  const selectCandidate = async (candidateId: string) => {
    setBusy(true);
    setMessage("");
    try {
      await tauriClient.selectDirectionCandidate<DirectionCandidate>(
        candidateId,
        "从作者控制页选择",
      );
      const nextHandoff = await tauriClient.getDirectorBootstrapHandoff<DirectorBootstrapHandoff>(
        candidateId,
      );
      setHandoff(nextHandoff);
      await loadCandidates();
      await loadMemoryBanks();
      setMessage(`已选择《${nextHandoff.suggested_title}》。`);
    } catch (error) {
      setMessage(`错误：${String(error)}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <h2 className="page-title">作者控制</h2>
      <div className="dashboard-grid">
        <section className="card-feature">
          <div className="section-header">
            <h3 className="section-title">总导演</h3>
            <span className="badge badge-active">已设检查点</span>
          </div>
          <div className="bible-edit-field">
            <label>灵感</label>
            <textarea
              className="text-input"
              value={inspiration}
              onChange={(event) => setInspiration(event.target.value)}
              rows={4}
              placeholder="雨夜车站、一张旧税票、一个消失的名字。"
            />
          </div>
          <div className="bible-edit-field">
            <label>候选数量</label>
            <select
              className="select"
              value={candidateCount}
              onChange={(event) => setCandidateCount(Number(event.target.value))}
            >
              <option value={2}>2</option>
              <option value={3}>3</option>
            </select>
          </div>
          <button
            className="btn btn-primary"
            onClick={generateCandidates}
            disabled={busy || !selected || !inspiration.trim()}
          >
            {busy ? "运行中..." : "生成方向"}
          </button>

          <div className="status-grid" style={{ gridTemplateColumns: "1fr", marginTop: 16 }}>
            {candidates.map((candidate) => (
              <article key={candidate.id} className="card">
                <div style={{ display: "flex", justifyContent: "space-between", gap: 12 }}>
                  <div>
                    <strong className="chapter-title">
                      {candidate.title_options[0] || "未命名方向"}
                    </strong>
                    {candidate.selected && <span className="badge badge-active">已选择</span>}
                  </div>
                  <button
                    className="btn btn-secondary btn-sm"
                    onClick={() => selectCandidate(candidate.id)}
                    disabled={busy}
                  >
                    选择
                  </button>
                </div>
                <p className="text-body">{candidate.core_hook}</p>
                <div className="text-meta">{candidate.first_30_chapter_promise}</div>
              </article>
            ))}
          </div>
        </section>

        <aside className="status-grid" style={{ gridTemplateColumns: "1fr" }}>
          <section className="card">
            <h3 className="section-title">硬事实</h3>
            <div className="text-body">启用中的硬事实会作为独立来源进入写作上下文。</div>
          </section>
          <section className="card">
            <h3 className="section-title">风格资产</h3>
            <div className="text-body">启用的风格资产会编译进提示词和预检载荷。</div>
          </section>
          <section className="card">
            <h3 className="section-title">记忆库</h3>
            <div className="status-grid" style={{ gridTemplateColumns: "1fr", marginTop: 12 }}>
              {memoryBanks.map((bank) => (
                <article key={bank.id} className="card">
                  <div style={{ display: "flex", justifyContent: "space-between", gap: 8 }}>
                    <strong>{bank.label}</strong>
                    <span className="text-meta">{bank.entries.length}</span>
                  </div>
                  {bank.entries.slice(0, 2).map((entry) => (
                    <div key={entry.source_key} className="text-meta">
                      {entry.source_key} · {entry.edit_command}
                    </div>
                  ))}
                </article>
              ))}
            </div>
          </section>
          {handoff && (
            <section className="card-feature">
              <h3 className="section-title">启动交接</h3>
              <div className="chapter-title">{handoff.suggested_title}</div>
              <div className="text-body">{handoff.positioning}</div>
              <div className="text-meta">{handoff.core_hook}</div>
            </section>
          )}
        </aside>
      </div>
      {message && (
        <div className={`msg-banner ${message.startsWith("错误") ? "msg-error" : "msg-success"}`}>
          {message}
        </div>
      )}
    </>
  );
}
