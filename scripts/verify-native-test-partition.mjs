// Test scheduling only; native geometry and its proof budgets remain in Rust.
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {spawnSync} from 'node:child_process'
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  writeFileSync,
} from 'node:fs'
import {relative, resolve} from 'node:path'
import {fileURLToPath, pathToFileURL} from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const sha = value => createHash('sha256').update(value).digest('hex')
const NON_RUST_FENCES = new Set([
  'ignore',
  'text',
  'plain',
  'plaintext',
  'sh',
  'bash',
  'shell',
  'console',
  'xml',
  'svg',
  'html',
  'json',
  'toml',
  'yaml',
  'yml',
  'md',
  'markdown',
  'sql',
  'c',
  'cpp',
  'glsl',
  'wgsl',
])

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

export function compiledTestTargets(jsonLines, workspaceRoot = root) {
  const seen = new Set()
  const targets = []
  for (const line of jsonLines.split(/\r?\n/u)) {
    if (!line.startsWith('{')) continue
    const msg = JSON.parse(line)
    if (msg.reason !== 'compiler-artifact' || !msg.profile?.test || !msg.executable) continue
    if (seen.has(msg.executable)) continue
    seen.add(msg.executable)
    const srcPath = msg.target?.src_path
      ? relative(workspaceRoot, msg.target.src_path).replaceAll('\\', '/')
      : msg.target?.name ?? 'test'
    targets.push({srcPath, executable: msg.executable})
  }
  return targets
}

export function hasRunnableDoctest(source) {
  let inFence = false
  for (const line of source.split(/\r?\n/u)) {
    const doc = /^\s*\/\/[!/](.*)$/u.exec(line)
    if (!doc) {
      inFence = false
      continue
    }
    const fence = /^\s*```(.*)$/u.exec(doc[1])
    if (!fence) continue
    if (inFence) {
      inFence = false
      continue
    }
    const tags = fence[1]
      .trim()
      .split(/[\s,]+/u)
      .filter(Boolean)
      .map(tag => tag.toLowerCase())
    if (tags.length === 0 || !tags.some(tag => NON_RUST_FENCES.has(tag))) return true
    inFence = true
  }
  return false
}

function walkRustSources(dir, out = []) {
  if (!existsSync(dir)) return out
  for (const entry of readdirSync(dir, {withFileTypes: true})) {
    const full = resolve(dir, entry.name)
    if (entry.isDirectory()) walkRustSources(full, out)
    else if (entry.isFile() && entry.name.endsWith('.rs')) out.push(full)
  }
  return out
}

export function cratesWithRunnableDoctests(cratesDir = resolve(root, 'crates')) {
  const packages = []
  for (const entry of readdirSync(cratesDir, {withFileTypes: true})) {
    if (!entry.isDirectory() || entry.name === 'vendor') continue
    const manifestPath = resolve(cratesDir, entry.name, 'Cargo.toml')
    if (!existsSync(manifestPath)) continue
    const manifest = readFileSync(manifestPath, 'utf8')
    const pkgMatch = /\[package\][\s\S]*?^\s*name\s*=\s*"([^"]+)"/mu.exec(manifest)
    if (!pkgMatch) continue
    if (/\[lib\][\s\S]*?^\s*doctest\s*=\s*false/mu.test(manifest)) continue
    const files = walkRustSources(resolve(cratesDir, entry.name, 'src'))
    if (files.some(file => hasRunnableDoctest(readFileSync(file, 'utf8')))) {
      packages.push(pkgMatch[1])
    }
  }
  return packages.sort()
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

function collectWorkspaceCatalogs(paths) {
  const env = {...process.env, CARGO_TERM_COLOR: 'never'}
  const build = spawnSync(
    'cargo',
    ['test', '--locked', '--manifest-path', 'crates/Cargo.toml', '--workspace', '--no-run', '--message-format=json'],
    {cwd: root, env, encoding: 'utf8', maxBuffer: 1 << 27},
  )
  assert(!build.error && build.status === 0, `Test build failed: ${build.error ?? build.stderr ?? build.status}`)
  const targets = compiledTestTargets(build.stdout, root)
  assert(targets.length > 0, 'No compiled test binaries reported by Cargo')
  const docPackages = cratesWithRunnableDoctests()
  const binarySelections = [
    ['--list'],
    ['aag', '--list'],
    ['--skip', 'aag', '--list'],
  ]
  const docSelections = [
    ['--', '--list'],
    ['aag', '--', '--list'],
    ['--', '--skip', 'aag', '--list'],
  ]
  for (let i = 0; i < paths.length; i++) {
    const chunks = []
    for (const target of targets) {
      const res = spawnSync(target.executable, binarySelections[i], {
        cwd: root,
        env,
        encoding: 'utf8',
        maxBuffer: 1 << 26,
      })
      assert(
        !res.error && res.status === 0,
        `Binary catalog failed (${target.executable}): ${res.error ?? res.stderr ?? res.status}`,
      )
      chunks.push(`     Running ${target.srcPath} (${target.executable})\n${res.stdout}`)
    }
    if (docPackages.length > 0) {
      const pkgArgs = docPackages.flatMap(pkg => ['-p', pkg])
      const docRes = spawnSync(
        'cargo',
        ['test', '--locked', '--manifest-path', 'crates/Cargo.toml', ...pkgArgs, '--doc', ...docSelections[i]],
        {cwd: root, env, encoding: 'utf8', maxBuffer: 1 << 26},
      )
      assert(
        !docRes.error && docRes.status === 0,
        `Doctest catalog failed: ${docRes.error ?? docRes.stderr ?? docRes.status}`,
      )
      chunks.push(docRes.stderr, docRes.stdout)
    }
    writeFileSync(paths[i], chunks.join(''), {flag: 'wx'})
  }
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
    collectWorkspaceCatalogs(paths)
  }
  assert(!existsSync(output), 'Refusing to replace catalog evidence')
  const bytes = paths.map(p => readFileSync(p))
  const report = {...verifyNativePartition(...bytes.map(b => b.toString('utf8'))),
    logs: paths.map((path, i) => ({path, sha256: sha(bytes[i]), byteLength: bytes[i].length}))}
  writeFileSync(output, JSON.stringify(report, null, 2) + '\n', {flag: 'wx'})
  console.log(`Native catalog: ${report.full.cases} = ${report.serial.cases} serial + ${report.parallel.cases} parallel; no omissions or overlap`)
}
if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) main(process.argv.slice(2))
