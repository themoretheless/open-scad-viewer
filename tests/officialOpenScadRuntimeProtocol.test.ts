import { describe, expect, it } from 'vitest'
import { sha256Buffer } from '../src/mcp/officialOpenScadRuntimePatch'
import {
  OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES,
  OFFICIAL_OPENSCAD_PROTOCOL_VERSION,
  isOfficialOpenScadWireRequest,
  isOfficialOpenScadWireTerminal,
  normalizeOfficialOpenScadProjectPath,
  type OfficialOpenScadWireRequest,
} from '../src/mcp/officialOpenScadRuntimeProtocol'

function request(): OfficialOpenScadWireRequest {
  const source = 'include <lib/helpers.scad>; answer();'
  const file = Buffer.from('module answer() { cube(42); }')
  return {
    protocolVersion: OFFICIAL_OPENSCAD_PROTOCOL_VERSION,
    jobId: 7,
    operation: 'export',
    source,
    sourceSha256: sha256Buffer(source),
    files: [{
      path: 'lib/helpers.scad',
      dataBase64: file.toString('base64'),
      sha256: sha256Buffer(file),
    }],
    format: 'stl',
    options: {
      experimentalFeatures: [],
      defines: ['size=42'],
      time: 0.5,
      backend: 'Manifold',
      hardWarnings: true,
      checkParameters: true,
      checkParameterRanges: true,
    },
  }
}

describe('official OpenSCAD child protocol', () => {
  it('accepts only correlated, hash-bound project requests', () => {
    const valid = request()
    expect(isOfficialOpenScadWireRequest(valid)).toBe(true)
    expect(isOfficialOpenScadWireRequest({ ...valid, sourceSha256: '0'.repeat(64) })).toBe(false)
    expect(isOfficialOpenScadWireRequest({
      ...valid,
      files: [{ ...valid.files[0], path: '../host.scad' }],
    })).toBe(false)
    expect(isOfficialOpenScadWireRequest({
      ...valid,
      files: [valid.files[0], valid.files[0]],
    })).toBe(false)
    expect(isOfficialOpenScadWireRequest({
      ...valid,
      options: { ...valid.options, experimentalFeatures: ['all'] },
    })).toBe(false)
  })

  it('normalizes no paths and reserves the entry and output names', () => {
    expect(normalizeOfficialOpenScadProjectPath('parts/body.stl')).toBe('parts/body.stl')
    for (const path of ['/etc/passwd', '../x', 'a/../x', './x', 'a//x', 'a\\x', 'main.scad', '__open_scad_result.stl']) {
      expect(() => normalizeOfficialOpenScadProjectPath(path)).toThrow()
    }
  })

  it('enforces the aggregate decoded project byte limit', () => {
    const valid = request()
    const half = Buffer.alloc(Math.floor(OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES / 2) + 1, 1)
    expect(isOfficialOpenScadWireRequest({
      ...valid,
      source: '',
      sourceSha256: sha256Buffer(''),
      files: ['large-a.dat', 'large-b.dat'].map(path => ({
        path,
        dataBase64: half.toString('base64'),
        sha256: sha256Buffer(half),
      })),
    })).toBe(false)
  })

  it('accepts only exact, correlated and content-verified terminals', () => {
    const validRequest = request()
    const data = Buffer.from('solid fixture')
    const terminal = {
      protocolVersion: 1,
      jobId: 7,
      operation: 'export',
      sourceSha256: validRequest.sourceSha256,
      runtimeVersion: '2026.09.01',
      durationMs: 12,
      logs: { stdout: [], stderr: [], truncated: false },
      status: 'succeeded',
      output: {
        format: 'stl',
        mimeType: 'model/stl',
        byteLength: data.byteLength,
        sha256: sha256Buffer(data),
        dataBase64: data.toString('base64'),
      },
    }
    expect(isOfficialOpenScadWireTerminal(terminal, validRequest)).toBe(true)
    expect(isOfficialOpenScadWireTerminal({ ...terminal, jobId: 8 }, validRequest)).toBe(false)
    expect(isOfficialOpenScadWireTerminal({
      ...terminal,
      output: { ...terminal.output, mimeType: 'application/octet-stream' },
    }, validRequest)).toBe(false)
    expect(isOfficialOpenScadWireTerminal({
      ...terminal,
      output: { ...terminal.output, byteLength: data.byteLength + 1 },
    }, validRequest)).toBe(false)
    expect(isOfficialOpenScadWireTerminal({ ...terminal, unexpected: true }, validRequest)).toBe(false)
  })
})
