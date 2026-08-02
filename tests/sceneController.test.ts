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

  it('normalizes malformed recovery state and ignores invalid visibility targets', () => {
    const controller = new SceneController()
    controller.publish({ meshes: [mesh('a')], visibility: [], selectedIndex: 8, isolated: true })
    expect(controller.state).toMatchObject({ visibility: [true], selectedIndex: null, isolated: false })
    const before = controller.state
    controller.setVisibility(4, false)
    expect(controller.state).toBe(before)
  })
})
