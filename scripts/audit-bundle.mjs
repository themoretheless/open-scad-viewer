import { createHash } from 'node:crypto'
import { readdir, readFile } from 'node:fs/promises'
import path from 'node:path'
import { pathToFileURL } from 'node:url'

export async function auditBundle(directory) {
  const assets = []
  const sources = new Map()
  let sourceMaps = 0
  async function visit(relative = '') {
    const entries = await readdir(path.join(directory, relative), { withFileTypes: true })
    for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
      const name = path.posix.join(relative, entry.name)
      if (entry.isDirectory()) {
        await visit(name)
      } else if (entry.isFile()) {
        const bytes = await readFile(path.join(directory, name))
        if (!name.endsWith('.map')) {
          assets.push({ file: name, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') })
          continue
        }
        if (!name.endsWith('.js.map')) continue
        sourceMaps++
        const map = JSON.parse(bytes.toString('utf8'))
        if (!Array.isArray(map.sources) || !Array.isArray(map.sourcesContent)) {
          throw new Error(`Expected sourcesContent in ${name}; build with vite build --sourcemap`)
        }
        map.sources.forEach((source, index) => {
          const content = map.sourcesContent[index]
          if (typeof content !== 'string') return
          // Content identity separates genuinely different transforms of one source.
          const sha256 = createHash('sha256').update(content).digest('hex')
          const resolved = path.posix.normalize(path.posix.join(path.posix.dirname(name), map.sourceRoot || '', source))
          const key = `${resolved}\0${sha256}`
          const record = sources.get(key) || { source: resolved, sha256, sourceBytes: Buffer.byteLength(content), chunks: new Set() }
          record.chunks.add(name.slice(0, -4))
          sources.set(key, record)
        })
      }
    }
  }
  await visit()
  if (!sourceMaps) throw new Error('No JavaScript source maps; first run vite build --sourcemap')
  const repeatedSources = [...sources.values()]
    .filter(record => record.chunks.size > 1)
    .map(record => ({ ...record, chunks: [...record.chunks].sort(), repeatedSourceBytes: record.sourceBytes * (record.chunks.size - 1) }))
    .sort((a, b) => b.repeatedSourceBytes - a.repeatedSourceBytes || a.source.localeCompare(b.source))
  return {
    schema: 1,
    caveat: 'Source bytes are unminified attribution, NOT removable bundle bytes. Source maps are excluded from asset totals; sourcemap comments remain. Repeated code may be required by separate worker realms.',
    totalAssetBytes: assets.reduce((total, asset) => total + asset.bytes, 0),
    sourceMaps,
    assets: assets.sort((a, b) => b.bytes - a.bytes || a.file.localeCompare(b.file)),
    repeatedSources,
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  console.log(JSON.stringify(await auditBundle(path.resolve(process.argv[2] || 'dist')), null, 2))
}
