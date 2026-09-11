/*
 * Qualification oracle for geometry-routing-contract-v1.
 *
 * Deliberately imports no production routing code or constants. Keep this
 * implementation small and literal: its purpose is to detect drift in the
 * production scanner, not to share that scanner's abstractions.
 */
export type ReferenceRouteOutcome = {
  kind: 'route'
  languageContract: 'legacy/current' | 'openscad-viewer/brep-1'
  engineClass: 'mesh' | 'brep'
  requiredCapabilities: string[]
} | {
  kind: 'error'
  message: string
  reportedContract: string | null
  line: number | null
}

const CAPABILITY = /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/

function error(
  message: string,
  reportedContract: string | null,
  line: number | null,
): ReferenceRouteOutcome {
  return { kind: 'error', message, reportedContract, line }
}

function visibleLine(
  raw: string,
  blockCommentAtStart: boolean,
  stringAtStart: boolean,
  escapedAtStart: boolean,
): { text: string; blockComment: boolean; string: boolean; escaped: boolean } {
  let text = stringAtStart ? '"' : ''
  let blockComment = blockCommentAtStart
  let string = stringAtStart
  let escaped = escapedAtStart
  for (let cursor = 0; cursor < raw.length; cursor++) {
    const current = raw[cursor]
    const next = raw[cursor + 1]
    if (blockComment) {
      if (current === '*' && next === '/') {
        blockComment = false
        cursor++
      }
      continue
    }
    if (string) {
      if (escaped) escaped = false
      else if (current === '\\') escaped = true
      else if (current === '"') {
        string = false
        text += '"'
      }
      continue
    }
    if (current === '"') {
      string = true
      text += current
      continue
    }
    if (current === '/' && next === '*') {
      blockComment = true
      cursor++
      continue
    }
    if (current === '/' && next === '/') {
      text += raw.slice(cursor)
      break
    }
    text += current
  }
  return { text, blockComment, string, escaped }
}

function hasWellFormedUnicode(value: string): boolean {
  for (let cursor = 0; cursor < value.length; cursor++) {
    const unit = value.charCodeAt(cursor)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const low = value.charCodeAt(++cursor)
      if (!(low >= 0xdc00 && low <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
}

export function referenceGeometryRoute(source: string): ReferenceRouteOutcome {
  if (!hasWellFormedUnicode(source)) {
    return error('Geometry source must contain well-formed Unicode.', null, null)
  }
  if (source.length > 250_000) {
    return error('Geometry source exceeds 250,000 characters.', null, null)
  }

  let languageContract: 'legacy/current' | 'openscad-viewer/brep-1' = 'legacy/current'
  let firstLanguageLine: number | null = null
  let bodyStarted = false
  let blockComment = false
  let string = false
  let escaped = false
  const capabilities = new Set<string>()

  const lines = source.split(/\r\n|\n|\r/)
  for (let cursor = 0; cursor < lines.length; cursor++) {
    const lineNumber = cursor + 1
    const visible = visibleLine(lines[cursor], blockComment, string, escaped)
    blockComment = visible.blockComment
    string = visible.string
    escaped = visible.escaped
    const comment = visible.text.indexOf('//')
    const directive = comment >= 0 ? visible.text.slice(comment) : visible.text
    const codeBefore = comment > 0 && visible.text.slice(0, comment).trim() !== ''
    const language = /^\s*\/\/\s*@language\s+(\S+)\s*$/.exec(directive)
    const requires = /^\s*\/\/\s*@requires\s+(.+?)\s*$/.exec(directive)
    const engine = /^\s*\/\/\s*@engine(?:\s+.*)?$/.exec(directive)

    if (language) {
      if (bodyStarted || codeBefore) {
        return error(
          'The @language directive must be in the leading source header.',
          language[1],
          lineNumber,
        )
      }
      if (firstLanguageLine !== null) {
        return error(
          `The @language directive is duplicated (first declared on line ${firstLanguageLine}).`,
          language[1],
          lineNumber,
        )
      }
      if (language[1] !== 'legacy/current' && language[1] !== 'openscad-viewer/brep-1') {
        return error(
          `Unsupported geometry language contract ${language[1]}.`,
          language[1],
          lineNumber,
        )
      }
      languageContract = language[1]
      firstLanguageLine = lineNumber
      continue
    }

    if (requires) {
      if (bodyStarted || codeBefore) {
        return error(
          'The @requires directive must be in the leading source header.',
          languageContract,
          lineNumber,
        )
      }
      const identifiers = requires[1].split(/[\s,]+/).filter(Boolean)
      if (identifiers.length === 0 || identifiers.some(identifier => !CAPABILITY.test(identifier))) {
        return error(
          'The @requires directive contains an invalid capability identifier.',
          languageContract,
          lineNumber,
        )
      }
      for (const identifier of identifiers) {
        capabilities.add(identifier)
        if (capabilities.size > 32) {
          return error(
            'The source may require at most 32 capabilities.',
            languageContract,
            lineNumber,
          )
        }
      }
      continue
    }

    if (engine) {
      return error(
        'The source cannot select an engine directly; choose a versioned @language contract.',
        languageContract,
        lineNumber,
      )
    }

    const malformed = /^\s*\/\/\s*@(language|requires|engine)\b/i.exec(directive)
    if (malformed) {
      return error(
        `Malformed @${malformed[1].toLowerCase()} directive.`,
        languageContract,
        lineNumber,
      )
    }
    if (visible.text.trim() !== '' && !/^\s*\/\//.test(visible.text)) bodyStarted = true
  }

  return {
    kind: 'route',
    languageContract,
    engineClass: languageContract === 'legacy/current' ? 'mesh' : 'brep',
    requiredCapabilities: [...capabilities].sort(),
  }
}
