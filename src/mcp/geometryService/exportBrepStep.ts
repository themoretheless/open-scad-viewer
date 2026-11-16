import type { MeshData } from '../../core/mesh'
import { isNativeGeometryArtifact, type NativeGeometryArtifact } from '../../core/nativeGeometry'
import { exportDirectStepV9 } from '../../kernel/cadNurbsStep'
import { inspectNurbsBrep, transformNurbsBrep, type NurbsBrep } from '../../kernel/geometry/brep'
import { warmGeometryKernel } from '../../kernel/geometry/kernel'
import { readNativeGeometryData } from '../../kernel/nativeGeometrySnapshot'
import { inspectProgressiveSweepSolidAdmission } from '../../kernel/sweepSolidAdmission'
import { ArtifactSizeError, InvalidGeometryError } from '../geometryService'

const utf8Encoder = new TextEncoder()

function isEmptyBrep(model: NurbsBrep): boolean {
  return (
    model.vertices.length === 0 &&
    model.edges.length === 0 &&
    model.loops.length === 0 &&
    model.faces.length === 0 &&
    model.shells.length === 0 &&
    model.bodies.length === 0
  )
}

function isIdentityMatrix4(transform: Float32Array): boolean {
  return transform.length === 16 && transform.every((value, index) => value === (index % 5 === 0 ? 1 : 0))
}

function transformMatrixRows(transform: Float32Array): number[][] {
  return Array.from({ length: 4 }, (_, row) => Array.from(transform.slice(row * 4, row * 4 + 4)))
}

function quotedStepString(value: string): string {
  let out = "'"
  for (const ch of value) {
    const code = ch.codePointAt(0)!
    if (ch === "'") out += "''"
    else if (code >= 32 && code <= 126 && ch !== '\\') out += ch
    else if (code <= 0xffff) out += `\\X2\\${code.toString(16).padStart(4, '0').toUpperCase()}\\X0\\`
    else out += `\\X4\\${code.toString(16).padStart(8, '0').toUpperCase()}\\X0\\`
  }
  return `${out}'`
}

function remapStepEntities(text: string, offset: number): string {
  return text.replace(/'(?:[^']|'')*'|#(\d+)/g, (token, id: string | undefined) =>
    id === undefined ? token : `#${Number(id) + offset}`,
  )
}

function assembleMultiBodyStep(breps: readonly NurbsBrep[]): string {
  const rows: string[] = []
  let nextId = 1
  const emit = (row: string) => {
    const id = nextId++
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
  const origin = emit("CARTESIAN_POINT('',(0.,0.,0.))")
  const axis = emit(`AXIS2_PLACEMENT_3D('',#${origin},$,$)`)
  const transformation = emit(`ITEM_DEFINED_TRANSFORMATION('','',#${axis},#${axis})`)
  const createProduct = (id: string, name: string) => {
    const prod = emit(`PRODUCT(${quotedStepString(id)},${quotedStepString(name)},'',(#${pc}))`)
    const form = emit(`PRODUCT_DEFINITION_FORMATION('1','',#${prod})`)
    const definition = emit(`PRODUCT_DEFINITION('design','',#${form},#${dc})`)
    const shape = emit(`PRODUCT_DEFINITION_SHAPE('','',#${definition})`)
    const representation = emit(`SHAPE_REPRESENTATION('',(#${axis}),#${context})`)
    emit(`SHAPE_DEFINITION_REPRESENTATION(#${shape},#${representation})`)
    return { definition, representation }
  }
  const root = createProduct('openscad-brep-scene', 'OpenSCAD B-rep Scene')

  for (let index = 0; index < breps.length; index++) {
    const bodyId = `body-${index + 1}`
    const bodyName = `Body ${index + 1}`
    const exported = exportDirectStepV9(breps[index]!).text
    const dataSection = exported.split('\nDATA;\n')[1]?.split('\nENDSEC;')[0]
    if (!dataSection) throw new InvalidGeometryError('Unexpected native STEP export format.')
    const mapped = remapStepEntities(dataSection, nextId - 1)
    const definition = Number(mapped.match(/^#(\d+)=PRODUCT_DEFINITION\(/m)?.[1])
    const representation = Number(
      mapped.match(/^#(\d+)=(?:ADVANCED_BREP|MANIFOLD_SURFACE)_SHAPE_REPRESENTATION\(/m)?.[1],
    )
    if (!definition || !representation) {
      throw new InvalidGeometryError('Native STEP export is missing its product representation.')
    }
    const renamed = mapped.replace(
      /PRODUCT\('model','model'/,
      () => `PRODUCT(${quotedStepString(bodyId)},${quotedStepString(bodyName)}`,
    )
    rows.push(renamed)
    for (const match of mapped.matchAll(/^#(\d+)=/gm)) {
      nextId = Math.max(nextId, Number(match[1]) + 1)
    }
    const usage = emit(
      `NEXT_ASSEMBLY_USAGE_OCCURRENCE(${quotedStepString(bodyId)},${quotedStepString(bodyName)},'',#${root.definition},#${definition},$)`,
    )
    const usageShape = emit(`PRODUCT_DEFINITION_SHAPE('','',#${usage})`)
    const relation = emit(
      `(REPRESENTATION_RELATIONSHIP('','',#${representation},#${root.representation})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#${transformation})SHAPE_REPRESENTATION_RELATIONSHIP())`,
    )
    emit(`CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#${relation},#${usageShape})`)
  }

  return (
    'ISO-10303-21;\nHEADER;\n' +
    "FILE_DESCRIPTION(('OpenSCAD B-rep assembly'),'2;1');\n" +
    "FILE_NAME('model.step','',(''),(''),'OpenSCAD Viewer','OpenSCAD Viewer','');\n" +
    "FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));\n" +
    `ENDSEC;\nDATA;\n${rows.join('\n')}\nENDSEC;\nEND-ISO-10303-21;\n`
  )
}

function hasProgressiveSweepProvenance(native: NativeGeometryArtifact, parsed: Record<string, unknown>): boolean {
  if (
    parsed.sweepEvidence !== undefined ||
    parsed.sweepBodyBoundaryEvidence !== undefined ||
    parsed.sweepStationReconstruction !== undefined ||
    parsed.sweepMiterReplay !== undefined ||
    parsed.sweepPlanarCapFaces !== undefined
  ) {
    return true
  }
  try {
    const doc = JSON.parse(native.documentJson) as { nodes?: unknown } | null
    return Boolean(doc && Array.isArray(doc.nodes))
  } catch {
    return false
  }
}

function extractVerifiedBrep(mesh: MeshData): NurbsBrep | null {
  const native = mesh.nativeGeometry
  if (!native || native.kind !== 'brep') {
    throw new InvalidGeometryError('STEP export requires exact B-rep geometry; mesh conversion to STEP is forbidden.')
  }
  if (!isNativeGeometryArtifact(native)) {
    throw new InvalidGeometryError('Invalid native B-rep snapshot; STEP export aborted.')
  }
  const parsed = readNativeGeometryData(native) as ({ geometry?: NurbsBrep } & Record<string, unknown>) | null
  const model = parsed?.geometry
  if (!parsed || !model) {
    throw new InvalidGeometryError('Missing native B-rep geometry payload for STEP export.')
  }
  inspectNurbsBrep(model)
  if (isEmptyBrep(model)) return null
  const brep = isIdentityMatrix4(mesh.transform)
    ? model
    : transformNurbsBrep(model, transformMatrixRows(mesh.transform))
  if (hasProgressiveSweepProvenance(native, parsed)) {
    inspectProgressiveSweepSolidAdmission(native, brep)
  }
  return brep
}

export async function exportCompiledBrepStep(meshes: readonly MeshData[], maxBytes: number): Promise<Uint8Array> {
  await warmGeometryKernel()
  const breps: NurbsBrep[] = []
  for (const mesh of meshes) {
    const brep = extractVerifiedBrep(mesh)
    if (brep) breps.push(brep)
  }
  if (breps.length === 0) {
    throw new InvalidGeometryError('STEP export requires at least one non-empty B-rep solid.')
  }
  const stepText = breps.length === 1 ? exportDirectStepV9(breps[0]!).text : assembleMultiBodyStep(breps)
  const encoded = utf8Encoder.encode(stepText)
  if (encoded.byteLength > maxBytes) {
    throw new ArtifactSizeError(encoded.byteLength, maxBytes)
  }
  return encoded
}
