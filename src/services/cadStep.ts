import {bodyPoints,type DirectBody} from './directModeling'
import {inspectPolygonMesh} from './geometry/polygon'
import {cross3,unit3,dot3} from './directSketchGeometry'
/** ISO 10303-21/AP214 faceted BREP in millimetres. No analytic surface reconstruction. */
export function exportFacetedStep(bodies:DirectBody[]):string{
 if(!bodies.length)throw Error('Select bodies to export.')
 const entities:string[]=[],add=(s:string)=>{entities.push(`#${entities.length+1}=${s};`);return '#'+entities.length},num=(v:number)=>{if(!Number.isFinite(v))throw Error('Invalid STEP coordinate.');const s=String(v);return s.includes('.')||/e/i.test(s)?s.replace('e','E'):s+'.'},tuple=(p:number[])=>'('+p.map(num).join(',')+')'
 const app=add("APPLICATION_CONTEXT('automotive_design')");add(`APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2000,${app})`)
 const pc=add(`PRODUCT_CONTEXT('',${app},'mechanical')`),product=add(`PRODUCT('model','model','',(${pc}))`),formation=add(`PRODUCT_DEFINITION_FORMATION('1','',${product})`),dc=add(`PRODUCT_DEFINITION_CONTEXT('part definition',${app},'design')`),definition=add(`PRODUCT_DEFINITION('design','',${formation},${dc})`),shape=add(`PRODUCT_DEFINITION_SHAPE('','',${definition})`)
 const length=add('(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.))'),angle=add('(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.))'),solidAngle=add('(NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT())'),uncertainty=add(`UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-6),${length},'distance_accuracy_value','')`),context=add(`(GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((${uncertainty})) GLOBAL_UNIT_ASSIGNED_CONTEXT((${length},${angle},${solidAngle})) REPRESENTATION_CONTEXT('','3D'))`)
 const solids=bodies.map(b=>{
  const r=inspectPolygonMesh(b.mesh);if(!r.closed||r.signedVolumeMm3<=0)throw Error('STEP export requires closed outward-oriented solids.')
  const p=bodyPoints(b),points=p.map(p=>add(`CARTESIAN_POINT('',${tuple(p)})`)),faces:string[]=[]
  for(let i=0;i<b.mesh.indices.length;i+=3){const ids=b.mesh.indices.slice(i,i+3),a=p[ids[0]],u=unit3(p[ids[1]].map((v,k)=>v-a[k])),n=unit3(cross3(u,p[ids[2]].map((v,k)=>v-a[k]))),loop=add(`POLY_LOOP('',(${ids.map(i=>points[i]).join(',')}))`),bound=add(`FACE_OUTER_BOUND('',${loop},.T.)`),normal=add(`DIRECTION('',${tuple(n)})`),direction=add(`DIRECTION('',${tuple(u)})`),placement=add(`AXIS2_PLACEMENT_3D('',${points[ids[0]]},${normal},${direction})`),plane=add(`PLANE('',${placement})`);faces.push(add(`FACE_SURFACE('',(${bound}),${plane},.T.)`))}
  return add(`FACETED_BREP('${b.name.replace(/[^\x20-\x7e]/g,'_').replace(/'/g,"''")}',${add(`CLOSED_SHELL('',(${faces.join(',')}))`)})`)
 });const representation=add(`FACETED_BREP_SHAPE_REPRESENTATION('',(${solids.join(',')}),${context})`);add(`SHAPE_DEFINITION_REPRESENTATION(${shape},${representation})`)
 return `ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('Faceted solids'),'2;1');\nFILE_NAME('model.step','${new Date().toISOString()}',(''),(''),'OpenSCAD Viewer','OpenSCAD Viewer','');\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\n${entities.join('\n')}\nENDSEC;\nEND-ISO-10303-21;\n`
}
/** Reads planar poly-loop BREP only. Reject unsupported topology instead of dropping it. */
export function importFacetedStep(text:string):DirectBody[]{
 if(text.length>20_000_000)throw Error('STEP file exceeds 20 MB.')
 if(!text.startsWith('ISO-10303-21;')||!text.includes('END-ISO-10303-21;'))throw Error('Invalid STEP exchange file.')
 if(/\b(ADVANCED_FACE|MANIFOLD_SOLID_BREP|BREP_WITH_VOIDS|MAPPED_ITEM|CONVERSION_BASED_UNIT)\s*\(/i.test(text))throw Error('This importer supports planar FACETED_BREP in SI units only; analytic faces, void shells, mapped instances and converted units are not supported.')
 const units=[...text.matchAll(/SI_UNIT\(\s*(\$|\.\w+\.)\s*,\s*\.METRE\.\s*\)/gi)].map(m=>m[1].toUpperCase());if(!units.length||new Set(units).size!==1||!['.MILLI.','$'].includes(units[0]))throw Error('STEP must use uniform metres or millimetres.');const scale=units[0]==='$'?1000:1
 const entities=new Map<string,{type:string;args:string}>();for(const m of text.matchAll(/#(\d+)\s*=\s*([A-Z_0-9]+)\s*\(((?:[^']|'(?:[^']|'')*')*?)\)\s*;/gi)){if(entities.has(m[1]))throw Error('Duplicate STEP entity.');entities.set(m[1],{type:m[2].toUpperCase(),args:m[3]})}
 const entity=(id:string,type?:string)=>{const e=entities.get(id);if(!e||type&&e.type!==type)throw Error('Unsupported or missing STEP '+(type??'entity')+' #'+id);return e},refs=(args:string)=>[...args.replace(/'(?:[^']|'')*'/g,"''").matchAll(/#(\d+)/g)].map(m=>m[1])
 const point=(id:string)=>{const args=entity(id,'CARTESIAN_POINT').args,tuple=args.match(/\(([^()]*)\)\s*$/)?.[1];const values=tuple?.split(',').map(v=>Number(v.replace(/D/i,'E'))*scale);if(!values||values.length!==3||!values.every(Number.isFinite))throw Error('Invalid STEP point.');return values}
 const solids=[...entities.values()].filter(e=>e.type==='FACETED_BREP');if(!solids.length)throw Error('No FACETED_BREP solids found.')
 return solids.map((solid,index)=>{const positions:number[]=[],indices:number[]=[],weld=new Map<string,number>();const vertex=(p:number[])=>{const key=p.join(',');let i=weld.get(key);if(i===undefined){i=positions.length/3;weld.set(key,i);positions.push(...p)}return i};const shell=entity(refs(solid.args)[0],'CLOSED_SHELL')
  for(const faceId of refs(shell.args)){const face=entity(faceId);if(!['FACE_SURFACE','FACE'].includes(face.type))throw Error('Unsupported STEP face.');const fr=refs(face.args),boundsList=face.args.match(/\(([^()]*)\)/)?.[1];if(!boundsList||refs(boundsList).length!==1)throw Error('STEP faces with holes are not supported.');if(face.type==='FACE_SURFACE')entity(fr[1],'PLANE');const bound=entity(fr[0]);if(!['FACE_OUTER_BOUND','FACE_BOUND'].includes(bound.type))throw Error('Unsupported STEP face bound.');const loop=entity(refs(bound.args)[0],'POLY_LOOP'),p=refs(loop.args).map(point);if(p.length<3)throw Error('Invalid STEP polygon.');if(/\.F\.\s*$/.test(bound.args))p.reverse();const normal=unit3(cross3(p[1].map((v,k)=>v-p[0][k]),p[2].map((v,k)=>v-p[0][k])));for(let i=0;i<p.length;i++){if(Math.abs(dot3(p[i].map((v,k)=>v-p[0][k]),normal))>1e-6)throw Error('Nonplanar STEP polygon.');const a=p[i],b=p[(i+1)%p.length],c=p[(i+2)%p.length];if(dot3(cross3(b.map((v,k)=>v-a[k]),c.map((v,k)=>v-b[k])),normal)<-1e-8)throw Error('Concave STEP poly-loop is not supported.')}const ids=p.map(vertex);for(let i=1;i<ids.length-1;i++)indices.push(ids[0],ids[i],ids[i+1])}
  const mesh={positions,indices},r=inspectPolygonMesh(mesh);if(!r.closed||r.signedVolumeMm3<=0)throw Error('STEP shell is not a closed outward-oriented solid.');return {id:crypto.randomUUID(),name:'STEP '+(index+1),mesh}
 })
}

export interface StepTessellatedPresentation {
 representationIdentity:string
 itemIdentity:string
 primitive:'triangles'|'strips-and-fans'
 normals:number[]
 colors:string[]
 layers:string[]
 textureCoordinates:number[]
 source:string
}
export interface TessellatedStepImport {bodies:DirectBody[];presentation:StepTessellatedPresentation[];metadataLoss:string[]}
const splitStepArgs=(value:string)=>{
 const out:string[]=[],start=0;let depth=0,quote=false,last=start
 for(let i=0;i<value.length;i++){const c=value[i];if(c==="'"&&value[i+1]==="'"){i++;continue}if(c==="'")quote=!quote
  else if(!quote&&c==='(')depth++;else if(!quote&&c===')')depth--;else if(!quote&&c===','&&depth===0){out.push(value.slice(last,i));last=i+1}}
 out.push(value.slice(last));return out.map(field=>field.trim())
}
const stepIdentity=(value:string)=>{let hash=2166136261;for(const byte of new TextEncoder().encode(value.replace(/#\d+/g,'#'))){hash^=byte;hash=Math.imul(hash,16777619)}return `step10:${(hash>>>0).toString(16).padStart(8,'0')}`}
const indexRows=(value:string)=>[...value.matchAll(/\(([\d,\s]+)\)/g)].map(match=>match[1].split(',').map(Number)).filter(row=>row.length>=3&&row.every(Number.isInteger))
const stripTriangles=(row:number[])=>Array.from({length:row.length-2},(_,i)=>i%2?[row[i+1],row[i],row[i+2]]:[row[i],row[i+1],row[i+2]])
const fanTriangles=(row:number[])=>Array.from({length:row.length-2},(_,i)=>[row[0],row[i+1],row[i+2]])

/** AP242 tessellated decoder. Presentation remains explicitly tessellated and is never promoted to B-rep. */
export function importTessellatedStepV10(text:string):TessellatedStepImport{
 if(new TextEncoder().encode(text).byteLength>20_000_000)throw Error('STEP tessellated file exceeds 20 MB.')
 if(!/\bTESSELLATED_SHAPE_REPRESENTATION\s*\(/i.test(text))throw Error('No TESSELLATED_SHAPE_REPRESENTATION product found.')
 const entities=new Map<string,{type:string;args:string}>()
 for(const match of text.matchAll(/#(\d+)\s*=\s*([A-Z_0-9]+)\s*\((.*?)\)\s*;/gis)){
  if(entities.has(match[1]))throw Error('Duplicate STEP tessellated entity.')
  entities.set(match[1],{type:match[2].toUpperCase(),args:match[3]})
 }
 const number=(value:string)=>Number(value.replace(/[dD]/,'E'))
 const pointLists=new Map<string,number[][]>()
 for(const [id,entity] of entities){
  if(entity.type!=='CARTESIAN_POINT_LIST_3D')continue
  const points=[...entity.args.matchAll(/\(\s*([+-]?(?:\d+\.?\d*|\.\d+)(?:[eEdD][+-]?\d+)?)\s*,\s*([+-]?(?:\d+\.?\d*|\.\d+)(?:[eEdD][+-]?\d+)?)\s*,\s*([+-]?(?:\d+\.?\d*|\.\d+)(?:[eEdD][+-]?\d+)?)\s*\)/g)]
    .map(match=>[number(match[1]),number(match[2]),number(match[3])])
  if(!points.length||points.some(point=>point.some(value=>!Number.isFinite(value))))throw Error(`Invalid CARTESIAN_POINT_LIST_3D #${id}.`)
  pointLists.set(id,points)
 }
 const sets=[...entities.entries()].filter(([,entity])=>['TRIANGULATED_FACE_SET','COMPLEX_TRIANGULATED_FACE_SET'].includes(entity.type))
 if(!sets.length)throw Error('TESSELLATED_SHAPE_REPRESENTATION contains no supported triangulated face set.')
 const representation=[...entities.entries()].find(([,entity])=>entity.type==='TESSELLATED_SHAPE_REPRESENTATION')
 const representationIdentity=stepIdentity(representation?.[1].args??'TESSELLATED_SHAPE_REPRESENTATION')
 const presentations:StepTessellatedPresentation[]=[]
 const bodies=sets.map(([id,entity],ordinal)=>{
  const coordinateId=entity.args.match(/#(\d+)/)?.[1],points=coordinateId&&pointLists.get(coordinateId)
  if(!points)throw Error(`Triangulated face set #${id} has no CARTESIAN_POINT_LIST_3D coordinates.`)
  const fields=splitStepArgs(entity.args)
  let triples=indexRows(entity.args).filter(row=>row.length===3)
  let primitive:StepTessellatedPresentation['primitive']='triangles'
  if(entity.type==='COMPLEX_TRIANGULATED_FACE_SET'&&fields.length>=7){
   const strips=indexRows(fields[5]),fans=indexRows(fields[6])
   if(strips.length||fans.length){triples=[...strips.flatMap(stripTriangles),...fans.flatMap(fanTriangles)];primitive='strips-and-fans'}
  }
  triples=triples.filter(indices=>indices.every(index=>index>=1&&index<=points.length))
  if(!triples.length)throw Error(`Triangulated face set #${id} has no valid coordinate indices.`)
  const normals=(fields[3]?.match(/[+-]?(?:\d+\.?\d*|\.\d+)(?:[eEdD][+-]?\d+)?/g)??[]).map(number).filter(Number.isFinite)
  const colors=[...entities.entries()].filter(([,candidate])=>/COLOUR|STYLE/.test(candidate.type)
    &&(candidate.type==='COLOUR_RGB'||candidate.args.includes(`#${id}`))).map(([key])=>`#${key}`)
  const layers=[...entities.entries()].filter(([,candidate])=>candidate.type==='PRESENTATION_LAYER_ASSIGNMENT'&&candidate.args.includes(`#${id}`)).map(([key])=>`#${key}`)
  const textureCoordinates=[...entities.values()].filter(candidate=>/TEXTURE.*COORDINATE/.test(candidate.type)&&candidate.args.includes(`#${id}`))
    .flatMap(candidate=>(candidate.args.match(/[+-]?(?:\d+\.?\d*|\.\d+)(?:[eEdD][+-]?\d+)?/g)??[]).map(number).filter(Number.isFinite))
  presentations.push({representationIdentity,itemIdentity:stepIdentity(entity.type+'('+entity.args+')'),primitive,
    normals,colors,layers,textureCoordinates,source:`#${id}=${entity.type}(${entity.args});`})
  return {id:crypto.randomUUID(),name:`STEP tessellated ${ordinal+1}`,
    mesh:{positions:points.flat(),indices:triples.flatMap(triangle=>triangle.map(index=>index-1))}}
 })
 return {bodies,presentation:presentations,metadataLoss:[]}
}
export const importTessellatedStep=(text:string):DirectBody[]=>importTessellatedStepV10(text).bodies
