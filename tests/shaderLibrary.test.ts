import { describe, expect, it } from 'vitest'
import {
  DEEP_MESH_WGSL,
  EDGE_WGSL,
  GRID_WGSL,
  LINE_WGSL,
  MESH_WGSL,
  MESH_VERTEX_STRIDE,
  MORPH_VERTEX_STRIDE,
  OBJECT_UNIFORM_LAYOUT,
  OBJ_STRUCT,
  SCENE_STRUCT,
  SELECTION_OVERLAY_WGSL,
  SECTION_CLIP_WGSL,
  immediateObjectShader,
  instancedObjectShader,
  sceneStruct,
} from '../src/services/shaders'

const OBJECT_SHADERS: Array<[string, string, 'V' | 'EdgeV']> = [
  ['mesh', MESH_WGSL, 'V'],
  ['deepMesh', DEEP_MESH_WGSL, 'V'],
  ['edge', EDGE_WGSL, 'EdgeV'],
]
const ALL_SHADERS = [...OBJECT_SHADERS.map(([name, code]) => [name, code] as const),
  ['line', LINE_WGSL], ['grid', GRID_WGSL], ['selectionOverlay', SELECTION_OVERLAY_WGSL]] as const

function balanced(code: string, open: string, close: string) {
  return (code.match(new RegExp(`\\${open}`, 'g')) ?? []).length
    === (code.match(new RegExp(`\\${close}`, 'g')) ?? []).length
}

describe('shader library modules', () => {
  it.each(ALL_SHADERS.map(([name, code]) => [name, code] as const))('%s composes into balanced WGSL', (_name, code) => {
    expect(code.length).toBeGreaterThan(0)
    expect(balanced(code, '{', '}')).toBe(true)
    expect(balanced(code, '(', ')')).toBe(true)
    expect(code).toContain('struct Scene')
    expect(code).toContain('@group(0) @binding(0) var<uniform> sc: Scene;')
    expect(code).toContain('@vertex fn vs(')
    expect(code).toContain('@fragment fn fs(')
  })

  it('object shaders share the exact Obj struct and scene chunk', () => {
    for (const [, code] of OBJECT_SHADERS) {
      expect(code).toContain(OBJ_STRUCT)
      expect(code).toContain(SCENE_STRUCT)
      expect(code).toContain('@group(1) @binding(0) var<uniform> ob: Obj;')
    }
    // Struct equality across modules: the scene uniform buffer is shared.
    const sceneLines = new Set(ALL_SHADERS.map(([, code]) => code.split('\n').find(line => line.startsWith('struct Scene'))))
    expect(sceneLines.size).toBe(2) // standard Scene + grid Scene with inverseVP
    expect(sceneStruct(', inverseVP: mat4x4f')).toContain('inverseVP: mat4x4f')
  })

  it('every object fragment shader honors the section clip with world position', () => {
    for (const [, code] of OBJECT_SHADERS) {
      expect(code).toContain(SECTION_CLIP_WGSL)
    }
  })

  it('grid extends the scene struct without touching the shared chunk', () => {
    expect(GRID_WGSL).toContain('inverseVP: mat4x4f')
    expect(SCENE_STRUCT).not.toContain('inverseVP')
  })

  it('morph blend reads ob.morph.x from the slot-1 attribute in object vertex shaders', () => {
    for (const [, code] of OBJECT_SHADERS) {
      expect(code).toContain('mix(fromPos, pos, ob.morph.x)')
    }
  })

  it('uniform layout matches the Obj struct fields', () => {
    // model(16) + nmat(16) + color(4) + style(4) + morph(4)
    expect(OBJECT_UNIFORM_LAYOUT.floats).toBe(44)
    expect(OBJECT_UNIFORM_LAYOUT.bytes).toBe(176)
    expect(OBJECT_UNIFORM_LAYOUT.styleFloatOffset).toBe(36)
    expect(OBJECT_UNIFORM_LAYOUT.styleByteOffset).toBe(36 * 4)
    expect(OBJECT_UNIFORM_LAYOUT.morphFloatOffset).toBe(40)
    expect(OBJECT_UNIFORM_LAYOUT.morphByteOffset).toBe(40 * 4)
    expect(MESH_VERTEX_STRIDE).toBe(24)
    expect(MORPH_VERTEX_STRIDE).toBe(12)
    expect(MESH_VERTEX_STRIDE / 4 + MESH_VERTEX_STRIDE / 4).toBe(12) // pos3 + norm3 floats
  })
})

describe('shader variants', () => {
  it('immediate variant serves style from the immediate address space', () => {
    for (const [, code] of OBJECT_SHADERS) {
      const variant = immediateObjectShader(code)
      expect(variant.startsWith('requires immediate_address_space;')).toBe(true)
      expect(variant).toContain('var<immediate> im_style: vec4f;')
      expect(variant).not.toContain('ob.style')
      expect(variant).toContain('objectStyle()')
    }
  })

  it.each(OBJECT_SHADERS.map(([name, code, output]) => [name, code, output] as const))(
    'instanced variant of %s reads per-instance records from storage', (_name, code, output) => {
      const variant = instancedObjectShader(code, output)
      expect(variant).toContain('var<storage, read> objects: array<Obj>;')
      expect(variant).toContain('@builtin(instance_index) instance: u32')
      expect(variant).toContain('let ob = objects[instance];')
      expect(variant).toContain(`@location(2) @interpolate(flat) instance: u32`)
      expect(variant).toContain('let ob = objects[v.instance];')
      // Exactly one vertex output location 2 (the flat instance id) and no leftover uniform Obj.
      expect(variant.split('var<uniform> ob: Obj;').length - 1).toBe(0)
      expect(variant).toContain(`return ${output}(instance, `)
    })

  it('variants throw when the source drifts from the contract', () => {
    expect(() => immediateObjectShader('fn nope() {}')).toThrow('missing object uniform')
    expect(() => instancedObjectShader('fn nope() {}', 'V')).toThrow('Instanced shader contract changed')
    expect(() => instancedObjectShader(MESH_WGSL, 'EdgeV')).toThrow('Instanced shader contract changed')
  })
})
