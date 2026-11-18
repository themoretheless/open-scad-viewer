import type { PublicToolError } from '../publicError'
import { OpenSCADParseError } from '../../language/openscadErrors'
import { OpenScadImportPositionedError } from '../../exchange/openScadImport'
import { OpenScadProjectCompileError } from '../../language/openScadProjectCompiler'
import { OpenScadProjectError } from '../../language/openScadProject'
import { OpenScadSurfaceError } from '../../language/openScadSurface'
import { OpenScadTextPositionedError } from '../../language/openScadText'
import { parseErrorMessage } from './isAbortError'

export function mapSourceCompilerError(error: unknown): { error: PublicToolError; internal: boolean } | null {
  if (error instanceof OpenScadImportPositionedError) {
    return {
      error: {
        code: 'source_syntax_error',
        message: parseErrorMessage(error),
        retryable: false,
        next_action: 'Correct the import() project asset or path at the reported location, then retry.',
        line: error.line,
        column: error.column,
        details: {
          diagnostic_code: error.importCode,
          source_path: error.sourcePath,
          ...(error.specifier.length === 0 ? {} : { specifier: error.specifier }),
          ...(error.assetPath === undefined ? {} : { asset_path: error.assetPath }),
          ...(error.format === undefined ? {} : { format: error.format }),
          ...(error.limit === undefined ? {} : { limit: error.limit }),
          ...(error.actual === undefined ? {} : { actual: error.actual }),
        },
      },
      internal: false,
    }
  }
  if (error instanceof OpenScadTextPositionedError) {
    return {
      error: {
        code: 'source_syntax_error',
        message: parseErrorMessage(error),
        retryable: false,
        next_action:
          'Correct the text() parameters or provide the requested font in the bounded project files, then retry.',
        line: error.line,
        column: error.column,
        details: {
          diagnostic_code: error.textCode,
          source_path: error.sourcePath,
          ...(error.font === undefined ? {} : { font: error.font }),
          ...(error.fontPath === undefined ? {} : { font_path: error.fontPath }),
          ...(error.faceIndex === undefined ? {} : { face_index: error.faceIndex }),
          ...(error.limit === undefined ? {} : { limit: error.limit }),
          ...(error.actual === undefined ? {} : { actual: error.actual }),
        },
      },
      internal: false,
    }
  }
  if (error instanceof OpenScadSurfaceError) {
    return {
      error: {
        code: 'source_syntax_error',
        message: parseErrorMessage(error),
        retryable: false,
        next_action: 'Correct the surface() project file or path at the reported location, then retry.',
        line: error.line,
        column: error.column,
        details: {
          diagnostic_code: error.code,
          source_path: error.details.sourcePath,
          ...(error.details.specifier === undefined ? {} : { specifier: error.details.specifier }),
          ...(error.details.assetPath === undefined ? {} : { asset_path: error.details.assetPath }),
          ...(error.details.limit === undefined ? {} : { limit: error.details.limit }),
          ...(error.details.actual === undefined ? {} : { actual: error.details.actual }),
        },
      },
      internal: false,
    }
  }
  if (error instanceof OpenScadProjectCompileError) {
    return {
      error: {
        code: 'source_syntax_error',
        message: error.message,
        retryable: false,
        next_action: 'Correct the include/use dependency graph or project source files, then retry.',
        details: {
          diagnostic_code: error.code,
          ...(error.details.directive === undefined ? {} : { directive: error.details.directive }),
          ...(error.details.importer === undefined ? {} : { source_path: error.details.importer }),
          ...(error.details.specifier === undefined ? {} : { specifier: error.details.specifier }),
          ...(error.details.path === undefined ? {} : { dependency_path: error.details.path }),
          ...(error.details.chain === undefined ? {} : { dependency_chain: error.details.chain.join(' -> ') }),
          ...(error.details.metric === undefined ? {} : { metric: error.details.metric }),
          ...(error.details.limit === undefined ? {} : { limit: error.details.limit }),
          ...(error.details.actual === undefined ? {} : { actual: error.details.actual }),
          ...(error.details.modifier === undefined ? {} : { modifier: error.details.modifier }),
        },
      },
      internal: false,
    }
  }
  if (error instanceof OpenScadProjectError) {
    return {
      error: {
        code: 'source_syntax_error',
        message: error.message,
        retryable: false,
        next_action: 'Correct the bounded project file or dependency path, then retry.',
        details: {
          diagnostic_code: error.code,
          ...(error.details.path === undefined ? {} : { path: error.details.path }),
          ...(error.details.limit === undefined ? {} : { limit: error.details.limit }),
          ...(error.details.actual === undefined ? {} : { actual: error.details.actual }),
        },
      },
      internal: false,
    }
  }
  if (error instanceof OpenSCADParseError) {
    return {
      error: {
        code: 'source_syntax_error',
        message: parseErrorMessage(error),
        retryable: false,
        next_action: 'Correct the source at the reported line and column, then retry.',
        line: error.line,
        column: error.column,
      },
      internal: false,
    }
  }
  return null
}
