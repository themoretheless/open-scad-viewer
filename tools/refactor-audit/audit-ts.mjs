// TypeScript / Svet audit: per-function metrics, import graph, type-safety escapes. Read-only.
// usage: node audit-ts.mjs <repo_root> <out.json> <dir>...
import {createRequire} from 'node:module'
import {readFileSync, readdirSync, statSync, writeFileSync, existsSync} from 'node:fs'
import {join, relative, resolve, dirname, extname} from 'node:path'
const [root, outPath, ...dirs] = process.argv.slice(2)
const ts = createRequire(join(root, 'package.json'))('typescript')
const SKIP = new Set(['node_modules', 'generated', 'dist', '.git', 'target', 'fixtures'])
const EXT = new Set(['.ts', '.mts', '.mjs', '.js', '.svt', '.tsx'])
const files = []
const walk = d => { for (const e of readdirSync(d, {withFileTypes: true})) {
  const p = join(d, e.name)
  if (e.isDirectory()) { if (!SKIP.has(e.name)) walk(p) }
  else if (EXT.has(extname(e.name)) && !e.name.endsWith('.d.ts')) files.push(p) } }
for (const d of dirs) if (existsSync(join(root, d))) walk(join(root, d))
const K = ts.SyntaxKind
const fnRows = [], fileRows = [], edges = []
const resolveImport = (from, spec) => {
  if (!spec.startsWith('.')) return null
  const clean = spec.replace(/\?.*$/, '')
  const base = resolve(dirname(from), clean)
  for (const c of [base, base + '.ts', base + '.mts', base + '.mjs', base + '.js', base + '.svt', join(base, 'index.ts'), base.replace(/\.js$/, '.ts'), base.replace(/\.mjs$/, '.mts')])
    if (existsSync(c) && statSync(c).isFile()) return c
  return null
}
for (const file of files) {
  const rel = relative(root, file)
  let text = readFileSync(file, 'utf8'); const raw = text
  let templateLines = 0
  if (file.endsWith('.svt')) {
    const m = /<script[^>]*>([\s\S]*?)<\/script>/.exec(text)
    const before = m ? text.slice(0, m.index).split('\n').length - 1 : 0
    templateLines = raw.split('\n').length - (m ? m[1].split('\n').length : 0)
    text = m ? '\n'.repeat(before) + m[1] : ''
  }
  const sf = ts.createSourceFile(rel, text, ts.ScriptTarget.Latest, true, file.endsWith('.mjs') || file.endsWith('.js') ? ts.ScriptKind.JS : ts.ScriptKind.TS)
  const lineOf = pos => sf.getLineAndCharacterOfPosition(pos).line + 1
  const rec = {file: rel, loc: raw.split('\n').length - (raw.endsWith('\n') ? 1 : 0), templateLines, fns: 0, any: 0, asCasts: 0, nonNull: 0, exports: 0, imports: 0, classes: 0, suppress: (raw.match(/@ts-ignore|@ts-expect-error|@ts-nocheck|eslint-disable/g) || []).length, todo: (raw.match(/\b(TODO|FIXME|HACK|XXX)\b/g) || []).length}
  const fnMetrics = body => {
    const m = {cyclo: 1, cognitive: 0, maxNest: 0}
    const visit = (n, nest) => {
      let inc = 0, child = nest
      switch (n.kind) {
        case K.FunctionDeclaration: case K.FunctionExpression: case K.ArrowFunction: case K.MethodDeclaration: case K.Constructor: case K.GetAccessor: case K.SetAccessor:
          if (n !== body) { child = nest + 1; m.maxNest = Math.max(m.maxNest, child) } break
        case K.IfStatement: {
          const elseIf = n.parent && n.parent.kind === K.IfStatement && n.parent.elseStatement === n
          m.cyclo++; m.cognitive += elseIf ? 1 : 1 + nest; child = elseIf ? nest : nest + 1
          if (n.elseStatement && n.elseStatement.kind !== K.IfStatement) m.cognitive++
          break }
        case K.ForStatement: case K.ForInStatement: case K.ForOfStatement: case K.WhileStatement: case K.DoStatement: case K.CatchClause:
          m.cyclo++; m.cognitive += 1 + nest; child = nest + 1; break
        case K.SwitchStatement: m.cognitive += 1 + nest; child = nest + 1; break
        case K.CaseClause: m.cyclo++; break
        case K.ConditionalExpression: m.cyclo++; m.cognitive += 1 + nest; child = nest + 1; break
        case K.BinaryExpression: {
          const op = n.operatorToken.kind
          if (op === K.AmpersandAmpersandToken || op === K.BarBarToken || op === K.QuestionQuestionToken) { m.cyclo++; m.cognitive++ }
          break }
      }
      if (child > nest) m.maxNest = Math.max(m.maxNest, child)
      ts.forEachChild(n, c => visit(c, child))
    }
    visit(body, 0)
    return m
  }
  const nameOf = n => {
    if (n.name && n.name.getText) return n.name.getText(sf)
    const p = n.parent
    if (!p) return '(anonymous)'
    if (p.kind === K.VariableDeclaration && p.name) return p.name.getText(sf)
    if (p.kind === K.PropertyAssignment && p.name) return p.name.getText(sf)
    if (p.kind === K.PropertyDeclaration && p.name) return p.name.getText(sf)
    if (p.kind === K.BinaryExpression) return p.left.getText(sf).slice(0, 60)
    return null
  }
  const classStack = []
  const visit = n => {
    switch (n.kind) {
      case K.AnyKeyword: rec.any++; break
      case K.AsExpression: if (n.type.kind !== K.TypeReference || n.type.getText(sf) !== 'const') rec.asCasts++; break
      case K.NonNullExpression: rec.nonNull++; break
      case K.ImportDeclaration: {
        rec.imports++
        const spec = n.moduleSpecifier.text
        const target = resolveImport(file, spec)
        const typeOnly = !!(n.importClause && n.importClause.isTypeOnly)
        edges.push({from: rel, to: target ? relative(root, target) : null, spec, typeOnly})
        break }
      case K.ExportDeclaration: {
        rec.exports++
        if (n.moduleSpecifier && n.moduleSpecifier.text) { const t = resolveImport(file, n.moduleSpecifier.text); edges.push({from: rel, to: t ? relative(root, t) : null, spec: n.moduleSpecifier.text, typeOnly: !!n.isTypeOnly, kind: 'reexport'}) }
        break }
      case K.ExportAssignment: rec.exports++; break
      case K.CallExpression: {
        if (n.expression.kind === K.ImportKeyword && n.arguments.length && ts.isStringLiteralLike(n.arguments[0])) { const sp = n.arguments[0].text; const t = resolveImport(file, sp); edges.push({from: rel, to: t ? relative(root, t) : null, spec: sp, typeOnly: false, kind: 'dynamic'}) }
        break }
      case K.NewExpression: {
        if (n.expression.getText(sf) === 'URL' && n.arguments && n.arguments.length >= 1 && ts.isStringLiteralLike(n.arguments[0])) { const sp = n.arguments[0].text; const t = resolveImport(file, sp); if (t) edges.push({from: rel, to: relative(root, t), spec: sp, typeOnly: false, kind: 'url'}) }
        break }
      case K.ImportType: {
        const a = n.argument && n.argument.literal; if (a && ts.isStringLiteralLike(a)) { const t = resolveImport(file, a.text); edges.push({from: rel, to: t ? relative(root, t) : null, spec: a.text, typeOnly: true, kind: 'importType'}) }
        break }
      case K.ClassDeclaration: rec.classes++; break
    }
    if (n.modifiers && n.modifiers.some(m => m.kind === K.ExportKeyword)) rec.exports++
    const isFn = n.kind === K.FunctionDeclaration || n.kind === K.MethodDeclaration || n.kind === K.Constructor || n.kind === K.GetAccessor || n.kind === K.SetAccessor || n.kind === K.FunctionExpression || n.kind === K.ArrowFunction
    if (n.kind === K.ClassDeclaration || n.kind === K.ClassExpression) classStack.push(n.name ? n.name.getText(sf) : '(class)')
    if (isFn && n.body) {
      const name = n.kind === K.Constructor ? 'constructor' : nameOf(n)
      const start = lineOf(n.getStart(sf)), end = lineOf(n.end)
      const loc = end - start + 1
      if (name || loc >= 12) {
        rec.fns++
        const m = fnMetrics(n)
        fnRows.push({file: rel, cls: classStack.at(-1) || '', name: name || '(callback)', line: start, loc, params: n.parameters.length, ...m})
      }
    }
    ts.forEachChild(n, visit)
    if (n.kind === K.ClassDeclaration || n.kind === K.ClassExpression) classStack.pop()
  }
  visit(sf)
  fileRows.push(rec)
}
writeFileSync(outPath, JSON.stringify({files: fileRows, fns: fnRows, edges}))
console.log(`files=${fileRows.length} fns=${fnRows.length} edges=${edges.length}`)
