import {writeFileSync} from 'node:fs'
import * as reference from './modelGraph-runtime-reference'
const fixtures:any[]=[]
function add(name:string,doc:any){try{const result=reference.compileModelGraph(doc);delete (result as any).document_sha256;fixtures.push({name,document:doc,result})}catch(e:any){fixtures.push({name,document:doc,error:{code:e.code,path:e.path,message:e.message,...(e.details?{details:e.details}:{})}})}}
for(const[k,v]of Object.entries(reference))if(k.endsWith('_EXAMPLE')&&v&&typeof v==='object'&&(v as any).language==='modelgraph/1')add(k,v)
const doc=(nodes:any[],root=nodes.at(-1).id,extra={})=>({language:'modelgraph/1',units:'mm',parameters:[],nodes,root,...extra})
const box={id:'solid',op:'box',size:[2,3,4]}
for(const op of ['union','intersection','hull'])add(op,doc([box,{id:'result',op,inputs:['solid','solid']}]))
add('difference',doc([box,{id:'result',op:'difference',base:'solid',subtract:['solid']}]))
for(const op of ['translate','rotate','scale'])add(op,doc([box,{id:'result',op,vector:op==='scale'?[1,2,-3]:[2,15,-3],input:'solid'}]))
for(const op of ['rectangle','circle','polygon']){const profile={id:'p',op,...(op==='rectangle'?{size:[2,3]}:op==='circle'?{radius:2}:{points:[[0,0],[3,0],[2,2],[0,3]]})};for(const feature of ['extrude','revolve','advanced_extrude','offset']){const node={id:'result',op:feature,input:'p',...(feature==='extrude'?{height:2}:feature==='revolve'?{angle:180}:feature==='advanced_extrude'?{height:3,twist:25,top_scale:[1,2],slices:4}:{distance:-.2,mode:'delta'})};add(op+'_'+feature,doc([profile,node,...(feature==='offset'?[{id:'out',op:'extrude',input:'result',height:2}]:[])]));}}
for(const op of ['projection','section'])add(op,doc([box,{id:'profile',op,input:'solid',...(op==='section'?{height:2}:{})},{id:'result',op:'extrude',input:'profile',height:2}]))
add('affine',doc([box,{id:'result',op:'affine',input:'solid',rows:[[1,0,0,2],[0,2,0,3],[0,0,-3,4]]}]))
add('mirror',doc([box,{id:'result',op:'mirror',input:'solid',normal:[0,1,0]}]))
add('cone',doc([{id:'result',op:'cone',height:3,radius_bottom:2,radius_top:0}]))
add('torus',doc([{id:'result',op:'torus',major_radius:3,minor_radius:1}]))
for(const op of ['linear_pattern','circular_pattern'])add(op,doc([box,{id:'result',op,input:'solid',count:4,...(op==='linear_pattern'?{step:[2,3,4]}:{angle_step:35})}]))
add('geometry_if',doc([box,{id:'no',op:'sphere',radius:-1},{id:'result',op:'if',condition:1,then:'solid',else:'no'}]))
add('map',doc([{...box,size:[{op:'add',args:[{local:'i'},1]},3,4]},{id:'result',op:'map',count:3,index:'i',input:'solid'}]))
add('collect',doc([{...box,size:[{local:'x'},3,4]},{id:'result',op:'collect',values:{op:'list',items:[1,2,3]},binding:'x',input:'solid'}]))
add('group',doc([box,{id:'result',op:'group',inputs:['solid','solid']}]))
const identity={origin:[0,0,0],rotation:[0,0,0]}
const assembly=[{id:'a',input:'solid',anchors:[{id:'top',...identity,origin:[0,0,4]}],placement:{origin:[10,20,30],rotation:[15,25,35]}},{id:'b',input:'solid',anchors:[{id:'base',origin:[2,3,4],rotation:[15,25,35]}],mate:{component:'a',anchor:'top',own_anchor:'base',gap:.3,rotation:[0,90,0]}}]
for(const joint of [null,{kind:'revolute',position:35,min:-90,max:90},{kind:'slider',position:3,min:0,max:10}]){const components=structuredClone(assembly);if(joint)(components[1]as any).mate.joint=joint;add('assembly'+joint?.kind,doc([box,{id:'result',op:'assembly',components:components.reverse()}]));}
add('nested_assembly',doc([box,{id:'child',op:'assembly',components:assembly},{id:'result',op:'assembly',components:[{id:'sub',input:'child',anchors:[],placement:{origin:[1,2,3],rotation:[20,30,40]}}]}]))
const example:any=reference.MODELGRAPH_SKETCH_EXAMPLE
for(const kind of ['underconstrained','inconsistent','not_converged','parallel','perpendicular','equal_length','coincident']){const d=structuredClone(example);const p=d.nodes[0];if(kind==='underconstrained'){p.constraints=[];p.allow_underconstrained=true;}else if(kind==='inconsistent')p.constraints=[{id:'a',kind:'fix',point:'a',at:[0,0]},{id:'b',kind:'fix',point:'a',at:[10,0]}];else if(kind==='not_converged'){p.points.forEach((p:any)=>p.position=[0,0]);p.constraints=[{id:'d',kind:'distance',a:'a',b:'b',value:10}];}else{p.constraints=[{id:'relation',kind,a:'a',b:'b',...(kind==='coincident'?{}:{c:'c',d:'d'})}];p.allow_underconstrained=true;}add('sketch_'+kind,d);}
for(const[name,node]of Object.entries({box_zero:{...box,size:[0,1,1]},negative_sphere:{id:'p',op:'sphere',radius:-1},bad_torus:{id:'p',op:'torus',major_radius:2,minor_radius:3},bad_cone:{id:'p',op:'cone',radius_bottom:0,radius_top:0,height:3}}))add(name,doc([node]));
add('unknown_root',doc([box],'missing'));add('unused',doc([box,{id:'p',op:'sphere',radius:2}]));add('duplicate_nodes',doc([box,box]));add('cycle',doc([{id:'a',op:'translate',vector:[1,2,3],input:'b'},{id:'b',op:'translate',vector:[1,2,3],input:'a'}]));
for(const[field,value]of [['anchors',[{id:'same',...identity},{id:'same',...identity}]],['placement',null]]){const c=structuredClone(assembly);if(value===null)delete(c[0]as any)[field as string];else(c[0]as any)[field as string]=value;add('invalid_assembly_'+field,doc([box,{id:'result',op:'assembly',components:c}]))}
writeFileSync('crates/modelgraph-runtime/tests/fixtures/emitter-parity.json',JSON.stringify(fixtures,null,2)+'\n');console.log('emitter fixtures',fixtures.length)
