import ts from 'typescript'
import {readFile,writeFile,mkdir} from 'node:fs/promises'
import path from 'node:path'
import {createHash} from 'node:crypto'
const sourcePath='src/features/DirectModeler.vue'
const text=await readFile(sourcePath,'utf8'),start=text.indexOf('<script setup'),bodyStart=text.indexOf('>',start)+1,end=text.indexOf('</script>',bodyStart)
if(start<0||end<=bodyStart)throw Error('Missing Vue setup script')
const source=ts.createSourceFile(sourcePath,text.slice(bodyStart,end),ts.ScriptTarget.Latest,true,ts.ScriptKind.TS)
const commands=[],dynamic=[]
const literal=node=>node&&(ts.isStringLiteral(node)||ts.isNoSubstitutionTemplateLiteral(node))?node.text:null
const checks=()=>Object.fromEntries(['mouse','keyboard','errorLocalization','retry','cancel','lateResults','documentHistory','recovery','multipleTabs','latency'].map(key=>[key,'unverified']))
function primitiveValues(node){
 if(ts.isVariableDeclaration(node)&&node.name.getText(source)==='primitiveKinds'){
  let value=node.initializer
  while(value&&(ts.isAsExpression(value)||ts.isParenthesizedExpression(value)))value=value.expression
  if(!value||!ts.isArrayLiteralExpression(value))throw Error('primitiveKinds must remain a literal inventory')
  return value.elements.map(element=>{const result=literal(element);if(result===null)throw Error('Nonliteral primitive kind');return result})
 }
 for(const child of node.getChildren(source)){const result=primitiveValues(child);if(result)return result}
}
const primitives=primitiveValues(source)
if(!primitives?.length)throw Error('Missing primitiveKinds inventory')
function visit(node){
 if(ts.isCallExpression(node)&&ts.isIdentifier(node.expression)&&['cmd','toolCmd'].includes(node.expression.text)){
  const [id,ru,en]=node.arguments.map(literal)
  const line=text.slice(0,bodyStart+node.getStart(source)).split('\n').length
  if(id!==null){
   commands.push({id:node.expression.text==='toolCmd'?'tool-'+id:id,ru:ru??node.arguments[1]?.getText(source),en:en??node.arguments[2]?.getText(source),dynamicLabels:ru===null||en===null,line,...checks()})
  }else if(node.arguments[0]?.getText(source)==='`add-${kind}`'){
   for(const kind of primitives)commands.push({id:'add-'+kind,ru:`primitiveLabel('${kind}')`,en:`primitiveLabel('${kind}')`,dynamicLabels:true,line,expansion:'primitiveKinds',...checks()})
   dynamic.push({line,expression:node.getText(source),status:'expanded-from-literal-primitiveKinds',count:primitives.length})
  }else if(node.arguments[0]?.getText(source)==='`tool-${value}`'){
   dynamic.push({line,expression:node.getText(source),status:'helper-expanded-at-toolCmd-call-sites'})
  }else dynamic.push({line,expression:node.getText(source),status:'requires-runtime-expansion'})
 }
 ts.forEachChild(node,visit)
}
visit(source)
const ids=new Set()
for(const command of commands){if(ids.has(command.id))throw Error('Duplicate literal command ID: '+command.id);ids.add(command.id)}
const output=path.resolve(process.argv[2]??'docs/qualification/cad-roadmap-2026-09-28/command-coverage-2026-10-01/source-inventory.json')
await mkdir(path.dirname(output),{recursive:true})
await writeFile(output,JSON.stringify({source:sourcePath,sourceSha256:createHash('sha256').update(text).digest('hex'),scope:'source inventory; execution coverage is not established',staticCommandCount:commands.filter(command=>!command.expansion).length,commandCount:commands.length,dynamicCallCount:dynamic.length,unresolvedDynamicCalls:dynamic.filter(call=>call.status==='requires-runtime-expansion').length,commands,dynamic},null,2)+'\n')
console.log(JSON.stringify({commands:commands.length,staticCommands:commands.filter(command=>!command.expansion).length,dynamicCalls:dynamic.length,output}))
