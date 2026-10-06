import type {DirectDocument} from './directModeling'
import {warmGeometryKernel} from './geometry/kernel'
import {solidMaterialRgb} from './solidMaterial'

// Part 21 transport only: all geometry is exported and validated by the Rust kernel.
const quoted = (value:string) => "'" + Array.from(value).map(c => {
  const n=c.codePointAt(0)!
  if(c==="'")return "''"
  if(n>=32&&n<=126&&c!=='\\')return c
  return n<=0xffff?`\\X2\\${n.toString(16).padStart(4,'0').toUpperCase()}\\X0\\`:`\\X4\\${n.toString(16).padStart(8,'0').toUpperCase()}\\X0\\`
}).join('') + "'"

/** Remap entity references without interpreting hashes inside STEP string literals. */
function remap(text:string,offset:number) {
  return text.replace(/'(?:[^']|'')*'|#(\d+)/g,(token,id:string|undefined)=>id===undefined?token:`#${Number(id)+offset}`)
}

/** Current Solid hierarchy: root → flat groups → bodies, including all hidden bodies.
 * Sketches/curves/surfaces are refused rather than silently omitted.
 */
export async function exportSolidStepAssembly(document:DirectDocument):Promise<string> {
  if(document.sourceBodies?.length)throw Error('STEP assembly cannot yet transfer exact source Body restrictions. Keep the native document.')
  const snapshot=structuredClone(document)
  if(!snapshot.bodies.length)throw Error('STEP assembly requires at least one body.')
  if(new Set(snapshot.bodies.map(body=>body.id)).size!==snapshot.bodies.length)throw Error('STEP assembly requires unique body IDs.')
  if(snapshot.sketches.length||snapshot.curves?.length||snapshot.surfaces?.length)
    throw Error('STEP assembly currently requires a body-only document. Export sketches separately as SVG and remove auxiliary curves/surfaces from a copy.')
  const missing=snapshot.bodies.filter(body=>!body.brep)
  if(missing.length)throw Error(`STEP assembly requires exact B-rep geometry: ${missing.map(b=>b.name).join(', ')}`)
  await warmGeometryKernel()
  const {exportDirectStepV9}=await import('./cadNurbsStep')
  const rows:string[]=[]
  let next=1
  const emit=(row:string)=>{const id=next++;rows.push(`#${id}=${row};`);return id}
  const app=emit("APPLICATION_CONTEXT('managed model based 3d engineering')")
  const pc=emit(`PRODUCT_CONTEXT('',#${app},'mechanical')`)
  const dc=emit(`PRODUCT_DEFINITION_CONTEXT('part definition',#${app},'design')`)
  const unit=emit('(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))')
  const context=emit(`(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#${unit}))REPRESENTATION_CONTEXT('','3D'))`)
  const point=emit("CARTESIAN_POINT('',(0.,0.,0.))")
  const axis=emit(`AXIS2_PLACEMENT_3D('',#${point},$,$)`)
  const transformation=emit(`ITEM_DEFINED_TRANSFORMATION('','',#${axis},#${axis})`)
  const product=(id:string,name:string)=>{
    const p=emit(`PRODUCT(${quoted(id)},${quoted(name)},'',(#${pc}))`)
    const f=emit(`PRODUCT_DEFINITION_FORMATION('1','',#${p})`)
    const definition=emit(`PRODUCT_DEFINITION('design','',#${f},#${dc})`)
    const shape=emit(`PRODUCT_DEFINITION_SHAPE('','',#${definition})`)
    const representation=emit(`SHAPE_REPRESENTATION('',(#${axis}),#${context})`)
    emit(`SHAPE_DEFINITION_REPRESENTATION(#${shape},#${representation})`)
    return {definition,representation}
  }
  const root=product('solid-scene','Solid scene')
  type Node=typeof root
  const attach=(parent:Node,child:Node,id:string,name:string)=>{
    const usage=emit(`NEXT_ASSEMBLY_USAGE_OCCURRENCE(${quoted(id)},${quoted(name)},'',#${parent.definition},#${child.definition},$)`)
    const shape=emit(`PRODUCT_DEFINITION_SHAPE('','',#${usage})`)
    const relation=emit(`(REPRESENTATION_RELATIONSHIP('','',#${child.representation},#${parent.representation})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#${transformation})SHAPE_REPRESENTATION_RELATIONSHIP())`)
    emit(`CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#${relation},#${shape})`)
  }
  const groups=new Map<string,Node>()
  for(const name of new Set([...(snapshot.groups??[]).map(g=>g.name),...snapshot.bodies.flatMap(b=>b.group?[b.group]:[])])){
    const group=product(`group:${name}`,name);groups.set(name,group);attach(root,group,`group:${name}`,name)
  }
  for(const body of snapshot.bodies){
    const exported=exportDirectStepV9(body.brep!).text
    const data=exported.split('\nDATA;\n')[1]?.split('\nENDSEC;')[0]
    if(!data)throw Error('Unexpected native STEP export format.')
    const offset=next-1
    const mapped=remap(data,offset)
    const definition=Number(mapped.match(/^#(\d+)=PRODUCT_DEFINITION\(/m)?.[1])
    const representation=Number(mapped.match(/^#(\d+)=(?:ADVANCED_BREP|MANIFOLD_SURFACE)_SHAPE_REPRESENTATION\(/m)?.[1])
    if(!definition||!representation)throw Error('Native STEP export is missing its product representation.')
    const renamed=mapped.replace(/PRODUCT\('model','model'/,()=>`PRODUCT(${quoted(body.id)},${quoted(body.name)}`)
    rows.push(renamed)
    for(const match of mapped.matchAll(/^#(\d+)=/gm))next=Math.max(next,Number(match[1])+1)
    if(body.material){
      const color=emit(`COLOUR_RGB(${quoted(body.material.name)},${solidMaterialRgb(body.material.color).map(c=>c===0?'0.':c===1?'1.':String(c)).join(',')})`)
      const fillColor=emit(`FILL_AREA_STYLE_COLOUR('',#${color})`)
      const fill=emit(`FILL_AREA_STYLE('',(#${fillColor}))`)
      const surfaceFill=emit(`SURFACE_STYLE_FILL_AREA(#${fill})`)
      const side=emit(`SURFACE_SIDE_STYLE('',(#${surfaceFill}))`)
      const usage=emit(`SURFACE_STYLE_USAGE(.BOTH.,#${side})`)
      const assignment=emit(`PRESENTATION_STYLE_ASSIGNMENT((#${usage}))`)
      const items=Array.from(mapped.matchAll(/^#(\d+)=(?:MANIFOLD_SOLID_BREP|BREP_WITH_VOIDS|SHELL_BASED_SURFACE_MODEL)\(/gm),m=>emit(`STYLED_ITEM('',(#${assignment}),#${m[1]})`))
      if(items.length)emit(`MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',(${items.map(id=>'#'+id).join(',')}),#${context})`)
    }
    attach(body.group?groups.get(body.group)!:root,{definition,representation},body.id,body.name)
  }
  return "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('Current Solid assembly'),'2;1');\nFILE_NAME('scene.step','',(''),(''),'OpenSCAD Viewer','OpenSCAD Viewer','');\nFILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));\nENDSEC;\nDATA;\n"+rows.join('\n')+'\nENDSEC;\nEND-ISO-10303-21;\n'
}
