import { EXECUTION_BY_ERROR } from '../geometryBuildEngine'
import type { GeometryManifestRevocationRecord } from '../geometryBuildEngine'
import type { GeometryEngineManifest, GeometryExecutionDescriptor } from '../../core/geometryExecution'
import type { GeometryEvaluationResult } from '../../core/build'
import { parseOpenSCAD } from '../openscadParser'
import { sha256Hex } from '../../core/sha256'

export function attachGeometryExecutionToError(
  error: unknown,
  execution: GeometryExecutionDescriptor,
): void {
  if (error !== null && (typeof error === 'object' || typeof error === 'function')
    && !EXECUTION_BY_ERROR.has(error)) {
    EXECUTION_BY_ERROR.set(error, execution)
  }
}

export function geometryExecutionForError(error: unknown): GeometryExecutionDescriptor | undefined {
  return error !== null && (typeof error === 'object' || typeof error === 'function')
    ? EXECUTION_BY_ERROR.get(error)
    : undefined
}

export function revocationAttestationPreimage(
  record: Omit<GeometryManifestRevocationRecord, 'attestationSha256'>,
): string {
  return [
    'geometry-manifest-revocation-v1',
    record.schemaVersion,
    record.epoch,
    record.manifestDigest,
    record.reason,
    record.effectiveAt,
    record.authority,
  ].join('\0')
}

export function createGeometryManifestRevocationRecord(
  record: Omit<GeometryManifestRevocationRecord, 'schemaVersion' | 'attestationSha256'>,
): GeometryManifestRevocationRecord {
  const unsigned = { schemaVersion: 1 as const, ...record }
  return Object.freeze({
    ...unsigned,
    attestationSha256: sha256Hex(revocationAttestationPreimage(unsigned)),
  })
}

export function cloneManifest(manifest: GeometryEngineManifest): GeometryEngineManifest {
  return {
    ...manifest,
    languageContracts: [...manifest.languageContracts],
    capabilities: [...manifest.capabilities],
    plannedCapabilities: [...manifest.plannedCapabilities],
    qualities: [...manifest.qualities],
    representations: [...manifest.representations],
    plannedRepresentations: [...manifest.plannedRepresentations],
    exportFormats: [...manifest.exportFormats],
    plannedExportFormats: [...manifest.plannedExportFormats],
    limits: { ...manifest.limits },
    qualification: { ...manifest.qualification },
    dependency: { ...manifest.dependency },
    rollbackCompatibility: { ...manifest.rollbackCompatibility },
  }
}

/**
 * Evaluates source with exact-solid recording, for the Solid workspace.
 *
 * The Solid worker records and builds the graph within its own realm.
 * Callers must not reach the parser
 * directly: keeping every parser consumer behind this facade is what the architecture
 * test in tests/coreMesh.test.ts pins.
 */
export function evaluateExactSolids(source: string): Promise<GeometryEvaluationResult> {
  return parseOpenSCAD(source, { recordExactSolids: true })
}
