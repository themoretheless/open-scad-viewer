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

const productionServer = productionRuntimeFiles.find(
  file => file.name === 'src/mcp/server.ts',
)!
const directWorker = productionRuntimeFiles.find(
  file => file.name === 'src/mcp/directGeometry.worker.ts',
)!
const directSupervisor = productionRuntimeFiles.find(
  file => file.name === 'src/mcp/directGeometrySupervisor.ts',
)!

const qualificationOnlyModule = /(?:manifoldPlanQualification(?:Supervisor|Protocol|\.worker)|manifoldPlanEvaluator)/u
const forbiddenDirectWorkerModule = /(?:duckdb|@modelcontextprotocol|(?:^|\/)(?:bounded|mcp)[^/]*transport(?:\.[cm]?[jt]s)?$|^(?:node:)?(?:fs(?:\/promises)?|net|tls|dgram|dns(?:\/promises)?|http|https|http2|child_process|cluster)$)/iu

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

async function resolveRelativeSource(from: URL, specifier: string): Promise<URL | null> {
  if (!specifier.startsWith('.')) return null
  const candidates = [
    new URL(specifier, from),
    new URL(`${specifier}.ts`, from),
    new URL(`${specifier}.tsx`, from),
    new URL(`${specifier}/index.ts`, from),
  ]
  if (/\.m?js$/u.test(specifier)) {
    candidates.push(new URL(specifier.replace(/\.m?js$/u, '.ts'), from))
  }
  for (const candidate of candidates) {
    try {
      await readFile(candidate, 'utf8')
      return candidate
    } catch {
      // Try the next TypeScript source resolution candidate.
    }
  }
  return null
}

async function transitiveSourceClosure(entry: SourceFile): Promise<Array<{
  readonly name: string
  readonly source: string
  readonly imports: string[]
}>> {
  const pending = [entry.url]
  const seen = new Set<string>()
  const files: Array<{ name: string; source: string; imports: string[] }> = []
  while (pending.length > 0) {
    const url = pending.pop()!
    if (seen.has(url.href)) continue
    seen.add(url.href)
    const source = await readFile(url, 'utf8')
    const imports = importedSpecifiers(source)
    files.push({
      name: decodeURIComponent(url.pathname).replace(/^.*\/src\//u, 'src/'),
      source,
      imports,
    })
    for (const specifier of imports) {
      const dependency = await resolveRelativeSource(url, specifier)
      if (dependency !== null && !seen.has(dependency.href)) pending.push(dependency)
    }
  }
  return files
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

  it('keeps the complete direct worker dependency graph free of host I/O and secrets', async () => {
    const closure = await transitiveSourceClosure(directWorker)
    expect(closure.map(file => file.name)).toEqual(expect.arrayContaining([
      'src/mcp/directGeometry.worker.ts',
      'src/mcp/directGeometryProtocol.ts',
      'src/services/geometryBuildEngine.ts',
      'src/services/openscadParser.ts',
    ]))
    for (const file of closure) {
      expect(
        file.imports.filter(specifier => forbiddenDirectWorkerModule.test(specifier)),
        file.name,
      ).toEqual([])
      expect(
        file.imports.filter(specifier => qualificationOnlyModule.test(specifier)),
        file.name,
      ).toEqual([])
      expect(file.source, file.name).not.toMatch(/\bprocess\s*\.\s*env\b/u)
      expect(file.source, file.name).not.toMatch(
        /\b(?:globalThis\s*\.\s*)?(?:fetch|WebSocket|EventSource)\s*\(/u,
      )
    }
  })

  it('does not inherit the MCP host environment into production geometry workers', async () => {
    const source = await readFile(directSupervisor.url, 'utf8')
    expect(source, directSupervisor.name).toMatch(/\benv\s*:\s*\{\s*\}/u)
    expect(source, directSupervisor.name).not.toMatch(/\bSHARE_ENV\b/u)
  })

  it('wires the disposable supervisor into the production headless service', async () => {
    const source = await readFile(productionServer.url, 'utf8')
    const imports = staticImports(source)

    expect(imports.filter(item => (
      /\bDirectGeometrySupervisor\b/u.test(item.bindings)
      && /(?:^|\/)directGeometrySupervisor(?:\.[cm]?[jt]s)?$/u.test(item.specifier)
    )), productionServer.name).toHaveLength(1)
    expect(source, productionServer.name).toMatch(
      /createGeometryRuntime:\s*\(\)\s*=>\s*new\s+DirectGeometrySupervisor\s*\(\s*\)/u,
    )
    expect(source, productionServer.name).toMatch(
      /new\s+HeadlessGeometryService\s*\(\s*defaultGeometryBuildEngine\s*,\s*runtime\s*\)/u,
    )
  })

  it('does not expose the qualificationOnly identity from production server or runtime code', async () => {
    for (const file of await readSources(productionRuntimeFiles)) {
      expect(file.source, file.name).not.toMatch(/\bqualificationOnly\b/u)
    }
  })
})
