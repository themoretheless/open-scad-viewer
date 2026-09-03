import { readdirSync, readFileSync } from 'node:fs'
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'
import ts from 'typescript'
import { describe, expect, it } from 'vitest'
import { meshTransferables, type MeshData } from '../src/core/mesh'

function fixtureMesh(): MeshData {
  return {
    vertices: new Float32Array(18),
    indices: new Uint32Array(3),
    edgeIndices: new Uint32Array(6),
    transform: new Float32Array(16),
    faceIds: new Uint32Array(1),
    bvh: {
      version: 1,
      vertexStride: 6,
      leafSize: 8,
      nodeCount: 1,
      bounds: new Float32Array(6),
      nodes: new Uint32Array(2),
      triangles: new Uint32Array(1),
    },
    color: [1, 1, 1, 1],
    provenance: [],
    topology: { boundary: 0, crease: 0, nonManifold: 0, degenerate: 0 },
  }
}

function meshBuffers(mesh: MeshData): ArrayBuffer[] {
  const buffers = [
    mesh.vertices.buffer,
    mesh.indices.buffer,
    mesh.edgeIndices.buffer,
    mesh.transform.buffer,
    mesh.faceIds.buffer,
    mesh.bvh.bounds.buffer,
    mesh.bvh.nodes.buffer,
    mesh.bvh.triangles.buffer,
  ]
  return buffers.map(buffer => {
    if (!(buffer instanceof ArrayBuffer)) throw new TypeError('Fixture must use transferable ArrayBuffers')
    return buffer
  })
}

function sourceFiles(root: string): string[] {
  return readdirSync(root, { withFileTypes: true }).flatMap(entry => {
    const path = join(root, entry.name)
    if (entry.isDirectory()) return sourceFiles(path)
    return /\.(?:ts|vue)$/.test(entry.name) ? [path] : []
  })
}

function moduleSpecifiersFromSource(source: string, fileName = 'boundary.ts'): string[] {
  const file = ts.createSourceFile(fileName, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)
  const specifiers: string[] = []
  const addStringLiteral = (node: ts.Expression | undefined) => {
    if (node && ts.isStringLiteralLike(node)) specifiers.push(node.text)
  }

  const visit = (node: ts.Node) => {
    if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) {
      addStringLiteral(node.moduleSpecifier)
    } else if (ts.isImportEqualsDeclaration(node)
      && ts.isExternalModuleReference(node.moduleReference)) {
      addStringLiteral(node.moduleReference.expression)
    } else if (ts.isCallExpression(node)
      && (node.expression.kind === ts.SyntaxKind.ImportKeyword
        || (ts.isIdentifier(node.expression) && node.expression.text === 'require'))) {
      addStringLiteral(node.arguments[0])
    }
    ts.forEachChild(node, visit)
  }
  visit(file)
  return specifiers
}

function moduleSpecifiers(path: string): string[] {
  const source = readFileSync(path, 'utf8')
  if (!path.endsWith('.vue')) return moduleSpecifiersFromSource(source, path)
  return [...source.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script>/gi)]
    .flatMap((match, index) => moduleSpecifiersFromSource(match[1], `${path}.${index}.ts`))
}

function isParserModule(specifier: string): boolean {
  const withoutQuery = specifier.replace(/[?#].*$/, '')
  return /(?:^|\/)openscadParser(?:\.[cm]?[jt]sx?)?$/.test(withoutQuery)
}

function isServiceModule(specifier: string): boolean {
  const withoutQuery = specifier.replace(/[?#].*$/, '')
  return withoutQuery.split('/').includes('services')
}

function isWithin(root: string, candidate: string): boolean {
  const pathFromRoot = relative(root, candidate)
  return pathFromRoot === ''
    || (!isAbsolute(pathFromRoot) && pathFromRoot !== '..' && !pathFromRoot.startsWith(`..${sep}`))
}

function isForbiddenBrowserImport(importer: string, specifier: string, mcpRoot: string): boolean {
  const withoutQuery = specifier.replace(/[?#].*$/, '')
  if (withoutQuery.startsWith('node:')) return true
  if (withoutQuery === '@duckdb/node-api' || withoutQuery.startsWith('@duckdb/node-api/')) return true
  if (withoutQuery === '@modelcontextprotocol/server'
    || withoutQuery.startsWith('@modelcontextprotocol/server/')) return true
  if (withoutQuery.startsWith('.')) {
    return isWithin(mcpRoot, resolve(dirname(importer), withoutQuery))
  }
  return withoutQuery === 'src/mcp'
    || withoutQuery.startsWith('src/mcp/')
    || withoutQuery === '/src/mcp'
    || withoutQuery.startsWith('/src/mcp/')
}

describe('core mesh publication', () => {
  it('includes every independently owned publication buffer', () => {
    const mesh = fixtureMesh()
    const expected = meshBuffers(mesh)

    expect(new Set(expected).size).toBe(8)
    expect(meshTransferables([mesh])).toEqual(expected)
  })

  it('deduplicates shared backing buffers across fields and meshes', () => {
    const mesh = fixtureMesh()
    mesh.transform = new Float32Array(mesh.vertices.buffer, 0, 16)
    const expected = [...new Set(meshBuffers(mesh))]

    expect(expected).toHaveLength(7)
    expect(meshTransferables([mesh, mesh])).toEqual(expected)
  })

  it('transfers and detaches every independently owned publication buffer', () => {
    const mesh = fixtureMesh()
    const buffers = meshTransferables([mesh])
    const clone = structuredClone(mesh, { transfer: buffers })

    expect(buffers.every(buffer => buffer.byteLength === 0)).toBe(true)
    expect(meshBuffers(clone).every(buffer => buffer.byteLength > 0)).toBe(true)
  })

  it('recognizes static, side-effect, export-from and dynamic module references', () => {
    const specifiers = moduleSpecifiersFromSource(`
      import parser from './static/openscadParser'
      import './side-effect/openscadParser.js'
      export { parser } from './exported/openscadParser.ts'
      const lazy = import('./dynamic/openscadParser.mjs')
      const legacy = require('./legacy/openscadParser.cjs')
    `)

    expect(specifiers).toHaveLength(5)
    expect(specifiers.every(isParserModule)).toBe(true)
  })

  it('keeps parser execution behind explicit browser-worker and engine-facade boundaries', () => {
    const testDir = dirname(fileURLToPath(import.meta.url))
    const sourceRoot = join(testDir, '..', 'src')
    const parserConsumers = sourceFiles(sourceRoot)
      .filter(path => moduleSpecifiers(path).some(isParserModule))
      .map(path => relative(sourceRoot, path))
      .sort()
    const coreServiceImports = sourceFiles(join(sourceRoot, 'core'))
      .flatMap(path => moduleSpecifiers(path)
        .filter(isServiceModule)
        .map(specifier => `${relative(sourceRoot, path)} -> ${specifier}`))
      .sort()

    expect(parserConsumers).toEqual([
      'mcp/independentOpenScadExecution.ts',
      'services/geometryBuildEngine.ts',
      'workers/geometry.worker.ts',
    ])
    expect(coreServiceImports).toEqual([])
  })

  it('keeps Node, DuckDB, and MCP server imports out of browser source', () => {
    const testDir = dirname(fileURLToPath(import.meta.url))
    const sourceRoot = join(testDir, '..', 'src')
    const mcpRoot = join(sourceRoot, 'mcp')
    const violations = sourceFiles(sourceRoot)
      .filter(path => !isWithin(mcpRoot, path))
      .flatMap(path => moduleSpecifiers(path)
        .filter(specifier => isForbiddenBrowserImport(path, specifier, mcpRoot))
        .map(specifier => `${relative(sourceRoot, path)} -> ${specifier}`))
      .sort()

    expect(isForbiddenBrowserImport(join(sourceRoot, 'main.ts'), './mcp/createServer', mcpRoot)).toBe(true)
    expect(isForbiddenBrowserImport(join(sourceRoot, 'main.ts'), 'node:fs', mcpRoot)).toBe(true)
    expect(isForbiddenBrowserImport(join(sourceRoot, 'main.ts'), '@duckdb/node-api', mcpRoot)).toBe(true)
    expect(isForbiddenBrowserImport(join(sourceRoot, 'main.ts'), '@modelcontextprotocol/server/stdio', mcpRoot)).toBe(true)
    expect(violations).toEqual([])
  })
})
