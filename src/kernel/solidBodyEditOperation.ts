// Kept apart from solidBodyEdit so UI guards do not pull the edge-feature and
// B-rep capability modules into the main modeler chunk.
export type SolidBodyEditOperation = 'push' | 'chamfer' | 'edge-fillet' | 'shell' | 'split'
export function isSolidBodyEdit(operation: unknown): operation is SolidBodyEditOperation {
  return ['push', 'chamfer', 'edge-fillet', 'shell', 'split'].includes(String(operation))
}
