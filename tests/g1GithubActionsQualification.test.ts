import { createHash } from 'node:crypto'
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { execFileSync, spawnSync } from 'node:child_process'
import { afterEach, describe, expect, it } from 'vitest'

const root = resolve(import.meta.dirname, '..')
const harness = resolve(root, 'scripts/g1-github-actions.mjs')
const planPath = resolve(root, 'docs/qualification/semantic-manifold-g1-plan-v34.json')
const plan = JSON.parse(readFileSync(planPath, 'utf8'))
const runtimeFreeze = JSON.parse(readFileSync(resolve(
  root, 'docs/qualification/environment-freeze/g1-runtime-browser-bindings-v1.json',
), 'utf8'))
const githubFreeze = JSON.parse(readFileSync(resolve(
  root, 'docs/qualification/environment-freeze/g1-github-actions-v34.json',
), 'utf8'))
const sourceSha = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim()
const temporaryRoots: string[] = []

const digest = (bytes: Buffer | string) => createHash('sha256').update(bytes).digest('hex')
const runnerKey = (environment: any) => environment.os === 'macos-15'
  ? 'macos-15-arm64'
  : environment.os
const evidenceProducers = [
  '.github/workflows/g1-qualification-clean.yml',
  'scripts/g1-github-actions.mjs',
]

function validateHarnessBinding(bundle: any): void {
  for (const path of evidenceProducers) {
    if (!bundle.paths.includes(path)) throw new Error(`unbound evidence producer: ${path}`)
  }
  const records = bundle.paths.map((path: string) => (
    `${path}\0${digest(readFileSync(resolve(root, path)))}\n`
  )).join('')
  if (digest(records) !== bundle.sha256.value || Buffer.byteLength(records) !== bundle.sha256.byteLength) {
    throw new Error('harness binding mismatch')
  }
}

function writeEvidence(directory: string, fragment: any, preflight: any): void {
  mkdirSync(directory, { recursive: true })
  writeFileSync(resolve(directory, 'fragment.json'), `${JSON.stringify(fragment, null, 2)}\n`)
  writeFileSync(resolve(directory, 'preflight.json'), `${JSON.stringify(preflight, null, 2)}\n`)
  writeFileSync(resolve(directory, 'stdout.log'), '')
  writeFileSync(resolve(directory, 'stderr.log'), '')
  const names = ['fragment.json', 'preflight.json', 'stdout.log', 'stderr.log']
  writeFileSync(resolve(directory, 'checksums.sha256'), `${names.map(name => (
    `${digest(readFileSync(resolve(directory, name)))}  ${name}`
  )).join('\n')}\n`)
}

function fixture(): { artifacts: string; first: string } {
  const directory = mkdtempSync(join(tmpdir(), 'g1-github-actions-'))
  temporaryRoots.push(directory)
  const artifacts = resolve(directory, 'artifacts')
  const bindings = {
    artifacts: Object.fromEntries(plan.bindings.artifacts.map((item: any) => [
      item.id, { value: item.sha256.value, byteLength: item.sha256.byteLength },
    ])),
    bundles: Object.fromEntries(plan.bindings.bundles.map((item: any) => [
      item.id, { value: item.sha256.value, byteLength: item.sha256.byteLength },
    ])),
  }
  let first = ''
  for (const row of plan.matrix) {
    for (const environmentId of row.executionEnvironmentIds) {
      const environment = plan.environments.find((item: any) => item.id === environmentId)
      const node = runtimeFreeze.nodeArtifacts[
        environment.class === 'vite-browser' ? 'vite-host-node22' : environmentId
      ]
      const runner = githubFreeze.runnerImages[runnerKey(environment)]
      const browserRevision = environment.browser
        ? githubFreeze.playwright.executedTree[environment.browser.engine]
        : null
      const browser = browserRevision ? {
        engine: environment.browser.engine,
        revision: browserRevision,
        provisionRoot: '/frozen/playwright',
        value: githubFreeze.playwright.browserTrees[browserRevision].treeSha256,
        byteLength: githubFreeze.playwright.browserTrees[browserRevision].manifestByteLength,
        entryCount: githubFreeze.playwright.browserTrees[browserRevision].entryCount,
        trees: Object.fromEntries(
          githubFreeze.playwright.requiredTrees[environment.browser.engine].map((revision: string) => {
            const tree = githubFreeze.playwright.browserTrees[revision]
            return [revision, {
              value: tree.treeSha256,
              byteLength: tree.manifestByteLength,
              entryCount: tree.entryCount,
            }]
          }),
        ),
      } : null
      const toolchain = {
        nodeVersion: `v${environment.runtime.version}`,
        npmVersion: '10.9.8',
        nodeArchive: {
          artifact: node.artifact,
          use: 'identity-evidence-only-not-executed',
          value: node.sha256,
          byteLength: environment.runtime.sha256.byteLength,
        },
        npmArchive: {
          npmSha1: runtimeFreeze.npm.shasum,
          npmIntegrity: runtimeFreeze.npm.integrity,
        },
      }
      const host = {
        platform: environment.os.startsWith('ubuntu') ? 'linux'
          : environment.os.startsWith('macos') ? 'darwin' : 'win32',
        architecture: environment.architecture === 'aarch64' ? 'arm64' : 'x64',
        runnerImage: runner.imageOS,
        runnerImageVersion: runner.imageVersion,
      }
      for (let runIndex = 1; runIndex <= row.work.cleanRunsRequired; runIndex += 1) {
        const fragmentId = `${row.id}/${environmentId}/run-${runIndex}`
        const target = resolve(artifacts, fragmentId.replaceAll('/', '--'))
        if (!first) first = target
        const preflight = {
          planId: plan.planId,
          candidateRunId: plan.executionProtocol.candidateRunId,
          planSha256: digest(readFileSync(planPath)),
          sourceSha,
          environmentId,
          bindings,
          runtime: toolchain,
          host,
          browser,
        }
        const fragment = {
          fragmentId,
          matrixRowId: row.id,
          environmentId,
          runIndex,
          seed: row.work.seeds[runIndex - 1],
          unitsPlanned: row.work.unitsPerCleanRun,
          unitsCompleted: row.work.unitsPerCleanRun,
          unitsRetried: 0,
          unitsSkipped: 0,
          status: 'passed',
          classification: 'clean-post-freeze',
          planId: plan.planId,
          candidateRunId: plan.executionProtocol.candidateRunId,
          planSha256: preflight.planSha256,
          sourceSha,
          qualificationClaim: 'none',
          sourceBindings: bindings,
          toolchain,
          host,
          browser,
        }
        writeEvidence(target, fragment, preflight)
      }
    }
  }
  return { artifacts, first }
}

function aggregate(artifacts: string, suffix: string, sha = sourceSha) {
  const output = resolve(dirname(artifacts), `result-${suffix}.json`)
  const child = spawnSync(process.execPath, [
    harness, 'aggregate', '--artifacts', artifacts, '--source-sha', sha, '--output', output,
  ], { cwd: root, encoding: 'utf8' })
  return { process: child, output }
}

afterEach(() => {
  for (const directory of temporaryRoots.splice(0)) rmSync(directory, { recursive: true, force: true })
})

describe('G1 V34 GitHub Actions evidence integrity', () => {
  it('binds the evidence producers and preserves the exact 4740-unit no-claim matrix', () => {
    expect(plan.executionProtocol).toMatchObject({
      candidateRunId: 'semantic-manifold-g1-candidate-run-v34',
      plannedWorkUnits: 4740,
      priorResultsMayBeImported: false,
    })
    expect(plan.lifecycle.qualificationClaim).toBe('none')
    const harnessBundle = plan.bindings.bundles.find((item: any) => (
      item.id === 'g1-qualification-harness-bundle'
    ))
    expect(harnessBundle.paths).toEqual(expect.arrayContaining([
      '.github/workflows/g1-qualification-clean.yml',
      'scripts/g1-github-actions.mjs',
      'scripts/run-g1-candidate-clean-fragment.mjs',
      'scripts/run-g1-ubuntu-docker-fragment.mjs',
    ]))
    expect(() => validateHarnessBinding(harnessBundle)).not.toThrow()
    expect(digest(readFileSync(resolve(root, 'docs/qualification/semantic-manifold-g1-plan-v33.json'))))
      .toBe('27b756158efdee25758bb012394fc056d1fb676e4f87af393ec2a4c4d784fbda')
  })

  it.each(evidenceProducers)('rejects an unbound %s producer', producer => {
    const harnessBundle = structuredClone(plan.bindings.bundles.find((item: any) => (
      item.id === 'g1-qualification-harness-bundle'
    )))
    harnessBundle.paths = harnessBundle.paths.filter((path: string) => path !== producer)
    expect(() => validateHarnessBinding(harnessBundle)).toThrow(/unbound evidence producer/u)
  })

  it('credits each complete fragment exactly once', () => {
    const { artifacts } = fixture()
    const result = aggregate(artifacts, 'complete')
    expect(result.process.status, result.process.stderr).toBe(0)
    expect(JSON.parse(readFileSync(result.output, 'utf8'))).toMatchObject({
      matrixPassed: true,
      completedWorkUnits: 4740,
      plannedWorkUnits: 4740,
      qualificationClaim: 'none',
    })
  })

  it.each(['missing', 'duplicate', 'environment mismatch'])('rejects %s artifacts', kind => {
    const { artifacts, first } = fixture()
    if (kind === 'missing') {
      rmSync(first, { recursive: true })
    } else if (kind === 'duplicate') {
      cpSync(first, resolve(artifacts, 'duplicate'), { recursive: true })
    } else {
      const fragment = JSON.parse(readFileSync(resolve(first, 'fragment.json'), 'utf8'))
      const preflight = JSON.parse(readFileSync(resolve(first, 'preflight.json'), 'utf8'))
      fragment.host.runnerImageVersion = 'wrong'
      preflight.host.runnerImageVersion = 'wrong'
      writeEvidence(first, fragment, preflight)
    }
    const result = aggregate(artifacts, kind.replace(' ', '-'))
    expect(result.process.status).not.toBe(0)
    expect(JSON.parse(readFileSync(result.output, 'utf8'))).toMatchObject({
      matrixPassed: false,
      qualificationClaim: 'none',
    })
  })

  it('rejects a source commit other than checkout HEAD', () => {
    const { artifacts } = fixture()
    const result = aggregate(artifacts, 'wrong-commit', '0'.repeat(40))
    expect(result.process.status).not.toBe(0)
    expect(result.process.stderr).toMatch(/checkout HEAD differs/u)
  })
})
