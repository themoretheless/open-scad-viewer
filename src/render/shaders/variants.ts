/**
 * Textual shader variants derived from the single-object shaders.
 *
 * Both transforms pin structural anchors and throw when a shader module
 * drifts from the contract, so a broken composition fails at pipeline
 * creation, not at draw time.
 */

/**
 * Per-draw style without a uniform buffer rewrite: `ob.style` is served from
 * the `immediate_address_space` instead of the Obj uniform.
 */
export function immediateObjectShader(source: string) {
  if (!source.includes('var<uniform> ob: Obj;'))
    throw new Error('Immediate shader contract changed: missing object uniform')
  return (
    `requires immediate_address_space;\nvar<immediate> im_style: vec4f;\nfn objectStyle() -> vec4f { return im_style; }\n` +
    source.replaceAll('ob.style', 'objectStyle()')
  )
}

export function supportsImmediateAddressSpace() {
  return (
    typeof navigator !== 'undefined' &&
    !!navigator.gpu &&
    navigator.gpu.wgslLanguageFeatures?.has('immediate_address_space') === true
  )
}

/**
 * Per-instance Obj records from a read-only storage buffer. `output` names
 * the vertex-output struct ('V' for mesh/deep shaders, 'EdgeV' for edges) and
 * doubles as the marker for the shader family.
 */
export function instancedObjectShader(source: string, output: 'V' | 'EdgeV') {
  const replacements: Array<[string, string]> = [
    ['var<uniform> ob: Obj;', 'var<storage, read> objects: array<Obj>;'],
    [`struct ${output} {`, `struct ${output} { @location(2) @interpolate(flat) instance: u32,`],
    ['@vertex fn vs(', '@vertex fn vs(@builtin(instance_index) instance: u32, '],
    [` -> ${output} {`, ` -> ${output} {\n  let ob = objects[instance];`],
    [`return ${output}(`, `return ${output}(instance, `],
    [
      `@fragment fn fs(v: ${output}) -> @location(0) vec4f {`,
      `@fragment fn fs(v: ${output}) -> @location(0) vec4f {\n  let ob = objects[v.instance];`,
    ],
  ]
  for (const [from, to] of replacements) {
    if (source.split(from).length !== 2) throw new Error(`Instanced shader contract changed: ${from}`)
    source = source.replace(from, to)
  }
  return source
}

/**
 * Instance-step variant of a mesh-surface shader for `InstancedSceneBuffers`.
 * The model matrix comes from a second vertex buffer stepped per instance
 * (locations 3..6, column-major), color and style (alpha, selected, edge,
 * hovered) from locations 7/8; color and style reach the fragment stage as
 * flat varyings. The normal matrix is the sign-corrected cofactor of the
 * model's upper 3x3, which equals the inverse transpose up to a positive scale
 * (the shaders normalize it). Instances never morph, so the morph input is
 * dropped, and the material tail takes the uniform path's defaults. The shadow
 * map moves from group(2) to group(1): the pipeline layout is scene + shadow,
 * with no per-entity object group. Morph and LOD entities keep the uniform
 * variant.
 */
export function instanceStepObjectShader(source: string) {
  const replacements: Array<[string, string]> = [
    ['@group(1) @binding(0) var<uniform> ob: Obj;', INSTANCE_STEP_PRELUDE],
    [
      'struct V {',
      'struct V { @location(2) @interpolate(flat) icolor: vec4f, @location(3) @interpolate(flat) istyle: vec4f,',
    ],
    [
      '@location(2) fromPos: vec3f) -> V {',
      'inst: InstanceIn) -> V {\n  let ob = instanceObj(mat4x4f(inst.m0, inst.m1, inst.m2, inst.m3), inst.color, inst.style);',
    ],
    ['mix(fromPos, pos, ob.morph.x)', 'pos'],
    ['return V(', 'return V(ob.color, ob.style, '],
    [
      '@fragment fn fs(v: V) -> @location(0) vec4f {',
      '@fragment fn fs(v: V) -> @location(0) vec4f {\n  let ob = instanceObj(mat4x4f(), v.icolor, v.istyle);',
    ],
  ]
  for (const [from, to] of replacements) {
    if (source.split(from).length !== 2) throw new Error(`Instance-step shader contract changed: ${from}`)
    source = source.replace(from, to)
  }
  if (source.includes('fromPos')) throw new Error('Instance-step shader contract changed: morph input still referenced')
  return source.replaceAll('@group(2) @binding(', '@group(1) @binding(')
}

const INSTANCE_STEP_PRELUDE = `struct InstanceIn { @location(3) m0: vec4f, @location(4) m1: vec4f, @location(5) m2: vec4f, @location(6) m3: vec4f, @location(7) color: vec4f, @location(8) style: vec4f }
fn instanceObj(model: mat4x4f, color: vec4f, style: vec4f) -> Obj {
  let a = model[0].xyz;
  let b = model[1].xyz;
  let c = model[2].xyz;
  let s = select(1.0, -1.0, dot(a, cross(b, c)) < 0.0);
  let nmat = mat4x4f(vec4f(cross(b, c) * s, 0.0), vec4f(cross(c, a) * s, 0.0), vec4f(cross(a, b) * s, 0.0), vec4f(0.0, 0.0, 0.0, 1.0));
  return Obj(model, nmat, color, style, vec4f(1.0, 0.0, 0.0, 0.0), vec3f(1.0), 0.0, vec3f(0.0), 0.7, 0.0);
}`

function applyContract(source: string, label: string, replacements: Array<[string, string]>) {
  for (const [from, to] of replacements) {
    if (source.split(from).length !== 2) throw new Error(`Instance-step ${label} shader contract changed: ${from}`)
    source = source.replace(from, to)
  }
  if (source.includes('fromPos'))
    throw new Error(`Instance-step ${label} shader contract changed: morph input still referenced`)
  return source
}

const INSTANCE_OBJ = 'let ob = instanceObj(mat4x4f(inst.m0, inst.m1, inst.m2, inst.m3), inst.color, inst.style);'

/**
 * Instance-step variant of the edge shader. The edge vertex layout keeps the
 * position at slot 0 (location 0); the instance record sits at slot 1
 * (locations 3..8, as in the surface variant). Color and style reach the
 * fragment stage as flat varyings; the edge shader reads only `ob.style`.
 */
export function instanceStepEdgeShader(source: string) {
  return applyContract(source, 'edge', [
    ['@group(1) @binding(0) var<uniform> ob: Obj;', INSTANCE_STEP_PRELUDE],
    [
      'struct EdgeV {',
      'struct EdgeV { @location(1) @interpolate(flat) icolor: vec4f, @location(2) @interpolate(flat) istyle: vec4f,',
    ],
    ['@location(1) fromPos: vec3f) -> EdgeV {', `inst: InstanceIn) -> EdgeV {\n  ${INSTANCE_OBJ}`],
    ['mix(fromPos, pos, ob.morph.x)', 'pos'],
    ['return EdgeV(', 'return EdgeV(ob.color, ob.style, '],
    [
      '@fragment fn fs(v: EdgeV) -> @location(0) vec4f {',
      '@fragment fn fs(v: EdgeV) -> @location(0) vec4f {\n  let ob = instanceObj(mat4x4f(), v.icolor, v.istyle);',
    ],
  ])
}

/**
 * Instance-step variant of the depth-only shadow shader: the model matrix
 * comes from the instance record at slot 1 and the pipeline layout is the
 * scene group alone.
 */
export function instanceStepShadowShader(source: string) {
  return applyContract(source, 'shadow', [
    ['@group(1) @binding(0) var<uniform> ob: Obj;', INSTANCE_STEP_PRELUDE],
    [
      '@location(2) fromPos: vec3f) -> @builtin(position) vec4f {',
      `inst: InstanceIn) -> @builtin(position) vec4f {\n  ${INSTANCE_OBJ}`,
    ],
    ['mix(fromPos, pos, ob.morph.x)', 'pos'],
  ])
}
