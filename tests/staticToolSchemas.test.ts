import { InMemoryTransport, McpServer, type JSONRPCMessage, type StandardSchemaWithJSON } from '@modelcontextprotocol/server'
import { describe, expect, it, vi } from 'vitest'
import { z } from 'zod/v4'
import { createStaticToolRegistration } from '../src/mcp/staticToolSchemas'
import { modelGraphSchema } from '../src/services/modelGraph'

type Response = {id: number; result?: Record<string, unknown>; error?: unknown}

function checkLocalReferences(root: Record<string, unknown>) {
  let references = 0
  const visit = (value: unknown) => {
    if (!value || typeof value !== 'object') return
    if ('$ref' in value) {
      const ref = String(value.$ref)
      expect(ref.startsWith('#/')).toBe(true)
      let target: unknown = root
      for (const segment of ref.slice(2).split('/')) {
        const key = segment.replace(/~1/g, '/').replace(/~0/g, '~')
        target = (target as Record<string, unknown> | undefined)?.[key]
      }
      expect(target, ref).toBeDefined()
      references++
    }
    for (const child of Object.values(value)) visit(child)
  }
  visit(root)
  expect(references).toBeGreaterThan(0)
}

describe('static MCP tool schemas', () => {
  it('reuses recursive schema conversion across listings and HTTP-style registrations without weakening validation', async () => {
    const registration = createStaticToolRegistration()
    const conversions = vi.fn()
    const handler = vi.fn()
    const document = {
      language: 'modelgraph/1', units: 'mm', parameters: [], root: 'shape',
      nodes: [{id: 'shape', op: 'sphere', radius: {
        op: 'match', input: {op: 'list', items: [2]}, arms: [
          {pattern: {kind: 'list', prefix: [{kind: 'type', name: 'int', pattern: {kind: 'bind', name: 'n'}}], suffix: []}, body: {local: 'n'}},
          {pattern: {kind: 'wildcard'}, body: 1},
        ],
      }}],
    }
    for (let instance = 0; instance < 2; instance++) {
      // Each stateless request reconstructs the Zod wrapper, as our HTTP server does.
      const original = z.object({document: modelGraphSchema}).strict()
      const standard = original['~standard']
      const schema: StandardSchemaWithJSON<z.input<typeof original>, z.output<typeof original>> = {
        '~standard': {...standard, jsonSchema: {...standard.jsonSchema, input: options => {
          conversions()
          return standard.jsonSchema.input(options)
        }}},
      }
      const server = new McpServer({name: 'schema-test', version: '1'})
      registration(server).registerTool('shape', {inputSchema: schema}, async input => {
        handler(input)
        return {content: [], structuredContent: {segments: input.document.segments}}
      })
      const [client, transport] = InMemoryTransport.createLinkedPair()
      const pending = new Map<number, (response: Response) => void>()
      let id = 0
      client.onmessage = message => {
        if ('id' in message && typeof message.id === 'number' && ('result' in message || 'error' in message)) {
          pending.get(message.id)?.(message as Response)
          pending.delete(message.id)
        }
      }
      const request = async (method: string, params: Record<string, unknown> = {}) => {
        const next = ++id
        const result = new Promise<Response>(resolve => pending.set(next, resolve))
        await client.send({jsonrpc: '2.0', id: next, method, params} as JSONRPCMessage)
        const response = await result
        expect(response.error).toBeUndefined()
        return response.result!
      }
      await client.start()
      await server.connect(transport)
      try {
        await request('initialize', {protocolVersion: '2025-11-25', capabilities: {}, clientInfo: {name: 'test', version: '1'}})
        await client.send({jsonrpc: '2.0', method: 'notifications/initialized'})
        for (let listing = 0; listing < 24; listing++) {
          const result = await request('tools/list')
          const tools = result.tools as Array<{inputSchema: Record<string, unknown>}>
          expect(tools).toHaveLength(1)
          if (listing === 0) checkLocalReferences(tools[0]!.inputSchema)
        }
        const valid = await request('tools/call', {name: 'shape', arguments: {document}})
        expect(valid.structuredContent).toEqual({segments: 48})
        const malformed = structuredClone(document)
        malformed.nodes[0]!.radius.arms[0]!.pattern.kind = 'unknown'
        const invalid = await request('tools/call', {name: 'shape', arguments: {document: malformed}})
        expect(invalid.isError).toBe(true)
      } finally {
        await server.close()
        await client.close()
      }
    }
    expect(conversions).toHaveBeenCalledTimes(1)
    expect(handler).toHaveBeenCalledTimes(2)
  })
})
