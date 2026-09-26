import { describe, expect, it } from 'vitest'
import {
  DEEP_MESH_WGSL,
  EDGE_WGSL,
  GRID_WGSL,
  LINE_WGSL,
  MESH_MATCAP_WGSL,
  MESH_PBR_WGSL,
  MESH_SECTION_CAP_WGSL,
  MESH_TOON_WGSL,
  MESH_UNLIT_WGSL,
  MESH_WGSL,
  MESH_SHADOW_WGSL,
  MESH_VERTEX_STRIDE,
  MORPH_VERTEX_STRIDE,
  OBJECT_UNIFORM_LAYOUT,
  OBJ_STRUCT,
  SCENE_STRUCT,
  SCENE_UNIFORM_LAYOUT,
  SELECTION_OVERLAY_WGSL,
  SECTION_CAP_WGSL,
  SECTION_CLIP_WGSL,
  getShader,
  hasShader,
  immediateObjectShader,
  instancedObjectShader,
  listShaders,
  sceneStruct,
} from '../src/services/shaders'

const OBJECT_SHADERS: Array<[string, string, 'V' | 'EdgeV']> = [
  ['mesh', MESH_WGSL, 'V'],
  ['meshPbr', MESH_PBR_WGSL, 'V'],
  ['meshMatcap', MESH_MATCAP_WGSL, 'V'],
  ['meshToon', MESH_TOON_WGSL, 'V'],
  ['meshUnlit', MESH_UNLIT_WGSL, 'V'],
  ['deepMesh', DEEP_MESH_WGSL, 'V'],
  ['edge', EDGE_WGSL, 'EdgeV'],
]
const ALL_SHADERS = [...OBJECT_SHADERS.map(([name, code]) => [name, code] as const),
  ['meshSectionCap', MESH_SECTION_CAP_WGSL],
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
    // line/selectionOverlay keep the short legacy struct (they never read
    // past `options`); everything else shares the themed struct.
    const sceneLines = new Set(ALL_SHADERS.map(([, code]) => code.split('\n').find(line => line.startsWith('struct Scene'))))
    expect(sceneLines.size).toBe(2) // short legacy Scene + themed Scene
    expect(sceneStruct().length).toBeGreaterThan(SCENE_STRUCT.length - 1)
    expect(sceneStruct()).toBe(SCENE_STRUCT)
  })

  it('every object fragment shader honors the section clip with world position', () => {
    for (const [, code] of OBJECT_SHADERS) {
      expect(code).toContain(SECTION_CLIP_WGSL)
    }
  })

  it('the section-cap shader inverts the clip test and fills flat with the theme cap color', () => {
    // Same vertex stage as mesh (layout, morph blend, transforms apply), but
    // the fragment keeps only the clipped side and shades unlit; the renderer
    // pairs it with front-face culling so interior back faces form the cap.
    expect(MESH_SECTION_CAP_WGSL).toContain(SCENE_STRUCT)
    expect(MESH_SECTION_CAP_WGSL).toContain(OBJ_STRUCT)
    expect(MESH_SECTION_CAP_WGSL).toContain('mix(fromPos, pos, ob.morph.x)')
    expect(MESH_SECTION_CAP_WGSL).toContain('if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) >= sc.section.w) { discard; }')
    expect(MESH_SECTION_CAP_WGSL).toContain('return vec4f(sc.capColor, 1.0);')
    expect(MESH_SECTION_CAP_WGSL).not.toContain(SECTION_CLIP_WGSL)
  })

  it('the matcap shader binds a group(2) capture texture with a 1x1 procedural fallback', () => {
    // One module serves both paths: a real capture switches to the textured
    // branch, while the renderer's 1×1 dummy keeps the procedural branch —
    // so the default matcap look is pixel-identical to the pre-texture one.
    expect(MESH_MATCAP_WGSL).toContain('@group(2) @binding(0) var matcapTex: texture_2d<f32>;')
    expect(MESH_MATCAP_WGSL).toContain('@group(2) @binding(1) var matcapSampler: sampler;')
    expect(MESH_MATCAP_WGSL).toContain('textureDimensions(matcapTex).x > 1u')
    expect(MESH_MATCAP_WGSL).toContain('textureSampleLevel(matcapTex, matcapSampler, uv, 0.0)')
    // The textured path tints by the selection/hover-aware base color and
    // keeps emission; alpha still flows through ob.style.x.
    expect(MESH_MATCAP_WGSL).toContain('captured * base + ob.emissive, ob.style.x')
  })

  it('the PBR shader binds a group(2) environment map with a 1x1 analytic fallback', () => {
    // One module serves both paths: a real equirect map switches to the IBL
    // branch, while the renderer's 1×1 dummy keeps the analytic key-light
    // branch — so the default PBR look is pixel-identical to the pre-IBL one.
    expect(MESH_PBR_WGSL).toContain('@group(2) @binding(0) var envTex: texture_2d<f32>;')
    expect(MESH_PBR_WGSL).toContain('@group(2) @binding(1) var envSampler: sampler;')
    expect(MESH_PBR_WGSL).toContain('textureDimensions(envTex).x > 1u')
    // IBL path: equirect lookup, cone-averaged diffuse irradiance, mip-aware
    // specular fallback, fresnel weighting; analytic branch preserved verbatim.
    expect(MESH_PBR_WGSL).toContain('fn envUv(dir: vec3f) -> vec2f {')
    expect(MESH_PBR_WGSL).toContain('fn envIrradiance(N: vec3f) -> vec3f {')
    expect(MESH_PBR_WGSL).toContain('textureNumLevels(envTex) > 1u')
    expect(MESH_PBR_WGSL).toContain('(kd * albedo / 3.14159265 + specular) * ndl')
  })

  it('mesh-surface shaders shade a flat epsilon accent near the section plane', () => {
    // The true cap lives in mesh_section_cap; the epsilon band stays as the
    // cut cue for open surfaces, now tinted toward the theme cap color.
    for (const code of [MESH_WGSL, MESH_PBR_WGSL, MESH_MATCAP_WGSL, MESH_TOON_WGSL, MESH_UNLIT_WGSL]) {
      expect(code).toContain(SECTION_CAP_WGSL)
      expect(code).toContain('sc.capColor')
    }
  })

  it('grid shares the themed scene struct and reads its grid color', () => {
    expect(GRID_WGSL).toContain(SCENE_STRUCT)
    expect(GRID_WGSL).toContain('inverseVP: mat4x4f')
    expect(GRID_WGSL).toContain('sc.gridColor')
  })

  it('themed shaders read selection/hover/xray colors from the Scene theme tail', () => {
    expect(SCENE_STRUCT).toContain('selectionColor: vec3f')
    expect(SCENE_STRUCT).toContain('hoverColor: vec3f')
    expect(SCENE_STRUCT).toContain('edgeColor: vec3f')
    expect(SCENE_STRUCT).toContain('xrayColor: vec3f')
    expect(SCENE_STRUCT).toContain('gridColor: vec3f')
    expect(SCENE_STRUCT).toContain('capColor: vec3f')
    for (const code of [MESH_WGSL, MESH_PBR_WGSL, MESH_MATCAP_WGSL, MESH_TOON_WGSL, MESH_UNLIT_WGSL]) {
      expect(code).toContain('sc.selectionColor')
      expect(code).toContain('sc.hoverColor')
      expect(code).not.toContain('vec3f(1.0, 0.52, 0.06)')
      expect(code).not.toContain('vec3f(0.12, 0.78, 1.0)')
    }
    expect(DEEP_MESH_WGSL).toContain('sc.xrayColor')
    expect(DEEP_MESH_WGSL).not.toContain('vec3f(1.0, 0.42, 0.06)')
    expect(EDGE_WGSL).toContain('sc.edgeColor')
    expect(EDGE_WGSL).not.toContain('vec3f(0.025, 0.03, 0.04)')
  })

  it('scene uniform layout keeps legacy offsets and appends the theme and shadow tails', () => {
    // vp(16) + eye(4) + light(4) + ambient(4) + section(4) + options(4)
    // + inverseVP(16) + 6 × (vec3 + pad) = 76 floats of legacy head+theme;
    // the shadow tail appends lightVP(16) + shadowParams(4) = 96 floats
    // = 384 bytes, so legacy theme offsets stay unchanged.
    expect(SCENE_UNIFORM_LAYOUT.floats).toBe(96)
    expect(SCENE_UNIFORM_LAYOUT.bytes).toBe(384)
    expect(SCENE_UNIFORM_LAYOUT.themeFloatOffset).toBe(52)
    expect(SCENE_UNIFORM_LAYOUT.themeByteOffset).toBe(52 * 4)
    expect(SCENE_UNIFORM_LAYOUT.hoverFloatOffset).toBe(56)
    expect(SCENE_UNIFORM_LAYOUT.edgeFloatOffset).toBe(60)
    expect(SCENE_UNIFORM_LAYOUT.xrayFloatOffset).toBe(64)
    expect(SCENE_UNIFORM_LAYOUT.gridFloatOffset).toBe(68)
    expect(SCENE_UNIFORM_LAYOUT.capFloatOffset).toBe(72)
    expect(SCENE_UNIFORM_LAYOUT.capByteOffset).toBe(72 * 4)
    // Shadow tail: lightVP (column-major mat4) then shadowParams.
    expect(SCENE_UNIFORM_LAYOUT.lightVPFloatOffset).toBe(76)
    expect(SCENE_UNIFORM_LAYOUT.lightVPByteOffset).toBe(76 * 4)
    expect(SCENE_UNIFORM_LAYOUT.shadowFloatOffset).toBe(92)
    expect(SCENE_UNIFORM_LAYOUT.shadowByteOffset).toBe(92 * 4)
  })

  it('morph blend reads ob.morph.x from the slot-1 attribute in object vertex shaders', () => {
    for (const [, code] of OBJECT_SHADERS) {
      expect(code).toContain('mix(fromPos, pos, ob.morph.x)')
    }
  })

  it('uniform layout matches the Obj struct fields', () => {
    // model(16) + nmat(16) + color(4) + style(4) + morph(4)
    // + baseColor(3) + metallic(1) + emissive(3) + roughness(1) + materialId(1) + pad(3)
    expect(OBJECT_UNIFORM_LAYOUT.floats).toBe(56)
    expect(OBJECT_UNIFORM_LAYOUT.bytes).toBe(224)
    expect(OBJECT_UNIFORM_LAYOUT.styleFloatOffset).toBe(36)
    expect(OBJECT_UNIFORM_LAYOUT.styleByteOffset).toBe(36 * 4)
    expect(OBJECT_UNIFORM_LAYOUT.morphFloatOffset).toBe(40)
    expect(OBJECT_UNIFORM_LAYOUT.morphByteOffset).toBe(40 * 4)
    // Material tail: legacy offsets above are unchanged.
    expect(OBJECT_UNIFORM_LAYOUT.materialFloatOffset).toBe(44)
    expect(OBJECT_UNIFORM_LAYOUT.materialByteOffset).toBe(44 * 4)
    expect(OBJECT_UNIFORM_LAYOUT.metallicFloatOffset).toBe(47)
    expect(OBJECT_UNIFORM_LAYOUT.emissiveFloatOffset).toBe(48)
    expect(OBJECT_UNIFORM_LAYOUT.emissiveByteOffset).toBe(48 * 4)
    expect(OBJECT_UNIFORM_LAYOUT.roughnessFloatOffset).toBe(51)
    expect(OBJECT_UNIFORM_LAYOUT.materialIdFloatOffset).toBe(52)
    expect(OBJECT_UNIFORM_LAYOUT.materialIdByteOffset).toBe(52 * 4)
    expect(MESH_VERTEX_STRIDE).toBe(24)
    expect(MORPH_VERTEX_STRIDE).toBe(12)
    expect(MESH_VERTEX_STRIDE / 4 + MESH_VERTEX_STRIDE / 4).toBe(12) // pos3 + norm3 floats
  })
})

describe('shader registry', () => {
  it('registers all twelve shipped shaders with their canonical sources', () => {
    const expected: Array<[string, string, string, boolean]> = [
      ['mesh', MESH_WGSL, 'object', true],
      ['meshPbr', MESH_PBR_WGSL, 'object', true],
      ['meshMatcap', MESH_MATCAP_WGSL, 'object', true],
      ['meshToon', MESH_TOON_WGSL, 'object', true],
      ['meshUnlit', MESH_UNLIT_WGSL, 'object', true],
      ['meshSectionCap', MESH_SECTION_CAP_WGSL, 'object', true],
      ['meshShadow', MESH_SHADOW_WGSL, 'object', false],
      ['deepMesh', DEEP_MESH_WGSL, 'object', true],
      ['edge', EDGE_WGSL, 'object', true],
      ['line', LINE_WGSL, 'line', false],
      ['grid', GRID_WGSL, 'grid', false],
      ['selectionOverlay', SELECTION_OVERLAY_WGSL, 'overlay', false],
    ]
    expect(listShaders().map(spec => spec.id).sort()).toEqual(expected.map(([id]) => id).sort())
    for (const [id, source, kind, supportsVariants] of expected) {
      const spec = getShader(id)
      expect(spec.source).toBe(source)
      expect(spec.kind).toBe(kind)
      expect(spec.supportsVariants).toBe(supportsVariants)
      expect(hasShader(id)).toBe(true)
    }
    expect(hasShader('nope')).toBe(false)
    expect(() => getShader('nope')).toThrow('unknown shader')
  })

  it('registry descriptors pin the pipeline defaults the renderer used to hand-wire', () => {
    expect(getShader('mesh')).toMatchObject({ blend: 'none', topology: 'triangle-list', vertexLayout: 'mesh',
      depth: { writeEnabled: true, compare: 'less' } })
    // Alternate mesh shading models share the mesh pipeline defaults.
    for (const id of ['meshPbr', 'meshMatcap', 'meshToon', 'meshUnlit']) {
      expect(getShader(id)).toMatchObject({ blend: 'none', topology: 'triangle-list', vertexLayout: 'mesh',
        depth: { writeEnabled: true, compare: 'less' } })
    }
    // Only meshMatcap binds the renderer-wide capture texture at group(2).
    expect(getShader('meshMatcap').usesMatcapBinding).toBe(true)
    for (const id of ['mesh', 'meshPbr', 'meshToon', 'meshUnlit', 'meshSectionCap', 'deepMesh', 'edge']) {
      expect(getShader(id).usesMatcapBinding ?? false).toBe(false)
    }
    // Only meshPbr binds the renderer-wide environment map at group(2).
    expect(getShader('meshPbr').usesEnvBinding).toBe(true)
    for (const id of ['mesh', 'meshMatcap', 'meshToon', 'meshUnlit', 'meshSectionCap', 'deepMesh', 'edge']) {
      expect(getShader(id).usesEnvBinding ?? false).toBe(false)
    }
    // Shadow map: every lit surface shader samples the key-light depth map;
    // meshUnlit stays shadow-free and grid participates from its group(1).
    for (const id of ['mesh', 'meshPbr', 'meshMatcap', 'meshToon', 'grid']) {
      expect(getShader(id).usesShadowBinding).toBe(true)
    }
    for (const id of ['meshUnlit', 'meshSectionCap', 'deepMesh', 'edge', 'line', 'selectionOverlay']) {
      expect(getShader(id).usesShadowBinding ?? false).toBe(false)
    }
    // meshShadow is the depth-only pass into the shadow map: no fragment stage,
    // no textual variants.
    expect(getShader('meshShadow').depthOnly).toBe(true)
    expect(getShader('meshShadow').supportsVariants).toBe(false)
    // Section cap: same depth/blend as the opaque surface pass, but front-face
    // culling so only clipped back faces fill the cut surface.
    expect(getShader('meshSectionCap')).toMatchObject({ blend: 'none', topology: 'triangle-list', vertexLayout: 'mesh',
      cullMode: 'front', depth: { writeEnabled: true, compare: 'less' } })
    expect(getShader('deepMesh')).toMatchObject({ blend: 'alpha', topology: 'triangle-list', vertexLayout: 'mesh',
      depth: { writeEnabled: false, compare: 'always' } })
    expect(getShader('edge')).toMatchObject({ blend: 'alpha', topology: 'line-list', vertexLayout: 'edge',
      depth: { writeEnabled: false, compare: 'less-equal' } })
    expect(getShader('line')).toMatchObject({ blend: 'alpha', topology: 'line-list', vertexLayout: 'line',
      depth: { writeEnabled: true, compare: 'less' } })
    expect(getShader('grid')).toMatchObject({ blend: 'alpha', topology: 'triangle-list', vertexLayout: 'grid',
      depth: { writeEnabled: true, compare: 'less' } })
    // Shadow binding flags: the four lit surface shaders + grid sample it;
    // meshShadow is the depth-only writer (no fragment, depth32float target).
    for (const id of ['mesh', 'meshPbr', 'meshMatcap', 'meshToon', 'grid']) {
      expect(getShader(id).usesShadowBinding).toBe(true)
    }
    for (const id of ['meshUnlit', 'meshSectionCap', 'deepMesh', 'edge', 'line', 'selectionOverlay', 'meshShadow']) {
      expect(getShader(id).usesShadowBinding ?? false).toBe(false)
    }
    expect(getShader('meshShadow')).toMatchObject({ depthOnly: true, vertexLayout: 'mesh',
      depth: { writeEnabled: true, compare: 'less' } })
    expect(getShader('selectionOverlay')).toMatchObject({ blend: 'alpha', topology: 'triangle-list', vertexLayout: 'line',
      depth: { writeEnabled: false, compare: 'less-equal' } })
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
