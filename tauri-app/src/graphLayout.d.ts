export interface GraphLayoutNode {
  id: string;
  node_type: string;
  degree?: number;
}

export interface GraphPosition {
  x: number;
  y: number;
}

export function clampGraphPosition(position: GraphPosition): GraphPosition;
export function createGraphBasePositions(nodes: GraphLayoutNode[]): Record<string, GraphPosition>;
export function flowGraphPosition(
  node: GraphLayoutNode,
  basePosition: GraphPosition,
  tick: number,
): GraphPosition;
export function positionFromClientPoint(
  rect: Pick<DOMRect, "left" | "top" | "width" | "height">,
  clientX: number,
  clientY: number,
): GraphPosition;
