const GRAPH_MIN = 7;
const GRAPH_MAX = 93;

function hashString(value) {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return Math.abs(hash >>> 0);
}

export function clampGraphPosition(position) {
  const clamp = (value) => Math.min(GRAPH_MAX, Math.max(GRAPH_MIN, Math.round(value * 100) / 100));
  return { x: clamp(position.x), y: clamp(position.y) };
}

export function createGraphBasePositions(nodes) {
  const groups = new Map();
  for (const node of nodes) {
    const type = node.node_type || "unknown";
    groups.set(type, [...(groups.get(type) || []), node]);
  }

  const positions = {};
  const orderedTypes = Array.from(groups.keys()).sort();
  orderedTypes.forEach((type, typeIndex) => {
    const group = [...(groups.get(type) || [])].sort((a, b) => String(a.id).localeCompare(String(b.id)));
    const radius = 18 + Math.min(typeIndex, 5) * 7;
    group.forEach((node, index) => {
      const seed = (hashString(`${node.node_type}:${node.id}`) % 360) * (Math.PI / 180);
      const angle = ((index / Math.max(group.length, 1)) * Math.PI * 2) + (typeIndex * 0.58) + (seed * 0.08);
      positions[node.id] = clampGraphPosition({
        x: 50 + Math.cos(angle) * radius,
        y: 50 + Math.sin(angle) * Math.min(radius * 0.82, 35),
      });
    });
  });
  return positions;
}

export function flowGraphPosition(node, basePosition, tick) {
  const seed = hashString(`${node.node_type || "node"}:${node.id || ""}`) / 9973;
  const degree = Number.isFinite(node.degree) ? node.degree : 0;
  const amplitude = Math.min(2.4, 1.1 + degree * 0.18);
  return clampGraphPosition({
    x: basePosition.x + Math.cos(tick * 0.9 + seed) * amplitude,
    y: basePosition.y + Math.sin(tick * 0.7 + seed * 1.7) * amplitude * 0.7,
  });
}

export function positionFromClientPoint(rect, clientX, clientY) {
  const width = rect.width || 1;
  const height = rect.height || 1;
  return clampGraphPosition({
    x: ((clientX - rect.left) / width) * 100,
    y: ((clientY - rect.top) / height) * 100,
  });
}
