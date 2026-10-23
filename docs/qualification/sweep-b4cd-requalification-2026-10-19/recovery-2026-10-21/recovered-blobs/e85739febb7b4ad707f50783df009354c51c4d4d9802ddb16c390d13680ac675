import type { McpServer, StandardSchemaWithJSON } from '@modelcontextprotocol/server'

type Converter = StandardSchemaWithJSON['~standard']['jsonSchema']['input']

function memoizedSchema(schema: StandardSchemaWithJSON): StandardSchemaWithJSON {
  const standard = schema['~standard']
  const memoize = (convert: Converter): Converter => {
    const cache = new Map<string, Record<string, unknown>>()
    return options => {
      // Library-specific overrides may change conversion semantics. The SDK's
      // normal tools/list path supplies only its fixed JSON Schema target.
      if (options.libraryOptions) return convert(options)
      let result = cache.get(options.target)
      if (!result) {
        result = convert(options)
        cache.set(options.target, result)
      }
      return result
    }
  }
  return {
    '~standard': {
      ...standard,
      validate: (...args) => standard.validate(...args),
      jsonSchema: {
        input: memoize(options => standard.jsonSchema.input(options)),
        output: memoize(options => standard.jsonSchema.output(options)),
      },
    },
  }
}

/** Use only for a fixed set of tool contracts, never dynamically changing tools.
 * HTTP creates a new server per request. Keep those contracts (and their JSON
 * conversions) across registrations while retaining each original validator.
 */
export function createStaticToolRegistration() {
  const inputs = new Map<string, StandardSchemaWithJSON>()
  const outputs = new Map<string, StandardSchemaWithJSON>()
  const stable = (cache: Map<string, StandardSchemaWithJSON>, name: string, schema?: StandardSchemaWithJSON) => {
    if (!schema) return undefined
    let result = cache.get(name)
    if (!result) {
      result = memoizedSchema(schema)
      cache.set(name, result)
    }
    return result
  }
  return (server: McpServer): McpServer => new Proxy(server, {
    get(target, property) {
      if (property === 'registerTool') {
        return (name: string, config: {inputSchema?: StandardSchemaWithJSON; outputSchema?: StandardSchemaWithJSON}, handler: unknown) =>
          Reflect.apply(target.registerTool, target, [name, {
            ...config,
            inputSchema: stable(inputs, name, config.inputSchema),
            outputSchema: stable(outputs, name, config.outputSchema),
          }, handler])
      }
      const value = Reflect.get(target, property, target)
      return typeof value === 'function' ? value.bind(target) : value
    },
  })
}
