import type {SnapGeometry} from './modelingSnaps'
import type {NurbsCurve} from './nurbsCurve'
const finite=(n:unknown):n is number=>typeof n==='number'&&Number.isFinite(n)
const vector=(v:unknown)=>Array.isArray(v)&&v.length===3&&v.every(finite)
/** Body preparation emits only points, lines and exact NURBS intervals. */
export function validBodySnapGeometry(value:unknown,maxPoints:number,maxSegments:number):boolean{
 if(!value||typeof value!=='object')return false
 const g=value as SnapGeometry
 if(!Array.isArray(g.points)||g.points.length>maxPoints||!Array.isArray(g.segments)||g.segments.length>maxSegments||g.circles!==undefined)return false
 const curves=new Set<NurbsCurve>()
 for(const p of g.points)if(!p||!vector(p.point)||!['vertex','midpoint','center','bounds-center'].includes(p.kind))return false
 for(const segment of g.segments){
  if(!segment||!vector(segment.a)||!vector(segment.b)||segment.arc!==undefined||'evaluate' in segment)return false
  if(segment.nurbs){
   const {curve,start,end}=segment.nurbs
   if(!curve||!finite(start)||!finite(end)||start>end)return false
   if(!curves.has(curve)){
    const {degree,knots,weights,controlPoints}=curve
    if(!Number.isSafeInteger(degree)||degree<1||!Array.isArray(controlPoints)||controlPoints.length<=degree||!controlPoints.every(vector)||!Array.isArray(weights)||weights.length!==controlPoints.length||!weights.every(w=>finite(w)&&w>0)||!Array.isArray(knots)||knots.length!==controlPoints.length+degree+1||!knots.every((k,i)=>finite(k)&&(i===0||k>=knots[i-1])))return false
    if(!(knots[degree]<knots[controlPoints.length]))return false
    curves.add(curve)
   }
   if(start<curve.knots[curve.degree]||end>curve.knots[curve.controlPoints.length])return false
  }
 }
 return true
}

/** Local sketch targets contain planar lines, NURBS intervals and analytic circles, never executable callbacks. */
export function validSketchSnapGeometry(value:unknown,maxPoints:number,maxSegments:number,maxCircles:number):boolean{
 if(!value||typeof value!=='object')return false
 const g=value as SnapGeometry,planar=(v:unknown)=>vector(v)&&(v as number[])[2]===0
 if(!Array.isArray(g.points)||g.points.length>maxPoints||!Array.isArray(g.segments)||g.segments.length>maxSegments||!Array.isArray(g.circles)||g.circles.length>maxCircles)return false
 if(!g.points.every(p=>p&&planar(p.point)&&['vertex','midpoint','center','bounds-center','quadrant'].includes(p.kind)))return false
 if(!validBodySnapGeometry({points:[],segments:g.segments},0,maxSegments)||!g.segments.every(s=>planar(s.a)&&planar(s.b)))return false
 const curves=new Set(g.segments.flatMap(s=>s.nurbs?[s.nurbs.curve]:[]))
 if([...curves].some(c=>!c.controlPoints.every(planar)))return false
 return g.circles.every(c=>c&&planar(c.center)&&finite(c.radius)&&c.radius>0&&finite(c.start)&&finite(c.sweep)&&Math.abs(c.sweep)<=360+1e-7)
}
