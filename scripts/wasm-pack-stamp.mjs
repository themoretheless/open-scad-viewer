import {createHash, randomUUID} from 'node:crypto'
import {readFileSync, statSync, writeFileSync, renameSync, rmSync} from 'node:fs'
import {dirname, relative, resolve} from 'node:path'
import {fileURLToPath} from 'node:url'

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex')
const schema = 'wasm-pack-stamp-v2'
const manifestLimit = 64 * 1024
const filePath = path => path instanceof URL ? fileURLToPath(path) : resolve(path)
function outputPaths(stampPath, outputs) {
  if (!Array.isArray(outputs) || outputs.length < 1 || outputs.length > 64) throw new Error('Need 1..64 packaging outputs')
  const root = dirname(filePath(stampPath))
  const paths = outputs.map(path => ({file: filePath(path), path: relative(root, filePath(path)).replaceAll('\\', '/')}))
    .sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0)
  if (new Set(paths.map(path => path.path)).size !== paths.length) throw new Error('Packaging outputs must be unique')
  return paths
}
function snapshot({file, path}) {
  const stat = statSync(file)
  if (!stat.isFile()) throw new Error('Packaging output must be a file')
  const bytes = readFileSync(file)
  if (bytes.length !== stat.size) throw new Error('Packaging output changed during hashing')
  return {path, byteLength: bytes.length, sha256: sha256(bytes)}
}
/** Integrity cache only, not build provenance or a multi-publisher transaction.
 * Old source-only stamps are misses. Validate every requested output, including
 * equal-length corruption; never open a path supplied by the stamp itself.
 */
export function packStampHit(stampPath, sourceBytes, outputs) {
  try {
    if (statSync(stampPath).size > manifestLimit) return false
    const manifest = JSON.parse(readFileSync(stampPath, 'utf8'))
    const paths = outputPaths(stampPath, outputs)
    if (manifest?.schema !== schema || manifest.sourceSha256 !== sha256(sourceBytes)
      || !Array.isArray(manifest.outputs) || manifest.outputs.length !== paths.length) return false
    return paths.every((output, i) => {
      const expected = manifest.outputs[i]
      if (!expected || expected.path !== output.path || !Number.isSafeInteger(expected.byteLength)
        || expected.byteLength < 0 || !/^[a-f0-9]{64}$/.test(expected.sha256)) return false
      // Reject oversized/truncated corrupt outputs before allocating a buffer.
      const stat = statSync(output.file)
      if (!stat.isFile() || stat.size !== expected.byteLength) return false
      const actual = snapshot(output)
      return actual.byteLength === expected.byteLength && actual.sha256 === expected.sha256
    })
  } catch {return false}
}
/** Publish an output integrity snapshot only after all artifacts are present.
 * A failed snapshot or write preserves the previous stamp. Paths are portable
 * relative labels; callers still supply the files checked on every cache hit.
 */
export function writePackStamp(stampPath, sourceBytes, outputs) {
  const manifest = {schema, sourceSha256: sha256(sourceBytes), outputs: outputPaths(stampPath, outputs).map(snapshot)}
  const text = `${JSON.stringify(manifest)}\n`
  if (Buffer.byteLength(text) > manifestLimit) throw new Error('Packaging stamp exceeds manifest limit')
  const destination = filePath(stampPath)
  const temporary = `${destination}.${process.pid}.${randomUUID()}.tmp`
  try {
    writeFileSync(temporary, text, {flag: 'wx'})
    renameSync(temporary, destination)
  } finally {rmSync(temporary, {force: true})}
}
