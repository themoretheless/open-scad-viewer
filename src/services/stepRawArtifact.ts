import {
  preflightStepInput,
  type ParserInputBudgets,
  type ParserPreflightStats,
} from './parserInputBudgets'

export interface RawStepEntity {
  readonly id: number
  readonly type: string
  readonly args: string
  readonly source: string
  readonly references: readonly number[]
}

export interface StepRawDiagnostic {
  readonly code: 'duplicate-entity' | 'dangling-reference'
  readonly entityId: number
  readonly referenceId?: number
  readonly message: string
}

/** Pass 1: syntax and references only; no units, topology, or geometry choices. */
export interface StepRawArtifact {
  readonly version: 1
  readonly format: 'step-part21'
  readonly source: string
  readonly entities: readonly RawStepEntity[]
  readonly entityIndex: Readonly<Record<number, RawStepEntity>>
  readonly diagnostics: readonly StepRawDiagnostic[]
  readonly stats: ParserPreflightStats
}

function references(args: string): readonly number[] {
  const withoutStrings = args.replace(/'(?:[^']|'')*'/g, "''")
  return Object.freeze([...withoutStrings.matchAll(/#(\d+)/g)].map(match => Number(match[1])))
}

export function buildStepRawArtifact(
  text: string,
  budgets?: ParserInputBudgets,
): StepRawArtifact {
  const stats = preflightStepInput(text, budgets)
  const entities: RawStepEntity[] = []
  const index: Record<number, RawStepEntity> = {}
  const diagnostics: StepRawDiagnostic[] = []
  const expression = /#(\d+)\s*=\s*([A-Z_0-9]+)\s*\(((?:[^']|'(?:[^']|'')*')*?)\)\s*;/gis
  for (const match of text.matchAll(expression)) {
    const id = Number(match[1]), type = match[2]!.toUpperCase(), args = match[3]!
    const entity: RawStepEntity = Object.freeze({
      id, type, args, source: match[0], references: references(args),
    })
    if (index[id]) {
      diagnostics.push(Object.freeze({
        code: 'duplicate-entity' as const,
        entityId: id,
        message: `Duplicate STEP entity #${id}`,
      }))
      continue
    }
    entities.push(entity)
    index[id] = entity
  }
  for (const entity of entities) {
    for (const referenceId of entity.references) {
      if (!index[referenceId]) diagnostics.push(Object.freeze({
        code: 'dangling-reference' as const,
        entityId: entity.id,
        referenceId,
        message: `STEP entity #${entity.id} references missing #${referenceId}`,
      }))
    }
  }
  return Object.freeze({
    version: 1 as const,
    format: 'step-part21' as const,
    source: text,
    entities: Object.freeze(entities),
    entityIndex: Object.freeze(index),
    diagnostics: Object.freeze(diagnostics),
    stats,
  })
}

export function rawStepEntity(artifact: StepRawArtifact, id: string | number): RawStepEntity | undefined {
  return artifact.entityIndex[Number(id)]
}

/** Diagnostics do not silently erase malformed identity at the semantic boundary. */
export function assertStepRawIdentity(artifact: StepRawArtifact): void {
  const duplicate = artifact.diagnostics.find(diagnostic => diagnostic.code === 'duplicate-entity')
  if (duplicate) throw new Error(duplicate.message)
}
