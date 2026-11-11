import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { dirname, extname, join, relative, resolve } from 'node:path'
import {
  analyzeExchangeCorpusFile,
  createExchangeCorpusReport,
  type ExchangeCorpusFormat,
  type ExchangeCorpusOrigin,
  type ExchangeCorpusSourceReport,
} from '../src/exchange/exchangeCorpus'

interface SourceManifest {
  id: string
  origin: ExchangeCorpusOrigin
  locator: string
  localDirectory?: string
  unavailable?: boolean
}

const root = resolve(import.meta.dirname, '..')
const args = process.argv.slice(2)
const option = (name: string, fallback: string) => args.includes(name) ? args[args.indexOf(name) + 1]! : fallback
const manifestPath = resolve(root, option('--manifest', 'tools/exchange-corpus.json'))
const outputPath = resolve(root, option('--output', 'tmp/exchange-corpus-report.json'))
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8')) as { version: 1; sources: SourceManifest[] }
if (manifest.version !== 1 || !Array.isArray(manifest.sources)) throw new Error('Invalid exchange corpus manifest')

function files(directory: string): string[] {
  if (!existsSync(directory)) return []
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const path = join(directory, entry.name)
    return entry.isDirectory() ? files(path) : entry.isFile() ? [path] : []
  }).sort()
}

function format(path: string): ExchangeCorpusFormat | null {
  const extension = extname(path).toLowerCase()
  return extension === '.step' || extension === '.stp' ? 'step'
    : extension === '.iges' || extension === '.igs' ? 'iges'
      : extension === '.stl' ? 'stl' : null
}

const sources: ExchangeCorpusSourceReport[] = manifest.sources.map(source => {
  const directory = source.localDirectory ? resolve(root, source.localDirectory) : undefined
  const available = !source.unavailable && !!directory && existsSync(directory)
  const availability = source.unavailable ? 'unavailable' as const
    : !available ? 'not-downloaded' as const
      : source.origin === 'self-authored' ? 'local' as const : 'downloaded' as const
  const results = available ? files(directory!).flatMap(path => {
    const kind = format(path)
    if (!kind) return []
    const bytes = readFileSync(path)
    return [analyzeExchangeCorpusFile(
      relative(root, path).replaceAll('\\', '/'), kind, bytes,
      createHash('sha256').update(bytes).digest('hex'),
    )]
  }) : []
  return { id: source.id, origin: source.origin, locator: source.locator, availability, files: results }
})
const report = createExchangeCorpusReport(sources)
mkdirSync(dirname(outputPath), { recursive: true })
writeFileSync(outputPath, JSON.stringify(report, null, 2) + '\n')
console.log(JSON.stringify({ output: relative(root, outputPath).replaceAll('\\', '/'), totals: report.totals }))
