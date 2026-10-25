import 'fake-indexeddb/auto'
import {readFileSync,writeFileSync} from 'node:fs'
import {parseDirectDocument} from '../src/services/directModeling'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
const [input,id,output]=process.argv.slice(2)
if(!input||!id||!output)throw Error('Provide document JSON, exact body ID and STEP output path')
const document=parseDirectDocument(readFileSync(input,'utf8'))
const matches=document.bodies.filter(body=>body.id===id)
if(matches.length!==1)throw Error('Expected exactly one matching body')
writeFileSync(output,await exportSolidStepCurrent(matches[0]!))
