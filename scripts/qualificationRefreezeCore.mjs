// Shared filesystem mechanics for explicit, versioned qualification re-freezes.
import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { existsSync, lstatSync, readFileSync, writeFileSync, linkSync, unlinkSync } from 'node:fs'
import { isAbsolute, relative, resolve, sep } from 'node:path'

export const REFREEZE_CORE = 'scripts/qualificationRefreezeCore.mjs'
export const digest = bytes => createHash('sha256').update(bytes).digest('hex')
export const jsonBytes = value => Buffer.from(`${JSON.stringify(value, null, 2)}\n`, 'utf8')
export const shaRecord = bytes => ({ sha256: digest(bytes), byteLength: bytes.byteLength })
export const bundleBytes = (paths, snapshot) => Buffer.from([...paths].sort()
  .map(path => `${path}\0${snapshot.get(path).sha256}\n`).join(''), 'utf8')

export function ordinaryBytes(root, path) {
  assert(typeof path === 'string' && /^[\x20-\x7e]+$/u.test(path), `Non-ASCII bound path: ${path}`)
  assert(!isAbsolute(path) && !path.split('/').some(part => part === '..' || part === ''), `Unsafe bound path: ${path}`)
  const target = resolve(root, path)
  assert(!relative(root, target).startsWith(`..${sep}`), `Path leaves repository: ${path}`)
  let current = root
  for (const component of path.split('/')) {
    current = resolve(current, component)
    assert(!lstatSync(current).isSymbolicLink(), `Symlink in bound path: ${path}`)
  }
  assert(lstatSync(target).isFile(), `Not an ordinary file: ${path}`)
  const bytes = readFileSync(target)
  assert(!bytes.includes(13), `CR byte in bound text file: ${path}`)
  return bytes
}

export function snapshotFiles(root, paths) {
  return new Map([...new Set(paths)].sort().map(path => [path, shaRecord(ordinaryBytes(root, path))]))
}

export function assertNoCandidateResults(root, candidateRunId) {
  for (const name of ['result.json', 'fragments.jsonl']) {
    assert(!existsSync(resolve(root, `output/qualification/${candidateRunId}/${name}`)),
      'The new candidate already has execution bookkeeping; a zero-counter re-freeze cannot reuse it')
  }
}

/** Publish all new versions with exclusive links; roll back only our own links. */
export function publishExclusive({ root, outputs, artifactBytes, assertStable }) {
  for (const path of Object.values(outputs)) assert(!existsSync(resolve(root, path)), `Refusing existing artifact: ${path}`)
  assertStable()
  const staged = []
  const published = []
  try {
    for (const [id, path] of Object.entries(outputs)) {
      const target = resolve(root, path)
      const temporary = `${target}.refresh-${randomUUID()}.tmp`
      writeFileSync(temporary, artifactBytes[id], { flag: 'wx' })
      staged.push(temporary)
      assertStable()
      linkSync(temporary, target)
      published.push({ target, temporary })
    }
    assertStable()
  } catch (error) {
    for (const { target, temporary } of published.reverse()) {
      if (existsSync(target) && lstatSync(target).ino === lstatSync(temporary).ino) unlinkSync(target)
    }
    throw error
  } finally {
    for (const temporary of staged) unlinkSync(temporary)
  }
}
