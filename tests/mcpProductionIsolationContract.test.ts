import { readFile } from 'node:fs/promises'
import { describe, expect, it } from 'vitest'

interface SourceFile {
  readonly name: string
  readonly url: URL
}

interface StaticImport {
  readonly bindings: string
  readonly specifier: string
}

const productionRuntimeFiles: readonly SourceFile[] = [
  { name: 'src/mcp/server.ts', url: new URL('../src/mcp/server.ts', import.meta.url) },
  { name: 'src/mcp/createServer.ts', url: new URL('../src/mcp/createServer.ts', import.meta.url) },
  { name: 'src/mcp/geometryService.ts', url: new URL('../src/mcp/geometryService.ts', import.meta.url) },
  {
    name: 'src/mcp/directGeometryProtocol.ts',
    url: new URL('../src/mcp/directGeometryProtocol.ts', import.meta.url),
  },
  {
    name: 'src/mcp/directGeometry.worker.ts',
    url: new URL('../src/mcp/directGeometry.worker.ts', import.meta.url),
  },
  {
    name: 'src/mcp/directGeometrySupervisor.ts',
    url: new URL('../src/mcp/directGeometrySupervisor.ts', import.meta.url),
  },
]

const directWorker = productionRuntimeFiles.find(
  file => file.name === 'src/mcp/directGeometry.worker.ts',
)!

const qualificationOnlyModule = /(?:manifoldPlanQualification(?:Supervisor|Protocol|\.worker)|manifoldPlanEvaluator)/u
const forbiddenDirectWorkerModule = /(?:duckdb|@modelcontextprotocol|(?:^|\/)(?:bounded|mcp)[^/]*transport(?:\.[cm]?[jt]s)?$|^(?:node:)?(?:fs(?:\/promises)?|net|http|https|child_process)$)/iu

function staticImports(source: string): StaticImport[] {
  return [...source.matchAll(/\bimport\s+([\s\S]*?)\s+from\s+(['"])([^'"]+)\2/gu)]
    .map(match => ({ bindings: match[1], specifier: match[3] }))
}

function importedSpecifiers(source: string): string[] {
  const fromImports = staticImports(source).map(item => item.specifier)
  const reExports = [...source.matchAll(/\bexport\s+[\s\S]*?\s+from\s+(['"])([^'"]+)\1/gu)]
    .map(match => match[2])
  const sideEffectImports = [...source.matchAll(/\bimport\s+(['"])([^'"]+)\1/gu)]
    .map(match => match[2])
  const dynamicImports = [...source.matchAll(/\bimport\s*\(\s*(['"])([^'"]+)\1\s*\)/gu)]
    .map(match => match[2])
  const workerUrls = [...source.matchAll(
    /\bnew\s+URL\s*\(\s*(['"])([^'"]+)\1\s*,\s*import\.meta\.url\s*\)/gu,
  )].map(match => match[2])
  return [...fromImports, ...reExports, ...sideEffectImports, ...dynamicImports, ...workerUrls]
}

async function readSources(files: readonly SourceFile[]): Promise<Array<{
  readonly name: string
  readonly source: string
}>> {
  return Promise.all(files.map(async file => ({
    name: file.name,
    source: await readFile(file.url, 'utf8'),
  })))
}

describe('MCP production geometry isolation contract', () => {
  it('keeps qualification-only modules out of every production entry point and runtime file', async () => {
    for (const file of await readSources(productionRuntimeFiles)) {
      const forbidden = importedSpecifiers(file.source).filter(
        specifier => qualificationOnlyModule.test(specifier),
      )
      expect(forbidden, file.name).toEqual([])
    }
  })

  it('keeps the direct worker on the default engine without storage, transport, or I/O imports', async () => {
    const source = await readFile(directWorker.url, 'utf8')
    const imports = staticImports(source)
    const engineImports = imports.filter(item => (
      /\bdefaultGeometryBuildEngine\b/u.test(item.bindings)
      && /(?:^|\/)geometryBuildEngine(?:\.[cm]?[jt]s)?$/u.test(item.specifier)
    ))

    expect(engineImports, directWorker.name).toHaveLength(1)
    expect(
      importedSpecifiers(source).filter(specifier => forbiddenDirectWorkerModule.test(specifier)),
      directWorker.name,
    ).toEqual([])
  })

  it('does not expose the qualificationOnly identity from production server or runtime code', async () => {
    for (const file of await readSources(productionRuntimeFiles)) {
      expect(file.source, file.name).not.toMatch(/\bqualificationOnly\b/u)
    }
  })
})
