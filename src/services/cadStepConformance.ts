export type StepSemanticConformance =
  | {status:'not-checked';validator:null;detail:string}
  | {status:'externally-conformant';validator:{name:string;version:string;sha256:string};detail:string}
  | {status:'externally-rejected';validator:{name:string;version:string;sha256:string};detail:string}

export const uncheckedStepConformance=():StepSemanticConformance=>({
  status:'not-checked',
  validator:null,
  detail:'Native bounded parsing is not an independent AP242 semantic validation.',
})

/** Refuses unbound validator claims at the product boundary. */
export function parseStepSemanticConformance(value:unknown):StepSemanticConformance {
  if(!value||typeof value!=='object')return uncheckedStepConformance()
  const candidate=value as Partial<StepSemanticConformance>
  if(candidate.status==='not-checked')return uncheckedStepConformance()
  const validator=(candidate as {validator?:{name?:unknown;version?:unknown;sha256?:unknown}}).validator
  if((candidate.status!=='externally-conformant'&&candidate.status!=='externally-rejected')
    ||!validator||typeof validator.name!=='string'||typeof validator.version!=='string'
    ||typeof validator.sha256!=='string'||!/^[0-9a-f]{64}$/.test(validator.sha256)
    ||typeof candidate.detail!=='string')return uncheckedStepConformance()
  return candidate as StepSemanticConformance
}

export interface StepHeaderMetadata {
  schema: string | null
  fileName: string | null
  author: string | null
  organization: string | null
  entityCount: number
  hasAdvancedBrep: boolean
  hasTessellatedShape: boolean
}

function unquoteStepString(raw: string | undefined): string | null {
  if (!raw) return null
  const trimmed = raw.trim()
  if (trimmed === '$' || trimmed === "''") return null
  const match = /^'((?:''|[^'])*)'$/.exec(trimmed)
  if (!match) return null
  const value = match[1]!.replace(/''/g, "'").trim()
  return value.length > 0 ? value : null
}

function extractFirstStepTupleString(tupleBody: string | undefined): string | null {
  if (!tupleBody) return null
  const match = /'((?:''|[^'])*)'/.exec(tupleBody)
  if (!match) return null
  const value = match[1]!.replace(/''/g, "'").trim()
  return value.length > 0 ? value : null
}

/**
 * Quickly audits ISO-10303-21 STEP AP203/AP214/AP242 header metadata (`FILE_SCHEMA`, `FILE_NAME`),
 * DATA section entity instance counts, and presence of `ADVANCED_BREP_SHAPE_REPRESENTATION` /
 * `MANIFOLD_SOLID_BREP` vs `TESSELLATED_SHAPE_REPRESENTATION` / `TRIANGULATED_FACE`.
 */
export function inspectStepHeaderMetadata(stepText: string): StepHeaderMetadata {
  if (typeof stepText !== 'string' || stepText.length === 0) {
    return {
      schema: null,
      fileName: null,
      author: null,
      organization: null,
      entityCount: 0,
      hasAdvancedBrep: false,
      hasTessellatedShape: false,
    }
  }

  // Strip block comments /* ... */ for reliable header & entity inspection
  const cleaned = stepText.replace(/\/\*[\s\S]*?\*\//g, ' ')

  let schema: string | null = null
  const schemaMatch = /FILE_SCHEMA\s*\(\s*\(\s*'((?:''|[^'])*)'/i.exec(cleaned)
  if (schemaMatch) {
    const rawSchema = schemaMatch[1]!.replace(/''/g, "'").trim()
    schema = rawSchema.length > 0 ? rawSchema : null
  }

  let fileName: string | null = null
  let author: string | null = null
  let organization: string | null = null

  // FILE_NAME ( 'name', 'time_stamp', ('author', ...), ('organization', ...), ... );
  const fileNameMatch = /FILE_NAME\s*\(\s*('(?:''|[^'])*'|\$)\s*,\s*('(?:''|[^'])*'|\$)\s*,\s*\(([^)]*)\)\s*,\s*\(([^)]*)\)/i.exec(
    cleaned,
  )
  if (fileNameMatch) {
    fileName = unquoteStepString(fileNameMatch[1])
    author = extractFirstStepTupleString(fileNameMatch[3])
    organization = extractFirstStepTupleString(fileNameMatch[4])
  } else {
    const simpleNameMatch = /FILE_NAME\s*\(\s*('(?:''|[^'])*'|\\$)/i.exec(cleaned)
    if (simpleNameMatch) {
      fileName = unquoteStepString(simpleNameMatch[1])
    }
  }

  // Count numbered entity instances `#123 = ...` in the STEP file
  const entityMatches = cleaned.match(/(?:^|[\r\n;])\s*#\d+\s*=/g)
  const entityCount = entityMatches ? entityMatches.length : 0

  const hasAdvancedBrep =
    /\b(?:ADVANCED_BREP_SHAPE_REPRESENTATION|MANIFOLD_SOLID_BREP|ADVANCED_FACE)\b/i.test(cleaned)
  const hasTessellatedShape =
    /\b(?:TESSELLATED_SHAPE_REPRESENTATION|TESSELLATED_SOLID|TESSELLATED_SHELL|TRIANGULATED_FACE|COMPLEX_TRIANGULATED_SURFACE)\b/i.test(
      cleaned,
    )

  return {
    schema,
    fileName,
    author,
    organization,
    entityCount,
    hasAdvancedBrep,
    hasTessellatedShape,
  }
}
