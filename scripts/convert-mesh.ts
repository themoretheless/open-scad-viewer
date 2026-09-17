#!/usr/bin/env node
/**
 * Convert a mesh file between formats using the repository's own decoders and
 * Rust writers. Run `npm run build:geometry` once, then:
 *
 *   node --import tsx scripts/convert-mesh.ts input.stl output.3mf
 *   node --import tsx scripts/convert-mesh.ts input.obj --to ply [--out dir/] [--weld 1e-4] [--source-format obj]
 *
 * Supported input: STL (ASCII/binary), OBJ, PLY (ASCII/binary), OFF, AMF, 3MF.
 * Supported output: stl, stl_binary, obj, ply, off, amf, 3mf.
 */
import { readFile, writeFile, mkdir } from 'node:fs/promises'
import { basename, dirname, extname, join, resolve } from 'node:path'
import {
  convertMeshFile,
  isMeshExportFormat,
  MESH_EXPORT_FORMATS,
  MESH_IMPORT_FORMATS,
  type MeshExportFormat,
  type MeshImportFormat,
} from '../src/services/meshConvert'
import { MeshImportError } from '../src/services/meshImport'

interface Options {
  input: string
  output?: string
  to?: MeshExportFormat
  outDir?: string
  weld?: number | false
  sourceFormat?: MeshImportFormat
  compressed: boolean
}

function usage(): never {
  console.error([
    'Usage: node --import tsx scripts/convert-mesh.ts <input> [<output>] [--to <format>] [--out <dir>] [--weld <tolerance>|off] [--source-format <format>] [--no-compress]',
    `  input formats:  ${MESH_IMPORT_FORMATS.join(', ')}`,
    `  output formats: ${MESH_EXPORT_FORMATS.join(', ')} (from <output> extension or --to)`,
  ].join('\n'))
  process.exit(2)
}

function parseArgs(argv: readonly string[]): Options {
  const positional: string[] = []
  const options: Partial<Options> = { compressed: true }
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]
    const value = () => argv[++i] ?? usage()
    switch (arg) {
      case '--to': {
        const format = value()
        if (!isMeshExportFormat(format)) usage()
        options.to = format
        break
      }
      case '--out': options.outDir = value(); break
      case '--weld': {
        const raw = value()
        if (raw === 'off' || raw === 'false') options.weld = false
        else {
          const tolerance = Number(raw)
          if (!Number.isFinite(tolerance) || tolerance < 0) usage()
          options.weld = tolerance
        }
        break
      }
      case '--source-format': {
        const format = value()
        if (!(MESH_IMPORT_FORMATS as readonly string[]).includes(format)) usage()
        options.sourceFormat = format as MeshImportFormat
        break
      }
      case '--no-compress': options.compressed = false; break
      case '-h': case '--help': usage()
      default:
        if (arg.startsWith('--')) usage()
        positional.push(arg)
    }
  }
  if (positional.length < 1 || positional.length > 2) usage()
  return { input: positional[0], output: positional[1], compressed: true, ...options }
}

function targetFormat(options: Options): MeshExportFormat {
  if (options.to) return options.to
  const extension = options.output ? extname(options.output).slice(1).toLowerCase() : ''
  if (extension === 'stl') return 'stl_binary'
  if (isMeshExportFormat(extension)) return extension
  usage()
}

async function main(): Promise<void> {
  const options = parseArgs(process.argv.slice(2))
  const format = targetFormat(options)
  const inputPath = resolve(options.input)
  const bytes = new Uint8Array(await readFile(inputPath))
  const result = await convertMeshFile(basename(inputPath), bytes, format, {
    format: options.sourceFormat,
    weld: options.weld,
    compressed: options.compressed,
  })
  const outputPath = options.output
    ? resolve(options.output)
    : join(options.outDir ? resolve(options.outDir) : dirname(inputPath), result.fileName)
  await mkdir(dirname(outputPath), { recursive: true })
  await writeFile(outputPath, result.data)
  const { source } = result
  console.log(`${source.format.toUpperCase()} → ${format}: ${outputPath}`)
  console.log(`  ${source.triangleCount.toLocaleString()} triangles, ${source.vertexCount.toLocaleString()} vertices`
    + (source.sourceVertexCount !== source.vertexCount ? ` (welded from ${source.sourceVertexCount.toLocaleString()})` : '')
    + (source.degenerateTriangles ? `, ${source.degenerateTriangles} degenerate dropped` : '')
    + `, ${result.data.byteLength.toLocaleString()} bytes`)
}

main().catch(error => {
  if (error instanceof MeshImportError) console.error(`Import error (${error.code}): ${error.message}`)
  else console.error(error instanceof Error ? error.message : String(error))
  process.exit(1)
})
