import {readFileSync,writeFileSync} from 'node:fs'
import {parseDirectDocument} from '../src/services/directModeling'
import {exportSolidBlenderSnapshot} from '../src/services/solidBlenderExchange'
const [input,output,projectId]=process.argv.slice(2)
if(!input||!output||!projectId)throw Error('Usage: node --import tsx scripts/export-blender-snapshot.mts input.json output.osv-blender.json stable-project-id')
writeFileSync(output,exportSolidBlenderSnapshot(parseDirectDocument(readFileSync(input,'utf8')),projectId))
