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
  if (!source.includes('var<uniform> ob: Obj;')) throw new Error('Immediate shader contract changed: missing object uniform')
  return `requires immediate_address_space;\nvar<immediate> im_style: vec4f;\nfn objectStyle() -> vec4f { return im_style; }\n`
    + source.replaceAll('ob.style', 'objectStyle()')
}

export function supportsImmediateAddressSpace() {
  return typeof navigator !== 'undefined'
    && !!navigator.gpu
    && navigator.gpu.wgslLanguageFeatures?.has('immediate_address_space') === true
}

/**
 * Per-instance Obj records from a read-only storage buffer. `output` names
 * the vertex-output struct ('V' for mesh/deep shaders, 'EdgeV' for edges) and
 * doubles as the marker for the shader family.
 */
export function instancedObjectShader(source: string, output: 'V' | 'EdgeV') {
  const replacements = [
    ['var<uniform> ob: Obj;', 'var<storage, read> objects: array<Obj>;'],
    [`struct ${output} {`, `struct ${output} { @location(2) @interpolate(flat) instance: u32,`],
    ['@vertex fn vs(', '@vertex fn vs(@builtin(instance_index) instance: u32, '],
    [` -> ${output} {`, ` -> ${output} {\n  let ob = objects[instance];`],
    [`return ${output}(`, `return ${output}(instance, `],
    [`@fragment fn fs(v: ${output}) -> @location(0) vec4f {`, `@fragment fn fs(v: ${output}) -> @location(0) vec4f {\n  let ob = objects[v.instance];`],
  ]
  for (const [from, to] of replacements) {
    if (source.split(from).length !== 2) throw new Error(`Instanced shader contract changed: ${from}`)
    source = source.replace(from, to)
  }
  return source
}
