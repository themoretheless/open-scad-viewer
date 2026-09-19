import { z } from 'zod/v4'
const n = z.number().finite().min(-1e6).max(1e6)
export const GEAR_DEFAULTS = { teeth:24,module:2,pressure_angle:20,thickness:8,bore:5,backlash:0.15,clearance:0.5,internal:false,rim_width:6,flank_segments:6 }
export const PLANETARY_DEFAULTS = { sun_teeth:24,planet_teeth:24,planet_count:3,module:2,pressure_angle:20,thickness:8,bore:5,backlash:0.15,clearance:0.5,rim_width:6,flank_segments:6,carrier_angle:0 }
export const THREAD_DEFAULTS = { diameter:12,pitch:1.5,length:12,internal:false,wall:3,clearance:0.2,starts:1,left_handed:false,segments_per_turn:32 }
export const mechanicalGeneratorSchema = z.discriminatedUnion('kind', [
  z.object({kind:z.literal('gear'),teeth:n.default(24),module:n.default(2),pressure_angle:n.default(20),thickness:n.default(8),bore:n.default(5),backlash:n.default(0.15),clearance:n.default(0.5),internal:z.boolean().default(false),rim_width:n.default(6),flank_segments:n.default(6)}).strict(),
  z.object({kind:z.literal('planetary_gears'),sun_teeth:n.default(24),planet_teeth:n.default(24),planet_count:n.default(3),module:n.default(2),pressure_angle:n.default(20),thickness:n.default(8),bore:n.default(5),backlash:n.default(0.15),clearance:n.default(0.5),rim_width:n.default(6),flank_segments:n.default(6),carrier_angle:n.default(0)}).strict(),
  z.object({kind:z.literal('thread'),diameter:n.default(12),pitch:n.default(1.5),length:n.default(12),internal:z.boolean().default(false),wall:n.default(3),clearance:n.default(0.2),starts:n.default(1),left_handed:z.boolean().default(false),segments_per_turn:n.default(32)}).strict(),
])
export type MechanicalGeneratorInput = z.infer<typeof mechanicalGeneratorSchema>
export function createMechanicalDocument(input:unknown) {
  const parsed = mechanicalGeneratorSchema.parse(input)
  const {kind,...options} = parsed
  const parameters:Array<{id:string;value:number;unit?:'mm'|'deg';integer?:boolean}> = []
  const node:Record<string,unknown> = {id:'part',op:kind}
  for (const [id,value] of Object.entries(options)) {
    if (typeof value !== 'number') {node[id]=value;continue}
    const unit = ['module','thickness','bore','backlash','clearance','rim_width','diameter','pitch','length','wall'].includes(id) ? 'mm' : ['pressure_angle','carrier_angle'].includes(id) ? 'deg' : undefined
    parameters.push({id,value,...(unit?{unit}:{}),...(['teeth','sun_teeth','planet_teeth','planet_count','flank_segments','starts','segments_per_turn'].includes(id)?{integer:true}:{})})
    node[id]={param:id}
  }
  return {language:'modelgraph/1',units:'mm',type_policy:'strict',parameters,nodes:[node],root:'part',segments:48}
}
export const MECHANICAL_GENERATOR_EXAMPLES = {
  gear:createMechanicalDocument({kind:'gear'}),
  planetary_gears:createMechanicalDocument({kind:'planetary_gears'}),
  thread:createMechanicalDocument({kind:'thread'}),
}
export const MECHANICAL_GENERATOR_GUIDE = `Mechanical generators: gear creates a sampled involute spur gear or internal ring; planetary_gears creates separate sun, ring and planet gears (no carrier plate, shafts or bearings); thread creates a helical externally threaded rod or internally threaded cylindrical sleeve. Call modelgraph_generate with kind gear, planetary_gears or thread and numeric options; omitted options use the accompanying schema defaults. It returns a parameterized ModelGraph/1 document, full-detail geometry analysis and bounded preview images. When the full mesh exceeds the preview triangle budget, images use a separate segments=12 document; preview_document_sha256 identifies that display document. Returned document, analysis and export remain full-detail. Planetary results include mechanical_parts with separate local ModelGraph documents and assembly poses; export these documents individually for printing. Read mechanical_reports in compile/check/report for design dimensions and limits. Keep the document and use modelgraph_set_parameters for changes. All three node types also support structured scalar expressions in numeric fields. Lengths are mm, angles degrees. Gear parameters: teeth,module,pressure_angle,thickness,bore,backlash,clearance,internal,rim_width,flank_segments. Planetary parameters: sun_teeth,planet_teeth,planet_count,module,pressure_angle,thickness,bore,backlash,clearance,rim_width,flank_segments,carrier_angle; ring teeth are derived, ring is fixed and carrier_angle drives the kinematic pose. planetary_gears must be the root and keeps parts separate. Thread parameters: diameter,pitch,length,internal,wall,clearance,starts,left_handed,segments_per_turn. Thread lead=starts*pitch; clearance is total radial gap between matching external/internal parts using equal settings. Bounds: gears have 3..256 teeth, module0.1..100mm, pressure_angle14.5..30deg, flank_segments3..12; minimum_external_teeth_without_undercut reports ceil(2/sin(pressure_angle)^2) (18 at20deg), not an acceptance limit. Smaller external gears use a radial flank below the base circle; generated undercut is not modeled. Internal gear bore must be0. Planet count2..6, with ring=sun+2*planet and (sun+ring) divisible by planet_count; incompatible spacing or involute contact is rejected. Threads allow starts1..4, segments_per_turn16..96 and at least8*starts; diameter>=2*pitch, length0.25..64 pitches, mesh<=3500 triangles. Combined mechanical source is limited to220000 characters; reduce resolution or size if a budget is exceeded. These sampled printable geometries do not certify manufacturing tolerances, tooth strength or fastener load capacity. Use common modelgraph_report and modelgraph_export (STL,3MF,OBJ,PLY,OFF,AMF); the graph is the source of truth.`
