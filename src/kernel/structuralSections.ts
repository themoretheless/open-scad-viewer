import {callGeometryRust} from './geometry/kernel'
import {normalizePolygonMesh,type PolygonMesh} from './geometry/polygon'

export interface StructuralSection {
  positionMm:number
  material:boolean
  contours:[number,number][][]
  boundaryContours:[number,number][][]
  sourceTriangles:number[][]
  properties:null|{
    areaMm2:number;centroidMm:[number,number];iuuMm4:number;ivvMm4:number;iuvMm4:number
    /** Principal axes: axis 1 carries the larger moment; angle from +x in (−π/2, π/2]. */
    principalAngleRad:number;i1Mm4:number;i2Mm4:number
    /** Radii of gyration √(I/A): inertia ellipse semi-axes. */
    r1Mm:number;r2Mm:number
    w1Mm3:number;w2Mm3:number
    /** Every axis is principal (Iuu == Ivv, Iuv == 0); angle is 0. */
    isotropic:boolean
  }
}
export interface BoundaryConnectivity {
  modelKind:'indexed-edge-boundary-components-v1'
  materialConnectivity:'not-established'|'classified-at-tolerance'
  components:{id:number;sourceTriangles:number[];signedVolumeMm3:number;boundsMm:[number[],number[]]}[]
  sharedVertices:{sourceVertex:number;components:number[]}[]
}
export type MaterialAudit =
  | {status:'unresolved';code:string;message:string}
  | {status:'classified';toleranceMm:number;materialRegions:number;
      shells:{id:number;firstTriangle:number;parent:number|null;depth:number;kind:'material'|'cavity'}[]}
export interface StructuralSections {
  modelKind:'finished-mesh-sections-v1'
  sourceMesh:PolygonMesh
  axis:'x'|'y'|'z'
  planeAxes:[number,number]
  volumeMm3:number
  triangleCount:number
  selfIntersections:'not-checked'|'checked-at-tolerance'
  materialAudit:MaterialAudit
  connectivity:BoundaryConnectivity
  sections:StructuralSection[]
}
export function inspectStructuralSections(mesh:PolygonMesh,axis:'x'|'y'|'z',stations:number[]):StructuralSections {
  const result=callGeometryRust<StructuralSections>('structural_sections',{mesh,axis,stations})
  // The echoed source mesh decodes as plain arrays; box it once, here.
  if(result.sourceMesh)normalizePolygonMesh(result.sourceMesh)
  return result
}

/** One straight thin-walled strip: centerline endpoints and uniform thickness. */
export interface TorsionStrip {a:[number,number];b:[number,number];thicknessMm:number}
/** Single closed cell: wall centerline polygon, one thickness per edge. */
export interface TorsionCell {centerline:[number,number][];thicknessMm:number[]}
export interface SectionTorsion {
  /** Open part, Σ b·t³/3 over strips. */
  openMm4:number
  /** Closed part, Bredt–Batho 4A²/∮ ds/t per cell. */
  closedMm4:number
  totalMm4:number
}

/** Thin-walled torsion constants from textbook formulas; junction
 * concentration factors, multi-cell sections, warping, and the shear center
 * are out of scope. Not a certified section-table lookup.
 */
export function sectionTorsion(open:TorsionStrip[],closed:TorsionCell[]):SectionTorsion {
  return callGeometryRust<SectionTorsion>('section_torsion',{open,closed})
}

/** Full Vlasov property set of an open thin-walled section, centroidal. */
export interface ThinWalledOpenSection {
  areaMm2:number
  centroidMm:[number,number]
  /** Direction of principal axis 1 (larger moment), from +x. */
  principalAngleRad:number
  i1Mm4:number
  i2Mm4:number
  /** Saint-Venant torsion constant Σ b·t³/3. */
  jMm4:number
  /** Warping constant ∫ ω̄² t ds over the centerline. */
  cwMm6:number
  /** Shear center in the input coordinates. */
  shearCenterMm:[number,number]
  /** Shear area for a shear force along principal axis 1 / 2. */
  shearArea1Mm2:number
  shearArea2Mm2:number
}

/** Shear center, warping constant Cw, and shear areas of an open thin-walled
 * section from its wall centerline graph. Strips may branch (I/H sections);
 * endpoints on another strip's interior split it (T junctions); coincident
 * endpoints snap within 1e-9 of the span. The graph must be one connected
 * tree — closed loops are refused (use sectionTorsion for single cells).
 * Not a certified section-table lookup.
 */
export function thinWalledOpenSection(strips:TorsionStrip[]):ThinWalledOpenSection {
  return callGeometryRust<ThinWalledOpenSection>('thin_walled_section',{strips})
}
