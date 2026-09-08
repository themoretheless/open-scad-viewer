import { compileTextNurbs } from './modelGraphTextNurbs'
import { compileModelGraph, ModelGraphError, type Expression } from './modelGraph'
import type { CustomizerParameter } from './scadCustomizer'

type Ast = { kind: string; value?: string; items?: Ast[]; args?: { name?: string; value: Ast }[]; left?: Ast; right?: Ast }
type Value = Expression | Value[] | { sequence: Expression } | { geometry: string } | { lambda: string; body: Ast; env: Map<string, Value> }
export const MODELGRAPH_TEXT_GUIDE = `ModelGraph Text/1 starts with // @modelgraph-text/1. Statements are separated by newlines or semicolons. Declare param radius = 20mm range 1mm..50mm, named parts with name = expression, and output with show name (otherwise the last named geometry is used). Expressions support + - * /, parentheses, numbers with mm/cm/m/in/deg/rad, and vectors. Calls use named arguments (radius: 20mm) or documented positional arguments. Pipelines pass geometry to the next operation: circle(20mm) |> extrude(10mm). Unary pure functions: double = x => x * 2. repeat(18, i => sphere(1mm) |> translate([i * 3mm, 0, 0])) makes a geometric pattern. Functions capture lexical bindings. No recursion, mutation or host-code evaluation. Repeat uses bounded ModelGraph map semantics. Supported calls: planetary_spinner(inner_radius,outer_radius,bore,gap,height,helix_angle) is the fixed 32/4/40-tooth, 18-planet herringbone prototype and must be the root; circle(radius), rectangle(size), box(size), sphere(radius), cylinder(radius,height), extrude(height), revolve(angle), translate(vector), rotate(vector), scale(vector), offset(distance), mirror(normal), union(parts...), intersection(parts...), subtract(parts...) after a pipe. Parameters require literal defaults; ranges use the same unit as the default. Fluent scalar checks: validate gap |> between(0mm, 0.6mm) |> message("Choose a valid gap"). Supported: greaterThan(x), atLeast(x), lessThan(x), atMost(x), equalTo(x), approximately(x, tolerance: t). assert also accepts scalar checks. between includes both endpoints. All scalar violations are collected in error.details before geometry emission; comparisons enforce compatible units. Geometry: assert body |> hasBodies(20) |> isWatertight() |> hasNoDegenerateTriangles(); assert measure(body).height |> approximately(10mm, tolerance: 0.01mm). Measurements width/depth/height are world-axis bounding-box extents in mm. Geometry assertions target only the final show root and survive in canonical geometry_assertions. Compile resolves expected values but does not execute geometry assertions; use modelgraph_check/report/export. Built checks report passed/failed/unknown, actual, expected and tolerance; failed or unknown checks block export. Body count means connected triangle components within each scene mesh, welding exact positions; not a mechanical assembly or collision test. Watertight tests closed oriented triangle topology, not self-intersections or printability. Checks have a 100000 triangle measurement budget; exceeding it returns unknown. Maximum 64 scalar constraints and 64 geometry checks; between uses two constraints. message accepts one JSON-escaped string up to 256 characters at the end of a chain. Ranges are expressions: 0..<18 excludes the end; 0..18 includes it when reached. Use 0mm..100mm by 5mm or 0deg..<360deg count 18. by and count are mutually exclusive. No implicit step for physical units or noninteger bounds. Without by/count, dimensionless integer ranges step +1; descending traversal requires negative by. Count is an integer 0..256; count 0 is empty, count 1 returns start; a nonempty exclusive equal-endpoint interval is invalid. Count samples include both endpoints with .., only start with ..<. Step ranges do not append unreachable endpoints. Compatible units are converted; zero/mismatched steps fail. Collection comprehensions: parts = [for angle in angles => planet.translate(x: 20mm).rotate(z: angle)]. Dotted calls are aliases for pipelines; translate/rotate/scale accept a vector or unique named x/y/z arguments. Generators support nested for clauses (Cartesian order), where predicates, immutable let bindings, comparisons == != < <= > >=, remainder %, and right-associative power **. Numeric examples: squares = [for i in 0..<8 where i % 2 == 0 let n = i ** 2 => n]. zip(a,b) requires equal lengths; enumerate(xs) yields index/value pairs, usable with for (i,value) in enumerate(xs). length(xs) and at(xs,index) inspect numeric sequences; indices are zero-based. Nested for clauses flatten their results. Each range/numeric generated sequence is limited to 256 elements; total evaluation allocation 16384, expanded geometry uses 4096 and graph depth 32, all enforced before geometry execution. show parts preserves separate geometry objects, including touching parts; union(parts) explicitly merges them. Use show [sun,ring,planets] for a group. Groups are not jointed assemblies. Transform each part in the generator or union the collection before transforming it; whole-collection transform syntax is intentionally rejected. Generated references use positional instance paths, not persistent user keys. No key clause, slicing, infinite ranges or lazy unbounded iteration yet. Range expressions and geometry collect nodes survive canonical JSON and recompute after modelgraph_set_parameters. Explicit reverse conversion: triangle_mesh(vertices,triangles) accepts indexed input. mesh_to_sdf() builds signed triangle distance for a closed oriented mesh; sdf_offset and sdf_tessellate work on the resulting field. mesh_to_subdivision(iterations:0..32) retains source topology and fits original-vertex samples at one refinement step; subdivision_tessellate(levels) emits its mesh. mesh_to_nurbs() creates exact faceted NURBS patches; mesh_fit_nurbs(max_deviation) creates approximate cubic point-normal patches, checking deviation on 9x9 samples per patch, not a certified bound. nurbs_patches_tessellate(segments) emits their mesh. mesh_to_nurbs_brep() builds exact planar trimmed NURBS topology, limited to 256 triangles, and works with brep_tessellate. These are explicit reconstruction choices, not recovery of unknown original CAD or coarse subdivision controls. Mesh sources are bounded; signed distance sources at most4096 triangles, NURBS/subdivision at most2048, sampled reconstruction work8million. Subdivision: subdivision(vertices,faces,levels) uniformly refines a convex polygon cage using own Rust Catmull-Clark (levels 0..5), emits a mesh and retains original face IDs. This is finite refinement, not exact limit evaluation; crease weights and adaptive subdivision are absent. Implicit fields: sdf_sphere(center,radius), sdf_box(center,half_size), sdf_torus(center,major_radius,minor_radius); sdf_union(a,b), sdf_intersection(a,b), sdf_difference(a,b), sdf_smooth_union(a,b,radius:3mm); pipe fields through sdf_offset(distance), sdf_translate(vector), sdf_tessellate(min,max,cells). Cells is a 3-vector of integers 1..64. Bounds must enclose the surface with positive boundary samples. CSG fields are not generally exact signed distances; extraction can miss sub-cell features. Field trees are bounded to 256 nodes/depth32. Both libraries exchange meshes with polygon-kernel; mesh CSG does not reconstruct control cages or implicit fields. Own Rust B-rep: brep_box(min,max) |> brep_tessellate(segments) creates a body and retains authored face IDs for surface selection. Own Rust geometry: nurbs_surface(degree_u,degree_v,knots_u,knots_v,control_points,weights), nurbs_curve(degree,knots,control_points,weights); pipe into surface_extrude(vector), surface_revolve(origin,axis,angle), tessellate(segments_u,segments_v), thicken(vector), transform(matrix). mesh_union(a,b), mesh_intersection(a,b), and a |> mesh_subtract(b) operate on two closed oriented meshes. These calls select modelgraph/nurbs-1 and own-nurbs execution; pass the returned document to modelgraph_nurbs_build/evaluate/export. They cannot be mixed with legacy primitives, groups, repeat or geometry assertions. NURBS compact compilation resolves numeric expressions and parameter defaults into a numeric document snapshot; edit the source parameters and recompile to recompute expressions. Dimensions are checked, bare lengths use mm and angles degrees. Curves must become tessellated surfaces to render. Source and Customizer remain the editable authoring representation. Existing ModelGraph/1 programs retain their original backend and behavior.`
export const isModelGraphText = (source: string) => /^\s*\/\/\s*@modelgraph-text\/1\b/.test(source)
export function compileModelGraphText(source: string) {
 if (source.length > 262144) throw new Error('ModelGraph Text exceeds 256 KiB.')
 const tokens: { text: string; start: number; end: number }[] = []
 const rx = /[ \t\r]+|\/\/[^\n]*|\/\*[\s\S]*?\*\/|\n|(?:\d+(?:\.(?!\.)\d*)?|\.\d+)(?:[eE][+-]?\d+)?(?:mm|cm|in|deg|rad|m)?|"(?:[^"\\\n]|\\.)*"|[A-Za-z_][A-Za-z_0-9]*|\|>|=>|\.\.<|\.\.|\*\*|==|!=|<=|>=|[()[\],.:;=+*/%<>-]/gy
 let at=0
 while(at<source.length){rx.lastIndex=at;const m=rx.exec(source);if(!m)throw new Error(`Unexpected character at ${at}: ${source[at]}`);at=rx.lastIndex;if(!/^[ \t\r]|^\/\//.test(m[0])&&!m[0].startsWith('/*'))tokens.push({text:m[0],start:m.index,end:at})}
 tokens.push({text:'EOF',start:at,end:at})
 let p=0,depth=0,serial=0
 const peek=()=>tokens[p]!.text
 const fail=(message:string):never=>{const t=tokens[p]!;const line=source.slice(0,t.start).split('\n').length;throw new Error(`ModelGraph Text line ${line}: ${message}`)}
 const take=(s?:string)=>{if(s&&peek()!==s)fail(`Expected ${s}, got ${peek()}`);return tokens[p++]!}
 const skip=()=>{while(peek()==='\n'||peek()===';')take()}
 const inner=()=>{while(peek()==='\n')take()}
 function argumentsList():NonNullable<Ast['args']> {
  take('(');inner();const args:NonNullable<Ast['args']>=[]
  while(peek()!==')'){
   let name:string|undefined
   if(tokens[p+1]?.text===':'){name=take().text;take(':')}
   args.push({name,value:expr()});inner();if(peek()!==',')break;take();inner()
  }
  take(')');return args
 }
 function expr(min=0):Ast {
  if(++depth>64)fail('Expression nesting exceeds 64')
  let a!:Ast;const t=take().text
  if(t.startsWith('"'))a={kind:'string',value:JSON.parse(t)}
  else if(t==='-')a={kind:'neg',left:expr(25)}
  else if(t==='('){inner();a=expr();inner();take(')')}
  else if(t==='['&& (inner(),peek()==='for')){
   const clauses:Ast[]=[]
   while(['for','let','where'].includes(peek())){
    const kind=take().text
    if(kind==='where')clauses.push({kind,left:expr(2)})
    else {
     const names:Ast[]=[]
     if(kind==='for'&&peek()==='('){take();do{names.push({kind:'name',value:take().text});if(peek()!==',')break;take()}while(true);take(')')}
     else names.push({kind:'name',value:take().text})
     if(names.some(n=>!/^[_A-Za-z]\w*$/.test(n.value!))||new Set(names.map(n=>n.value)).size!==names.length)fail('Invalid or duplicate binding')
     take(kind==='for'?'in':'=')
     clauses.push({kind,items:names,left:expr(2)})
    }
    inner()
   }
   take('=>');inner();a={kind:'comprehension',items:clauses,left:expr()};inner();take(']')
  }
  else if(t==='['){const items:Ast[]=[];inner();while(peek()!==']'){items.push(expr());inner();if(peek()!==',')break;take();inner()}take(']');a={kind:'array',items}}
  else if(/^\d|^\.\d/.test(t))a={kind:'number',value:t}
  else if(/^[A-Za-z_]\w*$/.test(t)&&t!=='EOF') {
   a={kind:'name',value:t}
   if(peek()==='=>'&&min<2){take();inner();a={kind:'lambda',value:t,left:expr()}}
   else if(peek()==='(')a={kind:'call',value:t,args:argumentsList()}
  } else fail(`Expected expression, got ${t}`)
  while(peek()==='.'){
   take();const name=take().text
   a=peek()==='('?{kind:'pipe',left:a,right:{kind:'call',value:name,args:argumentsList()}}:{kind:'member',left:a,value:name}
  }
  while(true){if(peek()==='\n'&&tokens[p+1]?.text==='|>')take();const op=peek(),prec=op==='|>'?1:['==','!=','<','<=','>','>='].includes(op)?3:['..','..<'].includes(op)?5:['+','-'].includes(op)?10:['*','/','%'].includes(op)?20:op==='**'?25:0;if(!prec||prec<min)break;take();inner();
   if(op==='..'||op==='..<'){
    const start=a,end=expr(6);let mode:string|undefined,amount:Ast|undefined
    if(peek()==='by'||peek()==='count'){mode=take().text;amount=expr(6)}
    if(peek()==='by'||peek()==='count')fail('Use either by or count, never both')
    a={kind:'interval',value:op,left:start,right:end,args:mode?[{name:mode,value:amount!}]:[]}
   }else a={kind:op==='|>'?'pipe':'binary',value:op,left:a,right:expr(op==='**'?prec:prec+1)}}
  depth--;return a!
 }
 const constraints:Record<string,unknown>[]=[];const checks:Record<string,unknown>[]=[]
 const nodes:Record<string,unknown>[]=[];const parameters:Record<string,unknown>[]=[];const customizer:CustomizerParameter[]=[];const env=new Map<string,Value>();let root=''
 const add=(node:Record<string,unknown>)=>{if(nodes.length>=128)fail('At most 128 geometry nodes');const id=`n${++serial}`;nodes.push({id,...node});return {geometry:id}}
 const collections=new Set<string>()
 const group=(values:Value[])=>{const g=add({op:'group',inputs:values.map(geometry)});collections.add(g.geometry);return g}
 const geometry=(v:Value):string=>{if(Array.isArray(v))return group(v).geometry;if(v&&typeof v==='object'&&!Array.isArray(v)&&'geometry'in v)return v.geometry;return fail('Expected geometry')}
 const scalar=(v:Value):Expression=>{if(Array.isArray(v)||typeof v==='object'&&('geometry'in v||'lambda'in v||'sequence'in v))return fail('Expected scalar');return v as Expression}
 const expression=(v:Value):Expression=>{
  if(Array.isArray(v))return {op:'list',items:v.map(expression)}
  if(v&&typeof v==='object'&&'sequence'in v)return v.sequence
  return scalar(v)
 }
 const list=(v:Value):Expression=>{
  if(Array.isArray(v))return {op:'list',items:v.map(expression)}
  if(v&&typeof v==='object'&&'sequence'in v)return v.sequence
  return fail('Expected a sequence')
 }
 const markCount=(v:Expression)=>{
  if(typeof v==='object'&&'param'in v){
   const parameter=parameters.find(p=>p.id===v.param),control=customizer.find(p=>p.name===v.param)
   if(parameter)parameter.integer=true;if(control)control.step=1
  }
 }
 const signatures:Record<string,string[]>={polygon_profile:['outer','holes'],polygon_extrude:['vector'],polygon_sweep:['path','up'],polygon_loft:['sections'],triangle_mesh:['vertices','triangles'],mesh_to_nurbs_brep:[],mesh_to_sdf:[],mesh_to_subdivision:['iterations'],subdivision_tessellate:['levels'],mesh_to_nurbs:[],mesh_fit_nurbs:['max_deviation'],nurbs_patches_tessellate:['segments'],subdivision:['vertices','faces','levels'],sdf_sphere:['center','radius'],sdf_box:['center','half_size'],sdf_torus:['center','major_radius','minor_radius'],sdf_tessellate:['min','max','cells'],sdf_offset:['distance'],sdf_translate:['vector'],brep_box:['min','max'],brep_tessellate:['segments'],nurbs_surface:['degree_u','degree_v','knots_u','knots_v','control_points','weights'],nurbs_curve:['degree','knots','control_points','weights'],surface_extrude:['vector'],surface_revolve:['origin','axis','angle'],tessellate:['segments_u','segments_v'],thicken:['vector'],transform:['matrix'],planetary_spinner:['inner_radius','outer_radius','bore','gap','height','helix_angle'],circle:['radius'],rectangle:['size'],box:['size'],sphere:['radius'],cylinder:['radius','height'],extrude:['height'],revolve:['angle'],translate:['vector'],rotate:['vector'],scale:['vector'],offset:['distance'],mirror:['normal']}
 function evaluate(a:Ast,e:Map<string,Value>,budget=0):Value {
  if(budget>64)fail('Function expansion exceeds 64')
  const ev=(b:Ast)=>evaluate(b,e,budget+1)
  if(a.kind==='number'){const m=a.value!.match(/^([\d.eE+-]+)(mm|cm|m|in|deg|rad)?$/)!;return m[2]?{op:'quantity',value:Number(m[1]),unit:m[2] as 'mm'}:Number(m[1])}
  if(a.kind==='name'){if(!e.has(a.value!))fail(`Unknown name ${a.value}`);return e.get(a.value!)!}
  if(a.kind==='array')return a.items!.map(ev)
  if(a.kind==='interval'){
   const mode=a.args![0],amount=mode?scalar(ev(mode.value)):undefined
   if(mode?.name==='count')markCount(amount!)
   return {sequence:{op:'interval',start:scalar(ev(a.left!)),end:scalar(ev(a.right!)),inclusive:a.value==='..',...(mode?{[mode.name==='by'?'step':'count']:amount}:{})}}
  }
  if(a.kind==='comprehension'){
   const lower=(offset:number,outer:Map<string,Value>):Value=>{
    const clause=a.items![offset]!
    if(clause.kind!=='for')return fail('Generator must start with for')
    let input=list(evaluate(clause.left!,outer,budget+1))
    const binding=`v${++serial}`,scope=new Map(outer)
    for(const [i,name] of clause.items!.entries())scope.set(name.value!,clause.items!.length===1?{local:binding}:{op:'at',input:{local:binding},index:i})
    let next=offset+1,condition:Expression|undefined
    while(next<a.items!.length&&a.items![next]!.kind!=='for'){
     const c=a.items![next++]!
     if(c.kind==='let')scope.set(c.items![0]!.value!,evaluate(c.left!,scope,budget+1))
     else {const v=scalar(evaluate(c.left!,scope,budget+1));condition=condition?{op:'and',args:[condition,v]}:v}
    }
    if(condition)input={op:'filter',input,function:{op:'lambda',parameters:[binding],body:condition}}
    const child=next<a.items!.length?lower(next,scope):evaluate(a.left!,scope,budget+1)
    if(child&&typeof child==='object'&&!Array.isArray(child)&&'geometry'in child){
     const result=add({op:'collect',values:input,binding,input:child.geometry});collections.add(result.geometry);return result
    }
    return {sequence:{op:next<a.items!.length?'flatmap':'map',input,function:{op:'lambda',parameters:[binding],body:expression(child)}}}
   }
   return lower(0,e)
  }
  if(a.kind==='lambda')return {lambda:a.value!,body:a.left!,env:new Map(e)}
  if(a.kind==='neg')return {op:'negate',value:scalar(ev(a.left!))}
  if(a.kind==='binary'){
   let left=scalar(ev(a.left!)),right=scalar(ev(a.right!));const op=a.value!
   if(op==='>'||op==='>=')[left,right]=[right,left]
   const operations:Record<string,'add'|'subtract'|'multiply'|'divide'|'mod'|'pow'|'lt'|'le'|'eq'>={'+':'add','-':'subtract','*':'multiply','/':'divide','%':'mod','**':'pow','<':'lt','>':'lt','<=':'le','>=':'le','==':'eq','!=':'eq'}
   const result:Expression={op:operations[op]!,args:[left,right]}
   return op==='!='?{op:'not',value:result}:result
  }
  let call=a,input:string|undefined
  if(a.kind==='pipe'){input=geometry(ev(a.left!));call=a.right!;if(call.kind!=='call')fail('Pipeline requires an operation call')}
  if(call.kind!=='call')return fail('Expected a call')
  if(['zip','enumerate','length','at'].includes(call.value!)){
   if(input||call.args!.some(arg=>arg.name))fail('Sequence functions require positional arguments')
   const args=call.args!.map(arg=>ev(arg.value))
   if(call.value==='zip'){if(args.length<2||args.length>8)fail('zip expects 2..8 sequences');return {sequence:{op:'zip',inputs:args.map(list)}}}
   if(args.length!==(call.value==='at'?2:1))fail('Invalid sequence function arity')
   const sequence=list(args[0]!)
   if(call.value==='length')return {op:'length',input:sequence}
   if(call.value==='at')return {op:'at',input:sequence,index:scalar(args[1]!)}
   return {sequence:{op:'enumerate',input:sequence}}
  }
  if(call.value==='repeat'){
   if(input||call.args!.length!==2||call.args!.some(x=>x.name))fail('repeat(count, i => geometry) expected')
   const count=scalar(ev(call.args![0]!.value)),fn=ev(call.args![1]!.value)
   if(typeof count==='object'&&'param'in count){const parameter=parameters.find(p=>p.id===count.param),control=customizer.find(p=>p.name===count.param);if(parameter)parameter.integer=true;if(control)control.step=1}
   if(typeof fn!=='object'||Array.isArray(fn)||!('lambda'in fn))return fail('repeat requires a lambda')
   const index=`i${++serial}`,scope=new Map(fn.env);scope.set(fn.lambda,{local:index})
   return add({op:'map',count,index,input:geometry(evaluate(fn.body,scope,budget+1))})
  }
  const fn=e.get(call.value!)
  if(fn&&typeof fn==='object'&&!Array.isArray(fn)&&'lambda'in fn){if(input||call.args!.length!==1||call.args![0]!.name)fail('Unary function expects one positional argument');const scope=new Map(fn.env);scope.set(fn.lambda,ev(call.args![0]!.value));return evaluate(fn.body,scope,budget+1)}
  const args=call.args!.map(arg=>({name:arg.name,value:ev(arg.value)}))
  if(['surface_sweep','surface_loft'].includes(call.value!)){
   if(args.some(a=>a.name))fail('Surface construction operands are positional');const inputs=args.map(a=>geometry(a.value));if(input)inputs.unshift(input);return add({op:call.value,inputs});
  }
  if(['sdf_union','sdf_intersection','sdf_difference','sdf_smooth_union'].includes(call.value!)) {
   const smooth=call.value==='sdf_smooth_union';
   const radiusArg=smooth?args.find(a=>a.name==='radius'):undefined;
   const geometryArgs=args.filter(a=>a!==radiusArg);
   if(geometryArgs.some(a=>a.name))fail('SDF operands are positional; smooth radius is named');
   const inputs=geometryArgs.map(a=>geometry(a.value));if(input)inputs.unshift(input);
   if(inputs.length!==2 || smooth&&!radiusArg)fail('SDF operation requires two fields; smooth union also requires radius');
   return add({op:call.value,inputs,...(radiusArg?{radius:radiusArg.value}:{})});
  }
  if(['mesh_union','mesh_intersection','mesh_subtract'].includes(call.value!)) {
   if(args.some(x=>x.name))fail('Mesh Boolean arguments are positional')
   const inputs=args.map(x=>geometry(x.value));if(input)inputs.unshift(input)
   if(inputs.length!==2)fail('Mesh Boolean operation requires exactly two meshes')
   return add({op:'mesh_boolean',inputs,operation:call.value==='mesh_union'?'union':call.value==='mesh_intersection'?'intersection':'difference'})
  }
  if(['union','intersection','subtract'].includes(call.value!)){if(args.some(x=>x.name))fail('Boolean arguments are positional');const inputs=args.map(x=>geometry(x.value));if(input)inputs.unshift(input);if(!inputs.length)fail('Boolean operation needs geometry');return call.value==='subtract'?add({op:'difference',base:inputs[0],subtract:inputs.slice(1)}):add({op:call.value,inputs})}
  if(['translate','rotate','scale'].includes(call.value!)&&args.some(a=>['x','y','z'].includes(a.name??''))){
   if(args.some(a=>!['x','y','z'].includes(a.name??''))||new Set(args.map(a=>a.name)).size!==args.length)fail('Use unique x/y/z arguments or one vector')
   const vector=['x','y','z'].map(axis=>args.find(a=>a.name===axis)?.value??(call.value==='scale'?1:0))
   args.splice(0,args.length,{name:'vector',value:vector})
  }
  const signature=signatures[call.value!];if(!signature)fail(`Unknown operation ${call.value}`)
  const fields:Record<string,unknown>={};for(let i=0;i<args.length;i++){const key=args[i]!.name??signature![i];if(!key||!signature!.includes(key)||key in fields)fail('Unknown or duplicate argument');fields[key]=args[i]!.value}
  const modifier=['polygon_extrude','polygon_sweep','mesh_to_nurbs_brep','mesh_to_sdf','mesh_to_subdivision','subdivision_tessellate','mesh_to_nurbs','mesh_fit_nurbs','nurbs_patches_tessellate','sdf_tessellate','sdf_offset','sdf_translate','brep_tessellate','surface_extrude','surface_revolve','tessellate','thicken','transform','extrude','revolve','translate','rotate','scale','offset','mirror'].includes(call.value!)
  if(modifier&&!input)fail(`${call.value} requires piped geometry`);if(!modifier&&input)fail(`${call.value} does not accept piped geometry`)
  if(input&&collections.has(input))fail('Transform each generated part inside the generator, or explicitly union the collection first')
  return add({op:call.value,...fields,...(input?{input}:{})})
 }
 function checkStatement(kind:string) {
  const chain:Ast[]=[];let subject=expr()
  while(subject.kind==='pipe'){chain.unshift(subject.right!);subject=subject.left!}
  if(!chain.length)fail('Check requires a pipeline')
  let message=kind+' check failed'
  const tail=chain.at(-1)!
  if(tail.kind==='call'&&tail.value==='message'){
   if(tail.args!.length!==1||tail.args![0]!.name||tail.args![0]!.value.kind!=='string')fail('message requires one string')
   message=tail.args![0]!.value.value!;chain.pop()
  }
  if(!chain.length)fail('At least one check is required')
  const measurement=subject.kind==='member'&&subject.left?.kind==='call'&&subject.left.value==='measure'
  let target:string|undefined
  let left:Expression|undefined
  if(measurement){if(kind!=='assert'||subject.left!.args!.length!==1||subject.left!.args![0]!.name)fail('assert measure(root).height/width/depth expected');target=geometry(evaluate(subject.left!.args![0]!.value,env))}
  else {const v=evaluate(subject,env);if(v&&typeof v==='object'&&!Array.isArray(v)&&'geometry'in v){if(kind!=='assert')fail('Use assert for geometry');target=v.geometry}else left=scalar(v)}
  for(const c of chain){
   if(c.kind!=='call')fail('Expected a check call')
   const args=c.args!
   const values=args.map(a=>scalar(evaluate(a.value,env)))
   const rule=c.value!
   if(target){
    if(measurement){if(!['height','width','depth'].includes(subject.value!)||rule!=='approximately'||args.length<1||args.length>2||args[0]!.name||(args.length===2&&args[1]!.name!=='tolerance'))fail('Measurement requires approximately(value, tolerance: value)')}
    else if(!['hasBodies','isWatertight','hasNoDegenerateTriangles'].includes(rule)||args.some(a=>a.name)||args.length!==(rule==='hasBodies'?1:0))fail('Unknown geometry check or invalid arguments')
    checks.push({id:`g${checks.length+1}`,target,check:measurement?subject.value:rule,message,...(values.length?{expected:values[0]}:{}),...(values.length>1?{tolerance:values[1]}:{})})
   }else {
    const relations:Record<string,string>={greaterThan:'gt',atLeast:'ge',lessThan:'lt',atMost:'le',equalTo:'eq',approximately:'eq'}
    if(rule==='between'){
     if(values.length!==2||args.some(a=>a.name))fail('between(min,max) expected')
     for(const [i,relation] of ['ge','le'].entries())constraints.push({id:`v${constraints.length+1}`,left,relation,right:values[i],message})
    }else {
     if(!relations[rule]||values.length<1||values.length>(rule==='approximately'?2:1)||args[0]!.name||(values.length===2&&args[1]!.name!=='tolerance'))fail('Unknown scalar check or invalid arguments')
     constraints.push({id:`v${constraints.length+1}`,left,relation:relations[rule],right:values[0],message,...(values.length===2?{tolerance:values[1]}:{})})
    }
   }
  }
 }
 skip();while(peek()!=='EOF'){
  if(peek()==='validate'||peek()==='assert'){checkStatement(take().text)}
  else if(peek()==='param'){
   take();const name=take().text;if(env.has(name))fail(`Duplicate name ${name}`);take('=');const start=tokens[p]!.start;let sign=1;if(peek()==='-'){sign=-1;take()}const token=take();const m=token.text.match(/^(\d+(?:\.\d*)?(?:[eE][+-]?\d+)?|\.\d+)(mm|cm|m|in|deg|rad)?$/);if(!m)fail('Parameter default must be a numeric literal')
   const value=sign*Number(m![1]),unit=m![2],parameter:Record<string,unknown>={id:name,value,...(unit?{unit}:{})};const control:CustomizerParameter={name,label:unit?`${name} (${unit})`:name,value,valueStart:start,valueEnd:token.end-(unit?.length??0)}
   if(peek()==='range'){take();const bound=()=>{let s=1;if(peek()==='-'){take();s=-1}const b=take().text.match(/^([\d.eE+-]+)(mm|cm|m|in|deg|rad)?$/);if(!b||b[2]!==unit)fail('Range units must match parameter units');return s*Number(b![1])};parameter.min=control.min=bound();take('..');parameter.max=control.max=bound();if(control.min!>control.max!||value<control.min!||value>control.max!)fail('Parameter default must lie in its range')}
   parameters.push(parameter);customizer.push(control);env.set(name,{param:name})
  }else if(peek()==='show'){take();root=geometry(evaluate(expr(),env))}
  else {const name=take().text;if(env.has(name))fail(`Duplicate name ${name}`);take('=');const value=evaluate(expr(),env);env.set(name,value);if(value&&typeof value==='object'&&!Array.isArray(value)&&'geometry'in value)root=value.geometry}
  if(!['EOF','\n',';'].includes(peek()))fail(`Unexpected ${peek()}`);skip()
 }
 if(!root)fail('No geometry to show')
 try {
 if(nodes.some(n=>['polygon_profile','polygon_loft','triangle_mesh','subdivision','sdf_sphere','sdf_box','sdf_torus','brep_box','nurbs_surface','nurbs_curve','mesh_boolean'].includes(String(n.op)))) {
  if(constraints.length||checks.length)fail('NURBS text checks are not supported; use the build topology report')
  const compiled=compileTextNurbs(nodes,parameters,root)
  return {...compiled,customizer,execution_target:'own-nurbs' as const,source:'',source_map:[],geometry_assertions:[],constraint_report:[],sketch_solutions:[],assembly_components:[],mechanical_reports:[],mechanical_parts:[]}
 }
 const compiled=compileModelGraph({language:'modelgraph/1',units:'mm',parameters,nodes,root,...(constraints.length?{constraints}:{}),...(checks.length?{geometry_assertions:checks}:{})})
 return {...compiled,customizer}
 } catch(error) {
  if(error instanceof ModelGraphError) Object.assign(error,{customizer})
  throw error
 }
}

export function modelGraphTextControls(source:string): {parameters:CustomizerParameter[]; errors:string[]} {
 try { return {parameters:compileModelGraphText(source).customizer,errors:[]} }
 catch(error) {
  if(error instanceof ModelGraphError && 'customizer' in error) return {
   parameters:error.customizer as CustomizerParameter[],
   errors:Array.isArray(error.details) ? error.details.filter(c=>!c.passed).map(c=>c.message) : [error.message],
  }
  return {parameters:[],errors:[]}
 }
}
