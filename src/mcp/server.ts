#!/usr/bin/env node
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { serveStdio, StdioServerTransport } from '@modelcontextprotocol/server/stdio'
import { defaultGeometryBuildEngine } from '../services/geometryBuildEngine'
import { BoundedTransport, MAX_MCP_SUBSCRIPTIONS } from './boundedTransport'
import { createOpenScadMcpServer } from './createServer'
import { DirectGeometrySupervisor } from './directGeometrySupervisor'
import { DuckDbModelStore } from './duckdbModelStore'
import {
  HeadlessGeometryService,
  type McpGeometryBuildRuntime,
} from './geometryService'
import type { ModelStore } from './modelStore'
import {
  OfficialOpenScadRuntimeSupervisor,
  type OfficialOpenScadRuntimeService,
} from './officialOpenScadRuntimeService'

export interface McpCliOptions {
  databasePath: string
  showHelp: boolean
}

interface ClosableModelStore extends ModelStore {
  close(): Promise<void>
}

interface ClosableGeometryRuntime extends McpGeometryBuildRuntime {
  close(): Promise<void>
}

export interface RunMcpServerDependencies {
  openStore(databasePath: string): Promise<ClosableModelStore>
  createGeometryRuntime(): ClosableGeometryRuntime
  createOfficialRuntime(): OfficialOpenScadRuntimeService
  createTransport(): BoundedTransport
  startServer: typeof serveStdio
}

const defaultDependencies: RunMcpServerDependencies = {
  openStore: databasePath => DuckDbModelStore.open(databasePath),
  createGeometryRuntime: () => new DirectGeometrySupervisor(),
  createOfficialRuntime: () => new OfficialOpenScadRuntimeSupervisor(),
  createTransport: () => new BoundedTransport(new StdioServerTransport()),
  startServer: serveStdio,
}

export function parseMcpCliOptions(
  argv: string[],
  environment: NodeJS.ProcessEnv = process.env,
  cwd = process.cwd(),
): McpCliOptions {
  let databasePath = environment.OPENSCAD_VIEWER_DUCKDB
    ? resolve(cwd, environment.OPENSCAD_VIEWER_DUCKDB)
    : resolve(cwd, '.open-scad-viewer.duckdb')
  let showHelp = false

  for (let index = 0; index < argv.length; index++) {
    const argument = argv[index]
    if (argument === '--memory') {
      databasePath = ':memory:'
    } else if (argument === '--db') {
      const value = argv[++index]
      if (!value) throw new TypeError('--db requires a file path')
      databasePath = value === ':memory:' ? value : resolve(cwd, value)
    } else if (argument === '--help' || argument === '-h') {
      showHelp = true
    } else {
      throw new TypeError(`Unknown argument: ${argument}`)
    }
  }
  return { databasePath, showHelp }
}

function helpText(): string {
  return `OpenSCAD Viewer MCP server

Usage: npm run --silent mcp -- [--db PATH | --memory]

Options:
  --db PATH   DuckDB file (default: .open-scad-viewer.duckdb)
  --memory    Use an ephemeral in-memory DuckDB database
  -h, --help  Show this help

Environment:
  OPENSCAD_VIEWER_DUCKDB  Alternate DuckDB path
`
}

export async function runMcpServer(
  options: McpCliOptions,
  dependencyOverrides: Partial<RunMcpServerDependencies> = {},
): Promise<{ close(): Promise<void> }> {
  const dependencies = { ...defaultDependencies, ...dependencyOverrides }
  const store = await dependencies.openStore(options.databasePath)
  let runtime: ClosableGeometryRuntime | null = null
  let officialRuntime: OfficialOpenScadRuntimeService | null = null
  let transport: BoundedTransport | null = null
  let handle: ReturnType<typeof serveStdio> | null = null
  let closePromise: Promise<void> | null = null
  const close = () => {
    closePromise ??= (async () => {
      const hostResults = await Promise.allSettled([
        handle?.close() ?? transport?.close() ?? Promise.resolve(),
        runtime?.close() ?? Promise.resolve(),
        officialRuntime?.close() ?? Promise.resolve(),
      ])
      const storeResult = await Promise.allSettled([store.close()])
      const errors = [...hostResults, ...storeResult]
        .filter((result): result is PromiseRejectedResult => result.status === 'rejected')
        .map(result => result.reason)
      if (errors.length) throw new AggregateError(errors, 'MCP server shutdown failed')
    })()
    return closePromise
  }
  try {
    runtime = dependencies.createGeometryRuntime()
    const activeOfficialRuntime = dependencies.createOfficialRuntime()
    officialRuntime = activeOfficialRuntime
    const geometry = new HeadlessGeometryService(defaultGeometryBuildEngine, runtime)
    const activeTransport = dependencies.createTransport()
    transport = activeTransport
    handle = dependencies.startServer(() => createOpenScadMcpServer({
      store,
      geometry,
      officialRuntime: activeOfficialRuntime,
      onRequestSettled: requestId => activeTransport.settleRequest(requestId),
      onRequestCancelled: requestId => activeTransport.cancelRequest(requestId),
    }), {
      transport: activeTransport,
      maxSubscriptions: MAX_MCP_SUBSCRIPTIONS,
      onerror: error => {
        console.error('MCP transport error:', error)
        process.exitCode = 1
        queueMicrotask(() => void close().catch(closeError => {
          console.error('MCP shutdown failed:', closeError)
        }))
      },
    })
  } catch (error) {
    try {
      await close()
    } catch (closeError) {
      throw new AggregateError(
        [error, closeError],
        'Could not start or clean up the OpenSCAD Viewer MCP server',
      )
    }
    throw error
  }
  console.error(`OpenSCAD Viewer MCP server ready (DuckDB: ${options.databasePath})`)

  return { close }
}

async function main() {
  let options: McpCliOptions
  try {
    options = parseMcpCliOptions(process.argv.slice(2))
  } catch (error) {
    console.error(error instanceof Error ? error.message : error)
    console.error(helpText())
    process.exitCode = 2
    return
  }
  if (options.showHelp) {
    process.stdout.write(helpText())
    return
  }

  try {
    const running = await runMcpServer(options)
    const stop = () => {
      void running.close().catch(error => {
        console.error('MCP shutdown failed:', error)
        process.exitCode = 1
      })
    }
    process.once('SIGINT', stop)
    process.once('SIGTERM', stop)
    process.stdin.once('end', stop)
  } catch (error) {
    console.error('Could not start OpenSCAD Viewer MCP server:', error)
    process.exitCode = 1
  }
}

const isMain = process.argv[1] !== undefined
  && fileURLToPath(import.meta.url) === resolve(process.argv[1])
if (isMain) void main()
