import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { z } from 'zod/v4'
import { expressionSchema, modelGraphSchema } from '../src/services/modelGraph'

const corpus = JSON.parse(readFileSync(new URL('../crates/modelgraph-runtime/tests/fixtures/schema-parity.json', import.meta.url), 'utf8')) as {
  cases: Array<{ name: string; input: unknown; expected?: unknown; error?: boolean }>
}

describe('MCP ModelGraph schema boundary', () => {
  it('preserves the original schema acceptance and defaults across the Rust parity corpus', () => {
    for (const fixture of corpus.cases) {
      const parsed = modelGraphSchema.safeParse(fixture.input)
      expect(parsed.success, fixture.name).toBe(!fixture.error)
      if (parsed.success) expect(parsed.data, fixture.name).toEqual(fixture.expected)
    }
  })

  it('exports recursive expression and complete document JSON schemas', () => {
    for (const schema of [expressionSchema, modelGraphSchema]) {
      const exported = z.toJSONSchema(schema)
      expect(JSON.stringify(exported)).toContain('"$ref"')
      expect(JSON.stringify(exported)).toContain('"checked"')
      expect(JSON.stringify(exported)).toContain('"flatmap"')
    }
    expect(z.toJSONSchema(modelGraphSchema)).toMatchObject({
      type: 'object',
      additionalProperties: false,
      properties: { language: { const: 'modelgraph/1' }, nodes: { type: 'array', maxItems: 128 } },
    })
  })
})
