/**
 * Column budget for the direct modeler workbench (roadmap C5): sketch pane,
 * viewport, command panel and the side dock that holds the scene tree and the
 * inspector. The numbers are exposed to CSS as `--dm-*` custom properties on
 * `.direct-workspace`; the stylesheet only reads them.
 */
export const WORKBENCH_TOKENS = {
  /** Side dock width when open: tab rail plus tree/inspector body. */
  dockWidth: 344,
  /** Collapsed dock: tab rail only. */
  dockRail: 46,
  /** Active command panel inside the 3D pane. */
  commandWidth: 245,
  /** Splitter between the sketch pane and the 3D pane. */
  splitter: 7,
  /** Narrowest 3D viewport that still shows geometry next to the command panel. */
  viewportMin: 420,
  /** Narrowest useful sketch pane. */
  sketchMin: 260,
  /** Height from which tree and inspector are stacked in the dock instead of tabbed. */
  stackedMinHeight: 760,
  /** Below this width the dock is hidden by the mobile stylesheet. */
  compactMaxWidth: 750,
} as const

export interface WorkbenchInput {
  width: number
  height: number
  dockOpen: boolean
  sketchPane: boolean
  /** Requested sketch pane share in percent of the split area. */
  split: number
  commandActive: boolean
}

export interface WorkbenchLayout {
  dock: number
  sketch: number
  viewport: number
  command: number
  /** Split share actually applied, in percent. */
  split: number
  /** Tree and inspector are both visible in the dock. */
  stacked: boolean
  /** Every region meets its minimum without overlap. */
  fits: boolean
}

export function workbenchLayout(input: WorkbenchInput): WorkbenchLayout {
  const t = WORKBENCH_TOKENS
  const compact = input.width <= t.compactMaxWidth
  const dock = compact ? 0 : input.dockOpen ? t.dockWidth : t.dockRail
  const command = input.commandActive && !compact ? t.commandWidth : 0
  const area = Math.max(0, input.width - dock)
  const room3d = t.viewportMin + command
  let sketch = 0
  let split = input.split
  if (input.sketchPane && area > t.splitter) {
    // The sketch pane yields to the viewport first, but never below its own minimum.
    const usable = area - t.splitter
    const max = Math.max(t.sketchMin, usable - room3d)
    sketch = Math.min(max, Math.max(t.sketchMin, (usable * input.split) / 100), usable)
    split = (sketch / usable) * 100
  }
  const viewport = area - (input.sketchPane ? sketch + t.splitter : 0) - command
  return {
    dock,
    sketch,
    viewport,
    command,
    split,
    stacked: !compact && input.dockOpen && input.height >= t.stackedMinHeight,
    fits: compact || viewport >= t.viewportMin,
  }
}

/** CSS custom properties consumed by `direct-modeler.css`. */
export function workbenchCssVars(layout: WorkbenchLayout): Record<string, string> {
  const t = WORKBENCH_TOKENS
  return {
    '--dm-dock-w': `${t.dockWidth}px`,
    '--dm-dock-rail': `${t.dockRail}px`,
    '--dm-command-w': `${t.commandWidth}px`,
    '--dm-viewport-min': `${t.viewportMin}px`,
    '--split': `${layout.split}%`,
  }
}
