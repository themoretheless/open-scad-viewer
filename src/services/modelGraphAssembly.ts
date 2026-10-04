export type Frame = { origin: [number, number, number]; rotation: [number, number, number] }
export type AssemblyComponent = { id: string; input: string; anchors: Array<Frame & { id: string }>; placement?: Frame; mate?: { component: string; anchor: string; own_anchor: string; gap: number; rotation: [number, number, number]; joint?: { kind: 'revolute' | 'slider'; position: number; min: number; max: number } } }
export type Matrix = number[]
export type PlacedAssemblyComponent = {
  id: string
  input: string
  joint: NonNullable<NonNullable<AssemblyComponent['mate']>['joint']> | null
  matrix: Matrix
  anchors: Array<{id: string; matrix: Matrix}>
}
