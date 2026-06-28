# Agent Workbench Memory Pet Design

Date: 2026-06-29

## Goal

Turn AI Novel Factory into an agent-style writing workbench inspired by popular open-source agent projects, while improving pet status visibility, reducing UI duplication, improving workflow efficiency without lowering writing quality, and making every finished chapter feed continuity memory back into RAG-assisted future writing.

## Agent Project References

The design uses patterns from these GitHub projects:

- OpenHands/OpenHands: task-first agent workspace, visible event stream, operator handoff, tool activity transparency.
- langchain-ai/langgraph: durable phase graph, named nodes, resumable state, explicit transitions.
- microsoft/autogen: role-based multi-agent collaboration and observable agent responsibilities.
- crewAIInc/crewAI: crew/role framing for coordinated specialist agents.
- FlowiseAI/Flowise: visual workflow mental model and compact node/status surfaces.

The app should not become a generic chatbot. It remains a long-form novel production tool whose agents are the drafting, review, repair, canon, memory, and publishing workers.

## Current Problems

- The navigation exposes many separate feature pages, which makes the app feel like a pile of tools instead of an agent cockpit.
- Dashboard, runtime, jobs, reviews, graph, bible, and learn all overlap around workflow visibility and context, but they do not communicate a single agent mental model.
- The pet only sees broad `loading/running/message/ragState` status, so it cannot explain what the agent is doing right now.
- The pet uses always-on CSS animation for active states and can feel busy on weaker machines.
- The pipeline already creates self-reflection learning entries after chapter completion, but it does not always store a deterministic chapter continuity memory that preserves key facts, open threads, and prose evidence for future writing.

## Product Direction

Use a quiet, dense, operational Agent OS style:

- Left rail has six high-level agent surfaces instead of many equivalent feature silos.
- Command bar speaks in agent language: selected project, running phase, RAG/memory status, scheduled publishing status.
- Dashboard becomes `Agent 总控台`: queue, phase timeline, live preview, operator controls, and memory signals.
- Runtime remains the deep control panel but is framed as `流程编排`.
- Memory/RAG surfaces are grouped as `记忆中枢`, while existing bible, graph, and learning tools remain reachable through internal controls or mode tabs rather than all competing in the main rail.
- Publishing and jobs are framed as `发布运维`.

## Navigation Contract

Main navigation should expose:

- `Agent 总控`
- `流程编排`
- `记忆中枢`
- `质量审稿`
- `发布运维`
- `项目设置`

Detailed legacy pages remain available through the selected surface, but duplicate top-level entries are removed from the main rail. No data or capability is deleted.

## Pet Contract

The pet is a lightweight status companion:

- It shows agent state: waiting, working, attention, memory, context, or idle.
- It shows the current phase label and progress percent when a pipeline event is available.
- It shows RAG state in a small indicator, not a verbose repeated line.
- It merges partial status events instead of resetting missing fields.
- It throttles pipeline status updates from the main window to reduce event and render churn.
- It avoids high-frequency canvas or JavaScript animation and honors `prefers-reduced-motion`.

## Memory/RAG Contract

Every successfully created chapter must produce a deterministic continuity memory entry after content is saved:

- `source_type`: `chapter_memory`
- `category`: `continuity_memory`
- `source_title`: `Chapter <sequence>: <title>`
- `pattern_name`: `章节记忆：<title>`
- `pattern_description`: compact summary, decision, score, and continuity hints
- `example_text`: excerpt from the final chapter body
- `application_notes`: how the next chapter should use the memory
- `metadata`: chapter id, chapter version id, sequence, decision, final score, word count

This memory is separate from self-reflection. Self-reflection improves craft; chapter memory improves continuity. Both can be selected by writing context in future chapters.

## Efficiency Rules

- Do not add an extra model call for deterministic chapter memory.
- Keep existing review and revision quality gates intact.
- Reuse existing `learning_entries` and writing context selection instead of adding a new table in this slice.
- Pet updates from pipeline events are throttled to no more than about 4 per second.
- UI changes should be contract-tested with Node tests, not only visual inspection.

## Acceptance Criteria

- Specs and plans exist under `docs/superpowers`.
- GitHub agent project references are captured in the spec.
- Top-level navigation uses the six Agent surfaces and removes duplicate legacy rail entries.
- UI copy uses Agent framing: `Agent 总控台`, `流程编排`, `记忆中枢`, `质量审稿`, `发布运维`.
- Pet status payload includes phase/progress fields.
- Pet merges partial status updates and renders phase/progress/RAG indicators.
- Pet pipeline status events are throttled from the main dashboard.
- Every completed chapter saves one deterministic `chapter_memory` learning entry.
- Existing self-reflection behavior remains.
- Full Rust and Node tests pass.
- Frontend production build passes.
- `git diff --check` passes.
