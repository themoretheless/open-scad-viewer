import { describe, expect, it, vi } from 'vitest'
import type { MeshData } from '../src/core/mesh'
import { SceneController } from '../src/services/sceneController'

const mesh = (entityId: string): MeshData => ({ entityId } as MeshData)

describe('SceneController', () => {
  it('owns publication, selection, visibility and isolation as one invariant-safe snapshot', () => {
    const controller = new SceneController<{ meshIndex: number }>()
    const listener = vi.fn()
    controller.subscribe(listener)
    controller.publish({
      meshes: [mesh('a'), mesh('b')],
      visibility: [true, true],
      selectedIndex: 1,
      isolated: true,
    })
    expect(controller.state).toMatchObject({ selectedIndex: 1, isolated: true, visibility: [true, true] })

    controller.applyRendererSelection(1, true, { meshIndex: 1 })
    controller.setVisibility(1, false)
    expect(controller.state).toMatchObject({ selectedIndex: null, selectedHit: null, isolated: false, visibility: [true, false] })
    expect(listener).toHaveBeenCalledTimes(3)
  })

  it('owns hover, measurement and section on the same snapshot', () => {
    const controller = new SceneController<{ meshIndex: number }>()
    controller.publish({
      meshes: [mesh('a'), mesh('b')],
      visibility: [true, true],
      selectedIndex: 0,
      isolated: false,
    })
    controller.applyRendererHover({ meshIndex: 1 })
    controller.setMeasurement({ points: [[0, 0, 0], [1, 0, 0]], distance: 1 }, true)
    controller.setSection({ enabled: true, axis: 'x', offset: 4, initialized: true })
    expect(controller.state).toMatchObject({
      hoveredHit: { meshIndex: 1 },
      measureActive: true,
      section: { enabled: true, axis: 'x', offset: 4 },
    })
    controller.setVisibility(1, false)
    expect(controller.state.hoveredHit).toBeNull()
    expect(controller.state.section.enabled).toBe(true)
    expect(controller.state.measurement?.distance).toBe(1)
  })

  it('normalizes malformed recovery state and ignores invalid visibility targets', () => {
    const controller = new SceneController()
    controller.publish({ meshes: [mesh('a')], visibility: [], selectedIndex: 8, isolated: true })
    expect(controller.state).toMatchObject({ visibility: [true], selectedIndex: null, isolated: false })
    const before = controller.state
    controller.setVisibility(4, false)
    expect(controller.state).toBe(before)
  })
})
