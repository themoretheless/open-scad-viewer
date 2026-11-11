// Layer order between the top-level modules of a crate (ADR 0012). A reference from a lower
// layer to a higher one is an upward edge. Lexical: counts `crate::<module>` paths outside
// comments and strings, resolving `pub use` aliases declared in the crate root.
import {readFileSync, existsSync} from 'node:fs'
import {resolve} from 'node:path'

export const MODULE_LAYERS = {
  // Module families of brep-core (refactor plan M8), ordered to minimise references
  // against the order; the remaining upward edges are pinned and may only shrink.
  'brep-core': {
    profile: 0, model: 1, minkowski: 2, contact: 3, sweep_offset: 4, parts: 5, source_body: 6,
    intersection: 7, boolean: 8, body_analysis: 9, features: 10,
  },
  'nurbs-core': {
    numerics: 0, core: 1, foundation: 2, certificates: 2, distance: 2, analysis: 3, trim: 3,
    intersection: 4, offset: 4, join: 4, editing: 4, construct: 5, sweeps: 5, tessellation: 5,
    transport: 6, domains: 7,
  },
}

function stripCommentsAndStrings(text) {
  return text.replace(/\/\*[\s\S]*?\*\//g, ' ').replace(/\/\/[^\n]*/g, ' ').replace(/r#*"(?:[^"\\]|\\.)*"#*|"(?:[^"\\]|\\.)*"/g, '""')
}

/** alias -> top-level module, from `[pub] use crate::a::b[ as c];` lines in the crate root. */
export function rootAliases(libText) {
  const aliases = {}
  for (const m of stripCommentsAndStrings(libText).matchAll(/(?:\bpub(?:\([^)]*\))?\s+)?\buse\s+(?:crate::)?([a-z_][\w]*)((?:::[a-z_][\w]*)*)(?:\s+as\s+([a-z_][\w]*))?\s*;/g)) {
    const segments = (m[1] + m[2]).split('::')
    aliases[m[3] ?? segments.at(-1)] = segments[0]
  }
  return aliases
}

/** First path segment of each top-level item in `use crate::{a::b, c, d::{e, f}}`, starting after `{`. */
export function braceHeads(text, start) {
  const heads = []
  let depth = 0, item = ''
  for (let i = start; i < text.length; i++) {
    const ch = text[i]
    if (ch === '{') depth++
    else if (ch === '}' && depth-- === 0) break
    if (ch === ',' && depth === 0) { heads.push(item); item = '' } else item += ch
  }
  heads.push(item)
  return heads.map(h => h.trim().split(/::|\s|\{/)[0]).filter(h => /^[a-z_]\w*$/.test(h))
}

/** Text with every inline `#[cfg(test)]` or `#[cfg(all(test, ...))]` module block removed (brace-matched). */
export function withoutTestModules(text) {
  const opener = /#\[cfg\((?:test|all\(\s*test\b[^\]]*\))\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{/g
  let out = '', from = 0, m
  while ((m = opener.exec(text))) {
    let depth = 1, i = m.index + m[0].length
    while (i < text.length && depth > 0) {
      if (text[i] === '{') depth++
      else if (text[i] === '}') depth--
      i++
    }
    out += text.slice(from, m.index)
    from = i
    opener.lastIndex = i
  }
  return out + text.slice(from)
}

/** Sorted `from -> to` strings for upward references in one crate. */
export function upwardModuleEdges(root, crate, files) {
  const layers = MODULE_LAYERS[crate]
  const src = `crates/${crate}/src/`
  const lib = resolve(root, src, 'lib.rs')
  if (!layers || !existsSync(lib)) return []
  const aliases = rootAliases(readFileSync(lib, 'utf8'))
  const edges = new Set()
  for (const path of files) {
    if (!path.startsWith(src) || !path.endsWith('.rs') || /(^|\/)tests?(\/|\.rs$)|_tests?\.rs$/.test(path)) continue
    const from = path.slice(src.length).split('/')[0].replace(/\.rs$/, '')
    if (layers[from] === undefined) continue
    // Inline `#[cfg(test)] mod tests { ... }` blocks are test code; production code may follow them.
    const text = withoutTestModules(stripCommentsAndStrings(readFileSync(resolve(root, path), 'utf8')))
    const heads = [...text.matchAll(/\bcrate::([a-z_][\w]*)/g)].map(m => m[1])
    for (const m of text.matchAll(/\bcrate::\{/g)) heads.push(...braceHeads(text, m.index + m[0].length))
    for (const head of heads) {
      const to = layers[head] !== undefined ? head : aliases[head]
      if (to !== undefined && to !== from && layers[to] !== undefined && layers[to] > layers[from]) edges.add(`${crate}: ${from} -> ${to}`)
    }
  }
  return [...edges].sort()
}
