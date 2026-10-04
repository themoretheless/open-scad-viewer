import ts from 'typescript'
import {readFile,writeFile,mkdir} from 'node:fs/promises'
import path from 'node:path'
const sourcePath='src/features/DirectModeler.vue'
const text=await readFile(sourcePath,'utf8'),start=text.indexOf('<script setup'),bodyStart=text.indexOf('>',start)+1,end=text.indexOf('</script>',bodyStart)
if(start<0||end<=bodyStart)throw Error('Missing Vue setup script')
const source=ts.createSourceFile(sourcePath,text.slice(bodyStart,end),ts.ScriptTarget.Latest,true,ts.ScriptKind.TS)
const commands=[],dynamic=[]
const literal=node=>node&&(ts.isStringLiteral(node)||ts.isNoSubstitutionTemplateLiteral(node))?node.text:null
function visit(node){
 if(ts.isCallExpression(node)&&ts.isIdentifier(node.expression)&&['cmd','toolCmd'].includes(node.expression.text)){
  const [id,ru,en]=node.arguments.map(literal)
  const line=text.slice(0,bodyStart+node.getStart(source)).split('\n').length
  if(id!==null){
   commands.push({id:node.expression.text==='toolCmd'?'tool-'+id:id,ru:ru??node.arguments[1]?.getText(source),en:en??node.arguments[2]?.getText(source),dynamicLabels:ru===null||en===null,line,mouse:'unverified',keyboard:'unverified',errorLocalization:'unverified',documentHistory:'unverified'})
  }else dynamic.push({line,expression:node.getText(source),status:'requires-runtime-expansion'})
 }
 ts.forEachChild(node,visit)
}
visit(source)
const ids=new Set()
for(const command of commands){if(ids.has(command.id))throw Error('Duplicate literal command ID: '+command.id);ids.add(command.id)}
const output=path.resolve(process.argv[2]??'docs/qualification/cad-roadmap-2026-09-28/command-coverage-2026-10-01/source-inventory.json')
await mkdir(path.dirname(output),{recursive:true})
await writeFile(output,JSON.stringify({source:sourcePath,scope:'source inventory; execution coverage is not established',staticCommandCount:commands.length,dynamicCallCount:dynamic.length,commands,dynamic},null,2)+'\n')
console.log(JSON.stringify({staticCommands:commands.length,dynamicCalls:dynamic.length,output}))
