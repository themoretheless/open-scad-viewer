#!/usr/bin/env node
// Move the pure helpers of a `<script setup>` block into a sibling TypeScript module.
//
// usage: node scripts/vue-extract-pure.mjs <Component.vue> <out.ts>
//
// A top-level function is pure when every top-level name it references is an import, a
// type, a constant with a literal-only initializer, or another pure function. Pure functions
// (and the literal constants they need) move verbatim, exported, into <out.ts>, which copies
// exactly the import specifiers it uses; the component imports them back by name. Functions
// that read component state (refs, computeds, props, emits, other locals) stay put.
import ts from 'typescript'
import { readFileSync, writeFileSync } from 'node:fs'
import { basename, dirname, relative } from 'node:path'

const [vuePath, outPath] = process.argv.slice(2)
if (!vuePath || !outPath) throw new Error('usage: vue-extract-pure.mjs <Component.vue> <out.ts>')
const vue = readFileSync(vuePath, 'utf8')
const open = vue.match(/<script setup lang="ts">\n/)
const start = open.index + open[0].length
const end = vue.indexOf('</script>', start)
const script = vue.slice(start, end)
const source = ts.createSourceFile('script.ts', script, ts.ScriptTarget.Latest, true)

const imports = new Map() // local name -> {decl, spec text, isType}
const decls = new Map() // name -> {node, kind}
for (const s of source.statements) {
  if (ts.isImportDeclaration(s)) {
    const clause = s.importClause
    if (!clause) continue
    if (clause.name) imports.set(clause.name.text, { decl: s, kind: 'default' })
    const nb = clause.namedBindings
    if (nb && ts.isNamespaceImport(nb)) imports.set(nb.name.text, { decl: s, kind: 'namespace' })
    if (nb && ts.isNamedImports(nb)) for (const e of nb.elements) imports.set(e.name.text, { decl: s, kind: 'named', element: e })
  } else if (ts.isFunctionDeclaration(s) && s.name) {
    decls.set(s.name.text, { node: s, kind: 'function' })
  } else if (ts.isVariableStatement(s)) {
    for (const d of s.declarationList.declarations) {
      if (ts.isIdentifier(d.name)) decls.set(d.name.text, { node: s, kind: 'variable', decl: d, single: s.declarationList.declarations.length === 1 })
      else for (const el of d.name.elements ?? []) if (ts.isBindingElement(el) && ts.isIdentifier(el.name)) decls.set(el.name.text, { node: s, kind: 'state' })
    }
  } else if (ts.isTypeAliasDeclaration(s) || ts.isInterfaceDeclaration(s) || ts.isEnumDeclaration(s)) {
    decls.set(s.name.text, { node: s, kind: 'type' })
  } else if (ts.isClassDeclaration(s) && s.name) {
    decls.set(s.name.text, { node: s, kind: 'state' })
  }
}

// Free identifiers of a node that name top-level declarations or imports.
function references(node) {
  const out = new Set()
  const visit = n => {
    if (ts.isIdentifier(n)) {
      const p = n.parent
      const isPropName = (ts.isPropertyAccessExpression(p) && p.name === n) || (ts.isPropertyAssignment(p) && p.name === n) ||
        (ts.isMethodDeclaration(p) && p.name === n) || (ts.isPropertySignature(p) && p.name === n) ||
        (ts.isQualifiedName(p) && p.right === n) || (ts.isPropertyDeclaration(p) && p.name === n)
      if (!isPropName && (decls.has(n.text) || imports.has(n.text))) out.add(n.text)
      if (ts.isShorthandPropertyAssignment(p) && (decls.has(n.text) || imports.has(n.text))) out.add(n.text)
    }
    ts.forEachChild(n, visit)
  }
  visit(node)
  return out
}

const LITERAL = new Set([ts.SyntaxKind.NumericLiteral, ts.SyntaxKind.StringLiteral, ts.SyntaxKind.TrueKeyword,
  ts.SyntaxKind.FalseKeyword, ts.SyntaxKind.NoSubstitutionTemplateLiteral, ts.SyntaxKind.NullKeyword])
function literalOnly(expr) {
  let ok = true
  const visit = n => {
    if (!ok) return
    if (ts.isCallExpression(n) || ts.isNewExpression(n) || ts.isArrowFunction(n) || ts.isFunctionExpression(n)) {
      // Object.freeze / new Set / new Map over literals are still constant data.
      const callee = ts.isCallExpression(n) || ts.isNewExpression(n) ? n.expression.getText(source) : ''
      if (!['Object.freeze', 'Set', 'Map'].includes(callee)) { ok = false; return }
    }
    if (ts.isIdentifier(n) && !ts.isPropertyAccessExpression(n.parent) && decls.has(n.text) && decls.get(n.text).kind !== 'type' && !pure.has(n.text)) ok = false
    if (ts.isIdentifier(n) && imports.has(n.text) && !ts.isTypeReferenceNode(n.parent)) ok = false
    ts.forEachChild(n, visit)
  }
  visit(expr)
  return ok
}

// Fixed point: start from everything, drop what references state.
const pure = new Set()
for (const [name, d] of decls) {
  if (d.kind === 'function' && !d.node.modifiers?.some(m => m.kind === ts.SyntaxKind.AsyncKeyword && false)) pure.add(name)
  if (d.kind === 'type') pure.add(name)
  if (d.kind === 'variable' && d.single && ts.isVariableDeclarationList(d.node.declarationList) &&
      (d.node.declarationList.flags & ts.NodeFlags.Const) && d.decl.initializer) pure.add(name)
}
for (let changed = true; changed;) {
  changed = false
  for (const name of [...pure]) {
    const d = decls.get(name)
    let ok
    if (d.kind === 'variable') ok = literalOnly(d.decl.initializer) && [...references(d.decl)].every(r => imports.has(r) || pure.has(r) || r === name)
    else ok = [...references(d.node)].every(r => imports.has(r) || pure.has(r) || r === name)
    if (ok && d.kind === 'function') {
      // Vue macros and lifecycle hooks are never pure helpers.
      const text = d.node.getText(source)
      if (/\b(defineProps|defineEmits|onMounted|onBeforeUnmount|nextTick|watch|ref|computed|reactive)\(/.test(text)) ok = false
    }
    if (!ok) { pure.delete(name); changed = true }
  }
}
// Types move only when a moving function needs them and they are themselves pure; keep the rest.
const moving = new Set([...pure].filter(n => decls.get(n).kind !== 'type'))
const neededTypes = new Set()
for (const n of moving) for (const r of references(decls.get(n).node)) if (decls.get(r)?.kind === 'type') neededTypes.add(r)
// A type stays in the component if the component (outside moving code) uses it; then the helper imports it... avoid cycles:
// only move a type when every type it references is also movable.
for (const t of neededTypes) if (pure.has(t)) moving.add(t)
for (let changed = true; changed;) {
  changed = false
  for (const n of [...moving]) for (const r of references(decls.get(n).node)) {
    if (decls.has(r) && !moving.has(r)) { moving.delete(n); changed = true; break }
  }
}
if (!moving.size) { console.log(JSON.stringify({ moved: 0 })); process.exit(0) }

// Build the helper module.
const usedImports = new Map()
for (const n of moving) for (const r of references(decls.get(n).node)) if (imports.has(r)) usedImports.set(r, imports.get(r))
const relOut = './' + relative(dirname(vuePath), outPath).replace(/\.ts$/, '')
const rebase = spec => spec.startsWith('.') ? './' + relative(dirname(outPath), `${dirname(vuePath)}/${spec}`).replace(/^(?!\.)/, '') : spec
const byModule = new Map()
for (const [name, info] of usedImports) {
  const spec = rebase(info.decl.moduleSpecifier.text)
  const isTypeOnly = info.decl.importClause.isTypeOnly || info.element?.isTypeOnly
  if (!byModule.has(spec)) byModule.set(spec, [])
  if (info.kind === 'named') {
    const el = info.element
    const text = (el.propertyName ? `${el.propertyName.text} as ` : '') + el.name.text
    byModule.get(spec).push(isTypeOnly ? `type ${text}` : text)
  } else if (info.kind === 'default') byModule.get(spec).push(`default as ${name}`)
  else byModule.get(spec).push(`* as ${name}`)
}
let header = ''
for (const [spec, names] of byModule) {
  const ns = names.find(n => n.startsWith('* as '))
  if (ns) header += `import ${ns} from '${spec.replace(/^\.\/\.\.\//, '../')}'\n`
  const rest = names.filter(n => !n.startsWith('* as '))
  if (rest.length) header += `import { ${rest.join(', ')} } from '${spec.replace(/^\.\/\.\.\//, '../')}'\n`
}
const ordered = source.statements.filter(s => [...moving].some(n => decls.get(n).node === s))
const body = ordered.map(s => {
  const full = s.getFullText(source).replace(/^\n+/, '')
  return /^\s*(\/\/[^\n]*\n|\/\*[\s\S]*?\*\/\s*)*export\b/.test(full) ? full : full.replace(/^((?:\s*(?:\/\/[^\n]*|\/\*[\s\S]*?\*\/)\s*\n)*\s*)/, '$1export ')
})
writeFileSync(outPath, `// Pure helpers of ${basename(vuePath)}: no component state, extracted by scripts/vue-extract-pure.mjs.\n${header}\n${body.join('\n\n')}\n`)

// Rewrite the component: drop moved statements, import them back.
let rest = script
for (const s of [...ordered].reverse()) rest = rest.slice(0, s.getFullStart()) + rest.slice(s.getEnd())
const exportedTypes = [...moving].filter(n => decls.get(n).kind === 'type')
const exportedValues = [...moving].filter(n => decls.get(n).kind !== 'type')
const lastImport = [...source.statements].filter(ts.isImportDeclaration).pop()
const back = `\nimport { ${exportedValues.join(', ')} } from '${relOut}'` +
  (exportedTypes.length ? `\nimport type { ${exportedTypes.join(', ')} } from '${relOut}'` : '')
const cut = lastImport.getEnd()
rest = rest.slice(0, cut) + back + rest.slice(cut)
writeFileSync(vuePath, vue.slice(0, start) + rest + vue.slice(end))
console.log(JSON.stringify({ moved: moving.size, values: exportedValues.length, types: exportedTypes.length }))
