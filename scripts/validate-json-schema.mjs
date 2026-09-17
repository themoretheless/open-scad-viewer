import {readFileSync} from 'node:fs'
import Ajv2020 from 'ajv/dist/2020.js'

const [, , schemaPath, ...documents]=process.argv
if(!schemaPath||!documents.length)throw new Error('Usage: validate-json-schema.mjs SCHEMA DOCUMENT...')
const schema=JSON.parse(readFileSync(schemaPath,'utf8'))
const ajv=new Ajv2020({allErrors:true,strict:false})
const validate=ajv.compile(schema)
for(const document of documents){
  const value=JSON.parse(readFileSync(document,'utf8'))
  if(!validate(value))throw new Error(`${document}: ${ajv.errorsText(validate.errors,{separator:'\n'})}`)
  console.log(`valid ${document}`)
}
