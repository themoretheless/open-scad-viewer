import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {z} from 'zod/v4'
import * as model from './modelGraph-runtime-reference.ts'
const schema:any=z.toJSONSchema(model.modelGraphSchema,{io:'input'})
const cases:any[]=[]
function specimen(s:any):any {
 if(s.default!==undefined)return structuredClone(s.default)
 if(s.$ref)return specimen(schema.$defs[s.$ref.split('/').at(-1)])
 if(s.const!==undefined)return s.const
 if(s.enum)return s.enum[0]
 if(s.anyOf||s.oneOf){const choices=s.anyOf??s.oneOf;return specimen(choices.find((x:any)=>x.type==='number')??choices[0])}
 if(s.type==='number'||s.type==='integer')return Math.max(s.minimum??0,0)
 if(s.type==='boolean')return false
 if(s.type==='string')return s.pattern?'shape':'message'
 if(s.type==='array')return s.prefixItems?s.prefixItems.map(specimen):Array.from({length:s.minItems??0},()=>specimen(s.items))
 if(s.type==='object')return Object.fromEntries(Object.entries(s.properties??{}).filter(([k])=>(s.required??[]).includes(k)).map(([k,v])=>[k,specimen(v)]))
 throw new Error('Unhandled '+JSON.stringify(s))
}
const base=(node:any)=>({language:'modelgraph/1',units:'mm',parameters:[],nodes:[node],root:'shape'})
const add=(name:string,input:any)=>{const p=model.modelGraphSchema.safeParse(input);cases.push(p.success?{name,input,expected:p.data}:{name,input,error:true})}
const addValid=(name:string,input:any)=>{if(!model.modelGraphSchema.safeParse(input).success)throw new Error('Bad generated fixture '+name);add(name,input)}
function eachVariant(s:any,callback:(s:any,name:string)=>void){for(const variant of s.oneOf??s.anyOf??[s]){const disc=variant.properties?.op??variant.properties?.kind;for(const name of disc?.enum??[disc?.const??variant.type??'value'])callback(disc?.enum?{...variant,properties:{...variant.properties,[variant.properties.op?'op':'kind']:{const:name}}}:variant,name)}}
for(const [name,input]of Object.entries(model)){if(name.endsWith('_EXAMPLE')&&(input as any).language==='modelgraph/1')addValid(name,input)}
addValid('skadis_box',JSON.parse(readFileSync('output/modelgraph-schema-profile-skadis.json','utf8')))
eachVariant(schema.properties.nodes.items,(s,name)=>{const node=specimen(s);addValid('node_'+name,base(node));for(const field of s.required??[]){const bad=structuredClone(node);delete bad[field];add('missing_'+name+'_'+field,base(bad))}const bad={...node,unsupported:true};add('unknown_'+name,base(bad))})
const expr=schema.$defs[Object.keys(schema.$defs).find(k=>schema.$defs[k].anyOf)!]
eachVariant(expr,(s,name)=>{const value=specimen(s);const node={id:'shape',op:'evaluate',value};addValid('expression_'+name,base(node));if(value&&typeof value==='object'){add('unknown_expression_'+name,base({...node,value:{...value,unsupported:true}}));for(const field of s.required??[]){const bad=structuredClone(value);delete bad[field];add('missing_expression_'+name+'_'+field,base({...node,value:bad}))}}})
eachVariant(schema.properties.functions.items,(s,name)=>{const doc=base({id:'shape',op:'sphere',radius:1});doc.functions=[specimen(s)];addValid('function_'+name,doc)})
const sketch=schema.properties.nodes.items.oneOf.find((s:any)=>s.properties.op.const==='sketch')
eachVariant(sketch.properties.constraints.items,(s,name)=>{const node=specimen(sketch);node.constraints=[specimen(s)];addValid('sketch_'+name,base(node))})
for(const [name,input]of [
 ['segments_min',{...base({id:'shape',op:'sphere',radius:1}),segments:12}],
 ['segments_max',{...base({id:'shape',op:'sphere',radius:1}),segments:128}],
 ['segments_too_low',{...base({id:'shape',op:'sphere',radius:1}),segments:11}],
 ['parameter_bounds',{...base({id:'shape',op:'sphere',radius:1}),parameters:[{id:'x',value:1000000,min:-1000000,max:1000000,unit:'mm',integer:true}]}],
 ['bad_parameter',{...base({id:'shape',op:'sphere',radius:1}),parameters:[{id:'x',value:1000001}]}],
 ['bad_nested_tuple',base({id:'shape',op:'affine',input:'x',rows:[[1,0,0,0],[0,1,0],[0,0,1,0]]})],
 ['geometry_assertions',{...base({id:'shape',op:'sphere',radius:1}),geometry_assertions:[{id:'a',target:'shape',check:'height',expected:2,tolerance:0.01,message:'height'}]}],
 ['invalid_assertion_check',{...base({id:'shape',op:'sphere',radius:1}),geometry_assertions:[{id:'a',target:'shape',check:'bogus',message:'bad'}]}],
 ['null_optional',{...base({id:'shape',op:'sphere',radius:1}),constraints:null}],
 ['too_many_nodes',{...base({id:'shape',op:'sphere',radius:1}),nodes:Array.from({length:129},()=>({id:'shape',op:'sphere',radius:1}))}],
 ['long_message',{...base({id:'shape',op:'sphere',radius:1}),assertions:[{condition:1,message:'😀'.repeat(129)}]}]
]as any[])add(name,input)
const report={description:'Canonical schema parity corpus captured from TypeScript Zod schema before Rust runtime migration; validates schema/default equivalence, not graph semantics.',source_sha256:createHash('sha256').update(readFileSync('benchmarks/modelgraph/modelGraph-runtime-reference.ts')).digest('hex'),cases}
writeFileSync('crates/modelgraph-runtime/tests/fixtures/schema-parity.json',JSON.stringify(report)+'\n')
console.log({cases:cases.length,valid:cases.filter(x=>!x.error).length,invalid:cases.filter(x=>x.error).length,bytes:JSON.stringify(report).length})
