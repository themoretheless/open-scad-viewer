import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import { WORKBENCH_TOKENS, workbenchCssVars, workbenchLayout } from '../src/ui/workbenchLayout'

const base = { dockOpen: true, sketchPane: true, split: 34, commandActive: true }

describe('workbench layout', () => {
  it('fits tree, inspector, viewport and command panel at 1440x900 without overlap', () => {
    const layout = workbenchLayout({ width: 1440, height: 900, ...base })
    expect(layout.fits).toBe(true)
    expect(layout.stacked).toBe(true)
    expect(layout.dock).toBe(WORKBENCH_TOKENS.dockWidth)
    expect(layout.command).toBe(WORKBENCH_TOKENS.commandWidth)
    expect(layout.viewport).toBeGreaterThanOrEqual(WORKBENCH_TOKENS.viewportMin)
    expect(layout.sketch).toBeGreaterThanOrEqual(WORKBENCH_TOKENS.sketchMin)
    expect(layout.sketch + WORKBENCH_TOKENS.splitter + layout.viewport + layout.command + layout.dock).toBe(1440)
  })

  it('clamps an oversized sketch split so the viewport keeps its minimum', () => {
    const layout = workbenchLayout({ width: 1440, height: 900, ...base, split: 75 })
    expect(layout.viewport).toBe(WORKBENCH_TOKENS.viewportMin)
    expect(layout.split).toBeLessThan(75)
    expect(layout.fits).toBe(true)
  })

  it('still fits at 1280 wide; at 1024 the sketch pane must be hidden or the dock collapsed', () => {
    expect(workbenchLayout({ width: 1280, height: 800, ...base }).fits).toBe(true)
    expect(workbenchLayout({ width: 1024, height: 768, ...base }).fits).toBe(false)
    expect(workbenchLayout({ width: 1024, height: 768, ...base, sketchPane: false }).fits).toBe(true)
    const collapsed = workbenchLayout({ width: 1024, height: 768, ...base, sketchPane: false, dockOpen: false })
    expect(collapsed.dock).toBe(WORKBENCH_TOKENS.dockRail)
    expect(collapsed.stacked).toBe(false)
  })

  it('tabs tree and inspector on short screens and drops the dock in the compact layout', () => {
    expect(workbenchLayout({ width: 1440, height: 700, ...base }).stacked).toBe(false)
    const compact = workbenchLayout({ width: 700, height: 900, ...base })
    expect(compact.dock).toBe(0)
    expect(compact.command).toBe(0)
    expect(compact.fits).toBe(true)
  })

  it('drives the stylesheet through --dm-* tokens and a grid row', () => {
    const vars = workbenchCssVars(workbenchLayout({ width: 1440, height: 900, ...base }))
    expect(vars['--dm-dock-w']).toBe(`${WORKBENCH_TOKENS.dockWidth}px`)
    expect(vars['--dm-command-w']).toBe(`${WORKBENCH_TOKENS.commandWidth}px`)
    const css = readFileSync(resolve('src/features/styles/direct-modeler.css'), 'utf8')
    expect(css).toMatch(/\.workspace-row\{display:grid;grid-template-columns:minmax\(0,1fr\) auto\}/)
    expect(css).toContain('.side-dock{width:var(--dm-dock-w,')
    expect(css).toContain('.side-dock.collapsed{width:var(--dm-dock-rail,')
    expect(css).toContain('.canvas-wrap>.operation-card{flex-basis:var(--dm-command-w,')
    const source = readFileSync(resolve('src/features/DirectModeler.svt'), 'utf8')
    expect(source).toContain(':style="layoutVars"')
    expect(source).toContain("'--split': layout.split + '%'")
  })
})
