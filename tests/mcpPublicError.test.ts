import { describe, expect, it, vi } from 'vitest'
import { AbortedError, OpenSCADParseError } from '../src/services/openscadParser'
import {
  GeometryBuildEngine,
  GeometryCapabilityUnavailableError,
  GeometryEngineUnavailableError,
  GeometryLanguageContractError,
} from '../src/services/geometryBuildEngine'
import {
  ArtifactSizeError,
  CustomizerValueError,
  GeometryBusyError,
  GeometryDeadlineExceededError,
  InvalidGeometryError,
} from '../src/mcp/geometryService'
import { CatalogQuotaError, ModelRevisionConflictError } from '../src/mcp/modelStore'
import {
  buildDiagnostic,
  ModelNotFoundError,
  PUBLIC_ERROR_CODES,
  publicToolError,
} from '../src/mcp/publicError'

describe('MCP public errors', () => {
  it('maps every frozen public code to its exact retry class', () => {
    const engine = new GeometryBuildEngine()
    const execution = engine.planSource('cube(1);', { quality: 'full', purpose: 'analysis' })
    const cases: Array<[unknown, typeof PUBLIC_ERROR_CODES[number], boolean]> = [
      [new ModelNotFoundError('missing'), 'model_not_found', false],
      [new ModelRevisionConflictError('part', 1, 2), 'revision_conflict', true],
      [new OpenSCADParseError('bad();', 0, 'bad'), 'source_syntax_error', false],
      [new GeometryLanguageContractError('bad contract', 'future/v1', 1), 'language_contract_unsupported', false],
      [new GeometryEngineUnavailableError(execution, 'not deployed'), 'engine_unavailable', false],
      [new GeometryCapabilityUnavailableError(execution, ['nurbs.surfaces']), 'capability_unavailable', false],
      [new ArtifactSizeError(2, 1), 'artifact_too_large', true],
      [new InvalidGeometryError('invalid'), 'invalid_geometry', false],
      [new CustomizerValueError('invalid'), 'invalid_argument', false],
      [new CatalogQuotaError('full', 'models', 1), 'quota_exceeded', false],
      [new GeometryBusyError(10), 'server_busy', true],
      [new GeometryDeadlineExceededError(30_000), 'deadline_exceeded', true],
      [new AbortedError(), 'cancelled', true],
      [new Error('unexpected'), 'internal_error', true],
    ]
    const log = vi.spyOn(console, 'error').mockImplementation(() => undefined)
    const mapped = cases.map(([error, code, retryable]) => {
      const result = publicToolError(error).error
      expect(result, code).toMatchObject({ code, retryable })
      return result.code
    })
    log.mockRestore()
    expect(mapped).toEqual(PUBLIC_ERROR_CODES)
  })

  it('maps expected failures to stable machine-readable recovery contracts', () => {
    expect(publicToolError(new ModelNotFoundError('missing:part')).error).toMatchObject({
      code: 'model_not_found',
      retryable: false,
      details: { model_id: 'missing:part', revision: null },
    })
    expect(publicToolError(new ModelRevisionConflictError('part', 2, 3)).error).toMatchObject({
      code: 'revision_conflict',
      retryable: true,
      details: { model_id: 'part', expected_revision: 2, actual_revision: 3 },
    })
    expect(publicToolError(new ArtifactSizeError(200, 100)).error).toMatchObject({
      code: 'artifact_too_large',
      details: { estimated_bytes: 200, max_bytes: 100 },
    })
    expect(publicToolError(new GeometryBusyError(400)).error).toMatchObject({
      code: 'server_busy',
      retryable: true,
      details: { retry_after_ms: 400 },
    })
    expect(publicToolError(new GeometryDeadlineExceededError(30_000)).error).toMatchObject({
      code: 'deadline_exceeded',
      retryable: true,
      details: { deadline_ms: 30_000 },
    })
    expect(publicToolError(new CustomizerValueError('Parameter size is invalid')).error).toMatchObject({
      code: 'invalid_argument',
      retryable: false,
    })
    expect(publicToolError(new CatalogQuotaError('Catalog full', 'models', 500)).error).toMatchObject({
      code: 'quota_exceeded',
      details: { quota: 'models', limit: 500 },
    })
  })

  it('maps source routing failures without suggesting cross-engine fallback', () => {
    const engine = new GeometryBuildEngine()
    const brepExecution = engine.planSource(
      '// @language openscad-viewer/brep-1\ncube(1);',
      { quality: 'full', purpose: 'analysis' },
    )
    expect(publicToolError(new GeometryEngineUnavailableError(brepExecution, 'not deployed')).error)
      .toMatchObject({
        code: 'engine_unavailable',
        retryable: false,
        details: {
          language_contract: 'openscad-viewer/brep-1',
          engine_class: 'brep',
          automatic_fallback: false,
        },
      })

    const manifoldExecution = engine.planSource('cube(1);', { quality: 'full', purpose: 'analysis' })
    expect(publicToolError(new GeometryCapabilityUnavailableError(
      manifoldExecution,
      ['nurbs.surfaces'],
    )).error).toMatchObject({
      code: 'capability_unavailable',
      details: { engine_class: 'manifold', missing_capabilities: 'nurbs.surfaces' },
    })

    expect(publicToolError(new GeometryLanguageContractError(
      'Unsupported geometry language contract future/unknown.',
      'future/unknown',
      1,
    )).error).toMatchObject({
      code: 'language_contract_unsupported',
      retryable: false,
      details: { reported_contract: 'future/unknown' },
      line: 1,
    })
  })

  it('marks transient provider readiness failures as retryable', () => {
    const engine = new GeometryBuildEngine()
    const execution = engine.planSource('cube(1);', { quality: 'full', purpose: 'analysis' })
    expect(publicToolError(new GeometryEngineUnavailableError(
      execution,
      'readiness timed out',
      'readiness-timeout',
    )).error).toMatchObject({
      code: 'engine_unavailable',
      retryable: true,
      details: { availability_cause: 'readiness-timeout', automatic_fallback: false },
    })
    expect(buildDiagnostic(new GeometryEngineUnavailableError(
      execution,
      'readiness timed out',
      'readiness-timeout',
    ))).toMatchObject({
      contractVersion: 1,
      code: 'engine_unavailable',
      retryable: true,
      details: { availability_cause: 'readiness-timeout', automatic_fallback: false },
    })
  })

  it('does not expose source excerpts through current or persisted diagnostics', () => {
    const source = 'cube(1);\nunsupported(); // SECRET_TOKEN'
    const error = new OpenSCADParseError(source, source.indexOf('unsupported'), 'Unsupported statement')

    expect(publicToolError(error).error).toMatchObject({
      code: 'source_syntax_error',
      line: 2,
      column: 1,
    })
    expect(JSON.stringify(publicToolError(error).error)).not.toContain('SECRET_TOKEN')
    expect(JSON.stringify(buildDiagnostic(error))).not.toContain('SECRET_TOKEN')
  })

  it('redacts unexpected internal failures of every generic error base class', () => {
    for (const error of [
      new Error('/private/catalog.duckdb failed with secret payload'),
      new TypeError('/private/type SECRET_TYPE'),
      new RangeError('/private/range SECRET_RANGE'),
    ]) {
      const mapped = publicToolError(error)
      expect(mapped.internal).toBe(true)
      expect(mapped.error).toMatchObject({ code: 'internal_error', retryable: true })
      expect(mapped.error.correlation_id).toMatch(/^[0-9a-f-]{36}$/)
      expect(JSON.stringify(mapped.error)).not.toContain('/private/')
      expect(JSON.stringify(mapped.error)).not.toContain('SECRET')
      expect(JSON.stringify(buildDiagnostic(error))).not.toContain('/private/')
      expect(JSON.stringify(buildDiagnostic(error))).not.toContain('SECRET')
    }
  })
})
