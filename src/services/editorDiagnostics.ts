import type { GeometryBuildError } from './geometryWorkerProtocol'

export interface EditorDiagnostic {
  readonly severity: 'error' | 'warning'
  readonly message: string
  readonly code?: string
  readonly line: number
  readonly column: number
  readonly start: number
  readonly end: number
}

export function offsetForLineColumn(source: string, line: number, column: number): number | null {
  if (!Number.isSafeInteger(line) || line < 1 || !Number.isSafeInteger(column) || column < 1) return null
  let offset = 0
  for (let currentLine = 1; currentLine < line; currentLine++) {
    const newline = source.indexOf('\n', offset)
    if (newline < 0) return null
    offset = newline + 1
  }
  const lineEnd = source.indexOf('\n', offset)
  return Math.min(offset + column - 1, lineEnd < 0 ? source.length : lineEnd)
}

export function diagnosticFromBuildError(source: string, error: GeometryBuildError): EditorDiagnostic | null {
  if (error.line === undefined || error.column === undefined) return null
  const start = error.start ?? offsetForLineColumn(source, error.line, error.column)
  if (start === null) return null
  const end = error.end === undefined
    ? Math.min(source.length, start + Math.max(1, source.slice(start).match(/^[\p{L}\p{N}_$]+/u)?.[0].length ?? 1))
    : Math.max(start, Math.min(source.length, error.end))
  return {
    severity: 'error', message: error.message, code: error.code,
    line: error.line, column: error.column, start, end,
  }
}

export function revealDiagnostic(editor: Pick<HTMLTextAreaElement, 'focus' | 'setSelectionRange'>, diagnostic: EditorDiagnostic) {
  editor.focus({ preventScroll: false })
  editor.setSelectionRange(diagnostic.start, diagnostic.end, 'forward')
}
