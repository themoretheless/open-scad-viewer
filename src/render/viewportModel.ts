import type { CameraState } from './cameraHistory'

export type ProjectionMode = 'perspective' | 'orthographic'
export type StandardView = 'iso' | 'front' | 'back' | 'left' | 'right' | 'top' | 'bottom'

export const ISO_YAW = Math.PI / 4
export const ISO_PITCH = Math.atan(1 / Math.sqrt(2))

export type ViewOrbitDirection = 'left' | 'right' | 'up' | 'down'

/**
 * Computes new orbit angles (yaw, pitch in radians) after stepping by 15° (fine) or 90° (quarter-turn).
 * Pitch is clamped to `[-Math.PI / 2, Math.PI / 2]` and yaw is wrapped into `(-Math.PI, Math.PI]`.
 */
export function computeSteppedOrbitAngles(
  yaw: number,
  pitch: number,
  direction: ViewOrbitDirection,
  stepDegrees: 15 | 90 = 15,
): { yaw: number; pitch: number } {
  const stepRad = (stepDegrees * Math.PI) / 180
  let nextYaw = yaw
  let nextPitch = pitch
  if (direction === 'left') nextYaw -= stepRad
  else if (direction === 'right') nextYaw += stepRad
  else if (direction === 'up') nextPitch += stepRad
  else if (direction === 'down') nextPitch -= stepRad

  nextYaw = Math.atan2(Math.sin(nextYaw), Math.cos(nextYaw))
  nextPitch = Math.max(-Math.PI / 2, Math.min(Math.PI / 2, nextPitch))
  return { yaw: nextYaw, pitch: nextPitch }
}

const ORIENTATIONS: Readonly<Record<StandardView, readonly [yaw: number, pitch: number]>> = Object.freeze({
  iso: [ISO_YAW, ISO_PITCH],
  front: [0, 0],
  back: [Math.PI, 0],
  left: [-Math.PI / 2, 0],
  right: [Math.PI / 2, 0],
  top: [0, Math.PI / 2],
  bottom: [0, -Math.PI / 2],
})

export function standardViewOrientation(view: StandardView) {
  return ORIENTATIONS[view]
}

export function standardViewForCamera(state: Pick<CameraState, 'yaw' | 'pitch'>): StandardView | null {
  const angularDistance = (a: number, b: number) => Math.abs(Math.atan2(Math.sin(a - b), Math.cos(a - b)))
  for (const [view, [yaw, pitch]] of Object.entries(ORIENTATIONS) as Array<[StandardView, readonly [number, number]]>) {
    if (angularDistance(state.yaw, yaw) <= 1e-7 && Math.abs(state.pitch - pitch) <= 1e-7) return view
  }
  return null
}

export interface AxisScreenVector {
  readonly x: number
  readonly y: number
  readonly depth: number
}

export interface AxesScreenProjection {
  readonly x: AxisScreenVector
  readonly y: AxisScreenVector
  readonly z: AxisScreenVector
}

/** Project Z-up world axes into the renderer's orbit-camera screen basis. */
export function projectAxesToScreen(yaw: number, pitch: number): AxesScreenProjection {
  const cy = Math.cos(yaw),
    sy = Math.sin(yaw)
  const cp = Math.cos(pitch),
    sp = Math.sin(pitch)
  if (Math.abs(cp) < 1e-10) {
    const sign = sp >= 0 ? 1 : -1
    return {
      x: { x: sign, y: 0, depth: 0 },
      y: { x: 0, y: 1, depth: 0 },
      z: { x: 0, y: 0, depth: sign },
    }
  }
  return {
    x: { x: cy, y: -sp * sy, depth: cp * sy },
    y: { x: sy, y: sp * cy, depth: -cp * cy },
    z: { x: 0, y: cp, depth: sp },
  }
}

/** A ViewCube direction: the sum of the outward normals of the faces it touches (each component -1, 0 or 1). */
export type ViewCubeDirection = readonly [x: -1 | 0 | 1, y: -1 | 0 | 1, z: -1 | 0 | 1]
export type ViewCubeTargetKind = 'face' | 'edge' | 'corner'

export interface ViewCubeTarget {
  readonly id: string
  readonly kind: ViewCubeTargetKind
  readonly direction: ViewCubeDirection
  readonly yaw: number
  readonly pitch: number
  /** Bilingual accessible name, `English / Русский`. */
  readonly label: string
}

const AXIS_WORDS = [
  { negative: ['left', 'слева'], positive: ['right', 'справа'] },
  { negative: ['front', 'спереди'], positive: ['back', 'сзади'] },
  { negative: ['bottom', 'снизу'], positive: ['top', 'сверху'] },
] as const

/**
 * Orbit angles that look at the origin from `direction`. Matches the renderer basis:
 * yaw 0 looks from -Y (front), yaw π/2 from +X (right), pitch π/2 from +Z (top).
 */
export function viewCubeOrientation(direction: readonly [number, number, number]): { yaw: number; pitch: number } {
  const [x, y, z] = direction
  const horizontal = Math.hypot(x, y)
  return { yaw: horizontal < 1e-12 ? 0 : Math.atan2(x, -y), pitch: Math.atan2(z, horizontal) }
}

function axisWords(direction: ViewCubeDirection) {
  return ([2, 1, 0] as const)
    .filter(axis => direction[axis] !== 0)
    .map(axis => AXIS_WORDS[axis][direction[axis] > 0 ? 'positive' : 'negative'])
}

/** Stable id such as `top-front-right`, ordered top/bottom, front/back, left/right. */
export function viewCubeTargetId(direction: ViewCubeDirection): string {
  return axisWords(direction)
    .map(word => word[0])
    .join('-')
}

function viewCubeTarget(direction: ViewCubeDirection): ViewCubeTarget {
  const words = axisWords(direction)
  const kind: ViewCubeTargetKind = words.length === 1 ? 'face' : words.length === 2 ? 'edge' : 'corner'
  const english = words.map(word => word[0]).join(' ')
  const russian = words.map(word => word[1]).join(', ')
  const prefix = { face: ['face', 'Грань'], edge: ['edge', 'Ребро'], corner: ['corner', 'Угол'] }[kind]
  return {
    id: viewCubeTargetId(direction),
    kind,
    direction,
    ...viewCubeOrientation(direction),
    label: `${english.charAt(0).toUpperCase()}${english.slice(1)} ${prefix[0]} / ${prefix[1]}: ${russian}`,
  }
}

/** All 26 ViewCube targets: 6 faces, 12 edges and 8 corners. */
export const VIEW_CUBE_TARGETS: readonly ViewCubeTarget[] = Object.freeze(
  ([-1, 0, 1] as const).flatMap(x =>
    ([-1, 0, 1] as const).flatMap(y =>
      ([-1, 0, 1] as const).flatMap(z => (x || y || z ? [viewCubeTarget([x, y, z])] : [])),
    ),
  ),
)

/** True when the camera already looks from `target` (within 1e-6 rad; yaw is free at the poles). */
export function viewCubeTargetActive(target: Pick<ViewCubeTarget, 'yaw' | 'pitch'>, yaw: number, pitch: number) {
  if (Math.abs(pitch - target.pitch) > 1e-6) return false
  if (Math.abs(Math.abs(target.pitch) - Math.PI / 2) <= 1e-6) return true
  return Math.abs(Math.atan2(Math.sin(yaw - target.yaw), Math.cos(yaw - target.yaw))) <= 1e-6
}

/** Fixed isometric drawing of the cube in the ViewCube SVG (viewBox 0 0 128 132), corners at ±1. */
export function viewCubeSvgPoint(x: number, y: number, z: number): readonly [number, number] {
  return [64 + 20 * x + 20 * y, 63 + 11 * x - 11 * y - 23 * z]
}

export interface ViewCubeHotspot {
  readonly target: ViewCubeTarget
  /** SVG polygons (one per visible face the edge or corner touches). */
  readonly polygons: readonly string[]
}

/**
 * Clickable edge and corner zones on the three drawn faces (top, front, right). Each face is cut 3×3 at
 * `margin`; the centre cell stays the face preset. Zones that share a direction merge into one hotspot.
 */
export function viewCubeHotspots(margin = 0.2): ViewCubeHotspot[] {
  const faces: Array<{ normal: ViewCubeDirection; u: 0 | 1 | 2; v: 0 | 1 | 2 }> = [
    { normal: [0, 0, 1], u: 0, v: 1 },
    { normal: [0, -1, 0], u: 0, v: 2 },
    { normal: [1, 0, 0], u: 1, v: 2 },
  ]
  const cuts = [-1, -1 + 2 * margin, 1 - 2 * margin, 1]
  const byId = new Map<string, { target: ViewCubeTarget; polygons: string[] }>()
  for (const face of faces) {
    for (let i = 0; i < 3; i++) {
      for (let j = 0; j < 3; j++) {
        if (i === 1 && j === 1) continue
        const direction = [...face.normal] as [-1 | 0 | 1, -1 | 0 | 1, -1 | 0 | 1]
        direction[face.u] = (i - 1) as -1 | 0 | 1
        direction[face.v] = (j - 1) as -1 | 0 | 1
        const corner = (a: number, b: number) => {
          const point = face.normal.map(c => c) as number[]
          point[face.u] = cuts[i + a]!
          point[face.v] = cuts[j + b]!
          return viewCubeSvgPoint(point[0]!, point[1]!, point[2]!)
            .map(c => Math.round(c * 100) / 100)
            .join(',')
        }
        const polygon = [corner(0, 0), corner(1, 0), corner(1, 1), corner(0, 1)].join(' ')
        const id = viewCubeTargetId(direction)
        let entry = byId.get(id)
        if (!entry) {
          entry = { target: VIEW_CUBE_TARGETS.find(target => target.id === id)!, polygons: [] }
          byId.set(id, entry)
        }
        entry.polygons.push(polygon)
      }
    }
  }
  return [...byId.values()]
}
