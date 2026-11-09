import {
  UntrustedParserWorkerClient,
  type UntrustedParserRunOptions,
} from './untrustedParserWorker'
import { buildStepRawArtifact, type StepRawArtifact } from './stepRawArtifact'

const shared = new UntrustedParserWorkerClient(() =>
  new Worker(new URL('../workers/untrustedParser.worker.ts', import.meta.url), { type: 'module' }))

export function cancelUntrustedParser(): void { shared.cancel() }

/** Raw STEP syntax/ref pass runs in a worker; semantic geometry consumes the returned artifact explicitly. */
export async function parseUntrustedStepRaw(
  text: string,
  options: UntrustedParserRunOptions = {},
): Promise<StepRawArtifact> {
  const artifact = await shared.run({ kind: 'step-raw', text }, options)
  // Structured clone does not preserve Object.freeze. Re-freeze without reparsing
  // on the host; all untrusted syntax work remains behind the worker boundary.
  return buildStepRawArtifactFromClone(artifact)
}

function buildStepRawArtifactFromClone(artifact: StepRawArtifact): StepRawArtifact {
  const entities = artifact.entities.map(entity => Object.freeze({
    ...entity,
    references: Object.freeze([...entity.references]),
  }))
  const entityIndex: Record<number, StepRawArtifact['entities'][number]> = {}
  for (const entity of entities) entityIndex[entity.id] = entity
  return Object.freeze({
    ...artifact,
    entities: Object.freeze(entities),
    entityIndex: Object.freeze(entityIndex),
    diagnostics: Object.freeze(artifact.diagnostics.map(diagnostic => Object.freeze({ ...diagnostic }))),
    stats: Object.freeze({ ...artifact.stats }),
  })
}

/** Node/headless corpus code uses the same pass directly; browser callers use the worker API above. */
export const parseStepRawInProcess = buildStepRawArtifact
