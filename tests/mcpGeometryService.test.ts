import { OpenSCADParseError } from '../src/services/openscadParser'
import { describe, expect, it, vi } from 'vitest'
import {
  ArtifactSizeError,
  GeometryBusyError,
  HeadlessGeometryService,
  InvalidGeometryError,
  MAX_ANALYSIS_OBJECTS,
  MAX_ANALYSIS_TEXT_LENGTH,
  MAX_PENDING_GEOMETRY_JOBS,
} from '../src/mcp/geometryService'
import { MAX_MODEL_SOURCE_LENGTH } from '../src/mcp/modelStore'
import {
  GeometryBuildEngine,
  geometryExecutionForError,
} from '../src/services/geometryBuildEngine'

describe('HeadlessGeometryService', () => {
  const service = new HeadlessGeometryService()

  it('returns bounded geometry, topology, provenance, and Customizer facts', async () => {
    const source = `size = 2; // [1:1:5]
color("red") cube(size);`
    const analysis = await service.analyze(source)

    expect(analysis).toMatchObject({
      execution: {
        languageContract: 'legacy/current',
        engineClass: 'mesh',
        purpose: 'analysis',
        evidence: 'runtime',
        automaticFallback: false,
      },
      quality: 'full',
      meshCount: 1,
      triangleCount: 12,
      volume: 8,
      surfaceArea: 24,
      bounds: { min: [0, 0, 0], max: [2, 2, 2] },
      parameters: [{ name: 'size', value: 2, min: 1, max: 5, step: 1 }],
    })
    expect(analysis.objects[0].sources.some(sourceFact => sourceFact.label === 'cube()')).toBe(true)
  })

  it('delegates capabilities and builds to an injected host runtime', async () => {
    const engine = new GeometryBuildEngine()
    const runtime = {
      capabilities: vi.fn(() => engine.capabilities()),
      build: vi.fn((
        source: string,
        quality: 'preview' | 'full',
        purpose: 'preview' | 'full' | 'analysis' | 'export',
        signal?: AbortSignal,
      ) => engine.buildSource(
        source,
        { quality, purpose },
        { shouldAbort: () => signal?.aborted ?? false },
      )),
    }
    const supervised = new HeadlessGeometryService(engine, runtime)

    await supervised.capabilities()
    const analysis = await supervised.analyze('cube(1);', 'preview')

    expect(runtime.capabilities).toHaveBeenCalledOnce()
    expect(runtime.build).toHaveBeenCalledWith('cube(1);', 'preview', 'analysis', undefined)
    expect(analysis.execution).toMatchObject({ purpose: 'analysis', evidence: 'runtime' })
  })

  it('keeps both engines discoverable and refuses unavailable B-rep without fallback', async () => {
    expect(await service.capabilities()).toMatchObject({
      sourceDirectedRouting: true,
      automaticFallback: false,
      engines: [
        { engineClass: 'mesh', permanent: true, availability: 'available' },
        { engineClass: 'brep', permanent: true, availability: 'unavailable' },
      ],
    })

    await expect(service.analyze(
      '// @language openscad-viewer/brep-1\ncube(1);',
    )).rejects.toMatchObject({
      name: 'GeometryEngineUnavailableError',
      execution: { engineClass: 'brep', automaticFallback: false },
    })
  })

  it('applies multiple Customizer replacements without invalidating source offsets', () => {
    const source = `width = 10; // [1:20]
label = "old"; // [old,new]
enabled = true;
cube(width);`
    const customized = service.customize(source, { width: 12, label: 'new', enabled: false })

    expect(customized.source).toContain('width = 12;')
    expect(customized.source).toContain('label = "new";')
    expect(customized.source).toContain('enabled = false;')
    expect(customized.applied).toEqual(['enabled', 'label', 'width'])
  })

  it('exports binary STL and enforces the caller-visible artifact budget', async () => {
    const exported = await service.export('cube(1);', 'stl')
    expect(exported.mimeType).toBe('model/stl')
    expect(exported.data.byteLength).toBe(84 + 12 * 50)

    await expect(service.export('cube(1);', 'stl', { maxBytes: 100 }))
      .rejects.toBeInstanceOf(ArtifactSizeError)
  })

  it('rejects non-finite geometry before analysis or export can expose corrupt meshes', async () => {
    await expect(service.analyze('cube([1e100, 1e100, 1e-200]);'))
      .rejects.toBeInstanceOf(OpenSCADParseError)
    await expect(service.export('translate([1e300, 0, 0]) cube(1);', 'stl'))
      .rejects.toBeInstanceOf(InvalidGeometryError)
  })

  it('rejects non-finite Customizer numbers before producing JSON output', async () => {
    const source = 'size = 1e999; cube(1);'

    await expect(service.analyze(source)).rejects.toThrow(/finite/i)
    expect(() => service.customize(source, {})).toThrow(/finite/i)
  })

  it('bounds the shared headless geometry queue', async () => {
    const pending = Array.from({ length: MAX_PENDING_GEOMETRY_JOBS }, () => (
      service.analyze('cube(1);', 'preview')
    ))

    let busy: unknown
    try {
      await service.analyze('cube(2);', 'preview')
    } catch (error) {
      busy = error
    }
    expect(busy).toBeInstanceOf(GeometryBusyError)
    expect(geometryExecutionForError(busy)).toMatchObject({
      languageContract: 'legacy/current',
      engineClass: 'mesh',
      evidence: 'planned',
      automaticFallback: false,
    })
    await Promise.all(pending)
  })

  it('attests each call separately when an already-aborted signal is reused', async () => {
    const controller = new AbortController()
    controller.abort(new Error('shared cancellation reason'))

    let legacyFailure: unknown
    let brepFailure: unknown
    try {
      await service.compile('cube(1);', 'preview', controller.signal, 'preview')
    } catch (error) {
      legacyFailure = error
    }
    try {
      await service.compile(
        '// @language openscad-viewer/brep-1\ncube(1);',
        'full',
        controller.signal,
        'full',
      )
    } catch (error) {
      brepFailure = error
    }

    expect(legacyFailure).not.toBe(brepFailure)
    expect(geometryExecutionForError(legacyFailure)).toMatchObject({
      languageContract: 'legacy/current',
      engineClass: 'mesh',
      quality: 'preview',
      purpose: 'preview',
      evidence: 'planned',
    })
    expect(geometryExecutionForError(brepFailure)).toMatchObject({
      languageContract: 'openscad-viewer/brep-1',
      engineClass: 'brep',
      quality: 'full',
      purpose: 'full',
      evidence: 'planned',
    })
  })

  it('rejects Customizer expansion beyond the source and wire budget', () => {
    expect(() => service.customize('label = "";', {
      label: 'x'.repeat(MAX_MODEL_SOURCE_LENGTH),
    })).toThrow(/source exceeds/i)
  })

  it('keeps detailed MCP output bounded without changing total metrics', async () => {
    const count = MAX_ANALYSIS_OBJECTS + 1
    const source = Array.from({ length: count }, (_, index) => (
      `translate([${index * 2}, 0, 0]) cube(1);`
    )).join('\n')
    const analysis = await service.analyze(source)

    expect(analysis.meshCount).toBe(count)
    expect(analysis.triangleCount).toBe(count * 12)
    expect(analysis.objects).toHaveLength(MAX_ANALYSIS_OBJECTS)
    expect(analysis.objectsTruncated).toBe(true)
  })

  it('bounds provenance identifiers derived from very long module names', async () => {
    const identifier = `module_${'x'.repeat(20_000)}`
    const analysis = await service.analyze(`module ${identifier}() { cube(1); }\n${identifier}();`)
    const sources = analysis.objects.flatMap(object => object.sources)

    expect(analysis).toMatchObject({ meshCount: 1, triangleCount: 12, detailsTruncated: true })
    expect(sources.length).toBeGreaterThan(0)
    for (const source of sources) {
      expect(source.label.length).toBeLessThanOrEqual(MAX_ANALYSIS_TEXT_LENGTH)
      expect(source.operationId?.length ?? 0).toBeLessThanOrEqual(MAX_ANALYSIS_TEXT_LENGTH)
      expect(source.instanceId?.length ?? 0).toBeLessThanOrEqual(MAX_ANALYSIS_TEXT_LENGTH)
    }
    expect(JSON.stringify(analysis).length).toBeLessThan(10_000)
  })
})
