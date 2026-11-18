import type { PublicToolError } from '../publicError'
import {
  OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES,
  OFFICIAL_OPENSCAD_MAX_WIRE_REQUEST_BYTES,
} from '../officialOpenScadRuntimeProtocol'
import { OfficialOpenScadRemoteError, OfficialOpenScadSupervisorError } from '../officialOpenScadRuntimeService'

export function mapOfficialRuntimeError(error: unknown): { error: PublicToolError; internal: boolean } | null {
  if (error instanceof OfficialOpenScadSupervisorError) {
    if (error.code === 'E_OFFICIAL_OPENSCAD_REQUEST_LIMIT') {
      return {
        error: {
          code: 'invalid_argument',
          message: 'The serialized official OpenSCAD request exceeds the transport limit.',
          retryable: false,
          next_action: 'Reduce source or project-file content and retry.',
          details: { max_bytes: OFFICIAL_OPENSCAD_MAX_WIRE_REQUEST_BYTES, runtime: 'official-openscad' },
        },
        internal: false,
      }
    }
    if (error.code === 'E_OFFICIAL_OPENSCAD_BUSY') {
      return {
        error: {
          code: 'server_busy',
          message: 'The official OpenSCAD runtime is at its concurrency limit.',
          retryable: true,
          next_action: 'Wait briefly, then retry the official OpenSCAD operation.',
          details: { retry_after_ms: 250, runtime: 'official-openscad' },
        },
        internal: false,
      }
    }
    if (error.code === 'E_OFFICIAL_OPENSCAD_CANCELLED') {
      return {
        error: {
          code: 'cancelled',
          message: 'The official OpenSCAD operation was cancelled.',
          retryable: true,
          next_action: 'Retry if the result is still needed.',
          details: { runtime: 'official-openscad' },
        },
        internal: false,
      }
    }
    if (error.code === 'E_OFFICIAL_OPENSCAD_DEADLINE') {
      return {
        error: {
          code: 'deadline_exceeded',
          message: 'The official OpenSCAD operation exceeded its deadline.',
          retryable: true,
          next_action: 'Reduce model complexity, increase timeout_ms within the advertised limit, or retry later.',
          details: { runtime: 'official-openscad' },
        },
        internal: false,
      }
    }
    if (error.code === 'E_OFFICIAL_OPENSCAD_UNAVAILABLE') {
      return {
        error: {
          code: 'engine_unavailable',
          message: 'The opt-in official OpenSCAD runtime is unavailable.',
          retryable: false,
          next_action:
            'Read openscad://official-runtime and run its setup_command, then restart or retry the MCP server.',
          details: { availability_cause: 'provider-missing', runtime: 'official-openscad', automatic_fallback: false },
        },
        internal: false,
      }
    }
    if (error.code === 'E_OFFICIAL_OPENSCAD_CLOSED') {
      return {
        error: {
          code: 'engine_unavailable',
          message: 'The official OpenSCAD runtime is closed.',
          retryable: false,
          next_action: 'Restart the local MCP server before retrying official OpenSCAD operations.',
          details: { availability_cause: 'revoked', runtime: 'official-openscad', automatic_fallback: false },
        },
        internal: false,
      }
    }
  }
  if (error instanceof OfficialOpenScadRemoteError) {
    if (error.code === 'E_OPENSCAD_OUTPUT_LIMIT') {
      return {
        error: {
          code: 'artifact_too_large',
          message: 'The official OpenSCAD export exceeded the advertised output limit.',
          retryable: true,
          next_action: 'Reduce model complexity or choose a more compact export format.',
          details: {
            estimated_bytes: null,
            max_bytes: OFFICIAL_OPENSCAD_MAX_OUTPUT_BYTES,
            runtime: 'official-openscad',
          },
        },
        internal: false,
      }
    }
    if (error.code === 'E_OPENSCAD_COMPILE' || error.code === 'E_OPENSCAD_NO_OUTPUT') {
      return {
        error: {
          code: 'source_syntax_error',
          message: 'The official OpenSCAD runtime rejected the source project.',
          retryable: false,
          next_action:
            'Inspect the bounded logs returned by openscad_official_check, correct the source or project files, and retry.',
          details: { runtime: 'official-openscad' },
        },
        internal: false,
      }
    }
  }
  return null
}
