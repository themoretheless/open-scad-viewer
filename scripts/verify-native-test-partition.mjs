// Test scheduling only; native geometry and its proof budgets remain in Rust.
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {spawnSync} from 'node:child_process'
import {existsSync, mkdirSync, openSync, closeSync, readFileSync, writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {fileURLToPath, pathToFileURL} from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const sha = value => createHash('sha256').update(value).digest('hex')
export function nativeTestCatalog(text) {
  const cases = new Set()
  let target
  for (const line of text.split(/\r?\n/u)) {
    const running = /\bRunning .*\(([^)]+)\)\s*$/u.exec(line)
    const docs = /^\s*Doc-tests\s+(.+)$/u.exec(line)
    if (running) target = `binary:${running[1]}`
    else if (docs) target = `docs:${docs[1]}`
    else {
      const test = /^(.+): (?:test|benchmark)$/u.exec(line)
      if (!test) continue
      assert(target, `Test without a target: ${line}`)
      const key = `${target}\0${test[1]}`
      assert(!cases.has(key), `Duplicate catalog entry: ${key}`)
      cases.add(key)
    }
  }
  return cases
}

export function verifyNativePartition(fullText, serialText, parallelText) {
  const full = nativeTestCatalog(fullText), serial = nativeTestCatalog(serialText)
  const parallel = nativeTestCatalog(parallelText)
  assert(full.size > 0, 'Empty complete catalog')
  assert(serial.size > 0, 'Empty recognition catalog')
  for (const key of serial) {
    assert(!parallel.has(key), `Test runs in both partitions: ${key}`)
    assert(full.has(key), `Unexpected recognition test: ${key}`)
  }
  for (const key of parallel) assert(full.has(key), `Unexpected parallel test: ${key}`)
  for (const key of full) assert(serial.has(key) || parallel.has(key), `Omitted test: ${key}`)
  const hash = set => sha([...set].sort().join('\n') + '\n')
  return {schema: 'native-test-partition/1', complete: true, disjoint: true,
    full: {cases: full.size, sha256: hash(full)},
    serial: {cases: serial.size, sha256: hash(serial)},
    parallel: {cases: parallel.size, sha256: hash(parallel)},
    selector: 'aag', qualificationClaim: 'none'}
}

function main(args) {
  let paths, output
  if (args[0] === '--existing') {
    assert.equal(args.length, 5, '--existing full.log serial.log parallel.log report.json')
    paths = args.slice(1, 4).map(p => resolve(p)); output = resolve(args[4])
  } else {
    assert(args.length <= 1, 'Expected an output directory')
    const directory = resolve(args[0] ?? 'output/qualification/native-test-catalog')
    mkdirSync(directory, {recursive: true})
    paths = ['full', 'serial', 'parallel'].map(name => resolve(directory, `${name}.log`))
    output = resolve(directory, 'partition.json')
    const base = ['test', '--locked', '--manifest-path', 'crates/Cargo.toml', '--workspace']
    const selections = [['--', '--list'], ['aag', '--', '--list'], ['--', '--skip', 'aag', '--list']]
    for (let i = 0; i < paths.length; i++) {
      const fd = openSync(paths[i], 'wx')
      let result
      try { result = spawnSync('cargo', [...base, ...selections[i]], {cwd: root, env: {...process.env, CARGO_TERM_COLOR: 'never'}, stdio: ['ignore', fd, fd]}) }
      finally {closeSync(fd)}
      assert(!result.error && result.status === 0, `Catalog failed: ${paths[i]}; ${result.error ?? result.status}`)
    }
  }
  assert(!existsSync(output), 'Refusing to replace catalog evidence')
  const bytes = paths.map(p => readFileSync(p))
  const report = {...verifyNativePartition(...bytes.map(b => b.toString('utf8'))),
    logs: paths.map((path, i) => ({path, sha256: sha(bytes[i]), byteLength: bytes[i].length}))}
  writeFileSync(output, JSON.stringify(report, null, 2) + '\n', {flag: 'wx'})
  console.log(`Native catalog: ${report.full.cases} = ${report.serial.cases} serial + ${report.parallel.cases} parallel; no omissions or overlap`)
}
if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) main(process.argv.slice(2))
