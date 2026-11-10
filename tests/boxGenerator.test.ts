import { beforeAll, describe, expect, it } from 'vitest'
import { BOX_DEFAULTS, generateBox } from '../src/services/boxGenerator'
import { extractSemanticEdges } from '../src/services/meshTopology'
import { GeometryBuildEngine } from '../src/kernel/geometryBuildEngine'
const engine = new GeometryBuildEngine()
async function executeIndependentOpenScad({source}:{source:string;files:never[];quality:string;time:number}) {
  return engine.buildSource(source,{quality:'full',purpose:'analysis'})
}

describe('box generator', () => {
  beforeAll(async()=>{await engine.initializeSource('cube(1);',{quality:'full',purpose:'analysis'})},120000)
  it('rejects impossible walls, weak perforation webs and excessive geometry', () => {
    for(const patch of [{wall:40},{organic:false,hole_pitch:6},{hook_width:60},{width:NaN}]) {
      expect(()=>generateBox({...BOX_DEFAULTS,...patch})).toThrow()
    }
  })
  it('builds a closed solid with the expected shell volume', async () => {
    const source=generateBox({...BOX_DEFAULTS,porous:false,hooks:false,floor_mesh:false})
    const {result}=await executeIndependentOpenScad({source,files:[],quality:'full',time:0})
    expect(result.volume).toBeCloseTo(120*75*80-114*69*76, 2)
    expect(result.meshes.length).toBe(1)
  }, 120000)
  it('builds the full default box through the admitted production provider', async () => {
    const {result}=await engine.buildSource(generateBox(),{quality:'full',purpose:'analysis'})
    expect(result.volume).toBeGreaterThan(0)
    expect(result.meshes.length).toBe(1)
    // Organic honeycomb web and localized hook pads save >55% material vs solid shell.
    expect(result.volume).toBeLessThan((120*75*80-114*69*76)*0.48)
    for(const mesh of result.meshes) {
      const {diagnostics}=extractSemanticEdges(mesh.vertices,mesh.indices)
      expect(diagnostics.boundary).toBe(0)
      expect(diagnostics.nonManifold).toBe(0)
    }
  },120000)
  it('keeps an organic variant manifold when rib width and pattern change',async()=>{
    const request={quality:'full',purpose:'analysis'} as const
    const base=await engine.buildSource(generateBox(),request)
    const variant=await engine.buildSource(generateBox({...BOX_DEFAULTS,rib:3.2,pattern_seed:7}),request)
    expect(variant.result.volume).toBeGreaterThan(base.result.volume)
    for(const mesh of variant.result.meshes) {
      const {diagnostics}=extractSemanticEdges(mesh.vertices,mesh.indices)
      expect(diagnostics.boundary).toBe(0)
      expect(diagnostics.nonManifold).toBe(0)
    }
  },120000)
  it.each([-15,15])('preserves a vertical back and the analytical shell volume at front tilt %s',async angle=>{
    const p={...BOX_DEFAULTS,porous:false,hooks:false,floor_mesh:false,front_angle:angle}
    const {result}=await engine.buildSource(generateBox(p),{quality:'full',purpose:'analysis'})
    const h=p.height-p.floor, slope=Math.tan(angle*Math.PI/180)
    const expected=p.width*p.depth*p.floor+2*p.wall*(p.depth*h+slope*h*h/2)+2*p.wall*(p.width-2*p.wall)*h
    expect(result.volume).toBeCloseTo(expected,1)
    for(const mesh of result.meshes) {
      const {diagnostics}=extractSemanticEdges(mesh.vertices,mesh.indices)
      expect(diagnostics.boundary).toBe(0)
      expect(diagnostics.nonManifold).toBe(0)
    }
  },120000)
  it('combines organic pores, rounded corners, front tilt and hook pads',async()=>{
    const p={...BOX_DEFAULTS,corner_radius:12,front_angle:15}
    const {result}=await engine.buildSource(generateBox(p),{quality:'full',purpose:'analysis'})
    expect(result.volume).toBeGreaterThan(0)
    for(const mesh of result.meshes) {
      const {diagnostics}=extractSemanticEdges(mesh.vertices,mesh.indices)
      expect(diagnostics.boundary).toBe(0)
      expect(diagnostics.nonManifold).toBe(0)
    }
  },120000)
  it('rejects a radius outside the footprint and a front that closes the opening',()=>{
    expect(()=>generateBox({...BOX_DEFAULTS,corner_radius:40})).toThrow(/радиус|радиу|Радиус|Крючки/)
    expect(()=>generateBox({...BOX_DEFAULTS,front_angle:-60})).toThrow(/Наклон/)
  })
  it('builds perforated walls, drain, dividers and connected hooks', async () => {
    const source=generateBox({...BOX_DEFAULTS,width:60,depth:40,height:40,organic:false,hole_pitch:12,hook_width:8,hook_drop:10,dividers:1,drain:true})
    const {result}=await executeIndependentOpenScad({source,files:[],quality:'full',time:0})
    expect(result.volume).toBeGreaterThan(0)
    expect(result.meshes.length).toBe(1)
  }, 120000)
  it('cuts organic porous web in internal dividers to save material while staying manifold', async () => {
    const request={quality:'full',purpose:'analysis'} as const
    const base=await engine.buildSource(generateBox({...BOX_DEFAULTS,dividers:0}),request)
    const withDivider=await engine.buildSource(generateBox({...BOX_DEFAULTS,dividers:1}),request)
    const solidDivider=BOX_DEFAULTS.wall*(BOX_DEFAULTS.depth-2*BOX_DEFAULTS.wall)*(BOX_DEFAULTS.height-BOX_DEFAULTS.floor)
    // Porous organic divider adds significantly less volume than a solid slab.
    expect(withDivider.result.volume).toBeGreaterThan(base.result.volume)
    expect(withDivider.result.volume-base.result.volume).toBeLessThan(solidDivider*0.85)
    for(const mesh of withDivider.result.meshes) {
      const {diagnostics}=extractSemanticEdges(mesh.vertices,mesh.indices)
      expect(diagnostics.boundary).toBe(0)
      expect(diagnostics.nonManifold).toBe(0)
    }
  }, 120000)
  it('supports adjustable stress_gradient and mesh_density while preserving manifold topology', async () => {
    const request={quality:'full',purpose:'analysis'} as const
    const uniform=await engine.buildSource(generateBox({...BOX_DEFAULTS,hooks:false,stress_gradient:0,mesh_density:0.8}),request)
    const graded=await engine.buildSource(generateBox({...BOX_DEFAULTS,hooks:false,stress_gradient:1.6,mesh_density:1.3}),request)
    expect(Math.abs(uniform.result.volume-graded.result.volume)).toBeGreaterThan(20)
    for(const res of [uniform.result,graded.result]) {
      expect(res.meshes.length).toBe(1)
      for(const mesh of res.meshes) {
        const {diagnostics}=extractSemanticEdges(mesh.vertices,mesh.indices)
        expect(diagnostics.boundary).toBe(0)
        expect(diagnostics.nonManifold).toBe(0)
      }
    }
    expect(()=>generateBox({...BOX_DEFAULTS,mesh_density:2,cell_size:1.5,rib:2.4})).toThrow(/плотности|Effective/)
    expect(()=>generateBox({...BOX_DEFAULTS,cell_size:1,rib:2.4})).not.toThrow()
    expect(()=>generateBox({...BOX_DEFAULTS,stress_gradient:3})).toThrow()
  }, 120000)
  it('builds with small wall cell_size=10 and print_optimized=false without exceeding Boolean work budget', async () => {
    const request={quality:'full',purpose:'analysis'} as const
    for (const hook_chamfer of [false, true]) {
      const {result}=await engine.buildSource(generateBox({...BOX_DEFAULTS,cell_size:10,print_optimized:false,hook_chamfer}),request)
      expect(result.volume).toBeGreaterThan(0)
      expect(result.meshes.length).toBeGreaterThanOrEqual(1)
    }
  }, 120000)
  it('builds the default ?generator=box configuration (cell_size=5, print_optimized=false, hook_chamfer=false/true) without exceeding Boolean work budget', async () => {
    const request={quality:'full',purpose:'analysis'} as const
    for (const hook_chamfer of [false, true]) {
      const {result}=await engine.buildSource(generateBox({...BOX_DEFAULTS,print_optimized:false,hook_chamfer}),request)
      expect(result.volume).toBeGreaterThan(0)
      expect(result.meshes.length).toBeGreaterThanOrEqual(1)
    }
    const volumes: number[] = []
    for (const cell_size of [1, 3, 5, 8, 12, 16]) {
      const {result}=await engine.buildSource(generateBox({...BOX_DEFAULTS,cell_size,print_optimized:false,hook_chamfer:false}),request)
      expect(result.volume).toBeGreaterThan(0)
      expect(result.meshes.length).toBeGreaterThanOrEqual(1)
      volumes.push(result.volume)
    }
    for (let i = 1; i < volumes.length; i++) {
      expect(Math.abs(volumes[i] - volumes[i - 1])).toBeGreaterThan(15)
    }
    const smallSharp = await engine.buildSource(generateBox({...BOX_DEFAULTS,cell_size:2,cell_radius:0,print_optimized:false,hook_chamfer:false}),request)
    const smallRounded = await engine.buildSource(generateBox({...BOX_DEFAULTS,cell_size:2,cell_radius:5,print_optimized:false,hook_chamfer:false}),request)
    expect(Math.abs(smallRounded.result.volume - smallSharp.result.volume)).toBeGreaterThan(50)
    for (const mesh of smallRounded.result.meshes) {
      expect(mesh.color).toEqual([0.26, 0.52, 0.96, 1])
    }
    const sharp = await engine.buildSource(generateBox({...BOX_DEFAULTS,cell_size:12,cell_radius:0,print_optimized:false,hook_chamfer:false}),request)
    const rounded = await engine.buildSource(generateBox({...BOX_DEFAULTS,cell_size:12,cell_radius:2,print_optimized:false,hook_chamfer:false}),request)
    const midRounded = await engine.buildSource(generateBox({...BOX_DEFAULTS,cell_size:12,cell_radius:5,print_optimized:false,hook_chamfer:false}),request)
    const maxRounded = await engine.buildSource(generateBox({...BOX_DEFAULTS,cell_size:12,cell_radius:50,print_optimized:false,hook_chamfer:false}),request)
    const largeCell = await engine.buildSource(generateBox({...BOX_DEFAULTS,cell_size:20,cell_radius:20,print_optimized:false,hook_chamfer:false}),request)
    expect(largeCell.result.volume).toBeGreaterThan(0)
    expect(Math.abs(rounded.result.volume - sharp.result.volume)).toBeGreaterThan(10)
    expect(Math.abs(midRounded.result.volume - rounded.result.volume)).toBeGreaterThan(20)
    expect(Math.abs(maxRounded.result.volume - midRounded.result.volume)).toBeGreaterThan(50)
  }, 120000)
  it('aligns organic pattern across all 4 vertical corner seams', async () => {
    const scad = generateBox({ ...BOX_DEFAULTS, depth: 40, bend_radius: 0, cell_size: 2, cell_radius: 2, print_optimized: false, hook_chamfer: false })
    const built = await engine.buildSource(scad, { quality: 'preview', purpose: 'preview' })
    const [mFloor, mFront, mLeft, mRight, mRear] = built.result.meshes
    const zRangesAt = (m: typeof mFront, pred: (x: number, y: number) => boolean) => {
      const zs: number[] = []
      for (let i = 0; i < m.vertices.length; i += 6) {
        const z = m.vertices[i + 2]
        if (z > 0.1 && z < 79.9 && pred(m.vertices[i], m.vertices[i + 1])) zs.push(Number(z.toFixed(2)))
      }
      return Array.from(new Set(zs)).sort((a, b) => a - b)
    }
    // Rear-left corner (x=3, y=3):
    const rearLeftZ = zRangesAt(mRear, (x, y) => Math.abs(x - 3) < 0.05 && Math.abs(y - 3) < 0.05)
    const leftRearZ = zRangesAt(mLeft, (x, y) => Math.abs(x - 3) < 0.05 && Math.abs(y - 3) < 0.05)
    // Rear-right corner (x=117, y=3):
    const rearRightZ = zRangesAt(mRear, (x, y) => Math.abs(x - 117) < 0.05 && Math.abs(y - 3) < 0.05)
    const rightRearZ = zRangesAt(mRight, (x, y) => Math.abs(x - 117) < 0.05 && Math.abs(y - 3) < 0.05)
    // Front-left corner (x=3, y=37):
    const frontLeftZ = zRangesAt(mFront, (x, y) => Math.abs(x - 3) < 0.05 && Math.abs(y - 37) < 0.05)
    const leftFrontZ = zRangesAt(mLeft, (x, y) => Math.abs(x - 3) < 0.05 && Math.abs(y - 37) < 0.05)
    // Front-right corner (x=117, y=37):
    const frontRightZ = zRangesAt(mFront, (x, y) => Math.abs(x - 117) < 0.05 && Math.abs(y - 37) < 0.05)
    const rightFrontZ = zRangesAt(mRight, (x, y) => Math.abs(x - 117) < 0.05 && Math.abs(y - 37) < 0.05)
    expect(frontLeftZ).toEqual(leftFrontZ)
    expect(frontRightZ).toEqual(rightFrontZ)
    expect(rearLeftZ).toEqual(leftRearZ)
    expect(rearRightZ).toEqual(rightRearZ)
  }, 120000)
  it('supports separate printable hook mounts (1 = T-lock slot, 2 = dovetail slide) laid flat on z=0', async () => {
    for (const hook_mount of [1, 2] as const) {
      for (const print_optimized of [false, true]) {
        const scad = generateBox({ ...BOX_DEFAULTS, hook_mount, hook_tolerance: 0.2, print_optimized, cell_size: 12 })
        const { result } = await engine.buildSource(scad, { quality: 'preview', purpose: 'preview' })
        expect(result.volume).toBeGreaterThan(0)
        // Detached hooks should sit in front of y=0 (at negative y), touch z=0, and stay compact in X and Z
        let minY = Infinity
        let minZAtNegY = Infinity
        let maxZAtNegY = -Infinity
        for (const m of result.meshes) {
          for (let i = 0; i < m.vertices.length; i += 6) {
            const y = m.vertices[i + 1]
            const z = m.vertices[i + 2]
            if (y < -10) {
              minY = Math.min(minY, y)
              minZAtNegY = Math.min(minZAtNegY, z)
              maxZAtNegY = Math.max(maxZAtNegY, z)
            }
          }
        }
        expect(minY).toBeLessThan(-15)
        expect(minZAtNegY).toBeCloseTo(0, 2)
        expect(maxZAtNegY).toBeLessThanOrEqual(BOX_DEFAULTS.hook_width + 0.1)
      }
    }
  }, 120000)
})
