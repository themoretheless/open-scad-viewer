/**
 * Canonical MCP server entrypoint and public contract re-exports.
 * Server assembly lives in `./createServer/createOpenScadMcpServer.ts`,
 * wire/error contracts in `./server/contracts.ts`, and Zod schemas in `./createServer/schemas/`.
 */

export { createOpenScadMcpServer } from './createServer/createOpenScadMcpServer'
export { persistedFailureStatus } from './createServer/persistedFailureStatus'

export * from './server/contracts'
export * from './createServer/schemas/commonSchemas'
export * from './createServer/schemas/executionAndBuildSchemas'
export * from './createServer/schemas/analysisAndComparisonSchemas'
export * from './createServer/schemas/officialSchemas'
export * from './createServer/schemas/independentSchemas'
