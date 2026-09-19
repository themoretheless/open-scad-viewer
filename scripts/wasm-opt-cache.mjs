import { createHash, randomUUID } from 'node:crypto'
import { mkdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex')
const maxEntryBytes = 32 * 1024 * 1024

/** Local derived-data cache; content hashes detect corruption, not hostile authors. */
export function cachedWasmOptimization({ input, tool, flags, cacheDir, optimize }) {
  const key = sha256(JSON.stringify({ version: 1, input: sha256(input), tool: sha256(tool), flags,
    node: process.version, platform: process.platform, arch: process.arch }))
  const file = join(cacheDir, `${key}.json`)
  try {
    if (statSync(file).size <= maxEntryBytes) {
      const entry = JSON.parse(readFileSync(file, 'utf8'))
      if (entry.key === key && typeof entry.bytes === 'string') {
        const bytes = Buffer.from(entry.bytes, 'base64')
        if (sha256(bytes) === entry.sha256 && WebAssembly.validate(bytes)) {
          console.log(`wasm-opt cache hit: ${key}`)
          return bytes
        }
      }
    }
  } catch { /* Missing or damaged cache entries are misses. */ }

  const bytes = optimize()
  if (!WebAssembly.validate(bytes)) throw new Error('wasm-opt produced an invalid module')
  const temporary = join(cacheDir, `.${key}-${randomUUID()}.tmp`)
  try {
    const entry = JSON.stringify({ key, sha256: sha256(bytes), bytes: Buffer.from(bytes).toString('base64') })
    if (Buffer.byteLength(entry) <= maxEntryBytes) {
      mkdirSync(cacheDir, { recursive: true })
      writeFileSync(temporary, entry, { flag: 'wx' })
      renameSync(temporary, file)
    }
  } catch (error) {
    console.warn(`wasm-opt cache write skipped: ${error.message}`)
  } finally {
    try { rmSync(temporary, { force: true }) } catch { /* Cache is optional. */ }
  }
  return bytes
}
