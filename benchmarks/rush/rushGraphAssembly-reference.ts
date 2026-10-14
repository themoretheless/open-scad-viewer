export type Frame = { origin: [number, number, number]; rotation: [number, number, number] }
export type AssemblyComponent = { id: string; input: string; anchors: Array<Frame & { id: string }>; placement?: Frame; mate?: { component: string; anchor: string; own_anchor: string; gap: number; rotation: [number, number, number]; joint?: { kind: 'revolute' | 'slider'; position: number; min: number; max: number } } }
export type Matrix = number[]
const identity = (): Matrix => [1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]
export function multiplyFrames(a: Matrix, b: Matrix): Matrix { return Array.from({length:16}, (_, i) => { const r=Math.floor(i/4), c=i%4; return [0,1,2,3].reduce((n,k)=>n+a[r*4+k]!*b[k*4+c]!,0) }) }
export function frameMatrix(frame: Frame): Matrix {
  const [x,y,z]=frame.rotation.map(v=>v*Math.PI/180) as [number,number,number]
  const cx=Math.cos(x),sx=Math.sin(x),cy=Math.cos(y),sy=Math.sin(y),cz=Math.cos(z),sz=Math.sin(z)
  const matrix=multiplyFrames(multiplyFrames([cz,-sz,0,0,sz,cz,0,0,0,0,1,0,0,0,0,1],[cy,0,sy,0,0,1,0,0,-sy,0,cy,0,0,0,0,1]),[1,0,0,0,0,cx,-sx,0,0,sx,cx,0,0,0,0,1])
  for(let i=0;i<3;i++)matrix[i*4+3]=frame.origin[i]!
  return matrix
}
function inverseRigid(m: Matrix): Matrix {
  const out=identity()
  for(let r=0;r<3;r++){for(let c=0;c<3;c++)out[r*4+c]=m[c*4+r]!;out[r*4+3]=-[0,1,2].reduce((sum,k)=>sum+out[r*4+k]!*m[k*4+3]!,0)}
  return out
}
/** Acyclic fixed mates: parent * targetAnchor * gap/rotation * inverse(ownAnchor). */
export function placeAssembly(components: AssemblyComponent[]) {
  const byId=new Map(components.map(c=>[c.id,c])), solved=new Map<string,Matrix>(), active=new Set<string>()
  if(byId.size!==components.length)throw new Error('Component IDs must be unique.')
  for(const component of components){
    if(new Set(component.anchors.map(a=>a.id)).size!==component.anchors.length)throw new Error(`Duplicate anchor in ${component.id}.`)
    if(Boolean(component.placement)===Boolean(component.mate))throw new Error(`Component ${component.id} must have exactly one placement or mate.`)
  }
  const anchor=(id:string,name:string)=>{const component=byId.get(id);if(!component)throw new Error(`Unknown component ${id}.`);const found=component.anchors.find(a=>a.id===name);if(!found)throw new Error(`Unknown anchor ${id}.${name}.`);return frameMatrix(found)}
  const solve=(id:string):Matrix=>{
    const cached=solved.get(id);if(cached)return cached
    const component=byId.get(id);if(!component)throw new Error(`Unknown component ${id}.`)
    if(active.has(id))throw new Error(`Cyclic mate through ${id}.`)
    active.add(id)
    let matrix:Matrix
    if(component.placement)matrix=frameMatrix(component.placement)
    else {const mate=component.mate!;if(mate.joint && (mate.joint.min > mate.joint.max || mate.joint.position < mate.joint.min || mate.joint.position > mate.joint.max))throw new Error(`Joint ${id} is outside its limits.`);const offset=frameMatrix({origin:[0,0,mate.joint?.kind==='slider'?mate.joint.position:0],rotation:[0,0,mate.joint?.kind==='revolute'?mate.joint.position:0]});matrix=multiplyFrames(multiplyFrames(multiplyFrames(solve(mate.component),anchor(mate.component,mate.anchor)),multiplyFrames(frameMatrix({origin:[0,0,mate.gap],rotation:mate.rotation}),offset)),inverseRigid(anchor(id,mate.own_anchor)))}
    if(matrix.some(v=>!Number.isFinite(v)||Math.abs(v)>1e6))throw new Error('Assembly transform exceeds numeric limits.')
    active.delete(id);solved.set(id,matrix);return matrix
  }
  return components.map(c=>({id:c.id,input:c.input,joint:c.mate?.joint ?? null,matrix:solve(c.id),anchors:c.anchors.map(a=>({id:a.id,matrix:multiplyFrames(solve(c.id),frameMatrix(a))}))}))
}
