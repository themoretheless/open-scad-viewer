import { CACHE_ONE_DAY } from '../../createServer'
import { EXAMPLES, EXAMPLE_CATALOG } from '../../../data/examples'
import { ResourceNotFoundError, ResourceTemplate } from '@modelcontextprotocol/server'
import { artifactUri, buildUri, independentArtifactUri, modelUri, officialArtifactUri, resourceId, resourceVariable, sha256 } from '../officialToolInputSchema'
import { buildToWire } from '../persistedFailureStatus'
import type { McpServerContext } from './context'

/** Templated resources: models, revisions, builds, artifacts and examples. */
export function registerTemplateResources(ctx: McpServerContext): void {
  registerSavedModel(ctx)
  registerSavedModelRevision(ctx)
  registerBuildResult(ctx)
  registerExportArtifact(ctx)
  registerIndependentExportArtifact(ctx)
  registerOfficialExportArtifact(ctx)
  registerExample(ctx)
}

function registerSavedModel(ctx: McpServerContext): void {
  const { store, server } = ctx
  const modelTemplate = new ResourceTemplate('openscad://models/{id}', {
    list: async () => ({
      resources: (await store.listModels(500)).map(model => ({
        uri: modelUri(model.id),
        name: model.name,
        description: `OpenSCAD source, revision ${model.revision}`,
        mimeType: 'text/x-openscad',
      })),
    }),
    complete: { id: async value => (await store.listModels(500)).map(model => model.id).filter(id => id.startsWith(value)) },
  })
  server.registerResource('saved-model', modelTemplate, {
    title: 'Saved OpenSCAD model',
    description: 'Versioned OpenSCAD source stored in DuckDB.',
    mimeType: 'text/x-openscad',
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const model = await store.getModel(id)
    if (!model) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'text/x-openscad', text: model.source }] }
  })
}

function registerSavedModelRevision(ctx: McpServerContext): void {
  const { store, server } = ctx
  const modelRevisionTemplate = new ResourceTemplate('openscad://models/{id}/revisions/{revision}', {
    list: undefined,
  })
  server.registerResource('saved-model-revision', modelRevisionTemplate, {
    title: 'Saved OpenSCAD model revision',
    description: 'An immutable OpenSCAD source snapshot stored in DuckDB.',
    mimeType: 'text/x-openscad',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const revisionValue = resourceVariable(uri, variables.revision)
    if (!/^\d+$/.test(revisionValue)) throw new ResourceNotFoundError(uri.href)
    const revision = Number(revisionValue)
    if (!Number.isSafeInteger(revision)) throw new ResourceNotFoundError(uri.href)
    const model = await store.getModelRevision(id, revision)
    if (!model) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'text/x-openscad', text: model.source }] }
  })
}

function registerBuildResult(ctx: McpServerContext): void {
  const { store, server } = ctx
  const buildTemplate = new ResourceTemplate('openscad://builds/{id}', {
    list: async () => ({
      resources: (await store.listBuilds({ limit: 500 })).map(build => ({
        uri: buildUri(build.id),
        name: `Build ${build.id}`,
        description: `${build.status} ${build.quality} build`,
        mimeType: 'application/json',
      })),
    }),
  })
  server.registerResource('build-result', buildTemplate, {
    title: 'OpenSCAD build result',
    description: 'A compact build result recorded in DuckDB.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const build = await store.getBuild(id)
    if (!build) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify(buildToWire(build), null, 2) }] }
  })
}

function registerExportArtifact(ctx: McpServerContext): void {
  const { store, server } = ctx
  const artifactTemplate = new ResourceTemplate('openscad://artifacts/{id}', {
    list: async () => ({
      resources: (await store.listArtifacts(100)).map(artifact => ({
        uri: artifactUri(artifact.id),
        name: artifact.fileName,
        description: `${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString('en-US')} bytes)`,
        mimeType: artifact.mimeType,
      })),
    }),
  })
  server.registerResource('export-artifact', artifactTemplate, {
    title: 'OpenSCAD export artifact',
    description: 'A bounded STL or OBJ artifact stored in DuckDB.',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const artifact = await store.getArtifact(id)
    if (!artifact) throw new ResourceNotFoundError(uri.href)
    return artifact.format === 'obj'
      ? { contents: [{ uri: uri.href, mimeType: artifact.mimeType, text: new TextDecoder().decode(artifact.data) }] }
      : { contents: [{ uri: uri.href, mimeType: artifact.mimeType, blob: Buffer.from(artifact.data).toString('base64') }] }
  })
}

function registerIndependentExportArtifact(ctx: McpServerContext): void {
  const { independentArtifacts, server } = ctx
  const independentArtifactTemplate = new ResourceTemplate(
    'openscad://independent-artifacts/{sha256}',
    {
      list: async () => ({
        resources: independentArtifacts.list().map(artifact => ({
          uri: independentArtifactUri(artifact.id),
          name: artifact.fileName,
          description: `Independent OpenSCAD ${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString('en-US')} bytes); session-local`,
          mimeType: artifact.mimeType,
        })),
      }),
      complete: {
        sha256: value => independentArtifacts.list().map(artifact => artifact.id)
          .filter(id => id.startsWith(value)),
      },
    },
  )
  server.registerResource('independent-export-artifact', independentArtifactTemplate, {
    title: 'Independent OpenSCAD export artifact',
    description: 'A bounded content-addressed STL or OBJ produced by the repository-owned engine and retained only for this MCP server session.',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceVariable(uri, variables.sha256)
    if (!/^[a-f0-9]{64}$/.test(id)) throw new ResourceNotFoundError(uri.href)
    const artifact = independentArtifacts.get(id)
    if (!artifact || artifact.sha256 !== id || sha256(artifact.data) !== id) {
      throw new ResourceNotFoundError(uri.href)
    }
    return artifact.format === 'obj'
      ? {
          contents: [{
            uri: uri.href,
            mimeType: artifact.mimeType,
            text: new TextDecoder().decode(artifact.data),
          }],
        }
      : {
          contents: [{
            uri: uri.href,
            mimeType: artifact.mimeType,
            blob: Buffer.from(artifact.data).toString('base64'),
          }],
        }
  })
}

function registerOfficialExportArtifact(ctx: McpServerContext): void {
  const { officialArtifacts, server } = ctx
  const officialArtifactTemplate = new ResourceTemplate('openscad://official-artifacts/{id}', {
    list: async () => ({
      resources: officialArtifacts.list().map(artifact => ({
        uri: officialArtifactUri(artifact.id),
        name: artifact.fileName,
        description: `Official OpenSCAD ${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString('en-US')} bytes); session-local`,
        mimeType: artifact.mimeType,
      })),
    }),
    complete: {
      id: value => officialArtifacts.list().map(artifact => artifact.id)
        .filter(id => id.startsWith(value)),
    },
  })
  server.registerResource('official-export-artifact', officialArtifactTemplate, {
    title: 'Official OpenSCAD export artifact',
    description: 'A bounded content-addressed upstream-runtime export retained only for this MCP server session.',
  }, async (uri, variables) => {
    const id = resourceVariable(uri, variables.id)
    if (!/^[a-f0-9]{64}$/.test(id)) throw new ResourceNotFoundError(uri.href)
    const artifact = officialArtifacts.get(id)
    if (!artifact) throw new ResourceNotFoundError(uri.href)
    return {
      contents: [{
        uri: uri.href,
        mimeType: artifact.mimeType,
        blob: Buffer.from(artifact.data).toString('base64'),
      }],
    }
  })
}

function registerExample(ctx: McpServerContext): void {
  const { server } = ctx
  const exampleNames = EXAMPLE_CATALOG.map(example => example.id).sort()
  const examplesById = new Map(EXAMPLE_CATALOG.map(example => [example.id, example]))
  const exampleTemplate = new ResourceTemplate('openscad://examples/{name}', {
    list: async () => ({
      resources: exampleNames.map(name => ({
        uri: `openscad://examples/${name}`,
        name: `${name}.scad`,
        description: examplesById.get(name)?.description.en ?? `Bundled OpenSCAD example: ${name}`,
        mimeType: 'text/x-openscad',
      })),
    }),
    complete: { name: value => exampleNames.filter(name => name.startsWith(value)) },
  })
  server.registerResource('example', exampleTemplate, {
    title: 'Bundled OpenSCAD example',
    description: 'An example from the browser workspace.',
    mimeType: 'text/x-openscad',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'public' },
  }, async (uri, variables) => {
    const name = resourceVariable(uri, variables.name)
    const source = EXAMPLES[name]
    if (source === undefined) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'text/x-openscad', text: source }] }
  })
}
