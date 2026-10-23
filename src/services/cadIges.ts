import {callGeometryRust} from './geometry/kernel'
import type {NurbsBrep} from './geometry/brep'
import type {StepIdentityReport} from './cadAnalyticStep'

export interface DirectIgesV2Certificate {
  capability:'iges-interchange/2'
  complete:true
  notes:string[]
}
export interface DirectIgesV2Report {
  identity:StepIdentityReport
  ignoredMetadata:string[]
  entityCount:number
  topologyCount:number
}
export interface DirectIgesV2Export extends DirectIgesV2Report {
  text:string
  certificate:DirectIgesV2Certificate
}
export interface DirectIgesV2Import extends DirectIgesV2Report {
  model:NurbsBrep
  certificate:DirectIgesV2Certificate
}

/** Strict 80-column direct IGES B-rep export; no constructor, AABB, or mesh route. */
export const exportDirectIgesV2=(model:NurbsBrep):DirectIgesV2Export=>
  callGeometryRust('brep_nurbs_export_iges_v2',{model})

/** Direct finite IGES topology import with explicit identity and ignored-metadata reporting. */
export const importDirectIgesV2=(text:string):DirectIgesV2Import=>
  callGeometryRust('brep_nurbs_import_iges_v2',{text})

export interface IgesHeaderMetadata {
  startComments: string[]
  globalDelimiter: string
  recordDelimiter: string
  productId: string | null
  fileName: string | null
  unitsCode: number | null
  unitsName: string | null
  directoryEntryCount: number
  hasNurbsSurface128: boolean
  hasTrimmedSurface144: boolean
}

const IGES_UNITS_BY_CODE: Readonly<Record<number, string>> = Object.freeze({
  1: 'IN',
  2: 'MM',
  3: 'CUSTOM',
  4: 'FT',
  5: 'MI',
  6: 'M',
  7: 'KM',
  8: 'MIL',
  9: 'UM',
  10: 'CM',
  11: 'UIN',
})

function parseHollerithToken(
  text: string,
  startIndex: number,
  paramDelim: string,
  recordDelim: string,
): { value: string | null; nextIndex: number } {
  let i = startIndex
  while (i < text.length && text[i] === ' ') i++
  if (i >= text.length || text[i] === paramDelim || text[i] === recordDelim) {
    return {
      value: null,
      nextIndex: i < text.length && text[i] === paramDelim ? i + 1 : i,
    }
  }
  const hMatch = /^(\d+)H/.exec(text.slice(i))
  if (hMatch) {
    const charCount = Number.parseInt(hMatch[1]!, 10)
    const contentStart = i + hMatch[0].length
    const contentEnd = contentStart + charCount
    const value = text.slice(contentStart, contentEnd)
    let next = contentEnd
    while (next < text.length && text[next] === ' ') next++
    if (next < text.length && text[next] === paramDelim) next++
    return { value, nextIndex: next }
  }
  let end = i
  while (end < text.length && text[end] !== paramDelim && text[end] !== recordDelim) {
    end++
  }
  const raw = text.slice(i, end).trim()
  const nextIndex = end < text.length && text[end] === paramDelim ? end + 1 : end
  return { value: raw.length > 0 ? raw : null, nextIndex }
}

/**
 * Quickly inspects IGES 5.3 files (80-column Start `S`, Global `G`, Directory `D`,
 * Parameter `P`, Terminate `T` sections) for header metadata, units, and NURBS/trimmed
 * surface entity presence (Entity 128 & Entity 144).
 */
export function inspectIgesHeaderMetadata(igesText: string): IgesHeaderMetadata {
  if (typeof igesText !== 'string' || igesText.length === 0) {
    return {
      startComments: [],
      globalDelimiter: ',',
      recordDelimiter: ';',
      productId: null,
      fileName: null,
      unitsCode: null,
      unitsName: null,
      directoryEntryCount: 0,
      hasNurbsSurface128: false,
      hasTrimmedSurface144: false,
    }
  }

  const lines = igesText.split(/\r?\n/)
  const startComments: string[] = []
  let globalContent = ''
  let directoryLinesCount = 0
  let hasNurbsSurface128 = false
  let hasTrimmedSurface144 = false

  for (const rawLine of lines) {
    if (rawLine.length < 73) continue
    const sectionCode = rawLine[72]
    const payload = rawLine.slice(0, 72)

    if (sectionCode === 'S') {
      const comment = payload.trim()
      if (comment.length > 0) startComments.push(comment)
    } else if (sectionCode === 'G') {
      globalContent += payload
    } else if (sectionCode === 'D') {
      directoryLinesCount++
      const typeField = payload.slice(0, 8).trim()
      const entityType = Number.parseInt(typeField, 10)
      if (entityType === 128) hasNurbsSurface128 = true
      else if (entityType === 144) hasTrimmedSurface144 = true
    } else if (sectionCode === 'P') {
      const trimmedPayload = payload.slice(0, 64).trim()
      if (/^128\s*,/.test(trimmedPayload)) hasNurbsSurface128 = true
      else if (/^144\s*,/.test(trimmedPayload)) hasTrimmedSurface144 = true
    }
  }

  let globalDelimiter = ','
  let recordDelimiter = ';'
  let productId: string | null = null
  let fileName: string | null = null
  let unitsCode: number | null = null
  let unitsName: string | null = null

  const trimmedGlobal = globalContent.trimStart()
  if (trimmedGlobal.length > 0) {
    let cursor = 0
    // Field 1: Parameter delimiter character
    if (trimmedGlobal.startsWith(',')) {
      globalDelimiter = ','
      cursor = 1
    } else {
      const m1 = /^(\d+)H(.)/.exec(trimmedGlobal)
      if (m1 && Number.parseInt(m1[1]!, 10) === 1) {
        globalDelimiter = m1[2]!
        cursor = m1[0].length
        while (cursor < trimmedGlobal.length && trimmedGlobal[cursor] === ' ') cursor++
        if (trimmedGlobal[cursor] === globalDelimiter) cursor++
      }
    }

    // Field 2: Record delimiter character
    while (cursor < trimmedGlobal.length && trimmedGlobal[cursor] === ' ') cursor++
    if (trimmedGlobal[cursor] === globalDelimiter) {
      recordDelimiter = ';'
      cursor++
    } else {
      const m2 = /^(\d+)H(.)/.exec(trimmedGlobal.slice(cursor))
      if (m2 && Number.parseInt(m2[1]!, 10) === 1) {
        recordDelimiter = m2[2]!
        cursor += m2[0].length
        while (cursor < trimmedGlobal.length && trimmedGlobal[cursor] === ' ') cursor++
        if (trimmedGlobal[cursor] === globalDelimiter) cursor++
      } else if (trimmedGlobal[cursor] === recordDelimiter) {
        cursor++
      }
    }

    // Parse remaining Global section fields sequentially (fields 3..15)
    const fields: Array<string | null> = [globalDelimiter, recordDelimiter]
    while (cursor < trimmedGlobal.length && fields.length < 25) {
      if (trimmedGlobal[cursor] === recordDelimiter) break
      const parsed = parseHollerithToken(trimmedGlobal, cursor, globalDelimiter, recordDelimiter)
      fields.push(parsed.value)
      if (parsed.nextIndex <= cursor) break
      cursor = parsed.nextIndex
    }

    productId = fields[2] ?? null
    fileName = fields[3] ?? null
    const rawUnitsFlag = fields[13]
    if (rawUnitsFlag !== null && rawUnitsFlag !== undefined) {
      const parsedCode = Number.parseInt(rawUnitsFlag, 10)
      if (Number.isFinite(parsedCode)) {
        unitsCode = parsedCode
      }
    }
    const rawUnitsName = fields[14]
    if (rawUnitsName && rawUnitsName.trim().length > 0) {
      unitsName = rawUnitsName.trim()
    } else if (unitsCode !== null && IGES_UNITS_BY_CODE[unitsCode]) {
      unitsName = IGES_UNITS_BY_CODE[unitsCode]!
    }
  }

  return {
    startComments,
    globalDelimiter,
    recordDelimiter,
    productId,
    fileName,
    unitsCode,
    unitsName,
    directoryEntryCount: Math.floor(directoryLinesCount / 2),
    hasNurbsSurface128,
    hasTrimmedSurface144,
  }
}
