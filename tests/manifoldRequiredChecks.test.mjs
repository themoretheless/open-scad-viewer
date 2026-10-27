import assert from 'node:assert/strict'
import {readFileSync, readdirSync} from 'node:fs'
import path from 'node:path'
import {test} from 'node:test'
import {fileURLToPath} from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const read = relative => readFileSync(path.join(root, relative), 'utf8')
const requiredTypeScript = [
  'tests/meshTopology.test.ts',
  'tests/meshManifoldDefects.test.ts',
  'tests/manifoldMetricsReport.test.ts',
  'tests/mainSolidWorkerClient.test.ts',
  'tests/mainSolidWorkerRuntime.test.ts',
  'tests/mainSolidWorkerTransport.test.ts',
  'tests/mainSolidWorkerRealBoundary.test.ts',
]
const requiredCommands = ['core', 'abi', 'property', 'ts'].map(name => `test:manifold:${name}`)

function rustFiles(relative) {
  const absolute = path.join(root, relative)
  return readdirSync(absolute, {withFileTypes:true}).flatMap(entry => {
    const child = path.join(relative, entry.name)
    return entry.isDirectory() ? rustFiles(child) : entry.name.endsWith('.rs') ? [child] : []
  })
}

test('manifold required checks are wired locally and into CI', () => {
  const scripts = JSON.parse(read('package.json')).scripts
  const ci = read('.github/workflows/ci.yml')
  for (const command of requiredCommands) {
    assert.equal(typeof scripts[command], 'string', `Missing ${command}`)
    assert.match(ci, new RegExp(`npm run ${command}(?:\\s|$)`, 'u'), `${command} is not mandatory in CI`)
  }
  assert.match(scripts['test:manifold'], /test:manifold:core/u)
  assert.match(scripts['test:manifold'], /test:manifold:abi/u)
  assert.match(scripts['test:manifold'], /test:manifold:property/u)
  assert.match(scripts['test:manifold'], /test:manifold:ts/u)
  for (const file of requiredTypeScript) {
    assert.ok(scripts['test:manifold:ts'].includes(file), `${file} is omitted from the required TS suite`)
  }
})

test('manifold required tests have no pre-existing skip, focus or ignore exceptions', () => {
  for (const file of requiredTypeScript) {
    const source = read(file)
    assert.doesNotMatch(source, /\b(?:describe|it|test)\s*\.\s*(?:skip|todo|only)\b|\.\s*skipIf\s*\(/u,
      `${file} contains a skipped, focused or todo test`)
  }
  const rust = [
    ...rustFiles('crates/manifold-core'),
    ...rustFiles('crates/manifold-ops'),
    'crates/geometry-bridge/src/tests/abi.rs',
  ]
  for (const file of rust) {
    assert.doesNotMatch(read(file), /#\s*\[\s*ignore(?:\s*\([^\]]*\))?\s*\]/u,
      `${file} contains an ignored required test`)
  }
})
