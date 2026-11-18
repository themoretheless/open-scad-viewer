// Function length ratchet input: every Rust and TypeScript function body longer than the limit.
// Rust is scanned lexically (comments, strings and char literals skipped; braces matched);
// TypeScript and `.svt` scripts use the TypeScript parser. Keys are `path::name@ordinal`, where the
// ordinal tells same-named functions in one file apart in source order.
import {readFileSync, existsSync} from 'node:fs'
import {resolve, extname} from 'node:path'
import {createRequire} from 'node:module'

export const FUNCTION_LINE_LIMIT = 150

/** Rust functions: [{name, line, lines}] for every `fn` with a body. */
export function rustFunctions(text) {
  const out = [], n = text.length
  let i = 0, line = 1
  const skip = () => {
    const c = text[i]
    if (c === '\n') { line++; i++; return true }
    if (c === '/' && text[i + 1] === '/') { while (i < n && text[i] !== '\n') i++; return true }
    if (c === '/' && text[i + 1] === '*') {
      let level = 1; i += 2
      while (i < n && level) {
        if (text[i] === '\n') line++
        if (text[i] === '/' && text[i + 1] === '*') { level++; i += 2 }
        else if (text[i] === '*' && text[i + 1] === '/') { level--; i += 2 }
        else i++
      }
      return true
    }
    if (c === 'r' && /^r#*"/.test(text.slice(i, i + 8)) && !/\w/.test(text[i - 1] ?? '')) {
      const hashes = /^r(#*)"/.exec(text.slice(i))[1], end = text.indexOf('"' + hashes, i + 2 + hashes.length)
      const stop = end < 0 ? n : end + 1 + hashes.length
      for (; i < stop; i++) if (text[i] === '\n') line++
      return true
    }
    if (c === '"') {
      i++
      while (i < n && text[i] !== '"') { if (text[i] === '\n') line++; i += text[i] === '\\' ? 2 : 1 }
      i++; return true
    }
    if (c === "'") {
      const m = /^'(\\.|[^\\'\n])'/.exec(text.slice(i, i + 12)) ?? /^'\\u\{[0-9a-fA-F]+\}'/.exec(text.slice(i, i + 14))
      i += m ? m[0].length : 1; return true
    }
    return false
  }
  const open = [] // stack of {name, line, depth}
  let depth = 0, pending = null
  while (i < n) {
    if (skip()) continue
    const c = text[i]
    if (c === '{') {
      depth++
      if (pending) { open.push({...pending, depth}); pending = null }
      i++; continue
    }
    if (c === '}') {
      if (open.length && open.at(-1).depth === depth) {
        const f = open.pop(); out.push({name: f.name, line: f.line, lines: line - f.line + 1})
      }
      depth--; i++; continue
    }
    if (c === ';' && pending) { pending = null; i++; continue } // trait method or extern declaration
    if (c === 'f' && text[i + 1] === 'n' && !/\w/.test(text[i - 1] ?? '')) {
      const m = /^fn\s+(r#)?([A-Za-z_]\w*)/.exec(text.slice(i, i + 80))
      if (m) { pending = {name: m[2], line}; i += m[0].length; continue }
    }
    i++
  }
  return out
}

let ts
function typescriptFunctions(path, text) {
  ts ??= createRequire(import.meta.url)('typescript')
  if (path.endsWith('.svt')) {
    const m = /<script[^>]*>([\s\S]*?)<\/script>/.exec(text)
    text = m ? '\n'.repeat(text.slice(0, m.index).split('\n').length - 1) + m[1] : ''
  }
  const sf = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true)
  const out = [], K = ts.SyntaxKind
  const lineOf = pos => sf.getLineAndCharacterOfPosition(pos).line + 1
  const visit = node => {
    const fn = [K.FunctionDeclaration, K.MethodDeclaration, K.Constructor, K.GetAccessor, K.SetAccessor, K.FunctionExpression, K.ArrowFunction].includes(node.kind)
    if (fn && node.body) {
      const p = node.parent
      const name = node.kind === K.Constructor ? 'constructor'
        : node.name?.getText(sf) ?? (p && (p.kind === K.VariableDeclaration || p.kind === K.PropertyAssignment || p.kind === K.PropertyDeclaration) ? p.name.getText(sf) : '(anonymous)')
      const start = lineOf(node.getStart(sf))
      out.push({name, line: start, lines: lineOf(node.end) - start + 1})
    }
    ts.forEachChild(node, visit)
  }
  visit(sf)
  return out
}

const EXCLUDE = [/^crates\/vendor\//, /\/generated\//, /\.d\.ts$/, /(^|\/)tests?\//, /\.test\.[cm]?ts$/, /(^|\/)examples\//, /(^|\/)benches\//]

/** {key: lines} for production functions longer than FUNCTION_LINE_LIMIT. */
export function measureFunctions(root, files, frozen = {}) {
  const sizes = {}
  for (const path of files) {
    const ext = extname(path)
    if (!['.rs', '.ts', '.mts', '.svt'].includes(ext) || EXCLUDE.some(p => p.test(path)) || frozen[path] !== undefined) continue
    if (path.startsWith('scripts/')) continue // historical executors and one-off tooling
    const full = resolve(root, path)
    if (!existsSync(full)) continue
    const text = readFileSync(full, 'utf8')
    const fns = ext === '.rs' ? rustFunctions(text) : typescriptFunctions(path, text)
    const seen = {}
    for (const f of fns) {
      const ordinal = seen[f.name] = (seen[f.name] ?? 0) + 1
      if (f.lines > FUNCTION_LINE_LIMIT) sizes[`${path}::${f.name}@${ordinal}`] = f.lines
    }
  }
  return sizes
}
