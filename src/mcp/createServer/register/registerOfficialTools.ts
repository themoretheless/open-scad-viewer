import { OFFICIAL_LANGUAGE_SUMMARY, officialArtifactSchema, officialCapabilitiesSchema, officialCheckSchema, officialExportSchema, officialFormatSchema, officialLanguageSummarySchema } from '../../createServer'
import { officialArtifactToWire, officialCapabilitiesToWire, officialCheckToWire, officialExportToWire, officialRunOptions, officialToolInputSchema, officialToolOutputSchema, sha256 } from '../officialToolInputSchema'
import { officialErrorResult, textResult } from '../persistedFailureStatus'
import { officialOpenScadMimeType } from '../../officialOpenScadRuntimeProtocol'
import { z } from 'zod/v4'
import type { McpServerContext } from './context'

/** Opt-in official OpenSCAD runtime: status, check and reference export. */
export function registerOfficialTools(ctx: McpServerContext): void {
  const { officialRuntime, officialArtifacts, server, notifyResourceListChanged } = ctx
  server.registerTool('openscad_official_status', {
    title: 'Upstream OpenSCAD oracle status',
    description: 'Report availability and integrity of the optional upstream qualification oracle. This provider is not the independent production engine or a fallback.',
    inputSchema: z.object({}).strict(),
    outputSchema: officialToolOutputSchema(z.object({
      language_contract: officialLanguageSummarySchema,
      official_runtime: officialCapabilitiesSchema,
    }).strict()),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async () => {
    try {
      return textResult({
        language_contract: OFFICIAL_LANGUAGE_SUMMARY,
        official_runtime: officialCapabilitiesToWire(await officialRuntime.capabilities()),
      })
    } catch (error) {
      return officialErrorResult(error)
    }
  })

  server.registerTool('openscad_official_check', {
    title: 'Check with the upstream OpenSCAD oracle',
    description: 'Run a bounded source project in the pinned upstream qualification oracle for differential comparison; this does not execute or attest the independent engine.',
    inputSchema: officialToolInputSchema({}),
    outputSchema: officialToolOutputSchema(z.object({ check: officialCheckSchema }).strict()),
    annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async (input, context) => {
    try {
      const result = await officialRuntime.check(
        input.source,
        officialRunOptions(input, context.mcpReq.signal),
      )
      return textResult({ check: officialCheckToWire(result) })
    } catch (error) {
      return officialErrorResult(error)
    }
  })

  server.registerTool('openscad_official_export', {
    title: 'Reference export with the upstream OpenSCAD oracle',
    description: 'Render a bounded source project in the pinned upstream oracle and expose a reference artifact; this is not an independent-engine export.',
    inputSchema: officialToolInputSchema({ format: officialFormatSchema }),
    outputSchema: officialToolOutputSchema(z.object({
      export: officialExportSchema,
      artifact: officialArtifactSchema,
    }).strict()),
    annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: true, openWorldHint: false },
  }, async (input, context) => {
    try {
      const exported = await officialRuntime.export(
        input.source,
        input.format,
        officialRunOptions(input, context.mcpReq.signal),
      )
      if (sha256(exported.data) !== exported.sha256
        || exported.format !== input.format
        || exported.mimeType !== officialOpenScadMimeType(input.format)
        || exported.fileName !== `model.${input.format}`) {
        throw new Error('Official OpenSCAD export failed its MCP integrity boundary')
      }
      const artifact = officialArtifacts.put({
        format: exported.format,
        fileName: exported.fileName,
        mimeType: exported.mimeType,
        data: exported.data,
      })
      notifyResourceListChanged()
      const artifactWire = officialArtifactToWire(artifact, exported.format)
      const structured = {
        export: officialExportToWire(exported),
        artifact: artifactWire,
      }
      const result = textResult(structured)
      return {
        ...result,
        content: [
          result.content[0],
          {
            type: 'resource_link' as const,
            uri: artifactWire.resource_uri,
            name: artifact.fileName,
            mimeType: artifact.mimeType,
            description: `Official OpenSCAD ${exported.format.toUpperCase()} export (${artifact.byteLength.toLocaleString('en-US')} bytes)`,
          },
        ],
      }
    } catch (error) {
      return officialErrorResult(error)
    }
  })
}
