import { validateProfilePolygon } from './modelGraphProfiles'
export type LoftSection = { z: number; scale: [number, number]; offset: [number, number] }
/** Convex parallel sections with positive axis scaling, joined with explicit triangles. */
export function buildModelGraphLoft(profile: [number, number][], sections: LoftSection[]) {
  const reject = (_code: string, _path: string, message: string): never => { throw new Error(message) }
  validateProfilePolygon(profile, 'profile', reject)
  const area = profile.reduce((sum, p, i) => { const q = profile[(i + 1) % profile.length]!; return sum + p[0] * q[1] - q[0] * p[1] }, 0)
  const ring = area > 0 ? profile : [...profile].reverse()
  for (let i = 0; i < ring.length; i++) {
    const a = ring[i]!, b = ring[(i + 1) % ring.length]!, c = ring[(i + 2) % ring.length]!
    if ((b[0]-a[0])*(c[1]-b[1])-(b[1]-a[1])*(c[0]-b[0]) <= 0) throw new Error('Loft profile must be strictly convex; omit collinear vertices.')
  }
  for (let i = 0; i < sections.length; i++) {
    const s = sections[i]!
    if (![s.z,...s.scale,...s.offset].every(Number.isFinite) || s.scale.some(v => v <= 0)) throw new Error('Loft sections require finite values and positive scales.')
    if (i && s.z <= sections[i-1]!.z) throw new Error('Loft section Z values must strictly increase.')
  }
  const points = sections.flatMap(s => ring.map(p => [p[0]*s.scale[0]+s.offset[0],p[1]*s.scale[1]+s.offset[1],s.z]))
  if (points.some(p => p.some(v => !Number.isFinite(v) || Math.abs(v)>1e6))) throw new Error('Loft coordinates exceed numeric limits.')
  const n = ring.length, last = (sections.length-1)*n
  const faces: number[][] = []
  // OpenSCAD face winding is clockwise when viewed from outside.
  for (let i=1;i<n-1;i++) { faces.push([0,i,i+1]); faces.push([last,last+i+1,last+i]) }
  for (let k=0;k<sections.length-1;k++) for (let i=0;i<n;i++) {
    const a=k*n+i,b=k*n+(i+1)%n,c=b+n,d=a+n
    faces.push([a,c,b],[a,d,c])
  }
  return { points, faces }
}
