import { createHash } from 'node:crypto'
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  fromJsonSchema,
  InMemoryTransport,
  type JSONRPCMessage,
} from '@modelcontextprotocol/server'
import { OPENSCAD_2021_01_CONTRACT } from '../src/core/openScad2021Contract'
import { createOpenScadMcpServer } from '../src/mcp/createServer'
import { DuckDbModelStore } from '../src/mcp/duckdbModelStore'
import { publicToolError } from '../src/mcp/publicError'
import {
  OfficialOpenScadRemoteError,
  OfficialOpenScadSupervisorError,
  type OfficialOpenScadCapabilities,
  type OfficialOpenScadCheckResult,
  type OfficialOpenScadExportResult,
  type OfficialOpenScadRunOptions,
  type OfficialOpenScadRuntimeService,
} from '../src/mcp/officialOpenScadRuntimeService'
import {
  OFFICIAL_OPENSCAD_FONT_FAMILY,
  OFFICIAL_OPENSCAD_FONT_FILENAME,
  OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
  OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
  OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
  OFFICIAL_OPENSCAD_RUNTIME_VERSION,
} from '../src/mcp/officialOpenScadRuntimePatch'
import {
  OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES,
  OFFICIAL_OPENSCAD_EXPORT_FORMATS,
  OFFICIAL_OPENSCAD_MAX_LOG_BYTES,
  OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES,
  OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
  OFFICIAL_OPENSCAD_MAX_PROJECT_FILES,
  OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES,
  OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS,
  type OfficialOpenScadExportFormat,
} from '../src/mcp/officialOpenScadRuntimeProtocol'

interface JsonRpcResponse {
  jsonrpc: '2.0'
  id: number
  result?: Record<string, unknown>
  error?: { code: number; message: string }
}

const closeables: Array<() => Promise<void>> = []

afterEach(async () => {
  vi.restoreAllMocks()
  await Promise.all(closeables.splice(0).map(close => close()))
})

function digest(value: string | Uint8Array): string {
  return createHash('sha256').update(value).digest('hex')
}

const CAPABILITIES: OfficialOpenScadCapabilities = Object.freeze({
  available: true,
  unavailableReason: null,
  expectedRuntimeVersion: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
  runtimeVersion: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
  archiveSha256: OFFICIAL_OPENSCAD_RUNTIME_ARCHIVE_SHA256,
  runtimeSha256: '1'.repeat(64),
  patchVersion: OFFICIAL_OPENSCAD_RUNTIME_PATCH_VERSION,
  defaultFont: Object.freeze({
    family: OFFICIAL_OPENSCAD_FONT_FAMILY,
    filename: OFFICIAL_OPENSCAD_FONT_FILENAME,
    sha256: '2'.repeat(64),
    licenseFilename: OFFICIAL_OPENSCAD_FONT_LICENSE_FILENAME,
    licenseSha256: '3'.repeat(64),
  }),
  isolation: Object.freeze({
    filesystem: 'MEMFS',
    nodePermissionModel: true,
    scadHostFileRead: false,
    scadHostFileWrite: false,
    networkApiExposed: false,
    networkSandboxEnforced: false,
    wasmMemoryLimitEnforced: false,
    processModel: 'one-shot',
  }),
  formats: OFFICIAL_OPENSCAD_EXPORT_FORMATS,
  experimentalFeatures: OFFICIAL_OPENSCAD_EXPERIMENTAL_FEATURES,
  defaults: Object.freeze({
    backend: 'Manifold',
    hardWarnings: true,
    checkParameters: true,
    checkParameterRanges: true,
    experimentsEnabled: false,
  }),
  limits: Object.freeze({
    sourceBytes: OFFICIAL_OPENSCAD_MAX_SOURCE_BYTES,
    projectFiles: OFFICIAL_OPENSCAD_MAX_PROJECT_FILES,
    projectFileBytes: OFFICIAL_OPENSCAD_MAX_PROJECT_FILE_BYTES,
    projectBytes: OFFICIAL_OPENSCAD_MAX_PROJECT_BYTES,
    outputBytes: OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES,
    logBytes: OFFICIAL_OPENSCAD_MAX_LOG_BYTES,
    logEntries: OFFICIAL_OPENSCAD_MAX_LOG_ENTRIES,
    timeoutMs: OFFICIAL_OPENSCAD_MAX_TIMEOUT_MS,
  }),
  setupCommand: 'npm run setup:openscad',
})

class FakeOfficialRuntime implements OfficialOpenScadRuntimeService {
  readonly checkCalls: Array<{ source: string; options?: OfficialOpenScadRunOptions }> = []
  readonly exportCalls: Array<{
    source: string
    format: OfficialOpenScadExportFormat
    options?: OfficialOpenScadRunOptions
  }> = []
  nextCheckError: unknown = null
  nextExportError: unknown = null
  nextExportOverrides: Partial<OfficialOpenScadExportResult> | null = null
  readonly exportedData = new Uint8Array([0, 1, 2, 3, 254, 255])

  async capabilities(): Promise<OfficialOpenScadCapabilities> {
    return CAPABILITIES
  }

  async check(
    source: string,
    options?: OfficialOpenScadRunOptions,
  ): Promise<OfficialOpenScadCheckResult> {
    this.checkCalls.push({ source, options })
    if (this.nextCheckError !== null) {
      const error = this.nextCheckError
      this.nextCheckError = null
      throw error
    }
    return {
      runtimeVersion: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
      sourceSha256: digest(source),
      csgSha256: digest('official-csg'),
      csgBytes: 12,
      durationMs: 4.5,
      logs: { stdout: ['ECHO: 42'], stderr: [], truncated: false },
      experimentalFeatures: options?.experimentalFeatures ?? [],
    }
  }

  async export(
    source: string,
    format: OfficialOpenScadExportFormat,
    options?: OfficialOpenScadRunOptions,
  ): Promise<OfficialOpenScadExportResult> {
    this.exportCalls.push({ source, format, options })
    if (this.nextExportError !== null) {
      const error = this.nextExportError
      this.nextExportError = null
      throw error
    }
    const result: OfficialOpenScadExportResult = {
      runtimeVersion: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
      sourceSha256: digest(source),
      format,
      mimeType: format === 'stl' ? 'model/stl' : 'application/octet-stream',
      fileName: `model.${format}`,
      data: this.exportedData,
      sha256: digest(this.exportedData),
      durationMs: 8.25,
      logs: { stdout: [], stderr: [], truncated: false },
      experimentalFeatures: options?.experimentalFeatures ?? [],
    }
    const overrides = this.nextExportOverrides
    this.nextExportOverrides = null
    return { ...result, ...overrides }
  }

  async close(): Promise<void> {}
}

async function connectedServer(officialRuntime: OfficialOpenScadRuntimeService) {
  const store = await DuckDbModelStore.open(':memory:')
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair()
  const server = createOpenScadMcpServer({ store, officialRuntime })
  const pending = new Map<number, (response: JsonRpcResponse) => void>()
  let nextId = 1
  clientTransport.onmessage = message => {
    if ('id' in message && typeof message.id === 'number' && ('result' in message || 'error' in message)) {
      pending.get(message.id)?.(message as JsonRpcResponse)
      pending.delete(message.id)
    }
  }
  await clientTransport.start()
  await server.connect(serverTransport)

  const request = async (method: string, params: Record<string, unknown> = {}) => {
    const id = nextId++
    const response = new Promise<JsonRpcResponse>((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error(`Timed out waiting for ${method}`)), 5_000)
      pending.set(id, value => {
        clearTimeout(timeout)
        resolve(value)
      })
    })
    await clientTransport.send({ jsonrpc: '2.0', id, method, params } as JSONRPCMessage)
    const value = await response
    if (value.error) throw new Error(`${value.error.code}: ${value.error.message}`)
    return value.result ?? {}
  }

  await request('initialize', {
    protocolVersion: '2025-11-25',
    capabilities: {},
    clientInfo: { name: 'official-runtime-test', version: '1.0.0' },
  })
  await clientTransport.send({
    jsonrpc: '2.0',
    method: 'notifications/initialized',
  } as JSONRPCMessage)
  closeables.push(async () => {
    await server.close()
    await store.close()
  })
  return { request }
}

describe('official OpenSCAD MCP facade', () => {
  it('classifies every official runtime failure without exposing internal messages', () => {
    const supervisorCases = [
      ['E_OFFICIAL_OPENSCAD_UNAVAILABLE', 'engine_unavailable', false],
      ['E_OFFICIAL_OPENSCAD_BUSY', 'server_busy', true],
      ['E_OFFICIAL_OPENSCAD_CANCELLED', 'cancelled', true],
      ['E_OFFICIAL_OPENSCAD_DEADLINE', 'deadline_exceeded', true],
      ['E_OFFICIAL_OPENSCAD_CLOSED', 'engine_unavailable', false],
      ['E_OFFICIAL_OPENSCAD_PROTOCOL', 'internal_error', true],
      ['E_OFFICIAL_OPENSCAD_CHILD_CRASH', 'internal_error', true],
    ] as const
    for (const [code, publicCode, retryable] of supervisorCases) {
      const mapped = publicToolError(new OfficialOpenScadSupervisorError(
        code,
        `/private/runtime SECRET_${code}`,
        1,
      ))
      expect(mapped.error).toMatchObject({ code: publicCode, retryable })
      expect(JSON.stringify(mapped.error)).not.toContain('/private/')
      expect(JSON.stringify(mapped.error)).not.toContain('SECRET_')
    }

    const remoteCases = [
      ['E_OPENSCAD_COMPILE', 'source_syntax_error', false],
      ['E_OPENSCAD_NO_OUTPUT', 'source_syntax_error', false],
      ['E_OPENSCAD_OUTPUT_LIMIT', 'artifact_too_large', true],
      ['E_OFFICIAL_RUNNER', 'internal_error', true],
    ] as const
    for (const [code, publicCode, retryable] of remoteCases) {
      const mapped = publicToolError(new OfficialOpenScadRemoteError(
        code,
        '/private/runtime SECRET_REMOTE',
        1,
        { stdout: [], stderr: [], truncated: false },
        1,
      ))
      expect(mapped.error).toMatchObject({ code: publicCode, retryable })
      expect(JSON.stringify(mapped.error)).not.toContain('/private/')
      expect(JSON.stringify(mapped.error)).not.toContain('SECRET_REMOTE')
    }
  })

  it('advertises typed status/check/export tools and forwards an isolated project', async () => {
    const runtime = new FakeOfficialRuntime()
    const { request } = await connectedServer(runtime)
    const listed = await request('tools/list') as {
      tools: Array<{ name: string; inputSchema: Record<string, unknown>; outputSchema: Record<string, unknown> }>
    }
    const tools = new Map(listed.tools.map(tool => [tool.name, tool]))
    expect([...tools.keys()]).toEqual(expect.arrayContaining([
      'openscad_official_status',
      'openscad_official_check',
      'openscad_official_export',
    ]))

    const status = await request('tools/call', {
      name: 'openscad_official_status',
      arguments: {},
    }) as { structuredContent: Record<string, unknown> }
    expect(status.structuredContent).toMatchObject({
      language_contract: {
        id: OPENSCAD_2021_01_CONTRACT.id,
        function_count: 38,
        module_count: 35,
        resource_uri: 'openscad://language/openscad-2021.01',
      },
      official_runtime: {
        provider_role: 'qualification-oracle',
        available: true,
        runtime_version: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
        isolation: {
          filesystem: 'MEMFS',
          scad_host_file_read: false,
          scad_host_file_write: false,
          network_api_exposed: false,
          network_sandbox_enforced: false,
        },
      },
    })
    expect(await fromJsonSchema(tools.get('openscad_official_status')!.outputSchema)['~standard']
      .validate(status.structuredContent)).not.toHaveProperty('issues')

    const source = 'include <parts/library.scad>; make_part();'
    const checked = await request('tools/call', {
      name: 'openscad_official_check',
      arguments: {
        source,
        files: [
          { path: 'parts/library.scad', text: 'module make_part() { cube(2); }' },
          { path: 'assets/sample.dat', data_base64: 'AAECAw==' },
        ],
        experimental_features: ['textmetrics'],
        defines: ['size=12'],
        time: 0.25,
        backend: 'CGAL',
        hard_warnings: false,
        check_parameters: false,
        check_parameter_ranges: false,
        timeout_ms: 2_000,
      },
    }) as { structuredContent: Record<string, unknown> }
    expect(await fromJsonSchema(tools.get('openscad_official_check')!.outputSchema)['~standard']
      .validate(checked.structuredContent)).not.toHaveProperty('issues')
    expect(checked.structuredContent).toMatchObject({
      check: {
        provider_role: 'qualification-oracle',
        language_contract: { id: OPENSCAD_2021_01_CONTRACT.id },
        runtime_version: OFFICIAL_OPENSCAD_RUNTIME_VERSION,
        source_sha256: digest(source),
        experimental_features: ['textmetrics'],
      },
    })
    expect(runtime.checkCalls).toHaveLength(1)
    expect(runtime.checkCalls[0].options).toMatchObject({
      experimentalFeatures: ['textmetrics'],
      defines: ['size=12'],
      time: 0.25,
      backend: 'CGAL',
      hardWarnings: false,
      checkParameters: false,
      checkParameterRanges: false,
      timeoutMs: 2_000,
    })
    expect(runtime.checkCalls[0].options?.files?.[0]).toEqual({
      path: 'parts/library.scad',
      data: 'module make_part() { cube(2); }',
    })
    expect(runtime.checkCalls[0].options?.files?.[1]).toEqual({
      path: 'assets/sample.dat',
      data: new Uint8Array([0, 1, 2, 3]),
    })

    const capabilities = await request('resources/read', { uri: 'openscad://capabilities' }) as {
      contents: Array<{ text: string }>
    }
    const capabilityDocument = JSON.parse(capabilities.contents[0].text) as {
      official_language_contract: Record<string, unknown>
      official_runtime_resource_uri: string
      official_runtime: Record<string, unknown>
    }
    expect(capabilityDocument).toMatchObject({
      official_runtime_resource_uri: 'openscad://official-runtime',
      official_runtime: { available: true, runtime_version: OFFICIAL_OPENSCAD_RUNTIME_VERSION },
    })
    expect(capabilityDocument.official_language_contract).toEqual({
      id: OPENSCAD_2021_01_CONTRACT.id,
      release: OPENSCAD_2021_01_CONTRACT.languageTarget.release,
      tag: OPENSCAD_2021_01_CONTRACT.languageTarget.tag,
      commit: OPENSCAD_2021_01_CONTRACT.languageTarget.commit,
      scope: 'stable-builtins-38-functions-35-modules',
      function_count: OPENSCAD_2021_01_CONTRACT.builtins.functions.length,
      module_count: OPENSCAD_2021_01_CONTRACT.builtins.modules.length,
      functions: OPENSCAD_2021_01_CONTRACT.builtins.functions.map(entry => entry.name),
      modules: OPENSCAD_2021_01_CONTRACT.builtins.modules.map(entry => entry.name),
      compatibility_tail_policy: OPENSCAD_2021_01_CONTRACT.executionRuntime.compatibilityTailPolicy,
      resource_uri: 'openscad://language/openscad-2021.01',
    })
    const dedicated = await request('resources/read', { uri: 'openscad://official-runtime' }) as {
      contents: Array<{ text: string }>
    }
    expect(JSON.parse(dedicated.contents[0].text)).toEqual(status.structuredContent)

    const language = await request('resources/read', {
      uri: 'openscad://language/openscad-2021.01',
    }) as { contents: Array<{ text: string }> }
    expect(JSON.parse(language.contents[0].text)).toEqual({
      contract: JSON.parse(JSON.stringify(OPENSCAD_2021_01_CONTRACT)),
    })
    const resources = await request('resources/list') as { resources: Array<{ uri: string }> }
    expect(resources.resources).toContainEqual(expect.objectContaining({
      uri: 'openscad://language/openscad-2021.01',
    }))
  })

  it('returns content-addressed session resources and validates export output on the wire', async () => {
    const runtime = new FakeOfficialRuntime()
    const { request } = await connectedServer(runtime)
    const listed = await request('tools/list') as {
      tools: Array<{ name: string; outputSchema: Record<string, unknown> }>
    }
    const exportTool = listed.tools.find(tool => tool.name === 'openscad_official_export')!
    const first = await request('tools/call', {
      name: 'openscad_official_export',
      arguments: { source: 'minkowski() { cube(1); sphere(1); }', format: 'stl' },
    }) as {
      content: Array<Record<string, unknown>>
      structuredContent: {
        export: { provider_role: string; language_contract: { id: string } }
        artifact: { id: string; resource_uri: string; sha256: string }
      }
    }
    expect(await fromJsonSchema(exportTool.outputSchema)['~standard']
      .validate(first.structuredContent)).not.toHaveProperty('issues')
    expect(first.content).toContainEqual(expect.objectContaining({
      type: 'resource_link',
      uri: first.structuredContent.artifact.resource_uri,
      mimeType: 'model/stl',
    }))
    expect(first.structuredContent.artifact).toMatchObject({
      id: expect.stringMatching(/^[a-f0-9]{64}$/),
      sha256: digest(runtime.exportedData),
    })
    expect(first.structuredContent.export.language_contract.id).toBe(OPENSCAD_2021_01_CONTRACT.id)
    expect(first.structuredContent.export.provider_role).toBe('qualification-oracle')

    const artifact = await request('resources/read', {
      uri: first.structuredContent.artifact.resource_uri,
    }) as { contents: Array<{ blob: string; mimeType: string }> }
    expect(artifact.contents).toEqual([expect.objectContaining({
      blob: Buffer.from(runtime.exportedData).toString('base64'),
      mimeType: 'model/stl',
    })])

    const second = await request('tools/call', {
      name: 'openscad_official_export',
      arguments: { source: 'minkowski() { cube(1); sphere(1); }', format: 'stl' },
    }) as { structuredContent: { artifact: { id: string } } }
    expect(second.structuredContent.artifact.id).toBe(first.structuredContent.artifact.id)

    const resources = await request('resources/list') as { resources: Array<{ uri: string }> }
    expect(resources.resources).toContainEqual(expect.objectContaining({
      uri: first.structuredContent.artifact.resource_uri,
    }))
    const templates = await request('resources/templates/list') as {
      resourceTemplates: Array<{ uriTemplate: string }>
    }
    expect(templates.resourceTemplates).toContainEqual(expect.objectContaining({
      uriTemplate: 'openscad://official-artifacts/{id}',
    }))
  })

  it('rejects unsafe project inputs before invoking the runtime', async () => {
    const runtime = new FakeOfficialRuntime()
    const { request } = await connectedServer(runtime)
    for (const argumentsValue of [
      { source: 'cube(1);', files: [{ path: '../host.scad', text: 'cube(2);' }] },
      { source: 'cube(1);', files: [{ path: 'part.scad', text: '', data_base64: '' }] },
      { source: 'cube(1);', files: [{ path: 'part.dat', data_base64: 'not-base64' }] },
      {
        source: 'cube(1);',
        files: [{ path: 'same.scad', text: '' }, { path: 'same.scad', text: '' }],
      },
      { source: 'cube(1);', unexpected: true },
    ]) {
      const rejected = await request('tools/call', {
        name: 'openscad_official_check',
        arguments: argumentsValue,
      }) as { isError: boolean; content: Array<{ text?: string }> }
      expect(rejected.isError).toBe(true)
      expect(rejected.content[0]?.text).toMatch(/invalid|validation|arguments/i)
    }
    expect(runtime.checkCalls).toHaveLength(0)
  })

  it('rejects inconsistent official export identity before caching an artifact', async () => {
    const runtime = new FakeOfficialRuntime()
    const { request } = await connectedServer(runtime)
    vi.spyOn(console, 'error').mockImplementation(() => undefined)

    for (const overrides of [
      { sha256: '0'.repeat(64) },
      { format: 'off' as const },
      { mimeType: 'application/octet-stream' },
      { fileName: 'untrusted.stl' },
    ]) {
      runtime.nextExportOverrides = overrides
      const rejected = await request('tools/call', {
        name: 'openscad_official_export',
        arguments: { source: 'cube(1);', format: 'stl' },
      }) as { isError: boolean; structuredContent: Record<string, unknown> }
      expect(rejected).toMatchObject({
        isError: true,
        structuredContent: { error: { code: 'internal_error' } },
      })
      expect(rejected.structuredContent).not.toHaveProperty('artifact')
    }
  })

  it('maps expected runtime failures to public codes and redacts internal details', async () => {
    const runtime = new FakeOfficialRuntime()
    const { request } = await connectedServer(runtime)
    const listed = await request('tools/list') as {
      tools: Array<{ name: string; outputSchema: Record<string, unknown> }>
    }
    const checkSchema = listed.tools.find(tool => tool.name === 'openscad_official_check')!.outputSchema
    const exportSchema = listed.tools.find(tool => tool.name === 'openscad_official_export')!.outputSchema

    runtime.nextCheckError = new OfficialOpenScadSupervisorError(
      'E_OFFICIAL_OPENSCAD_BUSY',
      '/private/runtime SECRET_BUSY',
      1,
    )
    const busy = await request('tools/call', {
      name: 'openscad_official_check',
      arguments: { source: 'cube(1);' },
    }) as { isError: boolean; structuredContent: { error: Record<string, unknown> } }
    expect(busy).toMatchObject({
      isError: true,
      structuredContent: { error: { code: 'server_busy', retryable: true } },
    })
    expect(JSON.stringify(busy)).not.toContain('SECRET_BUSY')
    expect(await fromJsonSchema(checkSchema)['~standard'].validate(busy.structuredContent))
      .not.toHaveProperty('issues')

    runtime.nextCheckError = new OfficialOpenScadRemoteError(
      'E_OPENSCAD_COMPILE',
      '/private/runtime SECRET_COMPILE',
      2,
      { stdout: [], stderr: ['ERROR: parser failed'], truncated: false },
      1,
    )
    const compile = await request('tools/call', {
      name: 'openscad_official_check',
      arguments: { source: 'broken(' },
    }) as { isError: boolean; structuredContent: { error: Record<string, unknown> } }
    expect(compile).toMatchObject({
      isError: true,
      structuredContent: {
        error: { code: 'source_syntax_error', retryable: false },
        official_logs: { stdout: [], stderr: ['ERROR: parser failed'], truncated: false },
        duration_ms: 1,
      },
    })
    expect(JSON.stringify(compile)).not.toContain('SECRET_COMPILE')
    expect(await fromJsonSchema(checkSchema)['~standard'].validate(compile.structuredContent))
      .not.toHaveProperty('issues')

    const log = vi.spyOn(console, 'error').mockImplementation(() => undefined)
    runtime.nextExportError = new OfficialOpenScadSupervisorError(
      'E_OFFICIAL_OPENSCAD_PROTOCOL',
      '/private/runtime SECRET_PROTOCOL',
      3,
    )
    const protocol = await request('tools/call', {
      name: 'openscad_official_export',
      arguments: { source: 'cube(1);', format: 'stl' },
    }) as { isError: boolean; structuredContent: { error: Record<string, unknown> } }
    expect(protocol).toMatchObject({
      isError: true,
      structuredContent: {
        error: {
          code: 'internal_error',
          retryable: true,
          correlation_id: expect.stringMatching(/^[0-9a-f-]{36}$/),
        },
      },
    })
    expect(JSON.stringify(protocol)).not.toContain('SECRET_PROTOCOL')
    expect(await fromJsonSchema(exportSchema)['~standard'].validate(protocol.structuredContent))
      .not.toHaveProperty('issues')

    runtime.nextExportError = new OfficialOpenScadRemoteError(
      'E_OFFICIAL_RUNNER',
      '/private/runtime SECRET_REMOTE',
      4,
      { stdout: [], stderr: ['SECRET_REMOTE_LOG'], truncated: false },
      2,
    )
    const remote = await request('tools/call', {
      name: 'openscad_official_export',
      arguments: { source: 'cube(1);', format: 'stl' },
    }) as { isError: boolean; structuredContent: Record<string, unknown> }
    expect(remote).toMatchObject({
      isError: true,
      structuredContent: {
        error: {
          code: 'internal_error',
          retryable: true,
          correlation_id: expect.stringMatching(/^[0-9a-f-]{36}$/),
        },
      },
    })
    expect(remote.structuredContent).not.toHaveProperty('official_logs')
    expect(remote.structuredContent).not.toHaveProperty('duration_ms')
    expect(JSON.stringify(remote)).not.toContain('SECRET_REMOTE')
    expect(await fromJsonSchema(exportSchema)['~standard'].validate(remote.structuredContent))
      .not.toHaveProperty('issues')
    expect(log).toHaveBeenCalledTimes(2)
  })
})
