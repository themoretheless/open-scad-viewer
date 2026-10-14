// Inspect the actual Cargo graph rather than a hand-maintained dependency diagram.
import {execFileSync} from 'node:child_process'
import {readFileSync,readdirSync} from 'node:fs'
import ts from 'typescript'
import {parse as parseVue} from '@vue/compiler-sfc'
import {dirname, resolve, sep} from 'node:path'
import {fileURLToPath} from 'node:url'

const root=resolve(dirname(fileURLToPath(import.meta.url)),'..')
const manifest=resolve(root,'crates/Cargo.toml')
const cargo=(args)=>execFileSync('cargo',[...args,'--manifest-path',manifest],{cwd:root,encoding:'utf8',stdio:['ignore','pipe','inherit']})
const metadata=JSON.parse(cargo(['metadata','--no-deps','--format-version','1']))
const packages=new Map(metadata.packages.map(p=>[p.name,p]))
const issues=[]
const internal=p=>p.dependencies.filter(d=>packages.has(d.name)&&d.source===null&&d.kind!=='dev')
const edges=Object.fromEntries([...packages].map(([name,p])=>[name,internal(p).map(d=>d.name)]))
const active=new Set(), complete=new Set()
function visit(name,chain=[]){
 if(active.has(name)){issues.push({rule:'acyclic',package:name,detail:[...chain,name].join(' -> ')});return}
 if(complete.has(name))return
 active.add(name)
 for(const dependency of edges[name])visit(dependency,[...chain,name])
 active.delete(name);complete.add(name)
}
for(const name of packages.keys())visit(name)
for(const p of packages.values()){
 const text=readFileSync(p.manifest_path,'utf8')
 // Cargo's metadata has already validated all manifests; this checks ownership.
 for(const line of text.split('\n')){
  if(/^\s*[^#\s]+\s*=\s*\{.*\bpath\s*=/.test(line)){
   issues.push({rule:'workspace-dependency-owner',package:p.name,detail:line.trim()})
  }
 }
}
const nativeKernels=['cad-predicates','osv-math','mesh-topology','mesh-query','mesh-section','mesh-io','planar-geometry','geometry-ops','polygon-core','sketch-core','subdivision-core','sdf-core','mechanical-core','nurbs-core','brep-topology','brep-core']
const adapters=new Set(['geometry-bridge','geometry-wasm','languages-bridge','languages-wasm','photogrammetry-ffi','photogrammetry-wasm'])
const native=[]
for(const name of nativeKernels){
 const lines=cargo(['tree','-p',name,'--no-default-features','--edges','normal','--prefix','none','--format','{p}']).trim().split('\n')
 const dependencies=[...new Set(lines.map(line=>line.split(' ')[0]))].sort()
 native.push({package:name,dependencies})
 for(const dependency of dependencies){
  if(dependency==='value-codec')issues.push({rule:'native-without-transport',package:name,detail:'value-codec is reachable with default features disabled'})
  if(adapters.has(dependency))issues.push({rule:'kernel-without-application',package:name,detail:dependency})
 }
}
// Production TypeScript and Vue scripts must not import comparison oracles.
const excludedRoots=['benchmarks','tests'].map(name=>resolve(root,name))
function inspectSource(directory){
 for(const entry of readdirSync(directory,{withFileTypes:true})){
  const path=resolve(directory,entry.name)
  if(entry.isDirectory()){inspectSource(path);continue}
  if(!/\.(tsx?|vue)$/.test(entry.name))continue
  let text=readFileSync(path,'utf8')
  if(entry.name.endsWith('.vue')){
   const {descriptor}=parseVue(text,{filename:path})
   text=[descriptor.script?.content,descriptor.scriptSetup?.content, ...[descriptor.script?.src,descriptor.scriptSetup?.src].filter(Boolean).map(src=>'import '+JSON.stringify(src))].filter(Boolean).join('\n')
  }
  const source=ts.createSourceFile(path,text,ts.ScriptTarget.Latest,true)
  function inspect(node){
   let specifier
   if(ts.isImportDeclaration(node)||ts.isExportDeclaration(node))specifier=node.moduleSpecifier
   else if(ts.isImportTypeNode(node)&&ts.isLiteralTypeNode(node.argument))specifier=node.argument.literal
   else if(ts.isImportEqualsDeclaration(node)&&ts.isExternalModuleReference(node.moduleReference))specifier=node.moduleReference.expression
   else if(ts.isCallExpression(node)&&(node.expression.kind===ts.SyntaxKind.ImportKeyword||(ts.isIdentifier(node.expression)&&node.expression.text==='require')))specifier=node.arguments[0]
   if(specifier&&(ts.isStringLiteral(specifier)||ts.isNoSubstitutionTemplateLiteral(specifier))){
    const name=specifier.text
    const target=name.startsWith('.')?resolve(dirname(path),name):name.startsWith('@/')?resolve(root,'src',name.slice(2)):null
    if(target&&excludedRoots.some(base=>target===base||target.startsWith(base+sep))){
     issues.push({rule:'application-without-test-oracle',package:path.slice(root.length+1),detail:name})
    }
   }
   ts.forEachChild(node,inspect)
  }
  inspect(source)
 }
}
inspectSource(resolve(root,'src'))
const report={workspacePackages:packages.size,edges,native,issues}
if(process.argv.includes('--json'))process.stdout.write(JSON.stringify(report,null,2)+'\n')
else{
 console.log(`Workspace: ${packages.size} packages; native kernels: ${native.length}`)
 if(issues.length===0)console.log('Library structure requirements pass.')
 else for(const issue of issues)console.log(`${issue.rule}: ${issue.package}: ${issue.detail}`)
}
if(process.argv.includes('--check')&&issues.length)process.exitCode=1
