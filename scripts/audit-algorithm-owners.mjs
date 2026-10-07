// Reproducible source census; ownership decisions live in the reviewed registry.
import {readFileSync, readdirSync, writeFileSync} from 'node:fs'
import {resolve, relative, dirname} from 'node:path'
import {fileURLToPath} from 'node:url'
import {createHash} from 'node:crypto'
import ts from 'typescript'
import {parse} from '@vue/compiler-sfc'
const root=resolve(dirname(fileURLToPath(import.meta.url)), '..')
const registry=JSON.parse(readFileSync(resolve(root,'docs/design/algorithm-owners.json'),'utf8'))
const rows=[]
function walk(dir){
 for(const entry of readdirSync(dir,{withFileTypes:true}).sort((a,b)=>a.name.localeCompare(b.name))){
  const absolute=resolve(dir,entry.name),file=relative(root,absolute)
  if(entry.isDirectory()){walk(absolute);continue}
  if(!/\.(ts|vue|mjs)$/.test(file))continue
  const raw=readFileSync(absolute,'utf8')
  let text=raw
  if(file.endsWith('.vue')){const {descriptor}=parse(raw,{filename:file});text=[descriptor.script?.content,descriptor.scriptSetup?.content].filter(Boolean).join('\n')}
  const source=ts.createSourceFile(file,text,ts.ScriptTarget.Latest,true)
  const functions=[],mathCalls=[]
  let loops=0
  function visit(node){
   if(ts.isFunctionDeclaration(node)||ts.isMethodDeclaration(node)||ts.isArrowFunction(node)||ts.isFunctionExpression(node)){
    const parent=node.parent
    const name=node.name?.getText(source)??(ts.isVariableDeclaration(parent)?parent.name.getText(source):'<callback>')
    functions.push({name,line:source.getLineAndCharacterOfPosition(node.getStart(source)).line+1})
   }
   if(ts.isForStatement(node)||ts.isForOfStatement(node)||ts.isForInStatement(node)||ts.isWhileStatement(node)||ts.isDoStatement(node))loops++
   if(ts.isCallExpression(node)&&ts.isPropertyAccessExpression(node.expression)&&node.expression.expression.getText(source)==='Math')mathCalls.push(node.expression.name.text)
   ts.forEachChild(node,visit)
  }
  visit(source)
  const decision=registry.decisions.find(d=>d.files.includes(file))
  const family=registry.families.find(d=>d.patterns.some(p=>file.startsWith(p)))
  rows.push({file,sha256:createHash('sha256').update(raw).digest('hex'),generated:/\/generated\//.test(file),owner:decision?.owner??family?.owner??'application-host',decision:decision?.id??family?.id??'host',functions,loops,mathCalls:[...new Set(mathCalls)].sort(),lineCoordinates:file.endsWith('.vue')?'combined-script':'file'})
 }
}
walk(resolve(root,'src'))
const sensitive=new Set(['sin','cos','tan','asin','acos','atan','atan2','hypot','sqrt'])
const unreviewed=rows.filter(r=>!r.generated&&r.mathCalls.some(m=>sensitive.has(m))&&!registry.decisions.some(d=>d.files.includes(r.file)))
const stale=registry.decisions.flatMap(d=>d.files.filter(f=>!rows.some(r=>r.file===f)).map(file=>({decision:d.id,file})))
const report={schemaVersion:1,scope:'Production src TS/Vue/MJS; file ownership and callable census. Not a proof of algorithm completeness or migration parity.',sourceFiles:rows.length,callables:rows.reduce((n,r)=>n+r.functions.length,0),unreviewedNumericalFiles:unreviewed.map(r=>r.file),staleDecisions:stale,rows}
const output=resolve(root,'docs/qualification/algorithm-owners-2026-10-03.json')
if(process.argv.includes('--write'))writeFileSync(output,JSON.stringify(report,null,2)+'\n')
if(process.argv.includes('--check')){
 const saved=JSON.parse(readFileSync(output,'utf8'))
 if(JSON.stringify(saved)!==JSON.stringify(report)){console.error('Ownership census changed. Review new sources/algorithms and regenerate with --write.');process.exitCode=1}
}
if(unreviewed.length||stale.length){console.error(JSON.stringify({unreviewed:report.unreviewedNumericalFiles,stale},null,2));process.exitCode=1}
console.log(`${rows.length} source files; ${report.callables} callables; ${unreviewed.length} unreviewed numerical files; ${stale.length} stale decisions.`)
