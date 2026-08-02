import { describe, expect, it, vi } from 'vitest'
import { WebGPURenderer, type PickHit } from '../src/services/webgpuRenderer'

const hit = (meshIndex: number): PickHit => ({
  meshIndex,
  triangleIndex: 0,
  faceId: 0,
  point: [0, 0, 0],
  normal: [0, 0, 1],
  barycentric: [1, 0, 0],
  source: null,
  backside: false,
  cycleIndex: 1,
  cycleCount: 2,
})

describe('WebGPURenderer batched visibility', () => {
  it('does not rebuild source overlays for selection changes unless isolation changes effective visibility', () => {
    const renderer = new WebGPURenderer()
    const internal = renderer as unknown as {
      meshes: Array<{ visible: boolean }>
      updateMeshStyles(): void
      rebuildSelectionOverlays(): void
      rebuildSourceHighlightOverlay(): void
    }
    internal.meshes = [{ visible: true }, { visible: true }]
    internal.updateMeshStyles = vi.fn()
    internal.rebuildSelectionOverlays = vi.fn()
    const rebuildSource = vi.fn()
    internal.rebuildSourceHighlightOverlay = rebuildSource
    vi.spyOn(renderer, 'requestRender').mockImplementation(() => undefined)

    renderer.selectMesh(0)
    renderer.selectMesh(1)
    expect(rebuildSource).not.toHaveBeenCalled()
    renderer.toggleIsolateSelection()
    expect(rebuildSource).toHaveBeenCalledTimes(1)
    renderer.selectMesh(0)
    expect(rebuildSource).toHaveBeenCalledTimes(2)
  })

  it('commits visibility, bounds, overlays, and hidden interaction state once', () => {
    const renderer = new WebGPURenderer()
    const firstBounds = {
      center: [0, 0, 0] as [number, number, number],
      radius: 1,
      min: [-1, -1, -1] as [number, number, number],
      max: [1, 1, 1] as [number, number, number],
    }
    const internal = renderer as unknown as {
      meshes: Array<{ visible: boolean; worldBounds: typeof firstBounds }>
      bounds: typeof firstBounds | null
      sceneAabbIndexDirty: boolean
      selected: number | null
      selectedHit: PickHit | null
      hovered: number | null
      hoveredHit: PickHit | null
      isolated: boolean
      combineBounds: (bounds: typeof firstBounds[]) => typeof firstBounds | null
      updateMeshStyles: () => void
      rebuildSelectionOverlays: () => void
      rebuildSourceHighlightOverlay: () => void
    }
    internal.meshes = [
      { visible: true, worldBounds: firstBounds },
      { visible: true, worldBounds: { ...firstBounds, center: [3, 0, 0] } },
      { visible: true, worldBounds: { ...firstBounds, center: [6, 0, 0] } },
    ]
    internal.selected = 1
    internal.selectedHit = hit(1)
    internal.hovered = 2
    internal.hoveredHit = hit(2)
    internal.isolated = true

    const combineBounds = vi.fn(() => firstBounds)
    const updateMeshStyles = vi.fn()
    const rebuildSelectionOverlays = vi.fn()
    const rebuildSourceHighlightOverlay = vi.fn()
    const requestRender = vi.spyOn(renderer, 'requestRender').mockImplementation(() => undefined)
    internal.combineBounds = combineBounds
    internal.updateMeshStyles = updateMeshStyles
    internal.rebuildSelectionOverlays = rebuildSelectionOverlays
    internal.rebuildSourceHighlightOverlay = rebuildSourceHighlightOverlay
    const selectionEvents: Array<[number | null, boolean, PickHit | null]> = []
    const hoverEvents: Array<PickHit | null> = []
    renderer.onSelectionChange = (...event) => selectionEvents.push(event)
    renderer.onHoverChange = event => hoverEvents.push(event)

    expect(renderer.setMeshVisibilityBatch([true, false, false])).toBe(true)

    expect(internal.meshes.map(mesh => mesh.visible)).toEqual([true, false, false])
    expect(internal.sceneAabbIndexDirty).toBe(true)
    expect(internal.bounds).toBe(firstBounds)
    expect(renderer.selectedIndex).toBeNull()
    expect(renderer.currentHit).toBeNull()
    expect(renderer.isIsolated).toBe(false)
    expect(selectionEvents).toEqual([[null, false, null]])
    expect(hoverEvents).toEqual([null])
    expect(combineBounds).toHaveBeenCalledOnce()
    expect(combineBounds).toHaveBeenCalledWith([firstBounds])
    expect(updateMeshStyles).toHaveBeenCalledOnce()
    expect(rebuildSelectionOverlays).toHaveBeenCalledOnce()
    expect(rebuildSourceHighlightOverlay).toHaveBeenCalledOnce()
    expect(requestRender).toHaveBeenCalledOnce()

    expect(renderer.setMeshVisibilityBatch([true, false, false])).toBe(false)
    expect(combineBounds).toHaveBeenCalledOnce()
    expect(requestRender).toHaveBeenCalledOnce()

    renderer.selectMesh(1)
    expect(renderer.selectedIndex).toBeNull()
    expect(selectionEvents).toEqual([[null, false, null]])
  })
})
