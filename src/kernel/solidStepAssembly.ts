import { validateDirectGroupHierarchy } from '../app/directGroupHierarchy'
import { nestSharedStepGroups } from './solidStepGroupHierarchy'
import type { DirectDocument } from '../app/directModeling'
import { warmGeometryKernel } from './geometry/kernel'
import { solidMaterialRgb } from './solidMaterial'

// Part 21 transport only: all geometry is exported and validated by the Rust kernel.
const quoted = (value: string) => {
  let out = "'"
  for (const c of value) {
    const n = c.codePointAt(0)!
    if (c === "'") out += "''"
    else if (n >= 32 && n <= 126 && c !== '\\') out += c
    else
      out +=
        n <= 0xffff
          ? `\\X2\\${n.toString(16).padStart(4, '0').toUpperCase()}\\X0\\`
          : `\\X4\\${n.toString(16).padStart(8, '0').toUpperCase()}\\X0\\`
  }
  return out + "'"
}

/** Remap entity references without interpreting hashes inside STEP string literals. */
function remap(text: string, offset: number) {
  return text.replace(/'(?:[^']|'')*'|#(\d+)/g, (token, id: string | undefined) =>
    id === undefined ? token : `#${Number(id) + offset}`,
  )
}

/** Attach Solid identifiers to the native writer's flat assembly records. */
function labelSharedAssembly(text: string, definitions: DirectDocument['bodies'], bodies: DirectDocument['bodies']) {
  return text.replace(
    /^(#\d+=)(?:PRODUCT\('(?:model|gear-(\d+))','(?:model|Gear \d+)'|PRODUCT\('planet-set','Planet gear set'|NEXT_ASSEMBLY_USAGE_OCCURRENCE\('gear-occurrence-(\d+)','Gear occurrence \d+')/gm,
    (record, prefix: string, definition: string | undefined, occurrence: string | undefined) => {
      if (occurrence !== undefined) {
        const body = bodies[Number(occurrence)]!
        return prefix + `NEXT_ASSEMBLY_USAGE_OCCURRENCE(${quoted(body.id)},${quoted(body.name)}`
      }
      if (record.includes("'planet-set'")) return prefix + "PRODUCT('solid-scene','Solid scene'"
      const body = definitions[Number(definition ?? 0)]!
      return prefix + `PRODUCT(${quoted(body.id)},${quoted(body.name)}`
    },
  )
}

/** Current Solid hierarchy: root → groups → bodies, including all hidden bodies.
 * Sketches/curves/surfaces are refused rather than silently omitted.
 */
export async function exportSolidStepAssembly(document: DirectDocument): Promise<string> {
  if (document.sourceBodies?.length)
    throw Error('STEP assembly cannot yet transfer exact source Body restrictions. Keep the native document.')
  const snapshot = structuredClone(document)
  if (!snapshot.bodies.length) throw Error('STEP assembly requires at least one body.')
  if (new Set(snapshot.bodies.map(body => body.id)).size !== snapshot.bodies.length)
    throw Error('STEP assembly requires unique body IDs.')
  if (snapshot.sketches.length || snapshot.curves?.length || snapshot.surfaces?.length)
    throw Object.assign(
      Error(
        'STEP assembly currently requires a body-only document. Export sketches separately as SVG and remove auxiliary curves/surfaces from a copy.',
      ),
      { code: 'DIRECT_STEP_ASSEMBLY_CONTENT' },
    )
  const missing = snapshot.bodies.filter(body => !body.brep)
  if (missing.length) throw Error(`STEP assembly requires exact B-rep geometry: ${missing.map(b => b.name).join(', ')}`)
  await warmGeometryKernel()
  const { exportDirectStepV9, exportSharedStepAssembly } = await import('./cadNurbsStep')
  validateDirectGroupHierarchy(snapshot.groups ?? [])
  const groupNames = [...new Set([
    ...(snapshot.groups ?? []).map(group => group.name),
    ...snapshot.bodies.flatMap(body => body.group ? [body.group] : []),
  ])]
  if (snapshot.bodies.some(body => body.instance)) {
    const definitions = snapshot.bodies.filter(body => !body.instance)
    const indices = new Map(definitions.map((body, i) => [body.id, i]))
    const identity = [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]
    const placements = snapshot.bodies.map(body => ({
      definition: indices.get(body.instance?.sourceId ?? body.id) ?? -1,
      transform: body.instance?.matrix ?? identity,
    }))
    let shared: string | undefined
    try {
      const materialCount = snapshot.bodies.filter(body => body.material).length
      const hierarchy = groupNames.length || materialCount ? {
        groups: groupNames,
        ...(materialCount > 0 && { colours: snapshot.bodies.map(({ material }) => material
          ? { name: material.name, rgb: solidMaterialRgb(material.color) }
          : null) }),
        parents: snapshot.bodies.map(body => body.group ? groupNames.indexOf(body.group) : null),
      } : undefined
      const text = exportSharedStepAssembly(definitions.map(body => body.brep!), placements, hierarchy)
      // Older kernels accept unknown request fields and can return a flat assembly.
      const retainsColours = !materialCount || (text.match(/^#\d+=PRESENTATION_STYLE_BY_CONTEXT\(/gm)?.length ?? 0) === materialCount
      if (retainsColours && (!hierarchy || (text.match(/^#\d+=PRODUCT\('group-\d+',/gm)?.length ?? 0) === groupNames.length))
        shared = text
    } catch {
      // Open shells, shear and other unsupported placements can still export
      // through the independently materialized current bodies below.
    }
    if (shared) {
      const labeledGroups = nestSharedStepGroups(shared, groupNames, snapshot.groups ?? []).replace(
        /^(#\d+=(?:PRODUCT|NEXT_ASSEMBLY_USAGE_OCCURRENCE)\()'group-(\d+)'/gm,
        (_, prefix: string, index: string) => prefix + quoted('group:' + groupNames[Number(index)]),
      )
      return labelSharedAssembly(labeledGroups, definitions, snapshot.bodies)
    }
  }
  const rows: string[] = []
  let next = 1
  const emit = (row: string) => {
    const id = next++
    rows.push(`#${id}=${row};`)
    return id
  }
  const app = emit("APPLICATION_CONTEXT('managed model based 3d engineering')")
  const pc = emit(`PRODUCT_CONTEXT('',#${app},'mechanical')`)
  const dc = emit(`PRODUCT_DEFINITION_CONTEXT('part definition',#${app},'design')`)
  const unit = emit('(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))')
  const context = emit(
    `(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#${unit}))REPRESENTATION_CONTEXT('','3D'))`,
  )
  const point = emit("CARTESIAN_POINT('',(0.,0.,0.))")
  const axis = emit(`AXIS2_PLACEMENT_3D('',#${point},$,$)`)
  const transformation = emit(`ITEM_DEFINED_TRANSFORMATION('','',#${axis},#${axis})`)
  const product = (id: string, name: string) => {
    const p = emit(`PRODUCT(${quoted(id)},${quoted(name)},'',(#${pc}))`)
    const f = emit(`PRODUCT_DEFINITION_FORMATION('1','',#${p})`)
    const definition = emit(`PRODUCT_DEFINITION('design','',#${f},#${dc})`)
    const shape = emit(`PRODUCT_DEFINITION_SHAPE('','',#${definition})`)
    const representation = emit(`SHAPE_REPRESENTATION('',(#${axis}),#${context})`)
    emit(`SHAPE_DEFINITION_REPRESENTATION(#${shape},#${representation})`)
    return { definition, representation }
  }
  const root = product('solid-scene', 'Solid scene')
  type Node = typeof root
  const attach = (parent: Node, child: Node, id: string, name: string) => {
    const usage = emit(
      `NEXT_ASSEMBLY_USAGE_OCCURRENCE(${quoted(id)},${quoted(name)},'',#${parent.definition},#${child.definition},$)`,
    )
    const shape = emit(`PRODUCT_DEFINITION_SHAPE('','',#${usage})`)
    const relation = emit(
      `(REPRESENTATION_RELATIONSHIP('','',#${child.representation},#${parent.representation})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#${transformation})SHAPE_REPRESENTATION_RELATIONSHIP())`,
    )
    emit(`CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#${relation},#${shape})`)
  }
  const groups = new Map<string, Node>()
  for (const name of groupNames) {
    const group = product(`group:${name}`, name)
    groups.set(name, group)
  }
  const parents = new Map(snapshot.groups?.map(group => [group.name, group.parent]))
  for (const [name, group] of groups) {
    const parent = parents.get(name)
    attach(parent ? groups.get(parent)! : root, group, `group:${name}`, name)
  }
  for (const body of snapshot.bodies) {
    const exported = exportDirectStepV9(body.brep!).text
    const data = exported.split('\nDATA;\n')[1]?.split('\nENDSEC;')[0]
    if (!data) throw Error('Unexpected native STEP export format.')
    const offset = next - 1
    const mapped = remap(data, offset)
    const definition = Number(mapped.match(/^#(\d+)=PRODUCT_DEFINITION\(/m)?.[1])
    const representation = Number(
      mapped.match(/^#(\d+)=(?:ADVANCED_BREP|MANIFOLD_SURFACE)_SHAPE_REPRESENTATION\(/m)?.[1],
    )
    if (!definition || !representation) throw Error('Native STEP export is missing its product representation.')
    const renamed = mapped.replace(/PRODUCT\('model','model'/, () => `PRODUCT(${quoted(body.id)},${quoted(body.name)}`)
    rows.push(renamed)
    for (const match of mapped.matchAll(/^#(\d+)=/gm)) next = Math.max(next, Number(match[1]) + 1)
    if (body.material) {
      const color = emit(
        `COLOUR_RGB(${quoted(body.material.name)},${solidMaterialRgb(body.material.color)
          .map(c => (c === 0 ? '0.' : c === 1 ? '1.' : String(c)))
          .join(',')})`,
      )
      const fillColor = emit(`FILL_AREA_STYLE_COLOUR('',#${color})`)
      const fill = emit(`FILL_AREA_STYLE('',(#${fillColor}))`)
      const surfaceFill = emit(`SURFACE_STYLE_FILL_AREA(#${fill})`)
      const side = emit(`SURFACE_SIDE_STYLE('',(#${surfaceFill}))`)
      const usage = emit(`SURFACE_STYLE_USAGE(.BOTH.,#${side})`)
      const assignment = emit(`PRESENTATION_STYLE_ASSIGNMENT((#${usage}))`)
      const items = Array.from(
        mapped.matchAll(/^#(\d+)=(?:MANIFOLD_SOLID_BREP|BREP_WITH_VOIDS|SHELL_BASED_SURFACE_MODEL)\(/gm),
        m => emit(`STYLED_ITEM('',(#${assignment}),#${m[1]})`),
      )
      if (items.length)
        emit(
          `MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',(${items.map(id => '#' + id).join(',')}),#${context})`,
        )
    }
    attach(body.group ? groups.get(body.group)! : root, { definition, representation }, body.id, body.name)
  }
  return (
    "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('Current Solid assembly'),'2;1');\nFILE_NAME('scene.step','',(''),(''),'OpenSCAD Viewer','OpenSCAD Viewer','');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));\nENDSEC;\nDATA;\n" +
    rows.join('\n') +
    '\nENDSEC;\nEND-ISO-10303-21;\n'
  )
}
