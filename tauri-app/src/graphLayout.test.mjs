import test from "node:test";
import assert from "node:assert/strict";

import {
  clampGraphPosition,
  createGraphBasePositions,
  flowGraphPosition,
  positionFromClientPoint,
} from "./graphLayout.js";

test("graph base positions are deterministic and bounded", () => {
  const nodes = [
    { id: "a", node_type: "character", degree: 2 },
    { id: "b", node_type: "location", degree: 1 },
    { id: "c", node_type: "character", degree: 0 },
  ];

  const first = createGraphBasePositions(nodes);
  const second = createGraphBasePositions([...nodes].reverse());

  assert.deepEqual(first.a, second.a);
  for (const position of Object.values(first)) {
    assert.equal(position.x >= 7 && position.x <= 93, true);
    assert.equal(position.y >= 7 && position.y <= 93, true);
  }
});

test("flow offsets move visible nodes without leaving the canvas", () => {
  const node = { id: "node-flow", node_type: "plot_thread", degree: 3 };
  const base = { x: 50, y: 50 };

  const early = flowGraphPosition(node, base, 0);
  const later = flowGraphPosition(node, base, 1.75);

  assert.notDeepEqual(early, later);
  assert.equal(later.x >= 7 && later.x <= 93, true);
  assert.equal(later.y >= 7 && later.y <= 93, true);
});

test("drag coordinates convert and clamp to graph percent space", () => {
  const rect = { left: 100, top: 50, width: 400, height: 300 };

  assert.deepEqual(positionFromClientPoint(rect, 300, 200), { x: 50, y: 50 });
  assert.deepEqual(positionFromClientPoint(rect, -50, 900), { x: 7, y: 93 });
  assert.deepEqual(clampGraphPosition({ x: 120, y: -20 }), { x: 93, y: 7 });
});
