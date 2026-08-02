/** Raised when cooperative cancellation supersedes an OpenSCAD evaluation. */
export class AbortedError extends Error {
  constructor() {
    super('Evaluation aborted: superseded by a newer request')
    this.name = 'AbortedError'
  }
}

/** Syntax or evaluation error carrying a stable source position. */
export class OpenSCADParseError extends Error {
  readonly line: number
  readonly column: number
  readonly code?: LanguageDiagnosticCode
  readonly start: number
  readonly end: number
  /** Source-free diagnostic detail supplied by the language evaluator. */
  readonly detail: string

  constructor(source: string, position: number, message: string, code?: LanguageDiagnosticCode, endPosition?: number) {
    const safePosition = Math.max(0, Math.min(position, source.length))
    const before = source.slice(0, safePosition)
    const line = before.split('\n').length
    const lineStart = before.lastIndexOf('\n') + 1
    const lineEnd = source.indexOf('\n', safePosition)
    const excerpt = source.slice(lineStart, lineEnd < 0 ? source.length : lineEnd)
    const column = safePosition - lineStart + 1
    super(`Line ${line}, column ${column}: ${message}\n${excerpt}\n${' '.repeat(Math.max(0, column - 1))}^`)
    this.name = 'OpenSCADParseError'
    this.line = line
    this.column = column
    this.code = code
    this.start = safePosition
    this.end = Math.max(safePosition, Math.min(endPosition ?? safePosition + 1, source.length))
    this.detail = message
  }
}
import type { LanguageDiagnosticCode } from '../core/languageContract'
