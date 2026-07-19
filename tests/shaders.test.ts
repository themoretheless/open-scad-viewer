import { describe, it, expect } from 'vitest'
import { SCENE_STRUCT_WGSL, MESH_WGSL, LINE_WGSL, OUTLINE_WGSL, SKY_WGSL } from '../src/renderer/shaders'

// The Scene uniform struct must stay a single source of truth: every shader
// that binds group(0) Scene embeds the exact same struct text. A drift here
// is a binding-layout mismatch WebGPU misreads as garbage floats, silently.
describe('WGSL shader sources', () => {
  const sceneShaders: Array<[string, string]> = [
    ['MESH_WGSL', MESH_WGSL],
    ['LINE_WGSL', LINE_WGSL],
    ['OUTLINE_WGSL', OUTLINE_WGSL],
  ]

  it.each(sceneShaders)('%s embeds the shared Scene struct exactly once', (_name, src) => {
    const occurrences = src.split(SCENE_STRUCT_WGSL).length - 1
    expect(occurrences).toBe(1)
  })

  it('SKY_WGSL does not bind the Scene struct', () => {
    expect(SKY_WGSL).not.toContain('struct Scene')
  })

  it('Scene struct field order matches what fillSceneData writes', () => {
    // Field order is load-bearing: the renderer writes raw float offsets.
    expect(SCENE_STRUCT_WGSL).toBe(
      'struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, clip: vec4f, fogParams: vec4f, fogColor: vec4f, _pad0: vec4f, _pad1: vec4f, sectionBox: vec4f }',
    )
  })

  it('every shader has a vertex and fragment entry point', () => {
    for (const src of [MESH_WGSL, LINE_WGSL, OUTLINE_WGSL, SKY_WGSL]) {
      expect(src).toContain('@vertex fn vs')
      expect(src).toContain('@fragment fn fs')
    }
  })
})
