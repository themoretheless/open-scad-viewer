import { callGeometryRust } from './geometry/kernel'
import { diagnoseManifoldInKernel } from './geometry/manifoldAnalysis'
import type { DrawingSheet } from '../app/drawingSheets'

/**
 * Orthographic drawing sheet of one triangle mesh: hidden-line views (visible
 * solid, hidden dashed, silhouettes of curved faces), an optional hatched
 * section and a print-check table. Geometry runs in Rust
 * (`mesh_section::drawing` via the `drawing_views` op; wall thickness and
 * clearance via `diagnoseManifoldInKernel`). Output is deterministic.
 */
export type DrawingViewName = 'front' | 'top' | 'right' | 'iso'

export interface DrawingMesh {
  positions: ArrayLike<number>
  indices: ArrayLike<number>
}

export interface DrawingViewsOptions {
  views?: DrawingViewName[]
  /** Cut plane `p . d = offset`, `d` pointing toward the viewer of `view`. */
  section?: { view: DrawingViewName; offset: number; spacing?: number }
  /** Wall thinner than this fails the check (mm). */
  minWallMm?: number
  /** Gap between separate shells below this fails the check (mm). */
  minClearanceMm?: number
  /** Faces overhanging more than this from vertical need support (degrees). */
  overhangAngleDeg?: number
  title?: string
}

export interface DrawingSegment {
  a: [number, number]
  b: [number, number]
  hidden: boolean
  silhouette: boolean
}

export interface DrawingPrintChecks {
  minWallMm: number | null
  thinTriangles: number
  wallOk: boolean
  minClearanceMm: number | null
  clearancePairs: number
  clearanceOk: boolean
  overhangAreaMm2: number
  overhangTriangles: number
  overhangThresholdDeg: number
  overhangOk: boolean
}

export interface DrawingViewsResult {
  svg: string
  views: { view: DrawingViewName; segments: DrawingSegment[] }[]
  section: { view: DrawingViewName; rings: [number, number][][]; hatch: [number, number][][]; area: number } | null
  printChecks: DrawingPrintChecks
}

interface RustDrawing {
  views: { view: DrawingViewName; segments: DrawingSegment[] }[]
  section: DrawingViewsResult['section']
  overhang: { thresholdDeg: number; area: number; triangles: number[] }
}

const VIEW_LABELS: Record<DrawingViewName, string> = { front: 'FRONT', top: 'TOP', right: 'RIGHT', iso: 'ISO' }

function fmt(x: number): string {
  const r = Math.round(x * 1e4) / 1e4
  return Object.is(r, -0) ? '0' : String(r)
}

function escapeXml(text: string): string {
  return text.replace(/[<>&"]/g, c => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', '"': '&quot;' })[c] ?? c)
}

export function drawingPrintChecks(mesh: DrawingMesh, overhang: RustDrawing['overhang'], options: DrawingViewsOptions) {
  const minWall = options.minWallMm ?? 0.8
  const minClearance = options.minClearanceMm ?? 0.2
  if (!(minWall >= 0) || !(minClearance > 0)) throw new RangeError('Print check thresholds must be positive.')
  const d = diagnoseManifoldInKernel(Float64Array.from(mesh.positions), Uint32Array.from(mesh.indices), {
    minThickness: minWall,
    clearanceThreshold: minClearance,
  })
  const gaps = d.clearancePairs.map(p => p.distance)
  const minClearanceMm = gaps.length ? Math.min(...gaps) : null
  return {
    minWallMm: d.minThickness,
    thinTriangles: d.thinTriangles.length,
    wallOk: d.thinTriangles.length === 0,
    minClearanceMm,
    clearancePairs: gaps.length,
    clearanceOk: gaps.length === 0,
    overhangAreaMm2: overhang.area,
    overhangTriangles: overhang.triangles.length,
    overhangThresholdDeg: overhang.thresholdDeg,
    overhangOk: overhang.triangles.length === 0,
  } satisfies DrawingPrintChecks
}

function bounds(points: Iterable<[number, number]>): [number, number, number, number] {
  const b: [number, number, number, number] = [Infinity, Infinity, -Infinity, -Infinity]
  for (const [x, y] of points) {
    b[0] = Math.min(b[0], x)
    b[1] = Math.min(b[1], y)
    b[2] = Math.max(b[2], x)
    b[3] = Math.max(b[3], y)
  }
  return b
}

export function createDrawingViews(mesh: DrawingMesh, options: DrawingViewsOptions = {}): DrawingViewsResult {
  const views = options.views ?? ['front', 'top', 'right', 'iso']
  const section = options.section
  if (section && !(Number.isFinite(section.offset) && (section.spacing ?? 2) > 0)) {
    throw new RangeError('Section offset and hatch spacing must be finite and positive.')
  }
  const raw = callGeometryRust<RustDrawing>('drawing_views', {
    positions: Array.from(mesh.positions),
    indices: Array.from(mesh.indices),
    views,
    section: section ? { view: section.view, offset: section.offset, spacing: section.spacing ?? 2 } : null,
    overhangAngleDeg: options.overhangAngleDeg ?? 45,
  })
  const printChecks = drawingPrintChecks(mesh, raw.overhang, options)
  const svg = drawingSvg(raw, printChecks, options.title ?? 'Drawing')
  return { svg, views: raw.views, section: raw.section, printChecks }
}

/**
 * Overall width and height of one view cell as dimension lines below and to the
 * right of the projected extents (model mm, so ISO and SECTION get them too).
 */
function overallDimensions(box: number[], scale: number, ox: number, oy: number): string {
  const w = box[2] - box[0]
  const h = box[3] - box[1]
  const x1 = ox + w * scale
  const y1 = oy + h * scale
  const label = (x: number, y: number, value: number, rotate: boolean) =>
    `<text x="${fmt(x)}" y="${fmt(y)}"${rotate ? ` transform="rotate(-90 ${fmt(x)} ${fmt(y)})"` : ''} text-anchor="middle" fill="black" stroke="none" font-family="sans-serif" font-size="2.5">${fmt(value)}</text>`
  const parts: string[] = []
  if (w > 1e-9) {
    const y = y1 + 4
    parts.push(
      `<path d="M${fmt(ox)} ${fmt(y1 + 1)}V${fmt(y + 1)}M${fmt(x1)} ${fmt(y1 + 1)}V${fmt(y + 1)}M${fmt(ox)} ${fmt(y)}H${fmt(x1)}"/>`,
    )
    parts.push(label((ox + x1) / 2, y - 0.8, w, false))
  }
  if (h > 1e-9) {
    const x = x1 + 4
    parts.push(
      `<path d="M${fmt(x1 + 1)} ${fmt(oy)}H${fmt(x + 1)}M${fmt(x1 + 1)} ${fmt(y1)}H${fmt(x + 1)}M${fmt(x)} ${fmt(oy)}V${fmt(y1)}"/>`,
    )
    parts.push(label(x - 0.8, (oy + y1) / 2, h, true))
  }
  return parts.length
    ? `<g class="dimension" data-width="${fmt(w)}" data-height="${fmt(h)}" stroke-width=".13">${parts.join('')}</g>`
    : ''
}

/** Drawing views as a `drawingSheets` sheet (size from the SVG), ready for `vectorDrawingPdf`. */
export function drawingViewsSheet(result: DrawingViewsResult, title = 'Drawing'): DrawingSheet {
  const size = /width="([\d.]+)mm" height="([\d.]+)mm"/.exec(result.svg)
  if (!size) throw new Error('Drawing SVG has no millimetre size.')
  return { svg: result.svg, width: Number(size[1]), height: Number(size[2]), title, kind: 'drawing' }
}

function drawingSvg(raw: RustDrawing, checks: DrawingPrintChecks, title: string): string {
  const cells: { label: string; body: (s: number, ox: number, oy: number) => string; box: number[] }[] = []
  for (const v of raw.views) {
    const box = bounds(v.segments.flatMap(s => [s.a, s.b]))
    cells.push({
      label: VIEW_LABELS[v.view],
      box,
      body: (s, ox, oy) => {
        const p = (q: [number, number]) => `${fmt(ox + (q[0] - box[0]) * s)} ${fmt(oy + (box[3] - q[1]) * s)}`
        const path = (list: DrawingSegment[]) => list.map(x => `M${p(x.a)}L${p(x.b)}`).join('')
        const visible = v.segments.filter(x => !x.hidden)
        const hidden = v.segments.filter(x => x.hidden)
        return (
          `<path class="visible" d="${path(visible)}" stroke-width=".35"/>` +
          (hidden.length
            ? `<path class="hidden" d="${path(hidden)}" stroke-width=".18" stroke-dasharray="1.2 .8"/>`
            : '')
        )
      },
    })
  }
  const sec = raw.section
  if (sec && sec.rings.length) {
    const box = bounds(sec.rings.flat())
    cells.push({
      label: `SECTION ${VIEW_LABELS[sec.view]}`,
      box,
      body: (s, ox, oy) => {
        const p = (q: [number, number]) => `${fmt(ox + (q[0] - box[0]) * s)} ${fmt(oy + (box[3] - q[1]) * s)}`
        const area = sec.rings.map(r => `M${r.map(p).join('L')}Z`).join('')
        const hatch = sec.hatch.map(h => `M${p(h[0])}L${p(h[1])}`).join('')
        return (
          `<path class="section" d="${area}" fill="#ddd" fill-rule="evenodd" stroke-width=".35"/>` +
          `<path class="hatch" d="${hatch}" stroke-width=".13"/>`
        )
      },
    })
  }
  const columns = Math.min(3, Math.max(1, cells.length))
  const cell = 90
  const rows = Math.ceil(cells.length / columns)
  const span = Math.max(1e-9, ...cells.map(c => Math.max(c.box[2] - c.box[0], c.box[3] - c.box[1])))
  const scale = (cell - 20) / span
  const width = 10 + columns * cell
  const height = 20 + rows * cell + 40
  const parts = cells.map((c, i) => {
    const ox = 10 + (i % columns) * cell + (cell - 20 - (c.box[2] - c.box[0]) * scale) / 2
    const oy = 20 + Math.floor(i / columns) * cell + 8
    const lx = 10 + (i % columns) * cell
    return `<g data-view="${escapeXml(c.label)}"><text x="${fmt(lx)}" y="${fmt(oy - 3)}" fill="black" stroke="none" font-family="sans-serif" font-size="3">${escapeXml(c.label)}</text>${c.body(scale, ox, oy)}${overallDimensions(c.box, scale, ox, oy)}</g>`
  })
  const mm = (x: number | null) => (x === null ? '—' : `${fmt(x)} mm`)
  const status = (ok: boolean) => (ok ? 'OK' : 'FAIL')
  const rowsText = [
    `Min wall: ${mm(checks.minWallMm)} · thin triangles ${checks.thinTriangles} · ${status(checks.wallOk)}`,
    `Clearance: ${mm(checks.minClearanceMm)} · close pairs ${checks.clearancePairs} · ${status(checks.clearanceOk)}`,
    `Overhang > ${fmt(checks.overhangThresholdDeg)}°: ${fmt(checks.overhangAreaMm2)} mm² · ${checks.overhangTriangles} triangles · ${status(checks.overhangOk)}`,
  ]
  const ty = 20 + rows * cell + 8
  const table = rowsText.map((t, i) => `<text x="10" y="${fmt(ty + i * 6)}">${escapeXml(t)}</text>`).join('')
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${fmt(width)}mm" height="${fmt(height)}mm" viewBox="0 0 ${fmt(width)} ${fmt(height)}">` +
    `<rect width="${fmt(width)}" height="${fmt(height)}" fill="white"/>` +
    `<text x="10" y="10" font-family="sans-serif" font-size="4">${escapeXml(title)} · scale ${fmt(scale)}:1 · mm</text>` +
    `<g fill="none" stroke="black" stroke-linecap="round">${parts.join('')}</g>` +
    `<g data-print-checks="true" font-family="sans-serif" font-size="3.2">${table}</g></svg>`
  )
}
