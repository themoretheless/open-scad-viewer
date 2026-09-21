import type { DirectSketch, Point2 } from './directModeling'
import type { Vec3 } from './directSketchGeometry'
export type SnapKind = 'vertex' | 'midpoint' | 'center' | 'edge' | 'intersection' | 'axis' | 'grid' | 'quadrant' | 'tangent' | 'perpendicular' | 'origin' | 'bounds-center'
export interface SnapPoint { point: Vec3; kind: SnapKind }
export interface SnapSegment { a: Vec3; b: Vec3; evaluate?: (t:number)=>Vec3 }
export interface SnapGeometry { points: SnapPoint[]; segments: SnapSegment[]; circles?: { center: Vec3; radius: number; start: number; sweep: number }[] }
export interface SnapResult { point: Vec3; kind: SnapKind | null; guide?: [Vec3, Vec3] }
const distance = (a: number[], b: number[]) => Math.hypot(...a.map((v, i) => v - b[i]))
const lerp = (a: Vec3, b: Vec3, t: number) => a.map((v, i) => v + (b[i] - v) * t) as Vec3
const priority: Record<SnapKind, number> = { vertex: 0, intersection: 0, center: 0, midpoint: 0, edge: 2, axis: 3, grid: 4, quadrant:0, tangent:1, perpendicular:1, origin:0, 'bounds-center':0 }
/** All hit distances are measured after projection, in CSS pixels. */
export function resolveModelingSnap(point: Vec3, geometry: SnapGeometry, options: {
  project: (p: Vec3) => Point2; radius?: number; grid: number; geometry: boolean; guides?: boolean; anchor?: Vec3;
  constrain?: (p: Vec3) => Vec3; gridAxes?: number[]; keypoints?: boolean; relations?: boolean;
}): SnapResult {
  const radius = options.radius ?? 10, screen = options.project(point)
  let best: { result: SnapResult; distance: number; priority: number } | undefined
  const offer = (target: Vec3, kind: SnapKind, guide?: [Vec3, Vec3]) => {
    if (!target.every(Number.isFinite)) return
    if(options.keypoints===false&&['center','bounds-center','midpoint','quadrant','origin'].includes(kind))return
    if(options.relations===false&&['tangent','perpendicular','intersection'].includes(kind))return
    const constrained = options.constrain?.(target) ?? target
    // A snap must actually reach its feature, not just look aligned in projection.
    if (distance(target, constrained) > 1e-6) return
    const d = distance(screen, options.project(target))
    if (d > radius) return
    const rank = priority[kind]
    if (!best || rank < best.priority || rank === best.priority && d < best.distance) best = { result: { point: [...target], kind, guide }, distance: d, priority: rank }
  }
  if (options.geometry) {
    for (const candidate of geometry.points) offer(candidate.point, candidate.kind)
    const nearby: SnapSegment[] = []
    for (const segment of geometry.segments) {
      const a = options.project(segment.a), b = options.project(segment.b), dx = b[0] - a[0], dy = b[1] - a[1]
      const t = Math.max(0, Math.min(1, ((screen[0] - a[0]) * dx + (screen[1] - a[1]) * dy) / (dx * dx + dy * dy || 1)))
      const target = lerp(segment.a, segment.b, t)
      if(distance(screen,options.project(target))<=radius)offer(segment.evaluate?.(t)??target, 'edge')
      if(options.anchor&&options.relations!==false&&!segment.evaluate){
        const direction=segment.b.map((v,i)=>v-segment.a[i]),length2=direction.reduce((n,v)=>n+v*v,0)
        const parameter=direction.reduce((n,v,i)=>n+v*(options.anchor![i]-segment.a[i]),0)/(length2||1)
        if(parameter>=0&&parameter<=1){const foot=lerp(segment.a,segment.b,parameter);offer(foot,'perpendicular',[options.anchor,foot])}
      }
      if (!segment.evaluate && nearby.length < 64 && distance(screen, options.project(target)) <= radius) nearby.push(segment)
    }
    for (let i = 0; i < nearby.length; i++) for (let j = i + 1; j < nearby.length; j++) {
      const a = nearby[i], b = nearby[j], u = a.b.map((v,k)=>v-a.a[k]), v = b.b.map((x,k)=>x-b.a[k]), w=a.a.map((x,k)=>x-b.a[k])
      const dot=(a:number[],b:number[])=>a.reduce((sum,x,k)=>sum+x*b[k],0),uu=dot(u,u),uv=dot(u,v),vv=dot(v,v),uw=dot(u,w),vw=dot(v,w),den=uu*vv-uv*uv
      if(Math.abs(den)<1e-12*Math.max(uu*vv,1e-20))continue
      const t=(uv*vw-vv*uw)/den,s=(uu*vw-uv*uw)/den
      if(t<0||t>1||s<0||s>1)continue
      const hit=lerp(a.a,a.b,t)
      if(distance(hit,lerp(b.a,b.b,s))<1e-6)offer(hit,'intersection')
    }
    const circles=geometry.circles??[]
    const onArc=(circle:NonNullable<SnapGeometry['circles']>[number],p:Vec3)=>{
      const degrees=Math.atan2(p[1]-circle.center[1],p[0]-circle.center[0])*180/Math.PI
      const travel=((circle.sweep>=0?degrees-circle.start:circle.start-degrees)%360+360)%360
      return Math.abs(circle.sweep)>=360-1e-6||travel<=Math.abs(circle.sweep)+1e-7
    }
    for (const circle of circles) {
      const angle = Math.atan2(point[1]-circle.center[1],point[0]-circle.center[0]), degrees=angle*180/Math.PI
      const travel=((circle.sweep>=0?degrees-circle.start:circle.start-degrees)%360+360)%360
      if(Math.abs(circle.sweep)>=360-1e-6||travel<=Math.abs(circle.sweep)) offer([circle.center[0]+circle.radius*Math.cos(angle),circle.center[1]+circle.radius*Math.sin(angle),circle.center[2]],'edge')
      if(options.anchor&&options.relations!==false){
        const x=options.anchor[0]-circle.center[0],y=options.anchor[1]-circle.center[1],d2=x*x+y*y,r2=circle.radius**2
        if(d2>r2+1e-10)for(const sign of [-1,1]){
          const k=r2/d2,h=sign*circle.radius*Math.sqrt(d2-r2)/d2
          const target:Vec3=[circle.center[0]+k*x-h*y,circle.center[1]+k*y+h*x,circle.center[2]]
          if(onArc(circle,target))offer(target,'tangent',[options.anchor,target])
        }
      }
      for(const segment of nearby){
        if(Math.abs(segment.a[2]-circle.center[2])>1e-7||Math.abs(segment.b[2]-circle.center[2])>1e-7)continue
        const dx=segment.b[0]-segment.a[0],dy=segment.b[1]-segment.a[1],x=segment.a[0]-circle.center[0],y=segment.a[1]-circle.center[1]
        const a=dx*dx+dy*dy,b=2*(x*dx+y*dy),c=x*x+y*y-circle.radius**2,discriminant=b*b-4*a*c
        if(a<1e-16||discriminant<0)continue
        for(const sign of [-1,1]){const t=(-b+sign*Math.sqrt(discriminant))/(2*a);if(t>=0&&t<=1){const hit=lerp(segment.a,segment.b,t);if(onArc(circle,hit))offer(hit,'intersection')}}
      }
    }
    for(let i=0;i<circles.length;i++)for(let j=i+1;j<circles.length;j++){
      const a=circles[i],b=circles[j],x=b.center[0]-a.center[0],y=b.center[1]-a.center[1],d=Math.hypot(x,y)
      if(Math.abs(a.center[2]-b.center[2])>1e-7||d<1e-9||d>a.radius+b.radius||d<Math.abs(a.radius-b.radius))continue
      const t=(a.radius*a.radius-b.radius*b.radius+d*d)/(2*d),h=Math.sqrt(Math.max(0,a.radius*a.radius-t*t))
      for(const sign of [-1,1]){const hit:Vec3=[a.center[0]+(t*x-sign*h*y)/d,a.center[1]+(t*y+sign*h*x)/d,a.center[2]];if(onArc(a,hit)&&onArc(b,hit))offer(hit,'intersection')}
    }
  }
  if (options.guides && options.anchor) {
    for (const axis of options.gridAxes ?? [0,1,2]) {
      const target=[...options.anchor] as Vec3
      target[axis]=options.grid>0?Math.round(point[axis]/options.grid)*options.grid:point[axis]
      offer(target,'axis',[options.anchor,target])
    }
  }
  if (best) return best.result
  if (Number.isFinite(options.grid) && options.grid > 0) {
    const target = [...point] as Vec3
    for (const axis of options.gridAxes ?? [0,1,2]) target[axis]=Math.round(target[axis]/options.grid)*options.grid
    return { point: options.constrain?.(target) ?? target, kind: 'grid' }
  }
  return { point: [...point], kind: null }
}
export function sketchSnapGeometry(sketches: readonly DirectSketch[]): SnapGeometry {
  const geometry: SnapGeometry = { points: [], segments: [], circles: [] }
  const p3 = (p: Point2): Vec3 => [p[0],p[1],0]
  for(const sketch of sketches) {
    const analytic=sketch.analytic
    if(analytic) {
      geometry.points.push({point:p3(analytic.center),kind:'center'})
      geometry.circles!.push({center:p3(analytic.center),radius:analytic.radius,start:analytic.start,sweep:analytic.sweep})
      const angles=analytic.kind==='circle'?[0,90,180,270]:[analytic.start,analytic.start+analytic.sweep/2,analytic.start+analytic.sweep]
      for(const angle of angles)geometry.points.push({point:[analytic.center[0]+analytic.radius*Math.cos(angle*Math.PI/180),analytic.center[1]+analytic.radius*Math.sin(angle*Math.PI/180),0],kind:analytic.kind==='circle'?'quadrant':angle===analytic.start+analytic.sweep/2?'midpoint':'vertex'})
      continue
    }
    if(sketch.closed&&sketch.points.length>=3){
      const [ox,oy]=sketch.points[0];let area=0,cx=0,cy=0
      for(let i=0;i<sketch.points.length;i++){
        const p=sketch.points[i],q=sketch.points[(i+1)%sketch.points.length],a=(p[0]-ox)*(q[1]-oy)-(q[0]-ox)*(p[1]-oy)
        area+=a;cx+=(p[0]+q[0]-2*ox)*a;cy+=(p[1]+q[1]-2*oy)*a
      }
      if(Math.abs(area)>1e-12)geometry.points.push({point:[ox+cx/(3*area),oy+cy/(3*area),0],kind:'center'})
      const xs=sketch.points.map(p=>p[0]),ys=sketch.points.map(p=>p[1]),bounds:Vec3=[(Math.min(...xs)+Math.max(...xs))/2,(Math.min(...ys)+Math.max(...ys))/2,0]
      if(!geometry.points.some(p=>p.kind==='center'&&distance(p.point,bounds)<1e-7))geometry.points.push({point:bounds,kind:'bounds-center'})
    }
    for(const point of sketch.points)geometry.points.push({point:p3(point),kind:'vertex'})
    for(let i=0;i<sketch.points.length-(sketch.closed?0:1);i++) {
      const a=p3(sketch.points[i]),b=p3(sketch.points[(i+1)%sketch.points.length])
      geometry.segments.push({a,b});geometry.points.push({point:lerp(a,b,.5),kind:'midpoint'})
    }
  }
  return geometry
}
