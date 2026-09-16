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
const v4PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v4.json')
const v5PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v5.json')
const v6PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v6.json')
const v7PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v7.json')
const v8PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v8.json')
const v9PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v9.json')
const v10PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v10.json')
const v11PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v11.json')
const v12PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v12.json')
const v13PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v13.json')
const v14PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v14.json')
const v15PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v15.json')
const v16PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v16.json')
const v17PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v17.json')
const v18PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v18.json')
const v19PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v19.json')
const v20PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v20.json')
const v21PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v21.json')
const v22PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v22.json')
const v23PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v23.json')
const v24PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v24.json')
const v25PlanPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v25.json')
const planPath = resolve(repositoryRoot, 'docs/qualification/semantic-manifold-g1-plan-v26.json')
const schema = JSON.parse(readFileSync(schemaPath, 'utf8')) as JsonObject
const v1Plan = JSON.parse(readFileSync(v1PlanPath, 'utf8')) as JsonObject
const v2Plan = JSON.parse(readFileSync(v2PlanPath, 'utf8')) as JsonObject
const v3Plan = JSON.parse(readFileSync(v3PlanPath, 'utf8')) as JsonObject
const v4Plan = JSON.parse(readFileSync(v4PlanPath, 'utf8')) as JsonObject
const v5Plan = JSON.parse(readFileSync(v5PlanPath, 'utf8')) as JsonObject
const v6Plan = JSON.parse(readFileSync(v6PlanPath, 'utf8')) as JsonObject
const v7Plan = JSON.parse(readFileSync(v7PlanPath, 'utf8')) as JsonObject
const v8Plan = JSON.parse(readFileSync(v8PlanPath, 'utf8')) as JsonObject
const v9Plan = JSON.parse(readFileSync(v9PlanPath, 'utf8')) as JsonObject
const v10Plan = JSON.parse(readFileSync(v10PlanPath, 'utf8')) as JsonObject
const v11Plan = JSON.parse(readFileSync(v11PlanPath, 'utf8')) as JsonObject
const v12Plan = JSON.parse(readFileSync(v12PlanPath, 'utf8')) as JsonObject
const v13Plan = JSON.parse(readFileSync(v13PlanPath, 'utf8')) as JsonObject
const v14Plan = JSON.parse(readFileSync(v14PlanPath, 'utf8')) as JsonObject
const v15Plan = JSON.parse(readFileSync(v15PlanPath, 'utf8')) as JsonObject
const v16Plan = JSON.parse(readFileSync(v16PlanPath, 'utf8')) as JsonObject
const v17Plan = JSON.parse(readFileSync(v17PlanPath, 'utf8')) as JsonObject
const v18Plan = JSON.parse(readFileSync(v18PlanPath, 'utf8')) as JsonObject
const v19Plan = JSON.parse(readFileSync(v19PlanPath, 'utf8')) as JsonObject
const v20Plan = JSON.parse(readFileSync(v20PlanPath, 'utf8')) as JsonObject
const v21Plan = JSON.parse(readFileSync(v21PlanPath, 'utf8')) as JsonObject
const v22Plan = JSON.parse(readFileSync(v22PlanPath, 'utf8')) as JsonObject
const v23Plan = JSON.parse(readFileSync(v23PlanPath, 'utf8')) as JsonObject
const v24Plan = JSON.parse(readFileSync(v24PlanPath, 'utf8')) as JsonObject
const v25Plan = JSON.parse(readFileSync(v25PlanPath, 'utf8')) as JsonObject
const plan = JSON.parse(readFileSync(planPath, 'utf8')) as JsonObject
const FROZEN_V1_SHA256 = '050a1dd7a30d19dd85a8ed16fd724f7579c2430f1d7bf03ed68009d46bd2cbfa'
const FROZEN_V2_SHA256 = '90452963dfdc47e492725c6bcd60a2dcdc7ffa748f848e95831c5d5750f5f0a2'
const FROZEN_V3_SHA256 = '40c7d885b34013270f45b4f2cd7ce29bb83293ee61bf8c60c11a6613bd3f282c'
const FROZEN_V4_SHA256 = '4d8b69fd2afeb6c828f0ab79b1ddaf6dac68b83a5b9b8bc0a3f59a5b63320ee1'
const FROZEN_V5_SHA256 = 'a5c4b365263d74b6d539d7f6539bcc4a571d055bd518b70ec1bb93dd6df2d9ee'
const FROZEN_V6_SHA256 = '5a0ef0359fd6c1f5ddad43715d621cd549da9a319dde9454e06dec9e67835566'
const FROZEN_V7_SHA256 = '6cf7e79bd0240b41ec7dbd7acedeb9a5818e72af13803e976ca19cf57d77d000'
const FROZEN_V8_SHA256 = 'ccede0404cc9655c9acac317dbf9b63bf1ee4aa3453c0cd3f337cf5b9d7b5b5f'
const FROZEN_V9_SHA256 = '0efdfe73a0b43f5190d2cc66a594dd1909399852d6a6a485fd41997fe563cf21'
const FROZEN_V10_SHA256 = '0ee191fbf83e80b95ec940e2f8286e4c5d80ce214941f1c4aba57405cb7f4ec9'
const FROZEN_V11_SHA256 = 'e758898a3539a91360ad4e52458dc4d0413e6c000f55043bdf0a216c496e3d0d'
const FROZEN_V12_SHA256 = '3534a6f16b6443cd6898863b86985feb4278aebcc6024faadc0423a0de51faea'
const FROZEN_V13_SHA256 = 'c736b508bade157c2a466c35963953e982ec4ef080219c7b0c271d4a1f0ada1b'
const FROZEN_V14_SHA256 = '15ff9a8eb6631ae9a425853b72df88384c9e218bdc46e83f1a3414961b90fb41'
const FROZEN_V15_SHA256 = 'fecc5f2f3b08b98549f9b34f3c80c659cfb698b889e46461c9726acfdd4a9a1a'
const FROZEN_V16_SHA256 = 'f421e75e4ef19ffc6cef0d36745428ea086e3797b911ed84dd00a5c2636b1622'
const FROZEN_V17_SHA256 = 'ce0f2121aa0014dcc9439e63654e71502fa33f13a07564a65a8e0ec22ff993e4'
const FROZEN_V18_SHA256 = '7e129bd193d33ee6c4205c26e6f9b5747e13d48cea06800946a1ef639f5f99ce'
const FROZEN_V19_SHA256 = '14fb0fdc79532dcd9a8bed3a93128f152fe3b68cc56529b6cc83561b5b899ed3'
const FROZEN_V20_SHA256 = 'c5d903ba5b7383ea79df20626de6f24c1cef9a564bb977adde5b5a362d75b9e6'
const FROZEN_V21_SHA256 = 'bfb889481c1a1b1f360380599858be9d117500e9f3ceb311cc8a642a7a10682b'
const FROZEN_V22_SHA256 = '63a55e60751a35af3c676de1a537477e845ea2439db7199cb4e14e89bdd56ea6'
const FROZEN_V23_SHA256 = 'a62609ebefe2f07994266b3bce8641c368de8280d7ff690d318f411c74810c0d'
const FROZEN_V24_SHA256 = '05f572b7afd1311f7056b2cb3102ef117d67a7d8cb48c5a81057685d0bc5b7c4'
const FROZEN_V25_SHA256 = '2361b59e14acadd72b94d348083390cd3ca74a906049aea582e37453297889c8'
const FROZEN_V19_STATUS_SHA256 = '1c22fe7e7a5451a067be0b6b99b62a4f2c3be2e8067518b10d06d04c51c68b25'
const FROZEN_G0_V3_SHA256 = 'c8c74e76f7e6ce9e85c8f1c4267e678ca8c243b16ec4e6dd050584b489c0c171'

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
    expect(validateJsonSchema(schema, v4Plan)).toEqual([])
    expect(validateJsonSchema(schema, v5Plan)).toEqual([])
    expect(validateJsonSchema(schema, v6Plan)).toEqual([])
    expect(validateJsonSchema(schema, v7Plan)).toEqual([])
    expect(validateJsonSchema(schema, v8Plan)).toEqual([])
    expect(validateJsonSchema(schema, v9Plan)).toEqual([])
    expect(validateJsonSchema(schema, v10Plan)).toEqual([])
    expect(validateJsonSchema(schema, v11Plan)).toEqual([])
    expect(validateJsonSchema(schema, v12Plan)).toEqual([])
    expect(validateJsonSchema(schema, v13Plan)).toEqual([])
    expect(validateJsonSchema(schema, v14Plan)).toEqual([])
    expect(validateJsonSchema(schema, v15Plan)).toEqual([])
    expect(validateJsonSchema(schema, v16Plan)).toEqual([])
    expect(validateJsonSchema(schema, v17Plan)).toEqual([])
    expect(validateJsonSchema(schema, v18Plan)).toEqual([])
    expect(validateJsonSchema(schema, v19Plan)).toEqual([])
    expect(validateJsonSchema(schema, v20Plan)).toEqual([])
    expect(validateJsonSchema(schema, v21Plan)).toEqual([])
    expect(validateJsonSchema(schema, v22Plan)).toEqual([])
    expect(validateJsonSchema(schema, plan)).toEqual([])
  })

  it('keeps the v1 -> … -> v22 amendment chain byte-immutable for on-disk plan files', () => {
    expect(createHash('sha256').update(readFileSync(v1PlanPath)).digest('hex'))
      .toBe(FROZEN_V1_SHA256)
    expect(createHash('sha256').update(readFileSync(v2PlanPath)).digest('hex'))
      .toBe(FROZEN_V2_SHA256)
    expect(createHash('sha256').update(readFileSync(v3PlanPath)).digest('hex'))
      .toBe(FROZEN_V3_SHA256)
    expect(createHash('sha256').update(readFileSync(v4PlanPath)).digest('hex'))
      .toBe(FROZEN_V4_SHA256)
    expect(createHash('sha256').update(readFileSync(v5PlanPath)).digest('hex'))
      .toBe(FROZEN_V5_SHA256)
    expect(createHash('sha256').update(readFileSync(v6PlanPath)).digest('hex'))
      .toBe(FROZEN_V6_SHA256)
    expect(createHash('sha256').update(readFileSync(v7PlanPath)).digest('hex'))
      .toBe(FROZEN_V7_SHA256)
    expect(createHash('sha256').update(readFileSync(v8PlanPath)).digest('hex'))
      .toBe(FROZEN_V8_SHA256)
    expect(createHash('sha256').update(readFileSync(v9PlanPath)).digest('hex'))
      .toBe(FROZEN_V9_SHA256)
    expect(createHash('sha256').update(readFileSync(v10PlanPath)).digest('hex'))
      .toBe(FROZEN_V10_SHA256)
    expect(createHash('sha256').update(readFileSync(v11PlanPath)).digest('hex'))
      .toBe(FROZEN_V11_SHA256)
    expect(createHash('sha256').update(readFileSync(v12PlanPath)).digest('hex'))
      .toBe(FROZEN_V12_SHA256)
    expect(createHash('sha256').update(readFileSync(v13PlanPath)).digest('hex'))
      .toBe(FROZEN_V13_SHA256)
    expect(createHash('sha256').update(readFileSync(v14PlanPath)).digest('hex'))
      .toBe(FROZEN_V14_SHA256)
    expect(createHash('sha256').update(readFileSync(v15PlanPath)).digest('hex'))
      .toBe(FROZEN_V15_SHA256)
    expect(createHash('sha256').update(readFileSync(v16PlanPath)).digest('hex'))
      .toBe(FROZEN_V16_SHA256)
    expect(createHash('sha256').update(readFileSync(v17PlanPath)).digest('hex'))
      .toBe(FROZEN_V17_SHA256)
    expect(createHash('sha256').update(readFileSync(v18PlanPath)).digest('hex'))
      .toBe(FROZEN_V18_SHA256)
    expect(createHash('sha256').update(readFileSync(v19PlanPath)).digest('hex'))
      .toBe(FROZEN_V19_SHA256)
    expect(createHash('sha256').update(readFileSync(v20PlanPath)).digest('hex'))
      .toBe(FROZEN_V20_SHA256)
    expect(createHash('sha256').update(readFileSync(v21PlanPath)).digest('hex'))
      .toBe(FROZEN_V21_SHA256)
    expect(createHash('sha256').update(readFileSync(v22PlanPath)).digest('hex'))
      .toBe(FROZEN_V22_SHA256)
    expect(frozenFileDigest('docs/qualification/g0-v3-g1-v19-refreeze-status-v1.json').value)
      .toBe(FROZEN_V19_STATUS_SHA256)
    expect(frozenFileDigest('docs/qualification/g0-toolchain-fingerprints-v3.json').value)
      .toBe(FROZEN_G0_V3_SHA256)
    expect(v19Plan).toMatchObject({processAmendment:{previousPlanId:'semantic-manifold-g1-plan-v18',previousPlanSha256:FROZEN_V18_SHA256,qualificationClaim:'none'}})
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
        priorEvidenceTreatment: 'discovery-only',
        qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v3',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v3/result.json',
      },
    })
    expect(v4Plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v4',
      processAmendment: {
        kind: 'post-freeze-harness-amendment',
        previousPlanId: 'semantic-manifold-g1-plan-v3',
        priorEvidenceTreatment: 'discovery-only',
        qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v4',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v4/result.json',
      },
    })
    expect(v5Plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v5',
      processAmendment: {
        kind: 'post-freeze-harness-amendment',
        previousPlanId: 'semantic-manifold-g1-plan-v4',
        priorEvidenceTreatment: 'discovery-only',
        qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v5',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v5/result.json',
      },
    })
    expect(v6Plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v6',
      processAmendment: {
        kind: 'post-freeze-harness-amendment',
        previousPlanId: 'semantic-manifold-g1-plan-v5',
        priorEvidenceTreatment: 'discovery-only',
        qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v6',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v6/result.json',
      },
    })
    expect(v7Plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v7',
      processAmendment: {
        kind: 'post-freeze-harness-amendment',
        previousPlanId: 'semantic-manifold-g1-plan-v6',
        priorEvidenceTreatment: 'discovery-only',
        qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v7',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v7/result.json',
      },
    })
    expect(v8Plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v8',
      processAmendment: {
        kind: 'post-freeze-harness-amendment',
        previousPlanId: 'semantic-manifold-g1-plan-v7',
        priorEvidenceTreatment: 'discovery-only',
        qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v8',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v8/result.json',
      },
    })
    expect(v9Plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v9',
      processAmendment: {
        kind: 'post-freeze-harness-amendment',
        previousPlanId: 'semantic-manifold-g1-plan-v8',
        priorEvidenceTreatment: 'discovery-only',
        qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v9',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v9/result.json',
      },
    })
    expect(v10Plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v10',
      processAmendment: {
        kind: 'post-freeze-harness-amendment',
        previousPlanId: 'semantic-manifold-g1-plan-v9',
        priorEvidenceTreatment: 'discovery-only',
        qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v10',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v10/result.json',
      },
    })
    expect(v11Plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v11',
      processAmendment: {
        kind: 'post-freeze-harness-amendment',
        previousPlanId: 'semantic-manifold-g1-plan-v10',
        priorEvidenceTreatment: 'discovery-only',
        qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v11',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v11/result.json',
      },
    })
    expect(v12Plan).toMatchObject({planId:'semantic-manifold-g1-plan-v12',processAmendment:{previousPlanId:'semantic-manifold-g1-plan-v11',priorEvidenceTreatment:'discovery-only',qualificationClaim:'none'},executionProtocol:{candidateRunId:'semantic-manifold-g1-candidate-run-v12',resultPath:'output/qualification/semantic-manifold-g1-candidate-run-v12/result.json'}})
    expect(v13Plan).toMatchObject({planId:'semantic-manifold-g1-plan-v13',processAmendment:{previousPlanId:'semantic-manifold-g1-plan-v12',priorEvidenceTreatment:'discovery-only',qualificationClaim:'none'},executionProtocol:{candidateRunId:'semantic-manifold-g1-candidate-run-v13',resultPath:'output/qualification/semantic-manifold-g1-candidate-run-v13/result.json'}})
    expect(v14Plan).toMatchObject({planId:'semantic-manifold-g1-plan-v14',processAmendment:{previousPlanId:'semantic-manifold-g1-plan-v13',priorEvidenceTreatment:'discovery-only',qualificationClaim:'none'},executionProtocol:{candidateRunId:'semantic-manifold-g1-candidate-run-v14',resultPath:'output/qualification/semantic-manifold-g1-candidate-run-v14/result.json'}})
    expect(v15Plan).toMatchObject({planId:'semantic-manifold-g1-plan-v15',processAmendment:{previousPlanId:'semantic-manifold-g1-plan-v14',priorEvidenceTreatment:'discovery-only',qualificationClaim:'none'},executionProtocol:{candidateRunId:'semantic-manifold-g1-candidate-run-v15',resultPath:'output/qualification/semantic-manifold-g1-candidate-run-v15/result.json'}})
    expect(v16Plan).toMatchObject({planId:'semantic-manifold-g1-plan-v16',processAmendment:{previousPlanId:'semantic-manifold-g1-plan-v15',priorEvidenceTreatment:'discovery-only',qualificationClaim:'none'},executionProtocol:{candidateRunId:'semantic-manifold-g1-candidate-run-v16',resultPath:'output/qualification/semantic-manifold-g1-candidate-run-v16/result.json'}})
    expect(v17Plan).toMatchObject({planId:'semantic-manifold-g1-plan-v17',processAmendment:{previousPlanId:'semantic-manifold-g1-plan-v16',previousPlanSha256:FROZEN_V16_SHA256,priorEvidenceTreatment:'discovery-only',qualificationClaim:'none'},executionProtocol:{candidateRunId:'semantic-manifold-g1-candidate-run-v17',resultPath:'output/qualification/semantic-manifold-g1-candidate-run-v17/result.json'}})
    expect(v18Plan).toMatchObject({planId:'semantic-manifold-g1-plan-v18',processAmendment:{previousPlanId:'semantic-manifold-g1-plan-v17',previousPlanSha256:FROZEN_V17_SHA256,priorEvidenceTreatment:'discovery-only',qualificationClaim:'none'},executionProtocol:{candidateRunId:'semantic-manifold-g1-candidate-run-v18',resultPath:'output/qualification/semantic-manifold-g1-candidate-run-v18/result.json'}})
    // v18 admission amendment may retarget environments/matrix/scope/budgets and
    // approvals while preserving claim-boundary, oracle, comparator, and cutover.
    for (const key of Object.keys(v17Plan)) {
      if ([
        'planId',
        'processAmendment',
        'bindings',
        'executionProtocol',
        'environments',
        'matrix',
        'unresolvedRows',
        'lifecycle',
        'approvals',
        'scope',
        'resourceBudgets',
      ].includes(key)) continue
      expect(v18Plan[key], `v18 preserves ${key}`).toEqual(v17Plan[key])
    }
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
    const v4Harness = arrayProperty(objectProperty(v4Plan, 'bindings'), 'bundles')
      .find(item => item.id === 'g1-qualification-harness-bundle')
    const v5Harness = arrayProperty(objectProperty(v5Plan, 'bindings'), 'bundles')
      .find(item => item.id === 'g1-qualification-harness-bundle')
    const v4StaticAudit = arrayProperty(objectProperty(v4Plan, 'bindings'), 'bundles')
      .find(item => item.id === 'g1-static-dependency-audit-bundle')
    const v5StaticAudit = arrayProperty(objectProperty(v5Plan, 'bindings'), 'bundles')
      .find(item => item.id === 'g1-static-dependency-audit-bundle')
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
    expect(v5Harness?.paths).toEqual(v4Harness?.paths)
    if (!Array.isArray(v4StaticAudit?.paths)) throw new Error('Expected v4 static-audit paths')
    expect(v4StaticAudit?.paths).not.toContain('src/core/openScad2021Contract.ts')
    expect(v5StaticAudit?.paths).toEqual([
      ...v4StaticAudit.paths.map(String),
      'src/core/openScad2021Contract.ts',
    ].sort())
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
        .toBe('frozen')
    }
  })

  it('v25 preserves the complete finite v24 contract and starts a separate candidate with zero clean work', () => {
    expect(v25Plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v25',
      processAmendment: {
        previousPlanId: 'semantic-manifold-g1-plan-v24',
        previousPlanSha256: FROZEN_V24_SHA256,
        priorEvidenceTreatment: 'discovery-only', qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v25',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v25/result.json',
        priorResultsMayBeImported: false, plannedWorkUnits: 4740,
      },
    })
    expect(Object.keys(v25Plan)).toEqual(Object.keys(v24Plan))
    for (const key of Object.keys(v24Plan)) {
      if (['planId', 'processAmendment', 'bindings', 'executionProtocol'].includes(key)) continue
      expect(v25Plan[key], `v25 preserves ${key}`).toEqual(v24Plan[key])
    }
    const oldProtocol = objectProperty(v24Plan, 'executionProtocol')
    expect(objectProperty(v25Plan, 'executionProtocol')).toEqual({
      ...oldProtocol,
      candidateRunId: 'semantic-manifold-g1-candidate-run-v25',
      resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v25/result.json',
      cleanRunDefinition: (oldProtocol.cleanRunDefinition as string[]).map((line, index) => (
        index === 0 ? line.replace('exact v24 frozen artifact', 'exact v25 frozen artifact') : line
      )),
    })
    const oldBindings = objectProperty(v24Plan, 'bindings')
    const newBindings = objectProperty(v25Plan, 'bindings')
    expect(Object.keys(newBindings)).toEqual(Object.keys(oldBindings))
    expect(newBindings.placeholderPolicy).toEqual(oldBindings.placeholderPolicy)
    for (const kind of ['artifacts', 'bundles']) {
      const oldRows = arrayProperty(oldBindings, kind)
      const newRows = arrayProperty(newBindings, kind)
      expect(newRows).toHaveLength(oldRows.length)
      newRows.forEach((row, index) => {
        const hash = objectProperty(row, 'sha256')
        const oldHash = objectProperty(oldRows[index], 'sha256')
        const expected = kind === 'bundles' && row.id === 'g1-candidate-bundle'
          ? {
              ...oldRows[index],
              paths: [...(oldRows[index].paths as string[]), 'src/core/ownRustCadEvidence.ts'].sort(),
              sha256: hash,
            }
          : { ...oldRows[index], sha256: hash }
        expect(row).toEqual(expected)
        expect(hash).toEqual({ ...oldHash, value: hash.value, byteLength: hash.byteLength })
      })
    }
    for (const id of ['frozen-oracle-manifest', 'frozen-reference-oracle',
      'frozen-reference-direct-evaluator', 'qualification-plan-schema', 'g1-runtime-browser-bindings']) {
      expect(arrayProperty(newBindings, 'artifacts').find(row => row.id === id))
        .toEqual(arrayProperty(oldBindings, 'artifacts').find(row => row.id === id))
    }
    const status = JSON.parse(readFileSync(resolve(repositoryRoot,
      'docs/qualification/g0-v8-g1-v25-refreeze-status-v1.json'), 'utf8')) as JsonObject
    expect(status).toMatchObject({
      statusId: 'g0-v8-g1-v25-refreeze-status-v1', qualificationClaim: 'none',
      qualificationApproval: 'not-approved', g0Closed: false,
      candidateRunId: 'semantic-manifold-g1-candidate-run-v25', priorResultsMayBeImported: false,
      priorEvidenceTreatment: 'discovery-only', completedWorkUnits: 0,
      completedCleanRuns: 0, plannedWorkUnits: 4740,
    })
    expect(arrayProperty(status, 'archives').map(row => row.path).sort()).toEqual([
      'docs/qualification/g0-toolchain-fingerprints-v1.json',
      'docs/qualification/g0-toolchain-fingerprints-v2.json',
      'docs/qualification/g0-toolchain-fingerprints-v3.json',
      'docs/qualification/g0-toolchain-fingerprints-v4.json',
      'docs/qualification/g0-toolchain-fingerprints-v5.json',
      'docs/qualification/g0-toolchain-fingerprints-v6.json',
      'docs/qualification/g0-toolchain-fingerprints-v7.json',
      'docs/qualification/g0-v3-g1-v19-refreeze-status-v1.json',
      'docs/qualification/g0-v4-g1-v21-refreeze-status-v1.json',
      'docs/qualification/g0-v5-g1-v22-refreeze-status-v1.json',
      'docs/qualification/g0-v6-g1-v23-refreeze-review.md',
      'docs/qualification/g0-v6-g1-v23-refreeze-status-v1.json',
      'docs/qualification/g0-v7-g1-v24-refreeze-review.md',
      'docs/qualification/g0-v7-g1-v24-refreeze-status-v1.json',
      'docs/qualification/g1-v20-refreeze-status-v1.json',
      ...Array.from({ length: 24 }, (_, index) => `docs/qualification/semantic-manifold-g1-plan-v${index + 1}.json`),
    ].sort())
    for (const archive of arrayProperty(status, 'archives')) {
      expect(frozenFileDigest(String(archive.path)).value).toBe(archive.sha256)
    }
    expect(arrayProperty(status, 'artifacts').map(row => row.path)).toEqual([
      'docs/qualification/g0-toolchain-fingerprints-v8.json',
      'docs/qualification/semantic-manifold-g1-plan-v25.json',
    ])
    for (const artifact of arrayProperty(status, 'artifacts')) {
      expect(frozenFileDigest(String(artifact.path)))
        .toEqual({ value: artifact.sha256, byteLength: artifact.byteLength })
    }
    const inputSnapshot = objectProperty(status, 'inputSnapshot')
    const inputs = arrayProperty(inputSnapshot, 'files')
    for (const input of inputs) {
      expect(input).toMatchObject({
        path: expect.any(String),
        sha256: expect.stringMatching(/^[0-9a-f]{64}$/u),
        byteLength: expect.any(Number),
      })
    }
    expect(inputSnapshot).toMatchObject({
      sha256: expect.stringMatching(/^[0-9a-f]{64}$/u),
      byteLength: expect.any(Number),
    })
    expect(arrayProperty(status, 'pendingRows')).toEqual(arrayProperty(v25Plan, 'matrix').map(row => {
      const work = objectProperty(row, 'work')
      return { id: row.id, environmentIds: row.executionEnvironmentIds,
        cleanRunsRequiredPerEnvironment: work.cleanRunsRequired,
        completedCleanRuns: 0, completedWorkUnits: 0, plannedWorkUnits: work.plannedUnits }
    }))
  })

  it('v26 preserves v25, rebinds current bytes, and keeps qualification blocked on u07', () => {
    expect(plan).toMatchObject({
      planId: 'semantic-manifold-g1-plan-v26',
      processAmendment: {
        previousPlanId: 'semantic-manifold-g1-plan-v25',
        previousPlanSha256: FROZEN_V25_SHA256,
        priorEvidenceTreatment: 'discovery-only', qualificationClaim: 'none',
      },
      executionProtocol: {
        candidateRunId: 'semantic-manifold-g1-candidate-run-v26',
        resultPath: 'output/qualification/semantic-manifold-g1-candidate-run-v26/result.json',
        priorResultsMayBeImported: false, plannedWorkUnits: 4740,
      },
    })
    expect(Object.keys(plan)).toEqual(Object.keys(v25Plan))
    for (const key of Object.keys(v25Plan)) {
      if (['planId', 'processAmendment', 'bindings', 'executionProtocol'].includes(key)) continue
      expect(plan[key], `v26 preserves ${key}`).toEqual(v25Plan[key])
    }
    const bindings = objectProperty(plan, 'bindings')
    const artifacts = arrayProperty(bindings, 'artifacts')
    const changed: string[] = []
    for (const artifact of artifacts) {
      const hash = objectProperty(artifact, 'sha256')
      if (hash.state !== 'frozen') continue
      const actual = frozenFileDigest(String(artifact.path))
      if (actual.value !== hash.value || actual.byteLength !== hash.byteLength) changed.push(String(artifact.path))
    }
    const bundles = arrayProperty(bindings, 'bundles')
    expect(artifacts).toHaveLength(16)
    expect(bundles).toHaveLength(5)
    for (const bundle of bundles) {
      const hash = objectProperty(bundle, 'sha256')
      if (!Array.isArray(bundle.paths)) throw new Error('Expected bundle paths')
      const paths = bundle.paths.map(String)
      expect(paths).toEqual([...paths].sort())
      expect(hash.state).toBe('frozen')
      const actual = frozenBundleDigest(paths)
      if (actual.value !== hash.value || actual.byteLength !== hash.byteLength) changed.push(String(bundle.id))
    }
    expect(changed, 'v26 frozen bindings must match current repository bytes').toEqual([])
    const allBoundPaths = boundPaths(plan)
    expect(new Set(allBoundPaths).size).toBe(allBoundPaths.length)
    expect(allBoundPaths).not.toContain('docs/qualification/semantic-manifold-g1-plan-v5.json')
    expect(allBoundPaths).not.toContain('docs/qualification/semantic-manifold-g1-plan-v6.json')
    expect(allBoundPaths).not.toContain('docs/qualification/semantic-manifold-g1-plan-v7.json')
    expect(allBoundPaths).not.toContain('docs/qualification/semantic-manifold-g1-plan-v8.json')
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
    expect(blockers).toEqual([])
    expect(admissionBlockers(v1Plan)).toHaveLength(14)
    expect(admissionBlockers(v2Plan)).toHaveLength(14)
    expect(admissionBlockers(v3Plan)).toHaveLength(14)
    expect(objectProperty(plan, 'lifecycle')).toMatchObject({
      status: 'frozen-pending-execution',
      qualificationClaim: 'none',
      executionAdmission: 'ready-clean-rerun',
    })
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
      'u07-clean-post-freeze-evidence',
    ])
  })

  it('requires actual Vite Chromium and WebKit and all six ID boundaries', () => {
    const environments = arrayProperty(plan, 'environments')
    const matrix = arrayProperty(plan, 'matrix')
    const browserRows = matrix.filter(row => row.surface === 'vite-browser')
    expect(browserRows.map(row => row.id)).toEqual([
      'browser-chromium-actual-vite',
      'browser-webkit-actual-vite',
    ])
    expect(browserRows.map(row => objectProperty(row, 'harness').command)).toEqual([
      'node scripts/run-browser-qualification-supervisor.mjs --mode actual --browser chromium --run-index <1|2|3>',
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
      { kind: 'vite-browser', engine: 'webkit', evidenceState: 'not-executed-clean-post-freeze' },
    ])
    const browserMemoryRow = matrix.find(row => row.id === 'browser-memory-slope')
    expect(browserMemoryRow).toBeDefined()
    expect(objectProperty(browserMemoryRow!, 'harness')).toMatchObject({
      command: 'node scripts/run-browser-qualification-supervisor.mjs --mode memory --browser <chromium|webkit> --run-index <1|2|3>',
      entrypoints: expect.arrayContaining([
        'scripts/run-browser-qualification-supervisor.mjs',
        'scripts/run-manifold-g1-memory-qualification.mjs',
      ]),
    })
    expect(browserMemoryRow!.executionEnvironmentIds).toEqual(['vite-chromium', 'vite-webkit'])
    expect(environments.map(item => item.id)).not.toContain('vite-firefox')

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
      principal: 'waived-solo-dual-role',
      principalType: 'human',
      organizationallyIndependent: false,
      status: 'recorded-nonapproval',
    }))
    expect(arrayProperty(approvals, 'records')).toContainEqual(expect.objectContaining({
      role: 'release-authority',
      principal: 'repository-owner',
      status: 'approved',
    }))
    expect(arrayProperty(approvals, 'records')).toContainEqual(expect.objectContaining({
      role: 'qualification-executor',
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
