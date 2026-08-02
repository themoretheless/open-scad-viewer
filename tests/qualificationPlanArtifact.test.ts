import { createHash } from 'node:crypto'
import { lstatSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

type Json = null | boolean | number | string | Json[] | { readonly [key: string]: Json }
type JsonObject = { readonly [key: string]: Json }

const repositoryRoot = resolve(import.meta.dirname, '..')
const schemaPath = resolve(repositoryRoot, 'docs/qualification/qualification-plan.schema.json')
const v1PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v1.json')
const v2PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v2.json')
const v3PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v3.json')
const planPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v4.json')
const schema = JSON.parse(readFileSync(schemaPath, 'utf8')) as JsonObject
const v1Plan = JSON.parse(readFileSync(v1PlanPath, 'utf8')) as JsonObject
const v2Plan = JSON.parse(readFileSync(v2PlanPath, 'utf8')) as JsonObject
const v3Plan = JSON.parse(readFileSync(v3PlanPath, 'utf8')) as JsonObject
const plan = JSON.parse(readFileSync(planPath, 'utf8')) as JsonObject
const FROZEN_V1_SHA256 = '07d07027f17815b3759e96914747b8703eb29b1fb1f6dbd88655a101f606668f'
const FROZEN_V2_SHA256 = '6c423be838e55c9a4b84cd8df5f850bd050d1ed4510dd138e30cc6379496f5d1'
const FROZEN_V3_SHA256 = '9af37517e2d0fa16c71e725398bbb23ab232162c4a6d272801c9490e4b63218a'
const FROZEN_V4_SHA256 = '86f9452cd11755e55ae0fcb47ca6ab4f2442c3812f524ad36d09e5e8d843e194'

function isObject(value: Json): value is JsonObject {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function sameJson(left: Json, right: Json): boolean {
  return JSON.stringify(left) === JSON.stringify(right)
}

function resolveRef(root: JsonObject, ref: string): JsonObject {
  if (!ref.startsWith('#/')) throw new Error(`Test validator supports local refs only: ${ref}`)
  let value: Json = root
  for (const encoded of ref.slice(2).split('/')) {
    const key = encoded.replaceAll('~1', '/').replaceAll('~0', '~')
    if (!isObject(value) || !(key in value)) throw new Error(`Unresolved schema ref: ${ref}`)
    value = value[key]
  }
  if (!isObject(value)) throw new Error(`Schema ref is not an object: ${ref}`)
  return value
}

/** Independent draft-2020-12 subset used by this schema; no production imports. */
function validateJsonSchema(
  candidateSchema: JsonObject,
  value: Json,
  path = '$',
  root = candidateSchema,
): string[] {
  const errors: string[] = []
  const ref = candidateSchema.$ref
  if (typeof ref === 'string') {
    errors.push(...validateJsonSchema(resolveRef(root, ref), value, path, root))
  }

  const allOf = candidateSchema.allOf
  if (Array.isArray(allOf)) {
    for (const branch of allOf) {
      if (isObject(branch)) errors.push(...validateJsonSchema(branch, value, path, root))
    }
  }

  const oneOf = candidateSchema.oneOf
  if (Array.isArray(oneOf)) {
    const matches = oneOf.filter(branch => (
      isObject(branch) && validateJsonSchema(branch, value, path, root).length === 0
    )).length
    if (matches !== 1) errors.push(`${path}: expected exactly one oneOf branch, got ${matches}`)
  }

  const conditional = candidateSchema.if
  if (isObject(conditional) && validateJsonSchema(conditional, value, path, root).length === 0) {
    const thenSchema = candidateSchema.then
    if (isObject(thenSchema)) errors.push(...validateJsonSchema(thenSchema, value, path, root))
  }

  if ('const' in candidateSchema && !sameJson(candidateSchema.const ?? null, value)) {
    errors.push(`${path}: const mismatch`)
  }
  const enumValues = candidateSchema.enum
  if (Array.isArray(enumValues) && !enumValues.some(item => sameJson(item, value))) {
    errors.push(`${path}: value is outside enum`)
  }

  const type = candidateSchema.type
  if (typeof type === 'string') {
    const typeMatches = type === 'null' ? value === null
      : type === 'array' ? Array.isArray(value)
        : type === 'object' ? isObject(value)
          : type === 'integer' ? Number.isInteger(value)
            : typeof value === type
    if (!typeMatches) return [...errors, `${path}: expected ${type}`]
  }

  if (typeof value === 'string') {
    if (typeof candidateSchema.minLength === 'number' && value.length < candidateSchema.minLength) {
      errors.push(`${path}: shorter than minLength`)
    }
    if (typeof candidateSchema.maxLength === 'number' && value.length > candidateSchema.maxLength) {
      errors.push(`${path}: longer than maxLength`)
    }
    if (typeof candidateSchema.pattern === 'string'
      && !(new RegExp(candidateSchema.pattern, 'u')).test(value)) {
      errors.push(`${path}: pattern mismatch`)
    }
  }

  if (typeof value === 'number') {
    if (typeof candidateSchema.minimum === 'number' && value < candidateSchema.minimum) {
      errors.push(`${path}: below minimum`)
    }
    if (typeof candidateSchema.maximum === 'number' && value > candidateSchema.maximum) {
      errors.push(`${path}: above maximum`)
    }
    if (typeof candidateSchema.exclusiveMinimum === 'number'
      && value <= candidateSchema.exclusiveMinimum) {
      errors.push(`${path}: not above exclusiveMinimum`)
    }
  }

  if (Array.isArray(value)) {
    if (typeof candidateSchema.minItems === 'number' && value.length < candidateSchema.minItems) {
      errors.push(`${path}: fewer than minItems`)
    }
    if (typeof candidateSchema.maxItems === 'number' && value.length > candidateSchema.maxItems) {
      errors.push(`${path}: more than maxItems`)
    }
    if (candidateSchema.uniqueItems === true) {
      const encoded = value.map(item => JSON.stringify(item))
      if (new Set(encoded).size !== encoded.length) errors.push(`${path}: duplicate array item`)
    }
    const items = candidateSchema.items
    if (isObject(items)) {
      value.forEach((item, index) => {
        errors.push(...validateJsonSchema(items, item, `${path}[${index}]`, root))
      })
    }
  }

  if (isObject(value)) {
    const keys = Object.keys(value)
    if (typeof candidateSchema.minProperties === 'number'
      && keys.length < candidateSchema.minProperties) {
      errors.push(`${path}: fewer than minProperties`)
    }
    const required = candidateSchema.required
    if (Array.isArray(required)) {
      for (const key of required) {
        if (typeof key === 'string' && !(key in value)) errors.push(`${path}.${key}: required`)
      }
    }
    const properties = isObject(candidateSchema.properties) ? candidateSchema.properties : {}
    for (const [key, propertySchema] of Object.entries(properties)) {
      if (key in value && isObject(propertySchema)) {
        errors.push(...validateJsonSchema(propertySchema, value[key], `${path}.${key}`, root))
      }
    }
    if (candidateSchema.additionalProperties === false) {
      for (const key of keys) {
        if (!(key in properties)) errors.push(`${path}.${key}: additional property`)
      }
    }
  }

  return errors
}

function objects(value: Json): JsonObject[] {
  if (Array.isArray(value)) return value.flatMap(objects)
  if (!isObject(value)) return []
  return [value, ...Object.values(value).flatMap(objects)]
}

function arrayProperty(value: JsonObject, key: string): JsonObject[] {
  const candidate = value[key]
  if (!Array.isArray(candidate) || !candidate.every(isObject)) {
    throw new Error(`Expected object array at $.${key}`)
  }
  return candidate
}

function objectProperty(value: JsonObject, key: string): JsonObject {
  const candidate = value[key]
  if (!isObject(candidate)) throw new Error(`Expected object at $.${key}`)
  return candidate
}

function assertUniqueIds(items: readonly JsonObject[], label: string): void {
  const ids = items.map(item => item.id)
  expect(ids, label).toEqual(expect.arrayContaining(ids))
  expect(new Set(ids).size, label).toBe(ids.length)
}

function frozenFileDigest(path: string): { value: string; byteLength: number } {
  const bytes = readFileSync(resolve(repositoryRoot, path))
  return {
    value: createHash('sha256').update(bytes).digest('hex'),
    byteLength: bytes.byteLength,
  }
}

function frozenBundleDigest(paths: readonly string[]): { value: string; byteLength: number } {
  const records = [...paths].sort().map(path => {
    const fileDigest = frozenFileDigest(path).value
    return Buffer.from(`${path}\0${fileDigest}\n`, 'utf8')
  })
  const stream = Buffer.concat(records)
  return {
    value: createHash('sha256').update(stream).digest('hex'),
    byteLength: stream.byteLength,
  }
}

function boundPaths(document: JsonObject): string[] {
  const bindings = objectProperty(document, 'bindings')
  return [
    ...arrayProperty(bindings, 'artifacts').map(item => String(item.path)),
    ...arrayProperty(bindings, 'bundles').flatMap(item => {
      if (!Array.isArray(item.paths)) throw new Error('Expected bundle path array')
      return item.paths.map(String)
    }),
  ]
}

function admissionBlockers(document: Json): string[] {
  return objects(document)
    .filter(item => item.state === 'unresolved-placeholder')
    .map(item => String(item.token))
}

describe('frozen-pending G1 QualificationPlan artifacts', () => {
  it('conforms to the machine-strict schema with closed object shapes', () => {
    expect(schema.$schema).toBe('https://json-schema.org/draft/2020-12/schema')
    expect(schema.additionalProperties).toBe(false)
    const strictObjectSchemas = objects(schema)
      .filter(item => item.type === 'object' && isObject(item.properties))
    expect(strictObjectSchemas.length).toBeGreaterThan(20)
    for (const objectSchema of strictObjectSchemas) {
      expect(objectSchema.additionalProperties).toBe(false)
    }
    expect(validateJsonSchema(schema, v1Plan)).toEqual([])
    expect(validateJsonSchema(schema, v2Plan)).toEqual([])
    expect(validateJsonSchema(schema, v3Plan)).toEqual([])
    expect(validateJsonSchema(schema, plan)).toEqual([])
  })

  it('keeps the v1 -> v2 -> v3 -> v4 amendment chain byte-immutable', () => {
    expect(createHash('sha256').update(readFileSync(v1PlanPath)).digest('hex'))
      .toBe(FROZEN_V1_SHA256)
    expect(createHash('sha256').update(readFileSync(v2PlanPath)).digest('hex'))
      .toBe(FROZEN_V2_SHA256)
    expect(createHash('sha256').update(readFileSync(v3PlanPath)).digest('hex'))
      .toBe(FROZEN_V3_SHA256)
    expect(createHash('sha256').update(readFileSync(planPath)).digest('hex'))
      .toBe(FROZEN_V4_SHA256)
    expect(v1Plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v1',
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v1',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v1/result.json',
      },
    })
    expect(v2Plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v2',
      processAmendment: {
        kind: 'post-freeze-harness-amendment',
        previousPlanId: 'semantic-manifold-g1-plan-v1',
        previousPlanSha256: FROZEN_V1_SHA256,
        priorEvidenceTreatment: 'discovery-only',
        qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v2',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v2/result.json',
      },
    })
    expect(v3Plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v3',
      processAmendment: {
        kind: 'post-freeze-harness-amendment',
        previousPlanId: 'semantic-manifold-g1-plan-v2',
        previousPlanSha256: FROZEN_V2_SHA256,
        priorEvidenceTreatment: 'discovery-only',
        qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v3',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v3/result.json',
      },
    })
    expect(plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v4',
      processAmendment: {
        kind: 'post-freeze-harness-amendment',
        previousPlanId: 'semantic-manifold-g1-plan-v3',
        previousPlanSha256: FROZEN_V3_SHA256,
        priorEvidenceTreatment: 'discovery-only',
        qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v4',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v4/result.json',
      },
    })
    expect(arrayProperty(v1Plan, 'unresolvedRows').map(item => item.id))
      .toContain('u06-browser-memory-probe')
    expect(arrayProperty(v2Plan, 'unresolvedRows').map(item => item.id))
      .not.toContain('u06-browser-memory-probe')
    expect(arrayProperty(v2Plan, 'unresolvedRows').map(item => item.id))
      .toEqual(expect.arrayContaining(['u04-actual-browser-runner', 'u05-all-id-isolation']))
    expect(arrayProperty(v3Plan, 'unresolvedRows').map(item => item.id))
      .not.toContain('u04-actual-browser-runner')
    expect(arrayProperty(v3Plan, 'unresolvedRows').map(item => item.id))
      .not.toContain('u05-all-id-isolation')
    const v1Harness = arrayProperty(objectProperty(v1Plan, 'bindings'), 'bundles')
      .find(item => item.id === 'g1-qualification-harness-bundle')
    const v2Harness = arrayProperty(objectProperty(v2Plan, 'bindings'), 'bundles')
      .find(item => item.id === 'g1-qualification-harness-bundle')
    const v3Harness = arrayProperty(objectProperty(v3Plan, 'bindings'), 'bundles')
      .find(item => item.id === 'g1-qualification-harness-bundle')
    const v4Harness = arrayProperty(objectProperty(plan, 'bindings'), 'bundles')
      .find(item => item.id === 'g1-qualification-harness-bundle')
    expect(v1Harness?.paths).toEqual([
      'tests/fixtures/browser-qualification.html',
      'tests/fixtures/browser-qualification.ts',
      'tests/fixtures/browser-qualification.worker.ts',
      'tests/fixtures/mcp-manifold-plan-qualification.worker.mjs',
      'tests/fixtures/mcp-manifold-plan-memory-probe.ts',
      'tests/fixtures/mcp-shadow-noncooperative.worker.mjs',
      'tests/fixtures/mcp-shadow-stdio-server.ts',
      'tests/geometryWorkerRealBoundary.test.ts',
      'tests/legacyDirectEvaluatorOracle.test.ts',
      'tests/manifoldPlanBackend.test.ts',
      'tests/manifoldPlanQualificationWorkerLane.test.ts',
      'tests/mcpManifoldPlanQualificationStdio.test.ts',
      'tests/mcpManifoldPlanQualificationSupervisor.test.ts',
      'tests/mcpManifoldPlanMemorySlope.test.ts',
    ])
    expect(v2Harness?.paths).toContain('scripts/run-manifold-g1-memory-qualification.mjs')
    expect(v2Harness?.paths).not.toContain('scripts/run-browser-qualification.mjs')
    expect(v3Harness?.paths).toEqual(expect.arrayContaining([
      'scripts/run-browser-qualification.mjs',
      'scripts/run-manifold-g1-memory-qualification.mjs',
      'tests/browserQualificationRunner.test.ts',
      'tests/browserMemoryQualificationRunner.test.ts',
      'tests/fixtures/browser-memory-qualification.html',
      'tests/fixtures/browser-memory-qualification.ts',
      'vite.qualification.config.ts',
    ]))
    expect(v4Harness?.paths).toEqual(expect.arrayContaining([
      'scripts/qualificationPlaywrightPackage.mjs',
      'scripts/run-browser-qualification-supervisor.mjs',
      'tests/browserQualificationSupervisor.test.ts',
      'tests/fixtures/browser-qualification-supervisor-child.mjs',
      'tests/fixtures/web-worker-node-harness.ts',
    ]))
    const historicalLockBinding = arrayProperty(objectProperty(v3Plan, 'bindings'), 'artifacts')
      .find(item => item.id === 'npm-lockfile')
    expect(historicalLockBinding).toMatchObject({
      purpose: expect.stringContaining('Playwright 1.62.1'),
      sha256: {
        state: 'unresolved-placeholder',
        token: '__ROOT_FREEZE_PACKAGE_LOCK_SHA256__',
      },
    })
    const packageManifest = JSON.parse(readFileSync(resolve(repositoryRoot, 'package.json'), 'utf8')) as {
      devDependencies: Record<string, string>
    }
    const packageLock = JSON.parse(readFileSync(resolve(repositoryRoot, 'package-lock.json'), 'utf8')) as {
      packages: Record<string, { devDependencies?: Record<string, string> }>
    }
    expect(packageManifest.devDependencies.playwright).toBeUndefined()
    expect(packageLock.packages[''].devDependencies?.playwright).toBeUndefined()
    expect(packageLock.packages['node_modules/playwright']).toBeUndefined()
    expect(packageLock.packages['node_modules/playwright-core']).toBeUndefined()
    const qualificationManifest = JSON.parse(readFileSync(
      resolve(repositoryRoot, 'tools/browser-qualification/package.json'),
      'utf8',
    )) as { private: boolean; devDependencies: Record<string, string> }
    const qualificationLock = JSON.parse(readFileSync(
      resolve(repositoryRoot, 'tools/browser-qualification/package-lock.json'),
      'utf8',
    )) as { packages: Record<string, { version?: string }> }
    expect(qualificationManifest).toMatchObject({
      private: true,
      devDependencies: { playwright: '1.62.1' },
    })
    expect(qualificationLock.packages['node_modules/playwright']?.version).toBe('1.62.1')
    expect(qualificationLock.packages['node_modules/playwright-core']?.version).toBe('1.62.1')
    for (const environment of arrayProperty(plan, 'environments')) {
      if (environment.browser === null) continue
      expect(objectProperty(objectProperty(environment, 'browser'), 'sha256').state)
        .toBe('unresolved-placeholder')
    }
  })

  it('recomputes every frozen artifact and disjoint bundle before rejecting unresolved execution', () => {
    for (const candidatePlan of [v1Plan, v2Plan, v3Plan, plan]) {
      const bindings = objectProperty(candidatePlan, 'bindings')
      const artifacts = arrayProperty(bindings, 'artifacts')
      for (const artifact of artifacts) {
        const hash = objectProperty(artifact, 'sha256')
        if (hash.state !== 'frozen') continue
        expect(frozenFileDigest(String(artifact.path))).toEqual({
          value: hash.value,
          byteLength: hash.byteLength,
        })
      }
    }
    const bindings = objectProperty(plan, 'bindings')
    const artifacts = arrayProperty(bindings, 'artifacts')
    const bundles = arrayProperty(bindings, 'bundles')
    expect(artifacts).toHaveLength(15)
    expect(bundles).toHaveLength(5)
    for (const bundle of bundles) {
      const hash = objectProperty(bundle, 'sha256')
      if (!Array.isArray(bundle.paths)) throw new Error('Expected bundle paths')
      const paths = bundle.paths.map(String)
      expect(paths).toEqual([...paths].sort())
      expect(hash.state).toBe('frozen')
      expect(frozenBundleDigest(paths)).toEqual({
        value: hash.value,
        byteLength: hash.byteLength,
      })
    }
    const allBoundPaths = boundPaths(plan)
    expect(new Set(allBoundPaths).size).toBe(allBoundPaths.length)
    expect(allBoundPaths).not.toContain('docs/qualification/semantic-manifold-g1-plan-v4.json')
    expect(allBoundPaths).not.toContain('tests/qualificationPlanArtifact.test.ts')
    for (const path of allBoundPaths) {
      expect(path).toMatch(/^[\x20-\x7e]+$/u)
      const stat = lstatSync(resolve(repositoryRoot, path))
      expect(stat.isFile(), path).toBe(true)
      expect(stat.isSymbolicLink(), path).toBe(false)
      expect(readFileSync(resolve(repositoryRoot, path)).includes(13), path).toBe(false)
    }
    expect(readFileSync(resolve(repositoryRoot, '.gitattributes'), 'utf8').split('\n'))
      .toEqual(expect.arrayContaining([
        '.gitattributes text eol=lf',
        '.npmrc text eol=lf',
        '*.html text eol=lf',
        '*.json text eol=lf',
        '*.md text eol=lf',
        '*.mjs text eol=lf',
        '*.ts text eol=lf',
        '*.vue text eol=lf',
      ]))
    const oracle = objectProperty(plan, 'oracle')
    const oracleManifest = JSON.parse(readFileSync(
      resolve(repositoryRoot, 'tests/fixtures/manifold-plan-oracle-v1.json'),
      'utf8',
    )) as { corpusSha256: string; cases: unknown[] }
    expect(oracle).toMatchObject({
      corpusSha256: oracleManifest.corpusSha256,
      caseCount: oracleManifest.cases.length,
      productionImportsAllowed: false,
    })

    const blockers = admissionBlockers(plan)
    expect(blockers).toHaveLength(10)
    expect(new Set(blockers).size).toBe(8)
    expect(admissionBlockers(v1Plan)).toHaveLength(14)
    expect(admissionBlockers(v2Plan)).toHaveLength(14)
    expect(admissionBlockers(v3Plan)).toHaveLength(14)
    expect(objectProperty(plan, 'lifecycle')).toMatchObject({
      status: 'frozen-pending-execution',
      qualificationClaim: 'none',
      executionAdmission: 'blocked-unresolved-bindings',
    })
    expect(blockers.every(token => token.startsWith('__ROOT_FREEZE_'))).toBe(true)
  })

  it('keeps references, work-unit arithmetic, clean-run seeds, and unresolved rows exact', () => {
    const environments = arrayProperty(plan, 'environments')
    const matrix = arrayProperty(plan, 'matrix')
    const scope = objectProperty(plan, 'scope')
    const bindings = objectProperty(plan, 'bindings')
    const comparator = objectProperty(plan, 'comparator')
    const unresolvedRows = arrayProperty(plan, 'unresolvedRows')
    for (const [label, items] of [
      ['environment IDs', environments],
      ['matrix IDs', matrix],
      ['included scope IDs', arrayProperty(scope, 'included')],
      ['excluded scope IDs', arrayProperty(scope, 'excluded')],
      ['artifact IDs', arrayProperty(bindings, 'artifacts')],
      ['bundle IDs', arrayProperty(bindings, 'bundles')],
      ['mutation IDs', arrayProperty(comparator, 'mutations')],
      ['unresolved IDs', unresolvedRows],
    ] as const) assertUniqueIds(items, label)

    const environmentIds = new Set(environments.map(item => item.id))
    const matrixIds = new Set(matrix.map(item => item.id))
    const boundPaths = new Set<Json>([
      ...arrayProperty(bindings, 'artifacts').map(item => item.path),
      ...arrayProperty(bindings, 'bundles').flatMap(item => (
        Array.isArray(item.paths) ? item.paths : []
      )),
    ])
    for (const item of arrayProperty(scope, 'included')) {
      const rowIds = item.evidenceRows
      expect(Array.isArray(rowIds)).toBe(true)
      for (const rowId of rowIds as Json[]) expect(matrixIds.has(rowId)).toBe(true)
    }

    let plannedUnits = 0
    for (const row of matrix) {
      const rowEnvironmentIds = row.executionEnvironmentIds
      expect(Array.isArray(rowEnvironmentIds)).toBe(true)
      for (const environmentId of rowEnvironmentIds as Json[]) {
        expect(environmentIds.has(environmentId)).toBe(true)
      }
      const work = objectProperty(row, 'work')
      const entrypoints = objectProperty(row, 'harness').entrypoints
      expect(Array.isArray(entrypoints)).toBe(true)
      for (const entrypoint of entrypoints as Json[]) expect(boundPaths.has(entrypoint)).toBe(true)
      const units = Number(work.unitsPerCleanRun)
        * Number(work.cleanRunsRequired)
        * (rowEnvironmentIds as Json[]).length
      expect(work.seeds).toHaveLength(Number(work.cleanRunsRequired))
      expect(work.plannedUnits).toBe(units)
      plannedUnits += units
    }
    expect(objectProperty(plan, 'executionProtocol')).toMatchObject({
      priorResultsMayBeImported: false,
      plannedWorkUnits: plannedUnits,
    })
    expect(unresolvedRows.map(item => item.id)).toEqual([
      'u02-runtime-binary-bindings',
      'u03-browser-binary-bindings',
      'u07-clean-post-freeze-evidence',
      'u08-organizational-review',
      'u09-release-decision',
    ])
  })

  it('requires actual Vite Chromium, Firefox, and WebKit and all six ID boundaries', () => {
    const environments = arrayProperty(plan, 'environments')
    const matrix = arrayProperty(plan, 'matrix')
    const browserRows = matrix.filter(row => row.surface === 'vite-browser')
    expect(browserRows.map(row => row.id)).toEqual([
      'browser-chromium-actual-vite',
      'browser-firefox-actual-vite',
      'browser-webkit-actual-vite',
    ])
    expect(browserRows.map(row => objectProperty(row, 'harness').command)).toEqual([
      'node scripts/run-browser-qualification-supervisor.mjs --mode actual --browser chromium --run-index <1|2|3>',
      'node scripts/run-browser-qualification-supervisor.mjs --mode actual --browser firefox --run-index <1|2|3>',
      'node scripts/run-browser-qualification-supervisor.mjs --mode actual --browser webkit --run-index <1|2|3>',
    ])
    for (const row of browserRows) {
      expect(objectProperty(row, 'work').cleanRunsRequired).toBe(3)
      expect(String(objectProperty(row, 'harness').isolation))
        .toContain('detached POSIX process group')
      expect(objectProperty(row, 'harness').entrypoints).toEqual(expect.arrayContaining([
        'scripts/run-browser-qualification-supervisor.mjs',
        'scripts/run-browser-qualification.mjs',
      ]))
    }
    expect(browserRows.map(row => {
      const environmentId = (row.executionEnvironmentIds as Json[])[0]
      const environment = environments.find(item => item.id === environmentId)
      return {
        kind: objectProperty(row, 'harness').kind,
        engine: objectProperty(environment!, 'browser').engine,
        evidenceState: row.evidenceState,
      }
    })).toEqual([
      { kind: 'vite-browser', engine: 'chromium', evidenceState: 'not-executed-clean-post-freeze' },
      { kind: 'vite-browser', engine: 'firefox', evidenceState: 'not-executed-clean-post-freeze' },
      { kind: 'vite-browser', engine: 'webkit', evidenceState: 'not-executed-clean-post-freeze' },
    ])
    const browserMemoryRow = matrix.find(row => row.id === 'browser-memory-slope')
    expect(browserMemoryRow).toBeDefined()
    expect(objectProperty(browserMemoryRow!, 'harness')).toMatchObject({
      command: 'node scripts/run-browser-qualification-supervisor.mjs --mode memory --browser <chromium|firefox|webkit> --run-index <1|2|3>',
      entrypoints: expect.arrayContaining([
        'scripts/run-browser-qualification-supervisor.mjs',
        'scripts/run-manifold-g1-memory-qualification.mjs',
      ]),
    })

    const boundaries = arrayProperty(plan, 'identifierBoundaries')
    expect(boundaries).toHaveLength(6)
    expect(new Set(boundaries.map(item => `${item.field}:${item.utf16CodeUnits}`))).toEqual(new Set([
      'entityId:256',
      'entityId:257',
      'instanceId:256',
      'instanceId:257',
      'operationId:256',
      'operationId:257',
    ]))
    for (const boundary of boundaries) {
      expect(boundary.surfaces).toEqual(['node-worker', 'vite-browser-worker', 'mcp-worker'])
      expect(boundary.expected).toBe(boundary.utf16CodeUnits === 256
        ? 'publish-success'
        : 'typed-bounded-refusal-before-success-publication')
    }
  })

  it('matches the existing MCP memory probe thresholds without treating its old run as evidence', () => {
    const budgets = objectProperty(plan, 'resourceBudgets').memoryExperiments
    if (!Array.isArray(budgets)) throw new Error('Expected memory experiment array')
    const mcp = budgets.find(item => isObject(item) && item.id === 'mcp-memory-job')
    if (!isObject(mcp)) throw new Error('Missing mcp-memory-job')
    expect(mcp).toMatchObject({
      warmupJobs: 4,
      measuredJobsPerEnvironment: 24,
      sampleEveryJobs: 1,
      gcPassesPerSample: 3,
      slopeBytesPerJobMax: {
        rss: 2 * 1024 * 1024,
        heapUsed: 128 * 1024,
        external: 128 * 1024,
        arrayBuffers: 64 * 1024,
      },
      endpointDriftBytesMax: {
        rss: 32 * 1024 * 1024,
        heapUsed: 4 * 1024 * 1024,
        external: 4 * 1024 * 1024,
        arrayBuffers: 2 * 1024 * 1024,
      },
    })
    const probeSource = readFileSync(
      resolve(repositoryRoot, 'tests/fixtures/mcp-manifold-plan-memory-probe.ts'),
      'utf8',
    )
    const thresholdSource = readFileSync(
      resolve(repositoryRoot, 'tests/mcpManifoldPlanMemorySlope.test.ts'),
      'utf8',
    )
    expect(probeSource).toContain('const WARMUP_JOBS = 4')
    expect(probeSource).toContain('const MEASURED_JOBS = 24')
    expect(probeSource).toContain('pass < 3')
    expect(thresholdSource).toContain("slope(record.samples, 'rss')).toBeLessThanOrEqual(2 * mebibyte)")
    expect(thresholdSource).toContain("slope(record.samples, 'heapUsed')).toBeLessThanOrEqual(128 * 1024)")
    expect(thresholdSource).toContain("slope(record.samples, 'external')).toBeLessThanOrEqual(128 * 1024)")
    expect(thresholdSource).toContain("slope(record.samples, 'arrayBuffers')).toBeLessThanOrEqual(64 * 1024)")
    expect(thresholdSource).toContain('last.rss - first.rss).toBeLessThanOrEqual(32 * mebibyte)')
    expect(thresholdSource).toContain('last.heapUsed - first.heapUsed).toBeLessThanOrEqual(4 * mebibyte)')
    expect(thresholdSource).toContain('last.external - first.external).toBeLessThanOrEqual(4 * mebibyte)')
    expect(thresholdSource).toContain('last.arrayBuffers - first.arrayBuffers).toBeLessThanOrEqual(2 * mebibyte)')
    expect(objectProperty(plan, 'processDeviation')).toMatchObject({
      priorEvidenceClassification: 'discovery-only',
      priorEvidenceEligible: false,
      requiredRemediation: 'clean-rerun-after-freeze',
    })
  })

  it('records role separation without approval, organizational independence, or cutover', () => {
    const approvals = objectProperty(plan, 'approvals')
    expect(approvals).toMatchObject({
      independenceClassification: 'role-separated-agents-not-organizationally-independent',
      qualificationApproval: 'not-approved',
    })
    for (const record of arrayProperty(approvals, 'records').filter(item => item.principalType === 'agent')) {
      expect(record).toMatchObject({
        organizationallyIndependent: false,
        status: 'recorded-nonapproval',
      })
    }
    expect(arrayProperty(approvals, 'records')).toContainEqual(expect.objectContaining({
      role: 'organizationally-independent-reviewer',
      principalType: 'unassigned',
      organizationallyIndependent: true,
      status: 'pending',
    }))
    expect(objectProperty(plan, 'cutoverPolicy')).toEqual({
      productionCutoverAuthorized: false,
      productionRegistryMutationAllowed: false,
      productBehaviorChangeAllowed: false,
      automaticFallbackChangeAllowed: false,
      qualificationMayActivateProvider: false,
      nextAuthorization: 'separate approved post-G1 plan',
    })
  })
})
