// Which `.rs` files under crates/ the compiler can reach. A file nobody reaches through
// `mod`, `#[path]` or `include!` is never compiled: it is either a stale copy or a lost test.
// Lexical, not a full parser: comments, strings and char literals are skipped, inline
// `mod name { ... }` blocks are tracked so nested `mod` declarations resolve like rustc's.
import {readFileSync, existsSync, readdirSync} from 'node:fs'
import {resolve, dirname, basename, join, relative, sep} from 'node:path'

/** Declarations in one file: [{name, path, inline: [outer inline module names]}] and include!() targets. */
export function scanModules(text) {
  const mods = [], includes = [], stack = []
  let depth = 0, pendingPath = null, i = 0
  const n = text.length
  while (i < n) {
    const c = text[i]
    if (c === '/' && text[i + 1] === '/') { while (i < n && text[i] !== '\n') i++; continue }
    if (c === '/' && text[i + 1] === '*') {
      let level = 1; i += 2
      while (i < n && level) {
        if (text[i] === '/' && text[i + 1] === '*') { level++; i += 2 }
        else if (text[i] === '*' && text[i + 1] === '/') { level--; i += 2 }
        else i++
      }
      continue
    }
    if (c === 'r' && /^r#*"/.test(text.slice(i, i + 8)) && !/[\w]/.test(text[i - 1] ?? '')) {
      const hashes = /^r(#*)"/.exec(text.slice(i))[1]
      const end = text.indexOf('"' + hashes, i + 2 + hashes.length)
      i = end < 0 ? n : end + 1 + hashes.length; continue
    }
    if (c === '"') {
      i++
      while (i < n && text[i] !== '"') i += text[i] === '\\' ? 2 : 1
      i++; continue
    }
    if (c === "'") {
      const m = /^'(\\.|[^\\'\n])'/.exec(text.slice(i, i + 12)) ?? /^'\\u\{[0-9a-fA-F]+\}'/.exec(text.slice(i, i + 14))
      if (m) { i += m[0].length; continue }
      i++; continue
    }
    if (c === '{') { depth++; i++; continue }
    if (c === '}') {
      depth--
      if (stack.length && stack.at(-1).depth === depth) stack.pop()
      i++; continue
    }
    if (c === '#' && text[i + 1] === '[') {
      const m = /^#\[\s*path\s*=\s*"([^"]+)"\s*\]/.exec(text.slice(i, i + 400))
      if (m) { pendingPath = m[1]; i += m[0].length; continue }
      i += 2; continue
    }
    if (/[A-Za-z_]/.test(c) && !/[\w]/.test(text[i - 1] ?? '')) {
      const word = /^[A-Za-z_][\w]*/.exec(text.slice(i, i + 64))[0]
      if (word === 'mod') {
        const m = /^mod\s+(r#)?([A-Za-z_][\w]*)\s*([;{])/.exec(text.slice(i, i + 200))
        if (m) {
          if (m[3] === ';') mods.push({name: m[2], path: pendingPath, inline: stack.map(s => s.name)})
          else { stack.push({name: m[2], depth, path: pendingPath}); depth++ }
          pendingPath = null; i += m[0].length; continue
        }
      } else if (word === 'include' || word === 'include_str') {
        const m = /^include!\s*\(\s*"([^"]+\.rs)"\s*\)/.exec(text.slice(i, i + 400))
        if (m) includes.push(m[1])
      } else if (word === 'pub') {
        // Visibility such as `pub(in crate::a::b)` sits between the attribute and `mod`.
        const m = /^pub\s*\([^)]*\)/.exec(text.slice(i, i + 300))
        if (m) { i += m[0].length; continue }
      } else if (!['unsafe', 'extern'].includes(word)) pendingPath = null
      i += word.length; continue
    }
    i++
  }
  return {mods, includes}
}

function targetRoots(crateDir) {
  const roots = []
  const add = p => { if (existsSync(p)) roots.push(p) }
  add(join(crateDir, 'src/lib.rs')); add(join(crateDir, 'src/main.rs')); add(join(crateDir, 'build.rs'))
  for (const dir of ['src/bin', 'tests', 'examples', 'benches']) {
    const full = join(crateDir, dir)
    if (!existsSync(full)) continue
    for (const e of readdirSync(full, {withFileTypes: true})) {
      if (e.isFile() && e.name.endsWith('.rs')) roots.push(join(full, e.name))
      else if (e.isDirectory()) add(join(full, e.name, 'main.rs'))
    }
  }
  const manifest = readFileSync(join(crateDir, 'Cargo.toml'), 'utf8')
  for (const m of manifest.matchAll(/^\[\[(?:bin|test|example|bench)\]\][^[]*?^path\s*=\s*"([^"]+)"/gms)) add(resolve(crateDir, m[1]))
  const lib = /^\[lib\][^[]*?^path\s*=\s*"([^"]+)"/ms.exec(manifest)
  if (lib) add(resolve(crateDir, lib[1]))
  return roots
}

/** Absolute paths of every `.rs` file reachable from a target root of a Cargo package below `crates/`. */
export function reachableRustFiles(root) {
  const seen = new Set()
  const crates = resolve(root, 'crates')
  for (const name of readdirSync(crates)) {
    const crateDir = join(crates, name)
    if (name === 'target' || name === 'vendor' || !existsSync(join(crateDir, 'Cargo.toml'))) continue
    const queue = targetRoots(crateDir).map(file => ({file, modDir: dirname(file)}))
    while (queue.length) {
      const {file, modDir} = queue.shift()
      if (seen.has(file)) continue
      seen.add(file)
      const {mods, includes} = scanModules(readFileSync(file, 'utf8'))
      const fileDir = dirname(file)
      for (const m of mods) {
        let child, childModDir
        if (m.path) {
          // rustc: #[path] is relative to the file's directory, or to the inline module's directory.
          const base = m.inline.length ? join(modDir, ...m.inline) : fileDir
          child = resolve(base, m.path); childModDir = dirname(child)
        } else {
          const base = join(modDir, ...m.inline)
          const flat = join(base, m.name + '.rs'), nested = join(base, m.name, 'mod.rs')
          child = existsSync(flat) ? flat : nested; childModDir = join(base, m.name)
        }
        if (existsSync(child)) queue.push({file: child, modDir: childModDir})
      }
      for (const inc of includes) {
        const target = resolve(fileDir, inc)
        if (existsSync(target)) queue.push({file: target, modDir: dirname(target)})
      }
    }
  }
  return seen
}

/** Rust module directory for children of `file`: its own dir for roots and mod.rs, else dir/<stem>. */
export function moduleDirOf(file, isRoot) {
  return isRoot || basename(file) === 'mod.rs' ? dirname(file) : join(dirname(file), basename(file, '.rs'))
}

/** Tracked `.rs` files under a workspace crate's src/, tests/, examples/ or benches/ that no target compiles. */
export function unreachableRustFiles(root, trackedFiles) {
  const reachable = reachableRustFiles(root)
  const packages = new Set(readdirSync(resolve(root, 'crates')).filter(n => existsSync(resolve(root, 'crates', n, 'Cargo.toml'))))
  return trackedFiles.filter(path => {
    const parts = path.split('/')
    return parts[0] === 'crates' && path.endsWith('.rs') && packages.has(parts[1]) && parts[1] !== 'vendor'
      && ['src', 'tests', 'examples', 'benches'].includes(parts[2]) && !reachable.has(resolve(root, path))
  }).map(p => p.split(sep).join('/')).sort()
}
export const _relative = relative
