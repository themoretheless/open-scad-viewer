import { compileModelGraph, type Expression } from './modelGraph'
import type { CustomizerParameter } from './scadCustomizer'

type Ast = { kind: string; value?: string; items?: Ast[]; args?: { name?: string; value: Ast }[]; left?: Ast; right?: Ast }
type Value = Expression | Value[] | { geometry: string } | { lambda: string; body: Ast; env: Map<string, Value> }
export const MODELGRAPH_TEXT_GUIDE = `ModelGraph Text/1 starts with // @modelgraph-text/1. Statements are separated by newlines or semicolons. Declare param radius = 20mm range 1mm..50mm, named parts with name = expression, and output with show name (otherwise the last named geometry is used). Expressions support + - * /, parentheses, numbers with mm/cm/m/in/deg/rad, and vectors. Calls use named arguments (radius: 20mm) or documented positional arguments. Pipelines pass geometry to the next operation: circle(20mm) |> extrude(10mm). Unary pure functions: double = x => x * 2. repeat(18, i => sphere(1mm) |> translate([i * 3mm, 0, 0])) makes a geometric pattern. Functions capture lexical bindings. No recursion, mutation or host-code evaluation. Repeat uses bounded ModelGraph map semantics. Supported calls: planetary_spinner(inner_radius,outer_radius,bore,gap,height,helix_angle) is the fixed 32/4/40-tooth, 18-planet herringbone prototype and must be the root; circle(radius), rectangle(size), box(size), sphere(radius), cylinder(radius,height), extrude(height), revolve(angle), translate(vector), rotate(vector), scale(vector), offset(distance), mirror(normal), union(parts...), intersection(parts...), subtract(parts...) after a pipe. Parameters require literal defaults; ranges use the same unit as the default. Existing JSON ModelGraph remains canonical.`
export const isModelGraphText = (source: string) => /^\s*\/\/\s*@modelgraph-text\/1\b/.test(source)
export function compileModelGraphText(source: string) {
 if (source.length > 262144) throw new Error('ModelGraph Text exceeds 256 KiB.')
 const tokens: { text: string; start: number; end: number }[] = []
 const rx = /[ \t\r]+|\/\/[^\n]*|\/\*[\s\S]*?\*\/|\n|(?:\d+(?:\.(?!\.)\d*)?|\.\d+)(?:[eE][+-]?\d+)?(?:mm|cm|in|deg|rad|m)?|[A-Za-z_][A-Za-z_0-9]*|\|>|=>|\.\.|[()[\],:;=+*/-]/gy
 let at=0
 while(at<source.length){rx.lastIndex=at;const m=rx.exec(source);if(!m)throw new Error(`Unexpected character at ${at}: ${source[at]}`);at=rx.lastIndex;if(!/^[ \t\r]|^\/\//.test(m[0])&&!m[0].startsWith('/*'))tokens.push({text:m[0],start:m.index,end:at})}
 tokens.push({text:'EOF',start:at,end:at})
 let p=0,depth=0,serial=0
 const peek=()=>tokens[p]!.text
 const fail=(message:string):never=>{const t=tokens[p]!;const line=source.slice(0,t.start).split('\n').length;throw new Error(`ModelGraph Text line ${line}: ${message}`)}
 const take=(s?:string)=>{if(s&&peek()!==s)fail(`Expected ${s}, got ${peek()}`);return tokens[p++]!}
 const skip=()=>{while(peek()==='\n'||peek()===';')take()}
 const inner=()=>{while(peek()==='\n')take()}
 function expr(min=0):Ast {
  if(++depth>64)fail('Expression nesting exceeds 64')
  let a!:Ast;const t=take().text
  if(t==='-')a={kind:'neg',left:expr(30)}
  else if(t==='('){inner();a=expr();inner();take(')')}
  else if(t==='['){const items:Ast[]=[];inner();while(peek()!==']'){items.push(expr());inner();if(peek()!==',')break;take();inner()}take(']');a={kind:'array',items}}
  else if(/^\d|^\.\d/.test(t))a={kind:'number',value:t}
  else if(/^[A-Za-z_]\w*$/.test(t)&&t!=='EOF') {
   a={kind:'name',value:t}
   if(peek()==='=>'){take();inner();a={kind:'lambda',value:t,left:expr()}}
   else if(peek()==='('){take();inner();const args:NonNullable<Ast['args']>=[];while(peek()!==')'){let name:string|undefined;if(tokens[p+1]?.text===':'){name=take().text;take(':')}args.push({name,value:expr()});inner();if(peek()!==',')break;take();inner()}take(')');a={kind:'call',value:t,args}}
  } else fail(`Expected expression, got ${t}`)
  while(true){if(peek()==='\n'&&tokens[p+1]?.text==='|>')take();const op=peek(),prec=op==='|>'?1:['+','-'].includes(op)?10:['*','/'].includes(op)?20:0;if(!prec||prec<min)break;take();inner();a={kind:op==='|>'?'pipe':'binary',value:op,left:a,right:expr(prec+1)}}
  depth--;return a!
 }
 const nodes:Record<string,unknown>[]=[];const parameters:Record<string,unknown>[]=[];const customizer:CustomizerParameter[]=[];const env=new Map<string,Value>();let root=''
 const add=(node:Record<string,unknown>)=>{if(nodes.length>=128)fail('At most 128 geometry nodes');const id=`n${++serial}`;nodes.push({id,...node});return {geometry:id}}
 const geometry=(v:Value)=>{if(v&&typeof v==='object'&&!Array.isArray(v)&&'geometry'in v)return v.geometry;return fail('Expected geometry')}
 const scalar=(v:Value):Expression=>{if(Array.isArray(v)||typeof v==='object'&&('geometry'in v||'lambda'in v))return fail('Expected scalar');return v as Expression}
 const signatures:Record<string,string[]>={planetary_spinner:['inner_radius','outer_radius','bore','gap','height','helix_angle'],circle:['radius'],rectangle:['size'],box:['size'],sphere:['radius'],cylinder:['radius','height'],extrude:['height'],revolve:['angle'],translate:['vector'],rotate:['vector'],scale:['vector'],offset:['distance'],mirror:['normal']}
 function evaluate(a:Ast,e:Map<string,Value>,budget=0):Value {
  if(budget>64)fail('Function expansion exceeds 64')
  const ev=(b:Ast)=>evaluate(b,e,budget+1)
  if(a.kind==='number'){const m=a.value!.match(/^([\d.eE+-]+)(mm|cm|m|in|deg|rad)?$/)!;return m[2]?{op:'quantity',value:Number(m[1]),unit:m[2] as 'mm'}:Number(m[1])}
  if(a.kind==='name'){if(!e.has(a.value!))fail(`Unknown name ${a.value}`);return e.get(a.value!)!}
  if(a.kind==='array')return a.items!.map(ev)
  if(a.kind==='lambda')return {lambda:a.value!,body:a.left!,env:new Map(e)}
  if(a.kind==='neg')return {op:'negate',value:scalar(ev(a.left!))}
  if(a.kind==='binary')return {op:({'+':'add','-':'subtract','*':'multiply','/':'divide'}as const)[a.value as '+'],args:[scalar(ev(a.left!)),scalar(ev(a.right!))]}
  let call=a,input:string|undefined
  if(a.kind==='pipe'){input=geometry(ev(a.left!));call=a.right!;if(call.kind!=='call')fail('Pipeline requires an operation call')}
  if(call.kind!=='call')return fail('Expected a call')
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
  if(['union','intersection','subtract'].includes(call.value!)){if(args.some(x=>x.name))fail('Boolean arguments are positional');const inputs=args.map(x=>geometry(x.value));if(input)inputs.unshift(input);if(!inputs.length)fail('Boolean operation needs geometry');return call.value==='subtract'?add({op:'difference',base:inputs[0],subtract:inputs.slice(1)}):add({op:call.value,inputs})}
  const signature=signatures[call.value!];if(!signature)fail(`Unknown operation ${call.value}`)
  const fields:Record<string,unknown>={};for(let i=0;i<args.length;i++){const key=args[i]!.name??signature![i];if(!key||!signature!.includes(key)||key in fields)fail('Unknown or duplicate argument');fields[key]=args[i]!.value}
  const modifier=['extrude','revolve','translate','rotate','scale','offset','mirror'].includes(call.value!)
  if(modifier&&!input)fail(`${call.value} requires piped geometry`);if(!modifier&&input)fail(`${call.value} does not accept piped geometry`)
  return add({op:call.value,...fields,...(input?{input}:{})})
 }
 skip();while(peek()!=='EOF'){
  if(peek()==='param'){
   take();const name=take().text;if(env.has(name))fail(`Duplicate name ${name}`);take('=');const start=tokens[p]!.start;let sign=1;if(peek()==='-'){sign=-1;take()}const token=take();const m=token.text.match(/^(\d+(?:\.\d*)?(?:[eE][+-]?\d+)?|\.\d+)(mm|cm|m|in|deg|rad)?$/);if(!m)fail('Parameter default must be a numeric literal')
   const value=sign*Number(m![1]),unit=m![2],parameter:Record<string,unknown>={id:name,value,...(unit?{unit}:{})};const control:CustomizerParameter={name,label:unit?`${name} (${unit})`:name,value,valueStart:start,valueEnd:token.end-(unit?.length??0)}
   if(peek()==='range'){take();const bound=()=>{let s=1;if(peek()==='-'){take();s=-1}const b=take().text.match(/^([\d.eE+-]+)(mm|cm|m|in|deg|rad)?$/);if(!b||b[2]!==unit)fail('Range units must match parameter units');return s*Number(b![1])};parameter.min=control.min=bound();take('..');parameter.max=control.max=bound();if(control.min!>control.max!||value<control.min!||value>control.max!)fail('Parameter default must lie in its range')}
   parameters.push(parameter);customizer.push(control);env.set(name,{param:name})
  }else if(peek()==='show'){take();root=geometry(evaluate(expr(),env))}
  else {const name=take().text;if(env.has(name))fail(`Duplicate name ${name}`);take('=');const value=evaluate(expr(),env);env.set(name,value);if(value&&typeof value==='object'&&!Array.isArray(value)&&'geometry'in value)root=value.geometry}
  if(!['EOF','\n',';'].includes(peek()))fail(`Unexpected ${peek()}`);skip()
 }
 if(!root)fail('No geometry to show')
 const compiled=compileModelGraph({language:'modelgraph/1',units:'mm',parameters,nodes,root})
 return {...compiled,customizer}
}
