#!/usr/bin/env node

import { createHash } from 'node:crypto'
import { execFileSync, spawn } from 'node:child_process'
import {
  createReadStream,
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readlinkSync,
  readdirSync,
  writeFileSync,
} from 'node:fs'
import { basename, dirname, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const PLAN_PATH = 'docs/qualification/semantic-manifold-g1-plan-v38.json'
const FREEZE_PATH = 'docs/qualification/environment-freeze/g1-runtime-browser-bindings-v1.json'
const GITHUB_FREEZE_PATH = 'docs/qualification/environment-freeze/g1-github-actions-v34.json'
const PLAN_ID = 'semantic-manifold-g1-plan-v38'
const CANDIDATE_ID = 'semantic-manifold-g1-candidate-run-v38'
const NPM_VERSION = '10.9.8'
const OUTPUT_ROOT = `output/qualification/${CANDIDATE_ID}/github-actions`
const FORBIDDEN_ENV = [
  'NODE_OPTIONS', 'NODE_PATH', 'NODE_PRESERVE_SYMLINKS',
  'NPM_CONFIG_PREFIX', 'NPM_CONFIG_CACHE', 'npm_config_registry',
  'HTTP_PROXY', 'HTTPS_PROXY', 'ALL_PROXY', 'http_proxy', 'https_proxy', 'all_proxy',
  'PLAYWRIGHT_BROWSERS_PATH', 'PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD',
  'LD_PRELOAD', 'DYLD_INSERT_LIBRARIES',
]

function fail(message) {
  throw new Error(message)
}

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex')
}

function sha256File(path) {
  const bytes = readFileSync(path)
  return { value: sha256(bytes), byteLength: bytes.byteLength }
}

function loadJson(path) {
  return JSON.parse(readFileSync(resolve(root, path), 'utf8'))
}

function loadPlan() {
  const plan = loadJson(PLAN_PATH)
  if (plan.planId !== PLAN_ID || plan.executionProtocol?.candidateRunId !== CANDIDATE_ID) {
    fail(`Expected ${PLAN_ID}/${CANDIDATE_ID}`)
  }
  if (plan.executionProtocol?.priorResultsMayBeImported !== false
      || plan.lifecycle?.qualificationClaim !== 'none') {
    fail('Frozen plan claim boundary changed')
  }
  return plan
}

function args(argv) {
  const result = new Map()
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index]
    const value = argv[index + 1]
    if (!key?.startsWith('--') || value === undefined) fail(`Invalid argument near ${key ?? '<end>'}`)
    result.set(key, value)
  }
  return result
}

function matrix() {
  const plan = loadPlan()
  const nodes = []
  const browsers = []
  const runnerByEnvironment = {
    'ubuntu-node20': 'ubuntu-24.04',
    'ubuntu-node22': 'ubuntu-24.04',
    'macos-node20': 'macos-15',
    'windows-node20': 'windows-2025',
    'vite-chromium': 'ubuntu-24.04',
    'vite-webkit': 'ubuntu-24.04',
  }
  for (const row of plan.matrix) {
    for (const environmentId of row.executionEnvironmentIds) {
      const environment = plan.environments.find(item => item.id === environmentId)
      if (!environment) fail(`Missing environment ${environmentId}`)
      const destination = environment.class === 'vite-browser' ? browsers : nodes
      for (let runIndex = 1; runIndex <= row.work.cleanRunsRequired; runIndex += 1) {
        destination.push({
          row: row.id,
          environment: environmentId,
          run_index: runIndex,
          runner: runnerByEnvironment[environmentId],
          node_version: environment.runtime.version,
          browser: environment.browser?.engine ?? '',
        })
      }
    }
  }
  const expectedUnits = [...nodes, ...browsers].reduce((total, entry) => {
    const row = plan.matrix.find(item => item.id === entry.row)
    return total + row.work.unitsPerCleanRun
  }, 0)
  if (expectedUnits !== plan.executionProtocol.plannedWorkUnits) {
    fail(`Matrix expands to ${expectedUnits}, expected ${plan.executionProtocol.plannedWorkUnits}`)
  }
  process.stdout.write(`${JSON.stringify({ nodes: { include: nodes }, browsers: { include: browsers } })}\n`)
}

function regularFrozenFile(relativePath) {
  const path = resolve(root, relativePath)
  const stat = lstatSync(path)
  if (!stat.isFile() || stat.isSymbolicLink()) fail(`Bound path is not an ordinary file: ${relativePath}`)
  const bytes = readFileSync(path)
  if (bytes.includes(13)) fail(`Bound text path contains CR bytes: ${relativePath}`)
  return bytes
}

function verifyBindings(plan) {
  const artifacts = {}
  for (const artifact of plan.bindings.artifacts) {
    const bytes = regularFrozenFile(artifact.path)
    const actual = { value: sha256(bytes), byteLength: bytes.byteLength }
    if (actual.value !== artifact.sha256.value || actual.byteLength !== artifact.sha256.byteLength) {
      fail(`Frozen artifact mismatch: ${artifact.path}`)
    }
    artifacts[artifact.id] = actual
  }
  const bundles = {}
  for (const bundle of plan.bindings.bundles) {
    const records = [...bundle.paths].sort().map(path => {
      const digest = sha256(regularFrozenFile(path))
      return `${path}\0${digest}\n`
    }).join('')
    const bytes = Buffer.from(records, 'utf8')
    const actual = { value: sha256(bytes), byteLength: bytes.byteLength }
    if (actual.value !== bundle.sha256.value || actual.byteLength !== bundle.sha256.byteLength) {
      fail(`Frozen bundle mismatch: ${bundle.id}`)
    }
    bundles[bundle.id] = actual
  }
  const rootManifest = loadJson('package.json')
  const rootLock = loadJson('package-lock.json')
  const forbiddenPackage = value => value && typeof value === 'object'
    && Object.keys(value).some(key => key === 'playwright' || key === 'playwright-core'
      || key.endsWith('/playwright') || key.endsWith('/playwright-core'))
  if (forbiddenPackage(rootManifest.dependencies) || forbiddenPackage(rootManifest.devDependencies)
      || forbiddenPackage(rootLock.packages)) {
    fail('Playwright escaped into the root dependency graph')
  }
  return { artifacts, bundles }
}

function normalizedArch(arch) {
  return arch === 'x64' ? 'x86_64' : arch === 'arm64' ? 'aarch64' : arch
}

function normalizedPlatform(platform) {
  return platform === 'linux' ? 'ubuntu' : platform === 'darwin' ? 'macos' : platform
}

function githubRunnerKey(environment) {
  if (environment.os === 'macos-15' && environment.architecture === 'aarch64') return 'macos-15-arm64'
  return environment.os
}

function gitOutput(arguments_) {
  return execFileSync('git', arguments_, { cwd: root, encoding: 'utf8' }).trim()
}

function verifySource(sourceSha) {
  if (!/^[0-9a-f]{40}$/u.test(sourceSha ?? '')) fail('source SHA must be 40 lowercase hexadecimal characters')
  if (process.env.GITHUB_SHA !== sourceSha) fail('GITHUB_SHA does not match dispatched source SHA')
  if (gitOutput(['rev-parse', 'HEAD']) !== sourceSha) fail('Checkout HEAD does not match dispatched source SHA')
  if (gitOutput(['status', '--porcelain', '--untracked-files=no']) !== '') {
    fail('Tracked checkout changed before qualification execution')
  }
  return sourceSha
}

function installedPackageIntegrity(freeze, environmentId) {
  const packageKey = environmentId === 'macos-node20'
    ? 'node_modules/@duckdb/node-bindings-darwin-arm64'
    : environmentId === 'windows-node20'
      ? 'node_modules/@duckdb/node-bindings-win32-x64'
      : 'node_modules/@duckdb/node-bindings-linux-x64'
  const esbuildKey = environmentId === 'macos-node20'
    ? 'node_modules/@esbuild/darwin-arm64'
    : environmentId === 'windows-node20'
      ? 'node_modules/@esbuild/win32-x64'
      : 'node_modules/@esbuild/linux-x64'
  const lock = loadJson('package-lock.json')
  const verified = {}
  for (const key of [packageKey, esbuildKey]) {
    const expected = freeze.nativePackages[key] ?? lock.packages?.[key]
    const actual = lock.packages?.[key]
    if (!expected || actual?.version !== expected.version || actual?.integrity !== expected.integrity) {
      fail(`Native package lock identity mismatch: ${key}`)
    }
    const packagePath = resolve(root, key, 'package.json')
    if (!existsSync(packagePath)) fail(`Installed native package missing: ${key}`)
    verified[key] = { version: actual.version, integrity: actual.integrity }
  }
  return verified
}

function verifyPlaywrightLicenses() {
  const manifest = loadJson('docs/qualification/playwright-license-manifest-v1.json')
  const verified = []
  for (const packageEvidence of manifest.packages) {
    const packageDirectory = resolve(root, 'tools/browser-qualification/node_modules', packageEvidence.name)
    if (!existsSync(packageDirectory)) {
      if (packageEvidence.optionalFor === process.platform) {
        fail(`Required qualification package missing: ${packageEvidence.name}`)
      }
      continue
    }
    for (const file of packageEvidence.licenseFiles) {
      const path = resolve(packageDirectory, file.path)
      const lexicalParent = `${packageDirectory}${sep}`
      if (!path.startsWith(lexicalParent)) fail(`License path escapes package: ${file.path}`)
      const actual = sha256File(path)
      if (actual.value !== file.sha256 || actual.byteLength !== file.byteLength) {
        fail(`Qualification license mismatch: ${packageEvidence.name}/${file.path}`)
      }
      verified.push({ package: packageEvidence.name, path: file.path, ...actual })
    }
  }
  return verified
}

function verifyTree(directory) {
  const entries = []
  function walk(current) {
    for (const name of readdirSync(current).sort()) {
      const path = join(current, name)
      const stat = lstatSync(path)
      if (stat.isDirectory()) walk(path)
      else if (stat.isFile()) {
        const relativePath = relative(directory, path).split(sep).join('/')
        entries.push(`F\0${relativePath}\0${sha256(readFileSync(path))}\n`)
      } else if (stat.isSymbolicLink()) {
        const target = readlinkSync(path)
        const absoluteTarget = resolve(dirname(path), target)
        if (absoluteTarget !== directory && !absoluteTarget.startsWith(`${directory}${sep}`)) {
          fail(`Browser tree symlink escapes revision: ${path}`)
        }
        const relativePath = relative(directory, path).split(sep).join('/')
        entries.push(`L\0${relativePath}\0${target}\n`)
      }
      else fail(`Browser tree contains non-file entry: ${path}`)
    }
  }
  walk(directory)
  const records = entries.sort().join('')
  const bytes = Buffer.from(records, 'utf8')
  return { value: sha256(bytes), byteLength: bytes.byteLength, entryCount: entries.length }
}

function browserIdentity(plan, environment) {
  if (!environment.browser) return null
  const freeze = loadJson(GITHUB_FREEZE_PATH).playwright
  const expectedRevision = freeze.executedTree[environment.browser.engine]
  const expected = freeze.browserTrees[expectedRevision]
  const provisionRoot = resolve(process.env.HOME ?? fail('HOME is required'), '.cache', 'ms-playwright')
  const trees = {}
  for (const revision of freeze.requiredTrees[environment.browser.engine]) {
    const revisionPath = resolve(provisionRoot, revision)
    if (!existsSync(revisionPath)) fail(`Frozen browser revision missing: ${revisionPath}`)
    const actual = verifyTree(revisionPath)
    const frozen = freeze.browserTrees[revision]
    if (actual.value !== frozen.treeSha256
        || actual.byteLength !== frozen.manifestByteLength
        || actual.entryCount !== frozen.entryCount) {
      fail(`Browser tree identity mismatch for ${revision}`)
    }
    trees[revision] = actual
  }
  const actual = trees[expectedRevision]
  if (actual.value !== environment.browser.sha256.value) fail(`Plan browser identity mismatch for ${expectedRevision}`)
  return { engine: environment.browser.engine, revision: expectedRevision, provisionRoot, ...actual, trees }
}

function verifyToolchain(environmentId, nodeArchivePath, npmArchivePath) {
  const plan = loadPlan()
  const freeze = loadJson(FREEZE_PATH)
  const environment = plan.environments.find(item => item.id === environmentId)
  if (!environment) fail(`Unknown environment ${environmentId}`)
  const frozenNode = freeze.nodeArtifacts[environmentId.startsWith('vite-')
    ? 'vite-host-node22'
    : environmentId]
  const nodeArchive = sha256File(resolve(nodeArchivePath))
  if (nodeArchive.value !== frozenNode.sha256
      || basename(nodeArchivePath) !== frozenNode.artifact
      || nodeArchive.byteLength !== environment.runtime.sha256.byteLength
      || process.version !== `v${frozenNode.nodeVersion}`) {
    fail(`Node identity mismatch for ${environmentId}`)
  }
  const npmArchive = readFileSync(resolve(npmArchivePath))
  const npmSha1 = createHash('sha1').update(npmArchive).digest('hex')
  const npmIntegrity = `sha512-${createHash('sha512').update(npmArchive).digest('base64')}`
  if (npmSha1 !== freeze.npm.shasum || npmIntegrity !== freeze.npm.integrity) {
    fail('npm archive identity mismatch')
  }
  if (normalizedArch(process.arch) !== environment.architecture
      || !environment.os.startsWith(normalizedPlatform(process.platform))) {
    fail(`Host identity mismatch: ${process.platform}/${process.arch} for ${environmentId}`)
  }
  const observedNpmVersion = execFileSync(
    process.platform === 'win32' ? 'npm.cmd' : 'npm',
    ['--version'],
    { cwd: root, encoding: 'utf8' },
  ).trim()
  if (observedNpmVersion !== NPM_VERSION) fail(`npm executable is ${observedNpmVersion}`)
  return {
    nodeArchive: {
      artifact: frozenNode.artifact,
      use: 'identity-evidence-only-not-executed',
      ...nodeArchive,
    },
    npmArchive: { npmSha1, npmIntegrity },
  }
}

function preflight(values) {
  const environmentId = values.get('--environment')
  const nodeArchivePath = values.get('--node-archive')
  const npmArchivePath = values.get('--npm-archive')
  const sourceSha = values.get('--source-sha')
  const output = values.get('--output')
  if (!environmentId || !nodeArchivePath || !npmArchivePath || !sourceSha || !output) {
    fail('preflight requires --environment, --node-archive, --npm-archive, --source-sha, and --output')
  }
  const plan = loadPlan()
  const environment = plan.environments.find(item => item.id === environmentId)
  if (!environment) fail(`Unknown environment ${environmentId}`)
  const npmVersion = process.env.npm_config_user_agent?.match(/^npm\/([^ ]+)/)?.[1]
  if (npmVersion && npmVersion !== NPM_VERSION) fail(`npm user agent is ${npmVersion}`)
  const inheritedForbidden = FORBIDDEN_ENV.filter(key => process.env[key] !== undefined)
  if (inheritedForbidden.length) fail(`Forbidden environment variables present: ${inheritedForbidden.join(', ')}`)
  const githubFreeze = loadJson(GITHUB_FREEZE_PATH)
  const expectedRunner = githubFreeze.runnerImages[githubRunnerKey(environment)]
  if (!expectedRunner
      || process.env.ImageOS !== expectedRunner.imageOS
      || process.env.ImageVersion !== expectedRunner.imageVersion) {
    fail(`Hosted runner image mismatch for ${environmentId}`)
  }
  const report = {
    schema: 'open-scad-viewer/g1-github-actions-preflight',
    schemaVersion: 1,
    planId: plan.planId,
    candidateRunId: CANDIDATE_ID,
    planSha256: sha256File(resolve(root, PLAN_PATH)).value,
    sourceSha: verifySource(sourceSha),
    environmentId,
    host: {
      platform: process.platform,
      architecture: process.arch,
      runnerImage: process.env.ImageOS ?? null,
      runnerImageVersion: process.env.ImageVersion ?? null,
    },
    runtime: {
      nodeVersion: process.version,
      npmVersion: NPM_VERSION,
      ...verifyToolchain(environmentId, nodeArchivePath, npmArchivePath),
    },
    bindings: verifyBindings(plan),
    nativePackages: installedPackageIntegrity(loadJson(FREEZE_PATH), environmentId),
    playwrightLicenseFiles: verifyPlaywrightLicenses(),
    browser: browserIdentity(plan, environment),
    forbiddenEnvironmentVariables: inheritedForbidden,
    verifiedAt: new Date().toISOString(),
  }
  mkdirSync(dirname(resolve(output)), { recursive: true })
  writeFileSync(resolve(output), `${JSON.stringify(report, null, 2)}\n`)
}

function expandCommand(template, runIndex, browser) {
  return template
    .replaceAll('<1|2|3>', String(runIndex))
    .replaceAll('<chromium|webkit>', browser)
    .replaceAll('<chromium|firefox|webkit>', browser)
}

function commandParts(command) {
  if (command.startsWith('npm test -- ')) {
    return {
      executable: process.platform === 'win32' ? 'npm.cmd' : 'npm',
      args: ['test', '--', ...command.slice('npm test -- '.length).split(/\s+/u).filter(Boolean)],
    }
  }
  if (command.startsWith('node ')) {
    const parts = command.split(/\s+/u).filter(Boolean)
    return { executable: process.execPath, args: parts.slice(1) }
  }
  fail(`Unsupported frozen command: ${command}`)
}

function cleanEnvironment(seed) {
  const allowed = [
    'HOME', 'USER', 'LOGNAME', 'TMPDIR', 'TMP', 'TEMP', 'LANG', 'LC_ALL',
    'PATH', 'PATHEXT', 'SYSTEMROOT', 'WINDIR', 'COMSPEC', 'NUMBER_OF_PROCESSORS',
  ]
  const environment = {}
  for (const key of allowed) if (process.env[key] !== undefined) environment[key] = process.env[key]
  return {
    ...environment,
    CI: '1',
    QUALIFICATION_SEED: seed,
    QUALIFICATION_PLAN_ID: PLAN_ID,
  }
}

async function digestFile(path) {
  const hash = createHash('sha256')
  let byteLength = 0
  for await (const chunk of createReadStream(path)) {
    hash.update(chunk)
    byteLength += chunk.byteLength
  }
  return { algorithm: 'sha256', value: hash.digest('hex'), byteLength }
}

async function runFragment(values) {
  const rowId = values.get('--row')
  const environmentId = values.get('--environment')
  const runIndex = Number(values.get('--run-index'))
  const output = resolve(values.get('--output') ?? '')
  const preflightPath = resolve(values.get('--preflight') ?? '')
  if (!rowId || !environmentId || !Number.isInteger(runIndex) || !values.get('--output')
      || !values.get('--preflight')) {
    fail('run requires --row, --environment, --run-index, --output, and --preflight')
  }
  const plan = loadPlan()
  const row = plan.matrix.find(item => item.id === rowId)
  const environment = plan.environments.find(item => item.id === environmentId)
  if (!row || !environment || !row.executionEnvironmentIds.includes(environmentId)
      || runIndex < 1 || runIndex > row.work.cleanRunsRequired) {
    fail(`Invalid fragment ${rowId}/${environmentId}/run-${runIndex}`)
  }
  const preflight = JSON.parse(readFileSync(preflightPath, 'utf8'))
  const currentPlanSha = sha256File(resolve(root, PLAN_PATH)).value
  if (preflight.planId !== PLAN_ID || preflight.candidateRunId !== CANDIDATE_ID
      || preflight.planSha256 !== currentPlanSha || preflight.environmentId !== environmentId
      || preflight.sourceSha !== process.env.GITHUB_SHA) {
    fail('Preflight identity does not match fragment')
  }
  const seed = row.work.seeds[runIndex - 1]
  const expanded = expandCommand(row.harness.command, runIndex, environment.browser?.engine ?? '')
  const invocation = commandParts(expanded)
  mkdirSync(output, { recursive: true })
  const stdoutPath = resolve(output, 'stdout.log')
  const stderrPath = resolve(output, 'stderr.log')
  const stdout = await import('node:fs').then(module => module.createWriteStream(stdoutPath, { flags: 'wx' }))
  const stderr = await import('node:fs').then(module => module.createWriteStream(stderrPath, { flags: 'wx' }))
  const startedAt = new Date().toISOString()
  const startedMs = Date.now()
  const timeoutMs = row.id === 'browser-memory-slope' ? 35 * 60 * 1000 : 20 * 60 * 1000
  const child = spawn(invocation.executable, invocation.args, {
    cwd: root,
    env: cleanEnvironment(seed),
    stdio: ['ignore', 'pipe', 'pipe'],
    detached: process.platform !== 'win32',
  })
  child.stdout.pipe(stdout)
  child.stderr.pipe(stderr)
  let timedOut = false
  const timer = setTimeout(() => {
    timedOut = true
    if (process.platform === 'win32') {
      spawn('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], { stdio: 'ignore' })
    } else {
      try {
        process.kill(-child.pid, 'SIGKILL')
      } catch {
        child.kill('SIGKILL')
      }
    }
  }, timeoutMs)
  const outcome = await new Promise((resolvePromise, reject) => {
    child.once('error', reject)
    child.once('close', (exitCode, signal) => resolvePromise({ exitCode, signal }))
  })
  clearTimeout(timer)
  await Promise.all([
    new Promise(resolvePromise => stdout.end(resolvePromise)),
    new Promise(resolvePromise => stderr.end(resolvePromise)),
  ])
  const passed = !timedOut && outcome.exitCode === 0
  const fragment = {
    schema: 'open-scad-viewer/g1-clean-run-fragment',
    schemaVersion: 1,
    fragmentId: `${row.id}/${environment.id}/run-${runIndex}`,
    matrixRowId: row.id,
    environmentId: environment.id,
    runIndex,
    seed,
    classification: 'clean-post-freeze',
    status: passed ? 'passed' : 'failed',
    unitsPlanned: row.work.unitsPerCleanRun,
    unitsCompleted: passed ? row.work.unitsPerCleanRun : 0,
    planId: plan.planId,
    candidateRunId: CANDIDATE_ID,
    planSha256: currentPlanSha,
    sourceSha: preflight.sourceSha,
    sourceBindings: preflight.bindings,
    toolchain: preflight.runtime,
    host: preflight.host,
    browser: preflight.browser,
    command: expanded,
    argv: [invocation.executable, ...invocation.args],
    startedAt,
    endedAt: new Date().toISOString(),
    durationMs: Date.now() - startedMs,
    exitCode: outcome.exitCode,
    signal: outcome.signal,
    timedOut,
    unitsRetried: 0,
    unitsSkipped: passed ? 0 : null,
    stdout: await digestFile(stdoutPath),
    stderr: await digestFile(stderrPath),
    isolation: {
      freshGithubHostedJob: true,
      sharedRealmAcrossCleanRuns: false,
      cancellation: process.platform === 'win32' ? 'taskkill-process-tree' : 'detached-process-group-sigkill',
      timeoutMs,
      memoryBudgetsEnforcedByFrozenHarness: row.surface === 'memory',
    },
    qualificationClaim: 'none',
  }
  writeFileSync(resolve(output, 'fragment.json'), `${JSON.stringify(fragment, null, 2)}\n`)
  const checksums = [
    ['fragment.json', sha256File(resolve(output, 'fragment.json')).value],
    ['preflight.json', sha256File(preflightPath).value],
    ['stdout.log', fragment.stdout.value],
    ['stderr.log', fragment.stderr.value],
  ]
  if (resolve(dirname(preflightPath)) !== output) {
    writeFileSync(resolve(output, 'preflight.json'), readFileSync(preflightPath))
    checksums[1][1] = sha256File(resolve(output, 'preflight.json')).value
  }
  writeFileSync(resolve(output, 'checksums.sha256'),
    `${checksums.map(([name, digest]) => `${digest}  ${name}`).join('\n')}\n`)
  if (!passed) process.exitCode = 1
}

function collectFragments(directory) {
  const paths = []
  function walk(current) {
    for (const name of readdirSync(current).sort()) {
      const path = join(current, name)
      const stat = lstatSync(path)
      if (stat.isDirectory()) walk(path)
      else if (name === 'fragment.json') paths.push(path)
    }
  }
  walk(directory)
  return paths
}

function aggregate(values) {
  const artifacts = resolve(values.get('--artifacts') ?? '')
  const output = resolve(values.get('--output') ?? '')
  const sourceSha = values.get('--source-sha')
  if (!values.get('--artifacts') || !values.get('--output') || !sourceSha) {
    fail('aggregate requires --artifacts, --output, and --source-sha')
  }
  if (!/^[0-9a-f]{40}$/u.test(sourceSha)) fail('source SHA must be 40 lowercase hexadecimal characters')
  if (process.env.GITHUB_SHA && process.env.GITHUB_SHA !== sourceSha) fail('Aggregate source SHA differs from GITHUB_SHA')
  if (gitOutput(['rev-parse', 'HEAD']) !== sourceSha) fail('Aggregate checkout HEAD differs from source SHA')
  const plan = loadPlan()
  const freeze = loadJson(FREEZE_PATH)
  const githubFreeze = loadJson(GITHUB_FREEZE_PATH)
  const aggregateRunner = githubFreeze.runnerImages['ubuntu-24.04']
  if (process.env.ImageOS && (
    process.env.ImageOS !== aggregateRunner.imageOS
    || process.env.ImageVersion !== aggregateRunner.imageVersion
  )) fail('Aggregate hosted runner image mismatch')
  const expectedPlanSha = sha256File(resolve(root, PLAN_PATH)).value
  const expected = new Map()
  for (const row of plan.matrix) {
    for (const environmentId of row.executionEnvironmentIds) {
      for (let runIndex = 1; runIndex <= row.work.cleanRunsRequired; runIndex += 1) {
        expected.set(`${row.id}/${environmentId}/run-${runIndex}`, {
          row, environmentId, runIndex, seed: row.work.seeds[runIndex - 1],
        })
      }
    }
  }
  const seen = new Map()
  const credited = new Map()
  const errors = []
  for (const path of collectFragments(artifacts)) {
    const errorsBefore = errors.length
    const fragment = JSON.parse(readFileSync(path, 'utf8'))
    const directory = dirname(path)
    const checksumPath = resolve(directory, 'checksums.sha256')
    if (!existsSync(checksumPath)) {
      errors.push(`missing checksums ${fragment.fragmentId ?? path}`)
    } else {
      const checksumLines = readFileSync(checksumPath, 'utf8').trim().split('\n')
      const checksums = new Map(checksumLines.map(line => {
        const match = /^([0-9a-f]{64})  (fragment\.json|preflight\.json|stdout\.log|stderr\.log)$/u.exec(line)
        if (!match) fail(`Malformed checksum line in ${checksumPath}`)
        return [match[2], match[1]]
      }))
      if (checksumLines.length !== 4 || checksums.size !== 4) {
        errors.push(`duplicate or missing checksum row ${fragment.fragmentId ?? path}`)
      }
      for (const name of ['fragment.json', 'preflight.json', 'stdout.log', 'stderr.log']) {
        const file = resolve(directory, name)
        if (!existsSync(file) || checksums.get(name) !== sha256File(file).value) {
          errors.push(`artifact checksum mismatch ${fragment.fragmentId ?? path}/${name}`)
        }
      }
    }
    if (seen.has(fragment.fragmentId)) {
      errors.push(`duplicate fragment ${fragment.fragmentId}`)
      continue
    }
    seen.set(fragment.fragmentId, fragment)
    const wanted = expected.get(fragment.fragmentId)
    if (!wanted) {
      errors.push(`unexpected fragment ${fragment.fragmentId}`)
      continue
    }
    if (fragment.matrixRowId !== wanted.row.id
        || fragment.environmentId !== wanted.environmentId
        || fragment.runIndex !== wanted.runIndex
        || fragment.seed !== wanted.seed
        || fragment.unitsPlanned !== wanted.row.work.unitsPerCleanRun
        || fragment.unitsCompleted !== wanted.row.work.unitsPerCleanRun
        || fragment.unitsRetried !== 0 || fragment.unitsSkipped !== 0
        || fragment.status !== 'passed'
        || fragment.classification !== 'clean-post-freeze'
        || fragment.planId !== PLAN_ID || fragment.candidateRunId !== CANDIDATE_ID
        || fragment.planSha256 !== expectedPlanSha
        || fragment.sourceSha !== sourceSha
        || fragment.qualificationClaim !== 'none') {
      errors.push(`invalid or failed fragment ${fragment.fragmentId}`)
    }
    for (const bundle of plan.bindings.bundles) {
      const actual = fragment.sourceBindings?.bundles?.[bundle.id]
      if (actual?.value !== bundle.sha256.value || actual?.byteLength !== bundle.sha256.byteLength) {
        errors.push(`source identity mismatch ${fragment.fragmentId}/${bundle.id}`)
      }
    }
    for (const artifact of plan.bindings.artifacts) {
      const actual = fragment.sourceBindings?.artifacts?.[artifact.id]
      if (actual?.value !== artifact.sha256.value || actual?.byteLength !== artifact.sha256.byteLength) {
        errors.push(`artifact identity mismatch ${fragment.fragmentId}/${artifact.id}`)
      }
    }
    const environment = plan.environments.find(item => item.id === fragment.environmentId)
    const frozenNode = freeze.nodeArtifacts[environment?.class === 'vite-browser'
      ? 'vite-host-node22'
      : fragment.environmentId]
    if (!environment || fragment.toolchain?.nodeVersion !== `v${environment.runtime.version}`
        || fragment.toolchain?.nodeArchive?.value !== frozenNode?.sha256
        || fragment.toolchain?.nodeArchive?.artifact !== frozenNode?.artifact
        || fragment.toolchain?.npmVersion !== NPM_VERSION
        || fragment.toolchain?.npmArchive?.npmIntegrity !== freeze.npm.integrity
        || fragment.toolchain?.npmArchive?.npmSha1 !== freeze.npm.shasum) {
      errors.push(`toolchain identity mismatch ${fragment.fragmentId}`)
    }
    const expectedRunner = environment && githubFreeze.runnerImages[githubRunnerKey(environment)]
    if (!expectedRunner
        || fragment.host?.runnerImage !== expectedRunner.imageOS
        || fragment.host?.runnerImageVersion !== expectedRunner.imageVersion) {
      errors.push(`hosted runner identity mismatch ${fragment.fragmentId}`)
    }
    if (environment?.browser) {
      const playwright = githubFreeze.playwright
      const revision = playwright.executedTree[environment.browser.engine]
      const frozenBrowser = playwright.browserTrees[revision]
      if (fragment.browser?.engine !== environment.browser.engine
          || fragment.browser?.revision !== revision
          || fragment.browser?.value !== frozenBrowser.treeSha256
          || fragment.browser?.byteLength !== frozenBrowser.manifestByteLength
          || fragment.browser?.entryCount !== frozenBrowser.entryCount) {
        errors.push(`browser identity mismatch ${fragment.fragmentId}`)
      }
      for (const tree of playwright.requiredTrees[environment.browser.engine]) {
        const expectedTree = playwright.browserTrees[tree]
        const actualTree = fragment.browser?.trees?.[tree]
        if (actualTree?.value !== expectedTree.treeSha256
            || actualTree?.byteLength !== expectedTree.manifestByteLength
            || actualTree?.entryCount !== expectedTree.entryCount) {
          errors.push(`browser support tree mismatch ${fragment.fragmentId}/${tree}`)
        }
      }
    } else if (fragment.browser !== null) {
      errors.push(`unexpected browser identity ${fragment.fragmentId}`)
    }
    const preflightPath = resolve(directory, 'preflight.json')
    if (existsSync(preflightPath)) {
      const preflight = JSON.parse(readFileSync(preflightPath, 'utf8'))
      if (preflight.planId !== PLAN_ID || preflight.candidateRunId !== CANDIDATE_ID
          || preflight.planSha256 !== expectedPlanSha || preflight.sourceSha !== sourceSha
          || preflight.environmentId !== fragment.environmentId
          || JSON.stringify(preflight.bindings) !== JSON.stringify(fragment.sourceBindings)
          || JSON.stringify(preflight.runtime) !== JSON.stringify(fragment.toolchain)
          || JSON.stringify(preflight.host) !== JSON.stringify(fragment.host)
          || JSON.stringify(preflight.browser) !== JSON.stringify(fragment.browser)) {
        errors.push(`preflight identity mismatch ${fragment.fragmentId}`)
      }
    }
    if (errors.length === errorsBefore) credited.set(fragment.fragmentId, fragment)
  }
  for (const id of expected.keys()) if (!seen.has(id)) errors.push(`missing fragment ${id}`)
  const completedWorkUnits = [...credited.values()]
    .reduce((total, fragment) => total + Number(fragment.unitsCompleted ?? 0), 0)
  const matrixPassed = errors.length === 0
    && completedWorkUnits === plan.executionProtocol.plannedWorkUnits
  const result = {
    schema: 'open-scad-viewer/g1-github-actions-aggregate',
    schemaVersion: 1,
    planId: PLAN_ID,
    candidateRunId: CANDIDATE_ID,
    planSha256: expectedPlanSha,
    sourceSha,
    expectedFragments: expected.size,
    receivedFragments: seen.size,
    creditedFragments: credited.size,
    plannedWorkUnits: plan.executionProtocol.plannedWorkUnits,
    completedWorkUnits,
    status: matrixPassed ? 'matrix-passed-awaiting-qualification-approval' : 'matrix-incomplete-or-failed',
    matrixPassed,
    qualificationClaim: 'none',
    qualificationApproval: plan.approvals.qualificationApproval,
    productionCutoverAuthorized: false,
    errors,
    generatedAt: new Date().toISOString(),
  }
  mkdirSync(dirname(output), { recursive: true })
  writeFileSync(output, `${JSON.stringify(result, null, 2)}\n`)
  writeFileSync(resolve(dirname(output), 'checksums.sha256'),
    `${sha256File(output).value}  ${basename(output)}\n`)
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`)
  if (!matrixPassed) process.exitCode = 1
}

async function main() {
  const [command, ...rest] = process.argv.slice(2)
  if (command === 'matrix') matrix()
  else if (command === 'preflight') preflight(args(rest))
  else if (command === 'run') await runFragment(args(rest))
  else if (command === 'aggregate') aggregate(args(rest))
  else fail('Usage: g1-github-actions.mjs <matrix|preflight|run|aggregate>')
}

main().catch(error => {
  console.error(error instanceof Error ? error.stack : error)
  process.exit(2)
})
