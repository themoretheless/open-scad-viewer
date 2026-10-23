export const GEOMETRY_LANGUAGE_CONTRACTS = Object.freeze([
  'legacy/current',
  'openscad-viewer/brep-1',
] as const)

export type GeometryLanguageContract = typeof GEOMETRY_LANGUAGE_CONTRACTS[number]

export const MAX_GEOMETRY_SOURCE_CHARACTERS = 250_000
const MAX_REQUIRED_CAPABILITIES = 32
const CAPABILITY_ID = /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/

export class GeometryLanguageContractError extends RangeError {
  constructor(
    message: string,
    readonly reportedContract: string | null,
    readonly line: number | null,
  ) {
    super(message)
    this.name = 'GeometryLanguageContractError'
  }
}

export interface GeometrySourceRoutingHeader {
  languageContract: GeometryLanguageContract
  requiredCapabilities: string[]
}

function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xD800 && unit <= 0xDBFF) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xDC00 && next <= 0xDFFF)) return false
    } else if (unit >= 0xDC00 && unit <= 0xDFFF) return false
  }
  return true
}

function sourceOutsideBlockComments(
  rawLine: string,
  initiallyInBlockComment: boolean,
  initiallyInString: boolean,
  initiallyEscaped: boolean,
): { line: string; inBlockComment: boolean; inString: boolean; escaped: boolean } {
  let line = initiallyInString ? '"' : ''
  let inBlockComment = initiallyInBlockComment
  let inString = initiallyInString
  let escaped = initiallyEscaped
  for (let index = 0; index < rawLine.length; index++) {
    const character = rawLine[index]
    const next = rawLine[index + 1]
    if (inBlockComment) {
      if (character === '*' && next === '/') {
        inBlockComment = false
        index++
      }
      continue
    }
    if (inString) {
      if (escaped) escaped = false
      else if (character === '\\') escaped = true
      else if (character === '"') {
        inString = false
        line += '"'
      }
      continue
    }
    if (character === '"') {
      inString = true
      line += character
      continue
    }
    if (character === '/' && next === '*') {
      inBlockComment = true
      index++
      continue
    }
    if (character === '/' && next === '/') {
      line += rawLine.slice(index)
      break
    }
    line += character
  }
  return { line, inBlockComment, inString, escaped }
}

/** Parses only the bounded source header; it deliberately has no engine-manifest dependency. */
export function parseGeometrySourceRoutingHeader(source: string): GeometrySourceRoutingHeader {
  if (!isWellFormedUnicode(source)) {
    throw new GeometryLanguageContractError(
      'Geometry source must contain well-formed Unicode.',
      null,
      null,
    )
  }
  if (source.length > MAX_GEOMETRY_SOURCE_CHARACTERS) {
    throw new GeometryLanguageContractError(
      `Geometry source exceeds ${MAX_GEOMETRY_SOURCE_CHARACTERS.toLocaleString('en-US')} characters.`,
      null,
      null,
    )
  }

  let languageContract: GeometryLanguageContract = 'legacy/current'
  let languageDirectiveLine: number | null = null
  let bodyStarted = false
  let inBlockComment = false
  let inString = false
  let escaped = false
  const requiredCapabilities = new Set<string>()

  for (const [index, rawLine] of source.split(/\r\n|\n|\r/).entries()) {
    const lineNumber = index + 1
    const visible = sourceOutsideBlockComments(rawLine, inBlockComment, inString, escaped)
    const line = visible.line
    inBlockComment = visible.inBlockComment
    inString = visible.inString
    escaped = visible.escaped
    const lineCommentStart = line.indexOf('//')
    const directiveLine = lineCommentStart >= 0 ? line.slice(lineCommentStart) : line
    const codeBeforeDirective = lineCommentStart > 0
      && line.slice(0, lineCommentStart).trim() !== ''
    const languageMatch = /^\s*\/\/\s*@language\s+(\S+)\s*$/.exec(directiveLine)
    const requiresMatch = /^\s*\/\/\s*@requires\s+(.+?)\s*$/.exec(directiveLine)
    const engineMatch = /^\s*\/\/\s*@engine(?:\s+.*)?$/.exec(directiveLine)

    if (languageMatch) {
      if (bodyStarted || codeBeforeDirective) {
        throw new GeometryLanguageContractError(
          'The @language directive must be in the leading source header.',
          languageMatch[1],
          lineNumber,
        )
      }
      if (languageDirectiveLine !== null) {
        throw new GeometryLanguageContractError(
          `The @language directive is duplicated (first declared on line ${languageDirectiveLine}).`,
          languageMatch[1],
          lineNumber,
        )
      }
      if (languageMatch[1] !== 'legacy/current' && languageMatch[1] !== 'openscad-viewer/brep-1') {
        throw new GeometryLanguageContractError(
          `Unsupported geometry language contract ${languageMatch[1]}.`,
          languageMatch[1],
          lineNumber,
        )
      }
      languageContract = languageMatch[1]
      languageDirectiveLine = lineNumber
      continue
    }

    if (requiresMatch) {
      if (bodyStarted || codeBeforeDirective) {
        throw new GeometryLanguageContractError(
          'The @requires directive must be in the leading source header.',
          languageContract,
          lineNumber,
        )
      }
      const identifiers = requiresMatch[1].split(/[\s,]+/).filter(Boolean)
      if (!identifiers.length || identifiers.some(identifier => !CAPABILITY_ID.test(identifier))) {
        throw new GeometryLanguageContractError(
          'The @requires directive contains an invalid capability identifier.',
          languageContract,
          lineNumber,
        )
      }
      for (const identifier of identifiers) {
        requiredCapabilities.add(identifier)
        if (requiredCapabilities.size > MAX_REQUIRED_CAPABILITIES) {
          throw new GeometryLanguageContractError(
            `The source may require at most ${MAX_REQUIRED_CAPABILITIES} capabilities.`,
            languageContract,
            lineNumber,
          )
        }
      }
      continue
    }

    if (engineMatch) {
      throw new GeometryLanguageContractError(
        'The source cannot select an engine directly; choose a versioned @language contract.',
        languageContract,
        lineNumber,
      )
    }

    const malformedReservedDirective = /^\s*\/\/\s*@(language|requires|engine)\b/i.exec(directiveLine)
    if (malformedReservedDirective) {
      throw new GeometryLanguageContractError(
        `Malformed @${malformedReservedDirective[1].toLowerCase()} directive.`,
        languageContract,
        lineNumber,
      )
    }

    if (line.trim() !== '' && !/^\s*\/\//.test(line)) bodyStarted = true
  }

  return { languageContract, requiredCapabilities: [...requiredCapabilities].sort() }
}
