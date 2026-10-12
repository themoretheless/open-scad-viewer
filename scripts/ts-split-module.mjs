#!/usr/bin/env node
// Split one oversized TypeScript module by moving its top-level function declarations into
// sibling submodules, without changing any importer.
//
// usage: node scripts/ts-split-module.mjs <src/path/file.ts> [--target LINES]
//
// Only `function` declarations move (with their JSDoc and overload signatures): they are
// hoisted and evaluate nothing at module load, so the import cycle between the module and
// its submodules cannot reorder any initialisation. Types, classes, constants and module
// state stay in the original file, which keeps its path and re-exports every moved export.
// Each submodule imports exactly the top-level names its functions mention, from the
// original module, a sibling submodule or the original external import.
import ts from 'typescript'
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs'
import { basename, dirname, join, relative } from 'node:path'

const args = process.argv.slice(2)
const targetIndex = args.indexOf('--target')
const target = targetIndex >= 0 ? Number(args.splice(targetIndex, 2)[1]) : 380
const file = args[0]
if (!file?.endsWith('.ts') || file.endsWith('.d.ts')) throw new Error('usage: ts-split-module.mjs <file.ts>')
const text = readFileSync(file, 'utf8')
const source = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)
const stem = file.slice(0, -3)
const base = basename(stem)
if (existsSync(stem)) throw new Error(`${stem} already exists`)

const isExported = node => node.modifiers?.some(m => m.kind === ts.SyntaxKind.ExportKeyword) ?? false
const isDefault = node => node.modifiers?.some(m => m.kind === ts.SyntaxKind.DefaultKeyword) ?? false

// Top-level names: imports (with their module and kind) and local declarations.
const imported = new Map() // name -> {module, imported, typeOnly}
const mutable = new Set() // top-level let/var bindings: only their own module may assign them
const declared = new Map() // name -> {kind, exported}
for (const statement of source.statements) {
  if (ts.isImportDeclaration(statement)) {
    const module = statement.moduleSpecifier.text
    const clause = statement.importClause
    if (!clause) continue
    const typeOnly = clause.isTypeOnly
    if (clause.name) imported.set(clause.name.text, { module, imported: 'default', typeOnly })
    const bindings = clause.namedBindings
    if (bindings && ts.isNamespaceImport(bindings)) imported.set(bindings.name.text, { module, imported: '*', typeOnly })
    if (bindings && ts.isNamedImports(bindings)) {
      for (const el of bindings.elements) {
        imported.set(el.name.text, { module, imported: (el.propertyName ?? el.name).text, typeOnly: typeOnly || el.isTypeOnly })
      }
    }
    continue
  }
  const exported = isExported(statement)
  if (ts.isFunctionDeclaration(statement) && statement.name) declared.set(statement.name.text, { kind: 'function', exported })
  else if ((ts.isClassDeclaration(statement) || ts.isEnumDeclaration(statement)) && statement.name) declared.set(statement.name.text, { kind: 'value', exported })
  else if ((ts.isInterfaceDeclaration(statement) || ts.isTypeAliasDeclaration(statement)) && statement.name) declared.set(statement.name.text, { kind: 'type', exported })
  else if (ts.isVariableStatement(statement)) {
    const isConst = (statement.declarationList.flags & ts.NodeFlags.Const) !== 0
    for (const d of statement.declarationList.declarations) {
      if (!isConst && ts.isIdentifier(d.name)) mutable.add(d.name.text)
      const visit = name => {
        if (ts.isIdentifier(name)) declared.set(name.text, { kind: 'value', exported })
        else name.elements?.forEach(e => e.name && visit(e.name))
      }
      visit(d.name)
    }
  } else if (ts.isModuleDeclaration(statement) && statement.name && ts.isIdentifier(statement.name)) {
    declared.set(statement.name.text, { kind: 'value', exported })
  }
}
if ([...source.statements].some(s => ts.isExportAssignment(s) || (isDefault(s)))) throw new Error('default exports are not supported')

function assignsModuleState(nodes) {
  let hit = false
  const assignOps = new Set([ts.SyntaxKind.EqualsToken, ts.SyntaxKind.PlusEqualsToken, ts.SyntaxKind.MinusEqualsToken,
    ts.SyntaxKind.AsteriskEqualsToken, ts.SyntaxKind.SlashEqualsToken, ts.SyntaxKind.BarBarEqualsToken,
    ts.SyntaxKind.AmpersandAmpersandEqualsToken, ts.SyntaxKind.QuestionQuestionEqualsToken, ts.SyntaxKind.BarEqualsToken,
    ts.SyntaxKind.AmpersandEqualsToken, ts.SyntaxKind.PercentEqualsToken])
  const visit = n => {
    if (hit) return
    if (ts.isBinaryExpression(n) && assignOps.has(n.operatorToken.kind) && ts.isIdentifier(n.left) && mutable.has(n.left.text)) hit = true
    if ((ts.isPrefixUnaryExpression(n) || ts.isPostfixUnaryExpression(n)) && ts.isIdentifier(n.operand) && mutable.has(n.operand.text)) hit = true
    ts.forEachChild(n, visit)
  }
  nodes.forEach(visit)
  return hit
}

// Movable units: a function declaration with its overloads, plus its leading trivia.
const statements = [...source.statements]
const units = []
for (let index = 0; index < statements.length; index++) {
  const s = statements[index]
  if (!ts.isFunctionDeclaration(s) || !s.name) continue
  let end = index
  while (s.body === undefined && end + 1 < statements.length) {
    end++
    const next = statements[end]
    if (!ts.isFunctionDeclaration(next) || next.name?.text !== s.name.text) throw new Error(`broken overload ${s.name.text}`)
    if (next.body) break
  }
  if (!assignsModuleState(statements.slice(index, end + 1))) units.push({ name: s.name.text, first: index, last: end, exported: isExported(s) })
  index = end
}
const unitText = u => text.slice(statements[u.first].getFullStart(), statements[u.last].getEnd()).replace(/^\n+/, '')
const unitLines = u => unitText(u).split('\n').length

// References: identifiers in expression/type positions that name a top-level binding.
function references(node, out) {
  const visit = n => {
    if (ts.isIdentifier(n)) {
      const p = n.parent
      const isName = p && ((ts.isPropertyAccessExpression(p) && p.name === n)
        || (ts.isPropertyAssignment(p) && p.name === n)
        || (ts.isPropertySignature(p) && p.name === n)
        || (ts.isPropertyDeclaration(p) && p.name === n)
        || (ts.isMethodDeclaration(p) && p.name === n)
        || (ts.isMethodSignature(p) && p.name === n)
        || (ts.isQualifiedName(p) && p.right === n)
        || (ts.isBindingElement(p) && p.propertyName === n)
        || (ts.isEnumMember(p) && p.name === n)
        || (ts.isGetAccessor(p) && p.name === n) || (ts.isSetAccessor(p) && p.name === n)
        || (ts.isLabeledStatement(p) && p.label === n)
        || ((ts.isBreakStatement(p) || ts.isContinueStatement(p)) && p.label === n))
      if (!isName) out.add(n.text)
      if (ts.isShorthandPropertyAssignment(p) && p.name === n) out.add(n.text)
    }
    ts.forEachChild(n, visit)
  }
  visit(node)
  return out
}

// Group consecutive units into chunks of about `target` lines.
const chunks = []
let current = []
let size = 0
for (const u of units) {
  const lines = unitLines(u)
  if (current.length && size + lines > target) { chunks.push(current); current = []; size = 0 }
  current.push(u); size += lines
}
if (current.length) chunks.push(current)
const usedNames = new Set()
const chunkName = chunk => {
  const pick = chunk.find(u => u.exported) ?? chunk[0]
  let name = pick.name
  let n = 2
  while (usedNames.has(name)) name = `${pick.name}${n++}`
  usedNames.add(name)
  return name
}
const owner = new Map() // function name -> chunk file stem
const chunkFiles = chunks.map(chunk => {
  const name = chunkName(chunk)
  chunk.forEach(u => owner.set(u.name, name))
  return { name, chunk }
})

const rel = (from, to) => {
  let r = relative(dirname(from), to)
  if (!r.startsWith('.')) r = './' + r
  return r
}
const importLines = (fromFile, refs, selfNames) => {
  const byModule = new Map()
  const add = (module, spec, typeOnly) => {
    if (!byModule.has(module)) byModule.set(module, { value: new Set(), type: new Set() })
    byModule.get(module)[typeOnly ? 'type' : 'value'].add(spec)
  }
  for (const name of [...refs].sort()) {
    if (selfNames.has(name)) continue
    if (owner.has(name)) {
      add(rel(fromFile, join(stem, owner.get(name))), name, false)
    } else if (declared.has(name)) {
      const d = declared.get(name)
      add(`../${base}`, name, d.kind === 'type')
    } else if (imported.has(name)) {
      const i = imported.get(name)
      const module = i.module.startsWith('.') ? rel(fromFile, join(dirname(file), i.module)) : i.module
      if (i.imported === '*') { add(module, `* as ${name}`, i.typeOnly); continue }
      if (i.imported === 'default') { add(module, `default as ${name}`, i.typeOnly); continue }
      add(module, i.imported === name ? name : `${i.imported} as ${name}`, i.typeOnly)
    }
  }
  const lines = []
  for (const [module, { value, type }] of byModule) {
    const star = [...value].find(s => s.startsWith('* as '))
    if (star) lines.push(`import ${star} from '${module}'`)
    const named = [...value].filter(s => !s.startsWith('* as '))
    if (named.length) lines.push(`import { ${named.join(', ')} } from '${module}'`)
    const starType = [...type].find(s => s.startsWith('* as '))
    if (starType) lines.push(`import type ${starType} from '${module}'`)
    const namedType = [...type].filter(s => !s.startsWith('* as '))
    if (namedType.length) lines.push(`import type { ${namedType.join(', ')} } from '${module}'`)
  }
  return lines
}

mkdirSync(stem, { recursive: true })
// The original module needs the declared names of the chunks it still references.
for (const { name, chunk } of chunkFiles) {
  const path = join(stem, `${name}.ts`)
  const refs = new Set()
  for (const u of chunk) for (let i = u.first; i <= u.last; i++) references(statements[i], refs)
  const self = new Set(chunk.map(u => u.name))
  const body = chunk.map(u => {
    const t = unitText(u).replace(/((?:\bimport|\bnew URL)\(\s*['"`])(\.{1,2}\/)/g, (_, head, dots) => head + (dots === './' ? '../' : '../../'))
    return u.exported ? t : t.replace(/^(async )?function\b/gm, 'export $1function')
  }).join('\n\n')
  const header = importLines(path, refs, self)
  writeFileSync(path, (header.length ? header.join('\n') + '\n\n' : '') + body.trim() + '\n')
}

// Declarations that stay in the original module but are referenced by a submodule must be exported.
const neededFromMain = new Set()
for (const { chunk } of chunkFiles) {
  const refs = new Set()
  for (const u of chunk) for (let i = u.first; i <= u.last; i++) references(statements[i], refs)
  for (const n of refs) if (declared.has(n) && !owner.has(n) && !declared.get(n).exported) neededFromMain.add(n)
}
const exportEdits = []
for (const st of statements) {
  if (ts.isImportDeclaration(st) || isExported(st)) continue
  const names = []
  if ((ts.isFunctionDeclaration(st) || ts.isClassDeclaration(st) || ts.isEnumDeclaration(st) || ts.isInterfaceDeclaration(st) || ts.isTypeAliasDeclaration(st)) && st.name) names.push(st.name.text)
  if (ts.isVariableStatement(st)) {
    const collect = name => {
      if (ts.isIdentifier(name)) names.push(name.text)
      else name.elements?.forEach(e => e.name && collect(e.name))
    }
    st.declarationList.declarations.forEach(d => collect(d.name))
  }
  if (names.some(n => neededFromMain.has(n))) exportEdits.push(st.getStart())
}

// Rewrite the original module: drop moved units, import what it still uses, re-export moved exports.
const moved = new Set(units.flatMap(u => {
  const out = []
  for (let i = u.first; i <= u.last; i++) out.push(i)
  return out
}))
const kept = statements.filter((_, i) => !moved.has(i))
let output = ''
const remainingRefs = new Set()
for (const s of kept) if (!ts.isImportDeclaration(s)) references(s, remainingRefs)
const needed = [...remainingRefs].filter(n => owner.has(n))
const byChunk = new Map()
for (const n of needed) {
  const c = owner.get(n)
  if (!byChunk.has(c)) byChunk.set(c, [])
  byChunk.get(c).push(n)
}
const exportsByChunk = new Map()
for (const u of units) if (u.exported) {
  const c = owner.get(u.name)
  if (!exportsByChunk.has(c)) exportsByChunk.set(c, [])
  exportsByChunk.get(c).push(u.name)
}
const lastImport = kept.reduce((at, s, i) => (ts.isImportDeclaration(s) ? i : at), -1)
const extra = []
for (const { name } of chunkFiles) {
  const spec = `./${base}/${name}`
  if (byChunk.has(name)) extra.push(`import { ${byChunk.get(name).sort().join(', ')} } from '${spec}'`)
}
for (const { name } of chunkFiles) {
  const spec = `./${base}/${name}`
  if (exportsByChunk.has(name)) extra.push(`export { ${exportsByChunk.get(name).join(', ')} } from '${spec}'`)
}
const keptPieces = []
statements.forEach((s, i) => {
  if (moved.has(i)) return
  keptPieces.push({ s, text: text.slice(s.getFullStart(), s.getEnd()) })
})
let insertAt = keptPieces.findIndex(p => p.s === kept[lastImport])
output = text.slice(0, statements[0]?.getFullStart() ?? 0)
keptPieces.forEach((p, i) => {
  let piece = p.text
  if (exportEdits.includes(p.s.getStart())) {
    const offset = p.s.getStart() - p.s.getFullStart()
    piece = piece.slice(0, offset) + 'export ' + piece.slice(offset)
  }
  output += piece
  if (i === insertAt) output += '\n' + extra.join('\n')
})
if (insertAt < 0) output = extra.join('\n') + '\n' + output
output = output.replace(/\n{3,}/g, '\n\n').replace(/^\n+/, '')
writeFileSync(file, output.trimEnd() + '\n')
console.log(JSON.stringify({
  file, chunks: chunkFiles.map(c => ({ name: c.name, functions: c.chunk.length })),
  originalLines: text.split('\n').length, remainingLines: output.split('\n').length,
}))
