import { checkModelGraphNumericType, type ModelGraphNumericType } from '../../src/services/modelGraphNumericType'
/** Lower compact expressions to a numeric NURBS document; source remains editable. */
import { compileModelGraphNurbs } from './modelGraphNurbs-runtime-reference'
import { createUnitArithmetic, LENGTH, SCALAR, ANGLE, type NumericValue, type Dimension, type Unit } from '../../src/services/modelGraphUnits'
export function compileTextNurbs(nodes: Record<string, unknown>[], parameters: Record<string, unknown>[], root: string) {
  const fail = (_code: string, path: string, message: string): never => { throw new Error(`ModelGraph Text ${path}: ${message}`) }
  const math = createUnitArithmetic(fail)
  const values = new Map(parameters.map(p => [p.id, p.unit ? math.quantity(p.value as number, p.unit as Unit, String(p.id)) : p.value as number]))
  function scalar(v: unknown, path: string, depth = 0): NumericValue {
    if (depth > 64) return fail('',path,'Expression depth exceeds 64')
    if (typeof v === 'number') return math.field(v,SCALAR,path,false)
    if (!v || typeof v !== 'object' || Array.isArray(v)) return fail('',path,'Expected a scalar expression')
    const e=v as Record<string,unknown>
    if ('param' in e) return values.get(e.param) ?? fail('',path,'Unknown parameter')
    if (e.op === 'checked') {
      for(const check of e.checks as unknown[])scalar(check,path,depth+1)
      return scalar(e.value,path,depth+1)
    }
    if (e.op === 'typed') return checkModelGraphNumericType(scalar(e.value,path,depth+1),e.type as ModelGraphNumericType,message=>fail('',path,message))
    if (e.op === 'if') return scalar(math.scalar(scalar(e.condition,path,depth+1),path)!==0?e.then:e.else,path,depth+1)
    if (e.op === 'quantity') return math.quantity(e.value as number,e.unit as Unit,path)
    if (Array.isArray(e.args) && e.args.length === 2) return math.binary(String(e.op),scalar(e.args[0],path,depth+1),scalar(e.args[1],path,depth+1),path)
    if ('value' in e) return math.unary(String(e.op),scalar(e.value,path,depth+1),path,false)
    return fail('',path,'This sequence/expression is not supported in a NURBS field')
  }
  function field(v: unknown, dimension: Dimension, path: string): unknown {
    return Array.isArray(v) ? v.map((x,i)=>field(x,dimension,`${path}/${i}`)) : math.field(scalar(v,path),dimension,path,false)
  }
  // Own geometry documents are numeric snapshots: resolve choices before
  // lowering fields so invalid dimensions in an unselected branch stay unused.
  if(nodes.some(node=>node.op==='if')) {
    const byId=new Map(nodes.map(node=>[String(node.id),node]))
    const selected=new Map<string,Record<string,unknown>>()
    const visit=(id:string,depth=0):string=>{
      if(depth>64) return fail('',id,'Conditional geometry depth exceeds 64')
      const node=byId.get(id)
      if(!node)return fail('',id,'Unknown geometry node')
      if(node.op==='if')return visit(String(math.scalar(scalar(node.condition,id),id)!==0?node.then:node.else),depth+1)
      if(selected.has(id))return id
      const n={...node};selected.set(id,n)
      if(typeof n.input==='string')n.input=visit(n.input,depth+1)
      if(Array.isArray(n.inputs))n.inputs=n.inputs.map(input=>visit(String(input),depth+1))
      return id
    }
    root=visit(root)
    nodes=[...selected.values()]
  }
  const lowered=nodes.map(node=>{
    const n:Record<string,unknown>={...node}
    if(n.op==='nurbs_surface')n.op='surface'
    if(n.op==='nurbs_curve')n.op='curve'
    const supported=['polygon_profile','polygon_extrude','polygon_sweep','polygon_loft','surface_sweep','surface_loft','extrude','revolve','triangle_mesh','mesh_to_nurbs_brep','mesh_to_sdf','mesh_to_subdivision','subdivision_tessellate','mesh_to_nurbs','mesh_fit_nurbs','nurbs_patches_tessellate','subdivision','sdf_sphere','sdf_box','sdf_torus','sdf_union','sdf_intersection','sdf_difference','sdf_smooth_union','sdf_offset','sdf_translate','sdf_tessellate','brep_box','brep_tessellate','surface','curve','surface_extrude','surface_revolve','tessellate','thicken','mesh_boolean','transform']
    if(!supported.includes(String(n.op)))fail('',String(n.id),`Operation ${n.op} cannot be mixed with own NURBS/mesh operations`)
    for(const [key,value] of Object.entries(n)) {
      if(['id','op','input','inputs','operation'].includes(key))continue
      if(key==='matrix') {
        if(!Array.isArray(value))fail('',key,'Expected a matrix')
        n[key]=(value as unknown[][]).map((row,i)=>row.map((x,j)=>field(x,i<3&&j===3?LENGTH:SCALAR,`${key}/${i}/${j}`)))
      } else n[key]=field(value,['height','outer','holes','sections','path','max_deviation','vertices','center','half_size','radius','major_radius','minor_radius','distance','min','max','control_points','vector','origin'].includes(key)?LENGTH:key==='angle'?ANGLE:SCALAR,key)
    }
    return n
  })
  return compileModelGraphNurbs({language:'modelgraph/nurbs-1',units:'mm',parameters:[],nodes:lowered,root})
}
