import { PendingWheel, type WheelResolver } from './wheelBuild'
import { reparameterizeNurbsCurveExact } from '../../kernel/nurbsFoundation'
import { approximateCurveEdit, approximateSurfaceEdit } from './approximateEdits'
import { callGeometryRust } from '../../kernel/geometry/kernel'
import { prepareExactFullRoundDisplay } from '../../kernel/geometry/brepExactDisplay'
import { buildProgressiveSweepNode } from './buildProgressiveSweepNode'
import { buildDirectMiterNode } from './buildDirectMiterNode'
import { readSweepCorrections } from './readSweepCorrections'
import { reconstructCertifiedProgressiveStations, type ProgressiveBrepBody } from '../../kernel/geometry/brep'
import type { StationResolver, StationArguments } from '../rushGraphNurbsKernel'
import {
  PendingProgressiveStations,
  PendingProgressiveBody,
  PendingProgressiveMiter,
  PendingProgressiveSweep,
} from '../rushGraphNurbsKernel'
import type {
  BodyArguments,
  BodyResolver,
  MiterArguments,
  MiterResolver,
  OwnNurbsRequest,
  SweepResolver,
  Value,
} from '../rushGraphNurbsKernel'
import {
  booleanNurbsBrep,
  chamferNurbsBrepEdges,
  createBrepBox,
  createBrepCylinder,
  createBrepFrustum,
  createBrepGear,
  createBrepSphere,
  createBrepTorus,
  createBrepTube,
  createCappedBrepLoftSurfaces,
  createCappedBrepLoftWithCaps,
  createNaturalBrepSectionLoft,
  createRationalBrepSectionLoft,
  createProgressiveBrepProfileBody,
  createProgressiveMiterBrepProfileBody,
  extrudeBrepCurves,
  extrudeBrepPolygon,
  filletNurbsBrepEdges,
  reconstructCertifiedMiterStations,
  revolveBrepProfile,
  tessellateNurbsBrep,
  transformCertifiedMiterBody,
  transformNurbsBrep,
} from '../../kernel/geometry/brep'
import type { CertifiedMiterBoundaryOwner, NurbsBrep } from '../../kernel/geometry/brep'
import {
  decomposeNurbsCurve,
  splitNurbsCurve,
  elevateNurbsCurve,
  evaluateNurbsCurvePoints,
  insertNurbsKnot,
  editNurbsCurveWeights,
  relocateNurbsCurveSeam,
  curveExactInsertKnot,
  curveExactRemoveKnot,
  curveExactReduceDegree,
  curveNormalizeWeights,
  curveCancelFactor,
  curveExactSplit,
  curveExactRestrict,
  curveExactPartition,
  curveExactElevate,
  reverseNurbsCurve,
  transformNurbsCurve,
  trimNurbsCurve,
  validateNurbsCurve,
} from '../../kernel/nurbsCurve'
import type { NurbsCurve } from '../../kernel/nurbsCurve'
import {
  elevateNurbsSurface,
  insertNurbsSurfaceKnot,
  isoNurbsCurve,
  editNurbsSurfaceWeights,
  relocateNurbsSurfaceSeam,
  surfaceExactInsertKnot,
  surfaceExactRemoveKnot,
  surfaceExactReduceDegree,
  surfaceCancelFactor,
  reverseNurbsSurface,
  splitNurbsSurface,
  transposeNurbsSurface,
  decomposeNurbsSurface,
  transformNurbsSurface,
  transformNurbsSurfacePatches,
  trimNurbsSurface,
  validateNurbsSurface,
} from '../../kernel/nurbsSurface'
import type { NurbsSurface } from '../../kernel/nurbsSurface'
import { evaluateSdf, tessellateSdfGpuAware as tessellateSdf } from '../../kernel/geometry/sdf'
import type { SdfField } from '../../kernel/geometry/sdf'
import {
  approximateArchimedeanSpiralNurbsCurve,
  approximateCatenaryNurbsCurve,
  approximateCatenoidNurbsPatches,
  approximateCatenoidNurbsSurface,
  cubicWaveNurbsSurface,
  opticalDome,
  corrugatedBellows,
  superellipsoidPatches,
  opticalLens,
  approximateClothoidNurbsCurve,
  approximateConicalHelixNurbsCurve,
  approximateCycloidNurbsCurve,
  approximateEllipticHelixNurbsCurve,
  approximateEpicycloidNurbsCurve,
  approximateHelicoidNurbsPatches,
  approximateHelicoidNurbsSurface,
  approximateHelixNurbsCurve,
  approximateHypocycloidNurbsCurve,
  approximateInvoluteNurbsCurve,
  approximateLissajousNurbsCurve,
  approximateLogarithmicSpiralNurbsCurve,
  approximateScrewNurbsSurfaces,
  approximateSphericalSpiralNurbsCurve,
  approximateToroidalSpiralNurbsCurve,
  approximateTorusKnotNurbsCurve,
  approximateTrochoidNurbsCurve,
  approximateVariablePitchHelixNurbsCurve,
  autoGuidedLoftNurbsCurves,
  boundaryFillNurbsSurfaces,
  checkedPipeNurbsSurface,
  checkedProfileSweepNurbsSurface,
  checkedRibbonNurbsSurface,
  checkedVariablePipeNurbsSurface,
  circleRectangleTransitionNurbsPatches,
  circleTransitionNurbsSurface,
  clampedLoftNurbsCurves,
  closedLoftNurbsCurves,
  composeNurbsCurves,
  controlTangentLoftNurbsCurves,
  coonsNurbsPatch,
  ellipseTransitionNurbsSurface,
  extrudeNurbsCurve,
  extrudeNurbsCurvePatches,
  formulaNurbsCurve,
  formulaNurbsSurface,
  framedSweepNurbsCurve,
  gordonNurbsSurface,
  guidedLoftNurbsCurves,
  hermiteNurbsCurve,
  loftAlignedNurbsCurves,
  loftNurbsCurves,
  matchNurbsLoftEnds,
  naturalLoftNurbsCurves,
  naturalSplineNurbsCurve,
  revolveNurbsCurve,
  scaledSweepNurbsCurve,
  sweepNurbsCurve,
  triangularNurbsPatch,
  twistSweepNurbsCurve,
  twoGuideSweepNurbsCurve,
} from '../../kernel/nurbsConstructors'
import {
  booleanPolygonMeshes,
  extrudePolygonProfile,
  inspectPolygonMesh,
  loftPolygonSections,
  normalizePolygonMesh,
  revolvePolygonProfile,
  sweepPolygonProfile,
} from '../../kernel/geometry/polygon'
import {
  createAutoCartesianGuidedLoft,
  createAutoControlTangentCartesianGuidedLoft,
  createCartesianGuidedLoft,
  createControlTangentCartesianGuidedLoft,
  createMappedNaturalLoft,
} from '../../kernel/nurbsFoundation'
import { compileRushGraphNurbs } from '../rushGraphNurbsCompiler'
import { tessellateNurbsSurface, thickenNurbsMesh } from '../../kernel/geometry/tessellation'
import {
  meshToNurbs,
  meshToNurbsBrep,
  meshToSdf,
  meshToSubdivision,
  tessellateNurbsPatches,
} from '../../kernel/geometry/reconstruction'
import { tessellateSubdivision } from '../../kernel/geometry/subdivision'

import { buildNativePrimitive } from './buildNativePrimitive'
import { selectBrepEdges } from './selectBrepEdges'
type NativeNode = ReturnType<typeof compileRushGraphNurbs>['resolved_document']['nodes'][number]
export function createOwnNurbsResolver(
  nodes: Map<string, NativeNode>,
  _request: OwnNurbsRequest,
  resolveSweep?: SweepResolver,
  resolveBody?: BodyResolver,
  resolveMiter?: MiterResolver,
  resolveStations?: StationResolver,
  resolveWheel?: WheelResolver,
) {
  const cache = new Map<string, Value>()
  const needCurve = (key: string) => {
    const result = get(key)
    if (result.kind !== 'curve') throw new Error(`Expected curve at ${key}.`)
    return result.data
  }
  const needSurface = (key: string) => {
    const result = get(key)
    if (result.kind !== 'surface') throw new Error(`Expected surface at ${key}.`)
    return result.data
  }
  const needMesh = (key: string) => {
    const result = get(key)
    if (result.kind !== 'mesh') throw new Error(`Expected tessellated mesh at ${key}.`)
    return result.data
  }
  const needBrep = (key: string): NurbsBrep => {
    const v = get(key)
    if (v.kind !== 'brep') throw new Error('Expected a B-rep body')
    return v.data
  }
  const needSdf = (key: string): SdfField => {
    const v = get(key)
    if (v.kind !== 'sdf') throw new Error('Expected SDF field')
    return v.data
  }
  const reconstructionReports: Record<string, unknown> = {}
  const progressiveBodySources = new Map<string, ProgressiveBrepBody>()
  const miterSources = new Map<string, CertifiedMiterBoundaryOwner>()
  const exactMiterPlacements = new Map<
    string,
    { body: CertifiedMiterBoundaryOwner; quantum: number; maxWork: number }
  >()
  const transferredTrimLoops = new Map<string, NurbsCurve[][]>()
  const constructionReports: Record<string, object> = {}
  let meshTriangles = 0
  function get(key: string): Value {
    const cached = cache.get(key)
    if (cached) return cached
    const n = nodes.get(key)
    if (!n) throw new Error('Unknown node ' + key)
    let result: Value
    try {
      const primitive = buildNativePrimitive(n)
      if (primitive) result = primitive
      else
        switch (n.op) {
          case 'curve_compose':
            result = { kind: 'curve', data: composeNurbsCurves(n.inputs.map(needCurve)) }
            break
          case 'curve': {
            const c: NurbsCurve = {
              degree: n.degree,
              knots: n.knots,
              controlPoints: n.control_points,
              weights: n.weights,
              periodic: n.periodic,
            }
            validateNurbsCurve(c)
            result = { kind: 'curve', data: c }
            break
          }
          case 'surface': {
            const s: NurbsSurface = {
              degreeU: n.degree_u,
              degreeV: n.degree_v,
              knotsU: n.knots_u,
              knotsV: n.knots_v,
              controlPoints: n.control_points,
              weights: n.weights,
              periodicU: n.periodic_u,
              periodicV: n.periodic_v,
            }
            validateNurbsSurface(s)
            result = { kind: 'surface', data: s }
            break
          }
          case 'curve_remove_knot':
          case 'curve_reduce_degree':
          case 'curve_rebuild':
            result = { kind: 'curve', data: approximateCurveEdit(needCurve(n.input), n) }
            break
          case 'surface_remove_knot':
          case 'surface_reduce_degree':
          case 'surface_rebuild':
            result = { kind: 'surface', data: approximateSurfaceEdit(needSurface(n.input), n) }
            break
          case 'surface_split':
            result = { kind: 'surface', data: splitNurbsSurface(needSurface(n.input), n.direction, n.parameter)[n.part] }
            break
          case 'surface_transpose':
            result = { kind: 'surface', data: transposeNurbsSurface(needSurface(n.input)) }
            break
          case 'surface_bezier_patch': {
            const patch = decomposeNurbsSurface(needSurface(n.input))[n.index]
            if (!patch) throw new Error('Bezier patch index outside surface')
            result = { kind: 'surface', data: patch }
            break
          }
          case 'curve_edit_weights':
            result = { kind: 'curve', data: editNurbsCurveWeights(needCurve(n.input), n.edits) }
            break
          case 'curve_relocate_seam':
            result = { kind: 'curve', data: relocateNurbsCurveSeam(needCurve(n.input), n.parameter, n.max_deviation, n.max_cells) }
            break
          case 'surface_edit_weights':
            result = { kind: 'surface', data: editNurbsSurfaceWeights(needSurface(n.input), n.edits) }
            break
          case 'surface_relocate_seam':
            result = { kind: 'surface', data: relocateNurbsSurfaceSeam(needSurface(n.input), n.direction, n.parameter, n.max_deviation, n.max_cells) }
            break
          case 'curve_exact_insert_knot':
            result = { kind: 'curve', data: curveExactInsertKnot(needCurve(n.input), n.parameter, n.count, n.max_work) }
            break
          case 'curve_exact_remove_knot':
            result = { kind: 'curve', data: curveExactRemoveKnot(needCurve(n.input), n.parameter, n.max_work) }
            break
          case 'curve_exact_reduce_degree':
            result = { kind: 'curve', data: curveExactReduceDegree(needCurve(n.input), n.max_work) }
            break
          case 'surface_exact_insert_knot':
            result = { kind: 'surface', data: surfaceExactInsertKnot(needSurface(n.input), n.direction, n.parameter, n.count, n.max_work) }
            break
          case 'surface_exact_remove_knot':
            result = { kind: 'surface', data: surfaceExactRemoveKnot(needSurface(n.input), n.direction, n.parameter, n.max_work) }
            break
          case 'surface_exact_reduce_degree':
            result = { kind: 'surface', data: surfaceExactReduceDegree(needSurface(n.input), n.direction, n.max_work) }
            break
          case 'curve_normalize_weights':
            result = { kind: 'curve', data: curveNormalizeWeights(needCurve(n.input)) }
            break
          case 'curve_cancel_factor':
            result = { kind: 'curve', data: curveCancelFactor(needCurve(n.input), n.max_work) }
            break
          case 'surface_cancel_factor':
            result = { kind: 'surface', data: surfaceCancelFactor(needSurface(n.input), n.direction, n.max_work) }
            break
          case 'curve_exact_split':
            result = { kind: 'curve', data: curveExactSplit(needCurve(n.input), n.parameter, n.part, n.max_work) }
            break
          case 'curve_exact_restrict':
            result = { kind: 'curve', data: curveExactRestrict(needCurve(n.input), n.domain, n.max_work) }
            break
          case 'curve_exact_partition':
            result = { kind: 'curve', data: curveExactPartition(needCurve(n.input), n.cuts, n.part, n.max_work) }
            break
          case 'curve_exact_elevate':
            result = { kind: 'curve', data: curveExactElevate(needCurve(n.input), n.degree, n.max_work) }
            break
          case 'assemble_exact_periodic_surface_axis':
          case 'assemble_periodic_surface_axis_with_exact_weight_scaling':
          case 'assemble_periodic_surface_axis_with_exact_knot_union':
          case 'assemble_periodic_surface_axis_with_exact_basis_matching':
          case 'assemble_periodic_surface_axis_with_exact_degree_matching':
          case 'assemble_exact_doubly_periodic_surface':
          case 'assemble_doubly_periodic_surface_with_exact_weight_scaling':
            {
              const layout = 'direction' in n ? { axis: n.direction } : { columns: n.columns }
              result = { kind: 'surface', data: callGeometryRust<NurbsSurface>(n.op, {
                inputs: n.inputs.map(needSurface), max_work: n.max_work, ...layout,
              }) }
              break
            }
          case 'rotate_periodic_surface_at_exact_knot':
          case 'insert_and_rotate_periodic_surface_seam':
          case 'open_exact_periodic_surface_axis':
          case 'restrict_exact_periodic_surface_axis':
          case 'restrict_exact_periodic_surface_over_turns':
          case 'restrict_exact_doubly_periodic_surface':
          case 'restrict_exact_doubly_periodic_surface_over_turns':
          case 'partition_exact_periodic_surface_axis':
          case 'partition_exact_periodic_surface_over_turns':
          case 'partition_exact_doubly_periodic_surface':
          case 'partition_exact_doubly_periodic_surface_over_turns':
          case 'remove_exact_periodic_surface_knot':
          case 'simplify_exact_periodic_surface_axis':
          case 'remove_exact_periodic_surface_seam_knot':
          case 'simplify_exact_doubly_periodic_surface':
          case 'reduce_exact_periodic_surface_period':
          case 'repeat_exact_periodic_surface_period':
            {
              const { id: _id, op, input, ...parameters } = n
              const { direction, ...rest } = 'direction' in parameters ? parameters : { ...parameters, direction: undefined }
              const data = callGeometryRust<NurbsSurface>(op, { surface: needSurface(input), ...rest, axis: direction })
              result = { kind: 'surface', data }
              break
            }
          case 'curve_axis_motion_partition_speed':
          case 'curve_axis_motion_partition_motion':
          case 'curve_axis_motion_partition_axes':
          case 'curve_axis_motion_limits':
          case 'curve_axis_motion_slowdown':
          case 'curve_axis_motion_multi_slowdown':
          case 'curve_axis_motion_duration':
          case 'curve_axis_motion_deadline': {
            const curve = needCurve(n.input)
            const mode = n.op.slice('curve_axis_motion_'.length)
            const report = callGeometryRust<object>('nurbs_curve_axis_motion', {
              curve, mode: mode === 'multi_slowdown' ? 'multiSlowdown' : mode,
              parameterInterval: n.interval, directions: 'axis_vector' in n ? [n.axis_vector] : n.directions, limits: 'speed_limit' in n ? [[n.speed_limit, 0, 0]] : n.limits,
              ...('parameter_tolerance' in n ? { parameterTolerance:n.parameter_tolerance } : { tolerances:n.tolerances }),
              maxCells: n.max_cells, ...('deadline' in n ? { deadline:n.deadline } : {}),
            })
            constructionReports[key] = { measurement: { operation: n.op, report } }
            result = { kind:'curve', data:curve }
            break
          }
          case 'curve_directional_travel_full':
          case 'curve_directional_travel_signed':
          case 'curve_check_backtracking': {
            const curve = needCurve(n.input)
            const report = callGeometryRust<object>('nurbs_' + n.op, {
              curve, direction: n.axis_vector, toleranceMm:n.tolerance_mm,maxCells:n.max_cells,
              ...(n.op === 'curve_check_backtracking' ? { parameterInterval:n.interval, reverseLimitMm:n.reverse_limit } : {}),
            })
            constructionReports[key] = { measurement:{operation:n.op,report} }
            result = { kind:'curve',data:curve }
            break
          }
          case 'surface_trim_contour_audit':
          case 'surface_trim_contour_station':
          case 'surface_trim_contour_station_box':
          case 'surface_trim_contour_divide':
          case 'surface_trim_patch_area_vector':
          case 'surface_trim_patch_projected_area': {
            const surface = needSurface(n.input)
            const operation = {
              surface_trim_contour_audit:'brep_audit_exact_periodic_trim_contour',
              surface_trim_contour_station:'brep_locate_exact_periodic_trim_contour_station',
              surface_trim_contour_station_box:'brep_enclose_exact_periodic_trim_contour_station',
              surface_trim_contour_divide:'brep_divide_exact_periodic_trim_contour_by_length',
              surface_trim_patch_area_vector:'brep_bound_exact_periodic_trim_patch_area',
              surface_trim_patch_projected_area:'brep_bound_exact_periodic_trim_directional_area',
            }[n.op]
            const report = callGeometryRust<object>(operation, {
              surface,contours:n.contours,maxWork:n.max_work,
              ...('contour' in n ? {contour:n.contour} : {}),
              ...('outer_contour' in n ? {outerContour:n.outer_contour} : {}),
              ...('distance_mm' in n ? {distanceMm:n.distance_mm} : {}),
              ...('divisions' in n ? {divisions:n.divisions} : {}),
              ...('tolerance_mm' in n ? {toleranceMm:n.tolerance_mm} : {}),
              ...('parameter_tolerance' in n ? {parameterTolerance:n.parameter_tolerance} : {}),
              ...('max_cells' in n ? {maxCells:n.max_cells} : {}),
              ...('tolerance_mm2' in n ? {toleranceMm2:n.tolerance_mm2} : {}),
              ...('normal' in n ? {normal:n.normal} : {}),
            })
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'surface_trim_range_length': {
            const surface=needSurface(n.input)
            const range=[n.range_start,n.range_end].map(pair=>{
              const [numerator,denominator]=pair.split('/')
              return {numerator,denominator}
            })
            const report=callGeometryRust<object>('brep_measure_exact_periodic_trim_range',{
              surface,contours:n.contours,contour:n.contour,segment:n.segment,range,
              toleranceMm:n.tolerance_mm,maxWork:n.max_work,maxCells:n.max_cells,
            })
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'evaluate_exact_wrapped_surface_point':
          case 'evaluate_exact_wrapped_surface_points':
          case 'lift_exact_periodic_surface_path':
          case 'exact_periodic_surface_path_winding':
          case 'exact_periodic_surface_path_signed_area':
          case 'exact_periodic_surface_path_centroid':
          case 'exact_periodic_surface_path_inertia': {
            const surface=needSurface(n.input)
            const report=callGeometryRust<object>(n.op,{
              surface,max_work:n.max_work,...('uv' in n ? {uv:n.uv} : {parameters:n.uv_points}),
            })
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'classify_exact_periodic_surface_path_point':
          case 'classify_exact_periodic_surface_path_points':
          case 'classify_exact_periodic_surface_region_points':
          case 'certify_exact_periodic_surface_path_simplicity':
          case 'check_exact_periodic_surface_path_contact':
          case 'relate_exact_periodic_surface_paths':
          case 'exact_periodic_surface_contour_forest':
          case 'exact_periodic_surface_region_area':
          case 'exact_periodic_surface_region_component_centroids':
          case 'exact_periodic_surface_region_component_inertia': {
            const {id:_id,op,input,...parameters}=n
            const surface=needSurface(input)
            const report=callGeometryRust<object>(op,{surface,...parameters})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'build_parallel_sweep_overlap':
          case 'build_profile_boolean_solid': {
            const {op,id:_id,...settings}=n
            const {model,report}=callGeometryRust<{model:NurbsBrep;report:object}>(op,settings)
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'brep',data:model}
            break
          }
          case 'triangulate_planar_path':
          case 'measure_path_second_moments':
          case 'bound_path_principal_moments':
          case 'measure_region_mass':
          case 'intersect_transverse_profiles':
          case 'measure_profile_overlap_area':
          case 'measure_profile_overlap_centroid':
          case 'classify_profile_overlap':
          case 'decompose_profile_boolean':
          case 'extract_profile_boolean_edges':
          case 'assemble_profile_boolean_loops':
          case 'triangulate_profile_boolean':
          case 'measure_profile_extrusion_inertia':
          case 'locate_profile_extrusion_points':
          case 'section_profile_extrusion':
          case 'section_profile_extrusion_parallel':
          case 'split_profile_extrusion_volume':
          case 'measure_parallel_sweep_overlap':
          case 'mesh_parallel_sweep_overlap':
          case 'clip_profile_extrusion_path':
          case 'bound_profile_extrusion_area':
          case 'extrude_profile_boolean_shell':
          case 'decompose_profile_overlap':
          case 'clip_path_to_perforated_region':
          case 'measure_region_second_moments':
          case 'measure_planar_path_area':
          case 'measure_planar_path_centroid':
          case 'simplify_exact_path':
          case 'project_to_planar_region':
          case 'project_point_to_path':
          case 'locate_planar_path_points':
          case 'clip_path_to_planar_region':
          case 'intersect_spatial_paths':
          case 'node_spatial_path':
          case 'find_path_self_intersections':
          case 'measure_spatial_path_distance':
          case 'simplify_path_with_tolerance': {
            const {op,id:_id,...settings}=n
            const {curve,report}=callGeometryRust<{curve:NurbsCurve;report:object}>(op,settings)
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'fit_cst_sides':
          case 'naca_five_digit_profile':
          case 'cst_spline_profile':
          case 'cst_section_integrals': {
            const {op,id:_id,...settings}=n
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve;chordMm:number}>(op,settings)
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'fit_sphere_cloud':
          case 'fit_cylinder_cloud':
          case 'fit_cone_cloud':
          case 'fit_torus_cloud':
          case 'fit_plane_cloud': {
            const {id:_id,op,...settings}=n
            const {surface,...report}=callGeometryRust<{surface:NurbsSurface;rmsMm:number;maxDeviationMm:number}>(op,settings)
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'surface_highlight_lines':
          case 'surface_reflection_lines':
          case 'surface_isophote_analysis':
          case 'surface_dent_detection': {
            const {id:_id,input,op,...options}=n
            const surface=needSurface(input)
            const report=callGeometryRust<object>(op,{surface,...options})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'curve_from_curvature_samples': {
            const {id:_id,op,...options}=n
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve;fitConverged:boolean}>(op,options)
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'fit_biarc_piece': {
            const {id:_id,input,op,...options}=n
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve;pieces:number}>(op,{curve:needCurve(input),...options})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'simplify_linear_curve':
          case 'extend_terminal_curve': {
            const {op,id:_id,input,...options}=n
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve;complete?:boolean;stable?:boolean}>(op,{curve:needCurve(input),...options})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'extend_surface_boundary': {
            const {op,id:_id,input,...options}=n
            const {surface,...report}=callGeometryRust<{surface:NurbsSurface;stable:boolean}>(op,{surface:needSurface(input),...options})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'affine_control_support': {
            const source=get(n.input)
            if(source.kind!=='curve' && source.kind!=='surface') throw new Error('Affine support needs a curve or surface')
            const report=callGeometryRust<object>(n.op,{[source.kind]:source.data,max_work:n.max_work})
            constructionReports[key]={measurement:{operation:n.op,report}}
            result=source
            break
          }
          case 'surface_trace_agreement': {
            const {id:_id,input,inputs,op,...settings}=n
            const surface=needSurface(input)
            const report=callGeometryRust<object>(op,{surface,curve:needCurve(inputs[0]),pcurve:needCurve(inputs[1]),...settings})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'signed_shell_volume': {
            const {input,inputs,id:_id,op,...settings}=n
            const surfaces=[input,...inputs].map(ref=>needSurface(ref))
            const report=callGeometryRust<object>(op,{surfaces,...settings})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surfaces[0]!}
            break
          }
          case 'geometry_bounds_union':
          case 'geometry_conditioning':
          case 'coordinate_frame_local':
          case 'coordinate_frame_world': {
            const source=get(n.input)
            if (source.kind!=='curve' && source.kind!=='surface') throw new Error('Expected a curve or surface')
            if(n.op==='geometry_bounds_union'){
              const items=[n.input,...n.inputs].map(ref=>{
                const geometry=get(ref)
                if(geometry.kind!=='curve' && geometry.kind!=='surface') throw new Error('Bounds require curves or surfaces')
                return {[geometry.kind]:geometry.data}
              })
              const report=callGeometryRust<object>(n.op,{items})
              constructionReports[key]={measurement:{operation:n.op,report}}
              result=source
            } else if(n.op==='geometry_conditioning'){
              const report=callGeometryRust<object>(n.op,{[source.kind]:source.data})
              constructionReports[key]={measurement:{operation:n.op,report}}
              result=source
            } else {
              const frame={origin:n.origin,frame_scale:n.frame_scale}
              result=source.kind==='curve'
                ? {kind:'curve',data:callGeometryRust<NurbsCurve>(n.op,{curve:source.data,...frame})}
                : {kind:'surface',data:callGeometryRust<NurbsSurface>(n.op,{surface:source.data,...frame})}
            }
            break
          }
          case 'shared_patch_mesh': {
            const {id:_id,input,inputs,op,...options}=n
            const {positions,indices,faceIds,...report}=callGeometryRust<{positions:number[];indices:number[];faceIds:number[];errorUpperMm:number}>(op,{surfaces:[input,...inputs].map(needSurface),...options})
            const mesh=normalizePolygonMesh({positions,indices,faceIds,sharedPatchVertices:true as const})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'mesh',data:{...mesh,report:inspectPolygonMesh(mesh)}}
            break
          }
          case 'bounded_curve_chord': {
            const {id:_id,input,op,...options}=n
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve;errorUpperMm:number;cells:number}>(op,{curve:needCurve(input),...options})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'bounded_surface_mesh': {
            const {id:_id,input,op,...options}=n
            const {positions,indices,uv,...report}=callGeometryRust<{positions:number[];indices:number[];uv:number[];errorUpperMm:number;cells:number}>(op,{surface:needSurface(input),...options})
            const mesh=normalizePolygonMesh({positions,indices,uv})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'mesh',data:{...mesh,report:inspectPolygonMesh(mesh)}}
            break
          }
          case 'split_periodic_plane_strip':
          case 'split_cell_by_plane': {
            const {id:_id,op,input,...settings}=n
            const surface=needSurface(input)
            const report=callGeometryRust<{trimLoops:NurbsCurve[][];workUsed:number}>(op,{surface,...settings})
            transferredTrimLoops.set(key,report.trimLoops)
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'store_plane_branch':
          case 'store_closed_plane_piece':
          case 'pair_plane_branch': {
            const {id:_id,op,input,...settings}=n
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve;workUsed:number}>(op,{surface:needSurface(input),...settings})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'close_plane_strip':
          case 'stitch_plane_strip':
          case 'trace_plane_strip':
          case 'sample_plane_branch':
          case 'plane_branch_uv_polyline':
          case 'plane_branch_xyz_polyline':
          case 'audit_plane_side':
          case 'refine_plane_side':
          case 'audit_plane_graphs': {
            const {id:_id,op,input,...settings}=n
            const surface=needSurface(input)
            const report=callGeometryRust<object>(op,{surface,...settings})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'parallel_plane_sections':
          case 'parallel_section_piece': {
            const {id:_id,op,input,...settings}=n
            const surface=needSurface(input)
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve|null;complete:boolean;sections:object[]}>(op,{surface,...settings})
            constructionReports[key]={measurement:{operation:op,report}}
            result=curve ? {kind:'curve',data:curve} : {kind:'surface',data:surface}
            break
          }
          case 'clean_offset_boundary':
          case 'planar_toolpath_edge': {
            const {input,inputs,id:_id,op,...settings}=n
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve|null;stop:string}>(op,{curves:[input,...inputs].map(needCurve),...settings})
            if (!curve) throw new Error(op+': '+report.stop)
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'embed_xy_curve': {
            result={kind:'curve',data:callGeometryRust<NurbsCurve>(n.op,{curve:needCurve(n.input),height:n.height})}
            break
          }
          case 'planar_offset_piece': {
            const {curves,report}=callGeometryRust<{curves:NurbsCurve[];report:object}>('curve_offset_bounded',{
              curve:needCurve(n.input),distance:n.distance_mm,toleranceMm:n.tolerance_mm,maxCells:n.max_cells,
            })
            const curve=curves[n.piece]
            if (!curve) throw new Error('Selected offset piece is outside the constructed curve list')
            constructionReports[key]={measurement:{operation:n.op,report:{...report,pieces:curves.length}}}
            result={kind:'curve',data:curve}
            break
          }
          case 'planar_knot_miter': {
            const {input,id:_id,op,...args}=n
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve;points:number[][];errorUpperMm:number}>(op,{curve:needCurve(input),...args})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'phase_aligned_section': {
            const {id:_id,op,input,inputs,...options}=n
            const {curve,phases}=callGeometryRust<{curve:NurbsCurve|null;phases:{status:string}[]}>(op,{
              curves:[input,...inputs].map(needCurve),...options,
            })
            if (!curve) throw new Error('Section phase alignment unresolved: '+phases.map(p=>p.status).join(', '))
            constructionReports[key]={measurement:{operation:op,report:{phases}}}
            result={kind:'curve',data:curve}
            break
          }
          case 'compatible_section':
          case 'oriented_section': {
            const {id:_id,op,input,inputs,...options}=n
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve;degree:number;knots:number[];orientation?:string}>(op,{
              curves:[input,...inputs].map(needCurve),...options,
            })
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'uv_loop_closure':
          case 'uv_loop_simplicity':
          case 'uv_trim_location': {
            const {id:_id,input,op,loops,...options}=n
            const surface=needSurface(input)
            const report=callGeometryRust<object>(op,{loops:loops.map(loop=>loop.map(needCurve)),...options})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'combine_surface_measures':
          case 'combine_surface_densities': {
            const {id:_id,input,inputs,op,...parameters}=n
            const surfaces=[input,...inputs].map(id=>needSurface(id))
            const report=callGeometryRust<object>(op,{surfaces,...parameters})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surfaces[0]!}
            break
          }
          case 'mass_weighted_centroid':
          case 'physical_principal_moments':
          case 'physical_moment_gaps':
          case 'find_principal_axis':
          case 'bound_axis_projector':
          case 'certify_principal_direction':
          case 'tensor_principal_bounds':
          case 'tensor_axis_bounds':
          case 'trim_set_axis_bounds':
          case 'periodic_trim_set_area':
          case 'periodic_trim_set_centroid': {
            const {id:_id,input,op,...parameters}=n
            const surface=needSurface(input)
            const report=callGeometryRust<object>(op,{surface,...parameters})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'trimmed_surface_area_in_window':
          case 'trimmed_surface_centroid':
          case 'trimmed_surface_area_inertia': {
            const {id:_id, input, op, loops, ...args}=n
            const surface=needSurface(input)
            if (loops && (args.outer_contour !== undefined || args.max_work !== undefined)) throw new Error('Outer contour and construction budget apply only to periodic contour construction.')
            const report=callGeometryRust<object>(op,{surface,...args,...(loops ? {loops:loops.map(loop=>loop.map(needCurve))} : {})})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'extract_input_range_curves':
          case 'extract_input_range_curve_pairs': {
            const {id: _id, op, input, piece, ...args} = n
            const surface = needSurface(input)
            const usePcurve = 'use_pcurve' in args && args.use_pcurve
            if (usePcurve && piece === undefined) throw new Error('Selecting a face-local curve requires a piece index.')
            const report = callGeometryRust<{result: {curve: NurbsCurve; pcurve?: NurbsCurve}[] | null; refusal: string | null}>(op, {surface, ...args})
            constructionReports[key] = {measurement: {operation: op, report}}
            if (piece === undefined) {
              result = {kind: 'surface', data: surface}
            } else {
              const selected = report.result?.[piece]
              const curve = usePcurve ? selected?.pcurve : selected?.curve
              if (!curve) throw new Error(report.refusal ?? 'Trim curve piece index is outside the extracted range.')
              result = {kind: 'curve', data: curve}
            }
            break
          }
          case 'locate_input_parameter':
          case 'locate_edge_parameter':
          case 'locate_input_range':
          case 'evaluate_input_point':
          case 'evaluate_input_tangents':
          case 'classify_input_tangent_join':
          case 'classify_input_vertex_join':
          case 'lift_exact_periodic_surface_region_components':
          case 'build_exact_periodic_surface_trim_components':
          case 'build_exact_periodic_surface_trim_patches':
          case 'build_exact_periodic_surface_trim_edges':
          case 'measure_input_contour_length':
          case 'measure_input_segment_length':
          case 'measure_input_segment_range_length':
          case 'locate_input_segment_at_length':
          case 'measure_input_contour_prefix_lengths':
          case 'locate_input_contour_at_lengths':
          case 'divide_input_contour_by_length':
          case 'exact_input_contour_projected_area':
          case 'exact_input_contour_polynomial_projected_area':
          case 'bound_input_contour_projected_area':
          case 'bound_input_patch_max_projected_area':
          case 'bound_input_patch_max_projection_direction': {
            const {id:_id,op,input,...parameters}=n
            const surface=needSurface(input)
            const report=callGeometryRust<object>(op,{surface,...parameters})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'surface_trim_sheet_display':
          case 'surface_trim_sheet_step': {
            const {id: _id, input, op, ...args} = n
            const delivery = callGeometryRust<{model: NurbsBrep | null; step: string | null; mesh: ReturnType<typeof tessellateNurbsBrep> | null; refusal: string | null; workUsed: number}>(op, {surface:needSurface(input), ...args})
            if (!delivery.model) throw new Error('Periodic sheet delivery refused: ' + delivery.refusal)
            constructionReports[key] = {measurement: {operation:op, report:{workUsed:delivery.workUsed, refusal:delivery.refusal}}}
            if (n.op === 'surface_trim_sheet_step') {
              if (!delivery.step) throw new Error('Missing periodic sheet STEP output')
              result = {kind:'brep', data:delivery.model, stepText:delivery.step}
            } else {
              if (!delivery.mesh) throw new Error('Missing periodic sheet display mesh')
              result = {kind:'brep', data:delivery.model, displayMesh:{segments:n.segments,mesh:normalizePolygonMesh(delivery.mesh)}}
            }
            break
          }
          case 'surface_trim_sheet':
          case 'surface_trim_sheet_edge_bindings':
          case 'surface_trim_sheet_input_bindings': {
            const operation = {
              surface_trim_sheet:'brep_build_exact_periodic_trim_sheet',
              surface_trim_sheet_edge_bindings:'brep_build_exact_periodic_trim_sheet_with_bindings',
              surface_trim_sheet_input_bindings:'brep_build_exact_periodic_trim_sheet_with_input_bindings',
            }[n.op]
            const {model,...report}=callGeometryRust<{model:NurbsBrep|null;workUsed:number;refusalStage:string|null;edgeBindings?:unknown[]}>(operation,{
              surface:needSurface(n.input),contours:n.contours,toleranceMm:n.tolerance_mm,maxWork:n.max_work,
            })
            if(!model) throw new Error('Trimmed sheet construction refused at ' + report.refusalStage + ' after ' + report.workUsed + ' work units.')
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'brep',data:model}
            break
          }
          case 'surface_trim_area':
          case 'surface_trim_mass_properties':
          case 'surface_trim_window_properties':
          case 'surface_trim_set_properties': {
            const surface = needSurface(n.input)
            if (n.op === 'surface_trim_set_properties' && !!n.axis_point !== !!n.axis_vector) {
              throw new Error('The inertia axis needs both its point and direction.')
            }
            const operation = {
              surface_trim_area:'brep_measure_exact_periodic_trim_surface_area',
              surface_trim_mass_properties:'brep_measure_exact_periodic_trim_surface_mass_properties',
              surface_trim_window_properties:'brep_measure_exact_periodic_trim_window_mass_properties',
              surface_trim_set_properties:'brep_measure_exact_periodic_trim_patch_set_mass_properties',
            }[n.op]
            const report = callGeometryRust<object>(operation,{
              surface,contours:n.contours,toleranceMm2:n.tolerance_mm2,toleranceUv:n.tolerance_uv,maxWork:n.max_work,
              maxSurfaceCells:n.max_surface_cells,maxTrimCells:n.max_trim_cells,
              ...('outer_contour' in n ? {outerContour:n.outer_contour} : {}),
              ...('uv_window' in n ? {uvWindow:n.uv_window} : {}),
              ...('moment_tolerance' in n ? {momentToleranceMm4:n.moment_tolerance} : {}),
              ...('axis_point' in n && n.axis_point ? {inertiaAxis:{pointMm:n.axis_point,direction:n.axis_vector}} : {}),
              ...('reference_point' in n ? {inertiaReferencePointMm:n.reference_point} : {}),
              ...('radius_of_gyration' in n ? {includeAxisRadiusOfGyration:n.radius_of_gyration} : {}),
            })
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'surface_measure_trimmed_mass': {
            const surface = needSurface(n.input)
            if (!!n.plane_point !== !!n.plane_normal || !!n.axis_point !== !!n.axis_vector) {
              throw new Error('A measurement plane or axis needs both its point and its direction.')
            }
            const report = callGeometryRust<object>('brep_measure_exact_periodic_trim_patch_set_density', {
              ...(n.principal_axes === undefined ? {} : {includePrincipalAxes:n.principal_axes}),
              ...(n.probe_point ? {principalAxisProbePointMm:n.probe_point} : {}),
              ...(n.probe_direction ? {principalAxisProbeDirection:n.probe_direction} : {}),
              ...(n.balance_tolerance === undefined ? {} : {axisBalanceToleranceMm:n.balance_tolerance}),
              ...(n.static_unbalance === undefined ? {} : {includeStaticUnbalance:n.static_unbalance}),
              ...(n.couple_unbalance === undefined ? {} : {includeCoupleUnbalance:n.couple_unbalance}),
              ...(n.plane_offsets ? {balancePlaneOffsetsMm:n.plane_offsets} : {}),
              ...(n.plane_tolerances ? {balancePlaneTolerancesKgMm:n.plane_tolerances} : {}),
              ...(n.weight_radii ? {counterweightRadiiMm:n.weight_radii} : {}),
              ...(n.weight_mass_limits ? {counterweightMassLimitsKg:n.weight_mass_limits} : {}),
              ...(n.installed_weights ? {installedCounterweights:n.installed_weights.map(([massBoundsKg,...radialOffsetBoundsMm])=>({massBoundsKg,radialOffsetBoundsMm}))} : {}),
              surface, includeAxisRadiusOfGyration:n.radius_of_gyration ?? false, contours:n.contours, toleranceMm2:n.tolerance_mm2, toleranceUv:n.tolerance_uv, maxWork:n.max_work,
              maxSurfaceCells:n.max_surface_cells,maxTrimCells:n.max_trim_cells,
              componentDensities:n.component_densities.map(([outerContour,lo,hi])=>({outerContour,densityBoundsKgPerMm2:[lo,hi]})),
              ...(n.plane_point ? {centerOfMassPlane:{pointMm:n.plane_point,normal:n.plane_normal}} : {}),
              ...(n.axis_point ? {inertiaAxis:{pointMm:n.axis_point,direction:n.axis_vector}} : {}),
              ...(n.reference_point ? {inertiaReferencePointMm:n.reference_point} : {}),
              ...(n.moment_tolerance === undefined ? {} : {momentToleranceKgMm2:n.moment_tolerance}),
            })
            constructionReports[key] = {measurement:{operation:n.op,report}}
            result = {kind:'surface',data:surface}
            break
          }
          case 'curve_axial_derivative_peak': {
            const curve = needCurve(n.input)
            const report = callGeometryRust<object>('nurbs_measure_curve_axial_derivative_peak', {
              curve, direction: n.axis_vector, parameterInterval: n.interval, order: n.order,
              tolerance: n.tolerance_mm, maxCells: n.max_cells,
              ...(n.parameter_tolerance === undefined ? {} : { parameterTolerance: n.parameter_tolerance }),
            })
            constructionReports[key] = { measurement: { operation: n.op, report } }
            result = { kind: 'curve', data: curve }
            break
          }
          case 'surface_edit_deviation':
          case 'surface_seam_g0':
          case 'surface_seam_g1':
          case 'surface_seam_g2': {
            const {id:_id,op,input,target_geometry,...options} = n
            const surface = needSurface(input)
            const report = callGeometryRust<object>(op,{surface,target:needSurface(target_geometry),...options})
            constructionReports[key] = {measurement:{operation:op,report}}
            result = {kind:'surface',data:surface}
            break
          }
          case 'curve_edit_deviation':
          case 'curve_join_continuity': {
            const {id:_id,op,input,target_geometry,...options} = n
            const curve = needCurve(input)
            const report = callGeometryRust<object>(op,{curve,target:needCurve(target_geometry),...options})
            constructionReports[key] = {measurement:{operation:op,report}}
            result = {kind:'curve',data:curve}
            break
          }
          case 'curve_self_contacts': {
            const { id:_id, op, input, ...options } = n
            const curve = needCurve(input)
            const report = callGeometryRust<object>(op,{curve,...options})
            constructionReports[key] = {measurement:{operation:op,report}}
            result = {kind:'curve',data:curve}
            break
          }
          case 'bounded_surface_partial_field':
          case 'surface_partial_field':
          case 'bounded_surface_area_normal_field':
          case 'surface_area_normal_field':
          case 'bounded_surface_gaussian_field':
          case 'bounded_surface_mean_squared_field':
          case 'bounded_surface_principal_gap_squared_field':
          case 'bounded_surface_metric_field':
          case 'surface_gaussian_field':
          case 'surface_mean_squared_field':
          case 'surface_principal_gap_squared_field':
          case 'surface_metric_field':
          case 'bounded_surface_mixed_partial_field':
          case 'surface_mixed_partial_field': {
            const {id:_id,op,input,return_field,...settings} = n
            const source = needSurface(input)
            const {surface,...report} = callGeometryRust<{surface:NurbsSurface;patchCount:number}>(op,{source,...settings})
            constructionReports[key] = {measurement:{operation:op,report}}
            result = {kind:'surface',data:return_field ? surface : source}
            break
          }
          case 'bounded_curve_derivative_field':
          case 'curve_derivative_field':
          case 'bounded_curve_torsion_field':
          case 'bounded_curve_speed_squared_field':
          case 'curve_torsion_field':
          case 'curve_speed_squared_field':
          case 'bounded_curve_binormal_field':
          case 'curve_binormal_field':
          case 'bounded_curve_curvature_squared_field':
          case 'curve_curvature_squared_field': {
            const {id:_id,op,input,return_field,...settings} = n
            const source = needCurve(input)
            const {curve,...report} = callGeometryRust<{curve:NurbsCurve;spanCount:number}>(op,{source,...settings})
            constructionReports[key] = {measurement:{operation:op,report}}
            result = {kind:'curve',data:return_field ? curve : source}
            break
          }
          case 'projective_curve_exact': {
            const source = needCurve(n.input)
            const {curve,...report} = callGeometryRust<{curve:NurbsCurve;workUsed:number}>(n.op,{source,coefficients:n.coefficients,max_work:n.max_work})
            constructionReports[key] = {measurement:{operation:n.op,report}}
            result = {kind:'curve',data:curve}
            break
          }
          case 'projective_surface_exact': {
            const source = needSurface(n.input)
            const {surface,...report} = callGeometryRust<{surface:NurbsSurface;workUsed:number}>(n.op,{source,coefficients:n.coefficients,max_work:n.max_work})
            constructionReports[key] = {measurement:{operation:n.op,report}}
            result = {kind:'surface',data:surface}
            break
          }
          case 'merge_patch_strip':
          case 'merge_patch_grid':
          case 'inspect_patch_grid': {
            const {input, inputs, op, id: _id, ...settings} = n
            const surfaces = [input, ...inputs].map(needSurface)
            const {surface, ...report} = callGeometryRust<{surface?: NurbsSurface; workUsed: number}>(
              op, {surfaces, ...settings},
            )
            constructionReports[key] = {measurement: {operation: op, report}}
            result = {kind: 'surface', data: surface ?? surfaces[0]!}
            break
          }
          case 'split_tensor_surface':
          case 'tile_tensor_surface': {
            const {input, op, id: _id, ...settings} = n
            const {surface, ...report} = callGeometryRust<{surface: NurbsSurface; workUsed: number}>(
              op, {source: needSurface(input), ...settings},
            )
            constructionReports[key] = {measurement: {operation: op, report}}
            result = {kind: 'surface', data: surface}
            break
          }
          case 'lift_uv_edge': {
            const surface = needSurface(n.target_geometry)
            const source = needCurve(n.input)
            const {curve, ...report} = callGeometryRust<{curve:NurbsCurve; workUsed:number}>(
              n.op, {surface, source, piece:n.piece, max_work:n.max_work},
            )
            constructionReports[key] = {measurement:{operation:n.op,report}}
            result = {kind:'curve', data:curve}
            break
          }
          case 'check_seam_gap':
          case 'check_seam_angle':
          case 'isolate_seam_gap':
          case 'isolate_seam_angle':
          case 'verify_uv_edge':
          case 'compare_seam_orientations':
          case 'compare_uv_traces':
          case 'compare_uv_tangents': {
            const surface = needSurface(n.input)
            const args = n.op === 'verify_uv_edge'
              ? {edge:needCurve(n.target_geometry)}
              : {right:needSurface(n.target_geometry),right_trace:needCurve(n.inputs[1])}
            const report=callGeometryRust<object>(n.op,{
              surface,trace:needCurve(n.inputs[0]),max_work:n.max_work,...args,
              ...('tolerance_mm' in n ? {tolerance_mm:n.tolerance_mm} : {}),
              ...('min_cosine_squared' in n ? {min_cosine_squared:n.min_cosine_squared} : {}),
              ...('parameter_width' in n ? {parameter_width:n.parameter_width} : {}),
            })
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'affine_clock_curve':
          case 'piecewise_clock_curve':
          case 'relayout_clock_curve':
          case 'certify_uv_chart':
          case 'classify_uv_seam':
          case 'isolate_uv_line':
          case 'isolate_uv_seam':
          case 'split_uv_seam':
          case 'transfer_uv_grid_piece':
          case 'pullback_uv_clock':
          case 'mobius_bezier_curve':
          case 'mobius_spline_curve':
          case 'fit_mobius_curve': {
            const {op, input, id: _id, ...settings} = n
            const {curve, ...report} = callGeometryRust<{curve: NurbsCurve; workUsed: number}>(
              op, {source: needCurve(input), ...settings},
            )
            constructionReports[key] = {measurement: {operation: op, report}}
            result = {kind: 'curve', data: curve}
            break
          }
          case 'affine_clock_surface':
          case 'piecewise_clock_surface':
          case 'relayout_clock_surface':
          case 'mobius_bezier_surface':
          case 'mobius_spline_surface':
          case 'fit_mobius_surface': {
            const {op, input, id: _id, ...settings} = n
            const {surface, ...report} = callGeometryRust<{surface: NurbsSurface; workUsed: number}>(
              op, {source: needSurface(input), ...settings},
            )
            constructionReports[key] = {measurement: {operation: op, report}}
            result = {kind: 'surface', data: surface}
            break
          }
          case 'relayout_trim_surface':
          case 'relayout_trim_piece':
          case 'affine_trim_surface':
          case 'transpose_trim_surface':
          case 'remap_trim_surface':
          case 'reclock_trim_surface':
          case 'affine_trim_piece':
          case 'transpose_trim_piece':
          case 'remap_trim_piece':
          case 'reclock_trim_piece': {
            const {input, loops, op, id: _id, ...settings} = n
            const args = {source: needSurface(input), loops: loops.map(loop => loop.map(needCurve)), ...settings}
            if (op === 'reclock_trim_piece' || op === 'relayout_trim_piece' || op === 'affine_trim_piece' || op === 'transpose_trim_piece' || op === 'remap_trim_piece') {
              const {curve, ...report} = callGeometryRust<{curve: NurbsCurve; workUsed: number}>(op,args)
              constructionReports[key] = {measurement:{operation:op,report}}
              result = {kind:'curve',data:curve}
            } else {
              const {surface, ...report} = callGeometryRust<{surface: NurbsSurface; trimLoops?: NurbsCurve[][]; workUsed: number}>(op,args)
              if (report.trimLoops) transferredTrimLoops.set(key,report.trimLoops)
              constructionReports[key] = {measurement:{operation:op,report}}
              result = {kind:'surface',data:surface}
            }
            break
          }
          case 'clip_common_path':
          case 'clip_union_path':
          case 'subtract_path_cutters':
          case 'classify_path_membership':
          case 'bound_overlap_lengths':
          case 'recover_overlap_paths': {
            const {id:_id,op,input,inputs,...settings}=n
            const {model,...report}=callGeometryRust<{model:NurbsBrep;workUsed:number}>(op,{source:needBrep(input),others:inputs.map(needBrep),...settings})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'brep',data:model}
            break
          }
          case 'build_expanded_plane_solid':
          case 'build_outward_plane_solid':
          case 'build_section_solid':
          case 'infer_rim_planes':
          case 'close_inferred_rims':
          case 'close_bounded_inferred_rims':
          case 'inspect_inferred_solid':
          case 'build_outward_inferred_solid':
          case 'build_expanded_solid':
          case 'build_inferred_solid':
          case 'close_expanded_planes':
          case 'project_polyhedral_body':
          case 'measure_polyhedral_frame':
          case 'measure_polyhedral_axis_gap':
          case 'section_polyhedral_faces':
          case 'assemble_polyhedral_section_loops':
          case 'measure_polyhedral_section_area':
          case 'measure_polyhedral_section_centroid':
          case 'bound_polyhedral_section_perimeter':
          case 'clip_polyhedral_segment':
          case 'locate_polyhedral_point':
          case 'measure_polyhedral_boundary_distance':
          case 'measure_polyhedral_signed_distance':
          case 'sample_polyhedral_signed_distances':
          case 'measure_swept_ball_gap':
          case 'bound_path_material_length':
          case 'clip_exterior_path':
          case 'clip_interior_path':
          case 'partition_solid_path':
          case 'bound_path_length_breakdown':
          case 'find_path_face_contacts':
          case 'recover_material_path':
          case 'clip_path_to_material':
          case 'find_path_boundary_events':
          case 'measure_swept_path_gap':
          case 'classify_swept_path_material':
          case 'bound_swept_first_contact':
          case 'bound_swept_face_ranges':
          case 'measure_polyhedral_ball_clearance':
          case 'locate_section_point':
          case 'measure_section_contour_distance':
          case 'measure_section_material_distance':
          case 'clip_section_segment':
          case 'clip_section_polyline':
          case 'measure_section_path_length':
          case 'classify_polyhedral_plane':
          case 'intersect_polyhedral_edges':
          case 'measure_polyhedral_volume':
          case 'measure_polyhedral_centroid':
          case 'measure_polyhedral_inertia':
          case 'measure_polyhedral_face_areas':
          case 'bound_polyhedral_area':
          case 'bound_polyhedral_edge_length':
          case 'close_section_planes':
          case 'build_bounded_section_solid':
          case 'close_bounded_planes':
          case 'attach_expanded_rims':
          case 'attach_section_rims':
          case 'discover_expanded_rims':
          case 'discover_section_rims':
          case 'attach_bounded_rims':
          case 'discover_bounded_rims': {
            const {id:_id,op,input,...settings}=n
            const {model,...report}=callGeometryRust<{model:NurbsBrep;workUsed:number}>(op,{source:needBrep(input),...settings,...(n.op==='measure_polyhedral_axis_gap'?{target:needBrep(n.target_geometry)}:{})})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'brep',data:model}
            break
          }
          case 'expanded_section_sheets':
          case 'section_brep_sheets':
          case 'bounded_section_brep_sheets': {
            const {id:_id,op,input,inputs,loops,...settings}=n
            const rings=[[input,...inputs],...loops].map(r=>r.map(needCurve))
            const {model,...report}=callGeometryRust<{model:NurbsBrep;workUsed:number}>(op,{rings,...settings})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'brep',data:model}
            break
          }
          case 'cap_section_wire':
          case 'cap_section_holes':
          case 'arrange_section_caps': {
            const {id:_id,op,input,inputs,...settings}=n
            const rings=[[input,...inputs],...('loops' in n?n.loops:[])].map(r=>r.map(needCurve))
            const {surface,...report}=callGeometryRust<{surface:NurbsSurface;trimLoops:NurbsCurve[][];workUsed:number}>(op,{...settings,rings})
            transferredTrimLoops.set(key,report.trimLoops)
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'exact_wire_area':
          case 'bound_wire_moments':
          case 'exact_wire_mass':
          case 'exact_wire_inertia': {
            const curve=needCurve(n.input)
            const report=callGeometryRust<object>(n.op,{outer:[curve,...n.inputs.map(needCurve)],
              holes:'loops' in n ? n.loops.map(r=>r.map(needCurve)) : [],
              ...(n.op==='bound_wire_moments'?{chart_origin:n.chart_origin,moment_tolerances:n.moment_tolerances,max_cells:n.max_cells}:{max_work:n.max_work})})
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'bound_world_face_mass':
          case 'bound_principal_face_moments':
          case 'bound_principal_face_directions':
          case 'world_face_mass':
          case 'principal_face_moments':
          case 'principal_face_directions':
          case 'coplanar_ray_contacts':
          case 'coplanar_ray_partition':
          case 'cast_trimmed_face_ray':
          case 'planar_face_perimeter':
          case 'planar_boundary_distance':
          case 'planar_material_distance':
          case 'signed_planar_face_area': {
            const surface=needSurface(n.input)
            const report=callGeometryRust<object>(n.op,{source:surface,loops:n.loops.map(r=>r.map(needCurve)),
              ...('tolerance_mm' in n ? {tolerance_mm:n.tolerance_mm,max_cells:n.max_cells} : {}),
              ...('parameter_width' in n ? {parameter_width:n.parameter_width} : {}),
              ...('ray_direction' in n ? {ray_direction:n.ray_direction} : {}),
              ...('query_point' in n ? {query_point:n.query_point} : {}),
              ...('max_winding_cells' in n ? {tolerance_uv:n.tolerance_uv,max_winding_cells:n.max_winding_cells} : {}),
              ...('moment_tolerances' in n ? {chart_origin:n.chart_origin,moment_tolerances:n.moment_tolerances,max_cells:n.max_cells} : {}),
              ...(n.op === 'signed_planar_face_area' ? {tolerance_mm2:n.tolerance_mm2,max_cells:n.max_cells} : {}),max_work:n.max_work})
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'project_planar_wire':
          case 'lift_planar_wire': {
            const grouping=n.op === 'project_planar_wire' ? {
              others:n.inputs.map(needSurface), contour_counts:n.contour_counts,patch_index:n.patch_index,
            } : {}
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve;workUsed:number}>(n.op,{
              source:needSurface(n.input),loops:n.loops.map(r=>r.map(needCurve)),
              loop_index:n.loop_index,piece:n.piece,max_work:n.max_work,...grouping,
            })
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'material_segment_curve':
          case 'store_section_edge':
          case 'optimize_section_edge':
          case 'store_section_edges': {
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve;workUsed:number}>(n.op,{
              source:needSurface(n.input),right:needSurface(n.target_geometry),loops:n.loops.map(r=>r.map(needCurve)),
              left_loop_count:n.left_loop_count,...('endpoint_tolerance' in n ? {endpoint_tolerance:n.endpoint_tolerance} : {parameter_width:n.parameter_width}),tolerance_uv:n.tolerance_uv,
              max_winding_cells:n.max_winding_cells,edge_index:n.edge_index,tolerance_mm:n.tolerance_mm,max_work:n.max_work})
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'classify_section_gaps':
          case 'assemble_section_material':
          case 'build_material_segments':
          case 'measure_section_material':
          case 'partition_face_events':
          case 'enclose_face_events':
          case 'match_quadratic_contacts':
          case 'shared_boundary_spans':
          case 'shared_boundary_contacts':
          case 'refine_boundary_section':
          case 'resolve_boundary_section':
          case 'bound_boundary_section': {
            const surface=needSurface(n.input)
            const report=callGeometryRust<object>(n.op,{source:surface,right:needSurface(n.target_geometry),
              loops:n.loops.map(r=>r.map(needCurve)),left_loop_count:n.left_loop_count,max_work:n.max_work,
              ...('max_winding_cells' in n ? {tolerance_uv:n.tolerance_uv,max_winding_cells:n.max_winding_cells} : {}),
              ...('endpoint_tolerance' in n ? {endpoint_tolerance:n.endpoint_tolerance} : {}),
              ...('max_passes' in n ? {max_passes:n.max_passes} : {}),
              ...('xyz_tolerance' in n ? {xyz_tolerance:n.xyz_tolerance} : {}),
              ...('parameter_width' in n ? {parameter_width:n.parameter_width} : {})})
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'classify_face_section':
          case 'clip_plane_section':
          case 'build_plane_section_segments': {
            const surface=needSurface(n.input)
            const report=callGeometryRust<object>(n.op,{source:surface,right:needSurface(n.target_geometry),
              loops:n.loops.map(r=>r.map(needCurve)),left_loop_count:n.left_loop_count,parameter_width:n.parameter_width,
              tolerance_uv:n.tolerance_uv,max_winding_cells:n.max_winding_cells,max_work:n.max_work})
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'intersect_support_planes': {
            const surface=needSurface(n.input)
            const report=callGeometryRust<object>(n.op,{source:surface,right:needSurface(n.target_geometry),max_work:n.max_work})
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'active_surface_hull':
          case 'project_plane_chart':
          case 'lift_plane_chart':
          case 'shared_plane_chart': {
            const args = n.op === 'shared_plane_chart' ? {others:n.inputs.map(needSurface),patch_index:n.patch_index}
              : n.op === 'lift_plane_chart' ? {edited:needSurface(n.target_geometry)} : {}
            const {surface,...report} = callGeometryRust<{surface:NurbsSurface;workUsed:number}>(n.op,{
              source:needSurface(n.input),max_work:n.max_work,...args,
            })
            constructionReports[key] = {measurement:{operation:n.op,report}}
            result = {kind:'surface',data:surface}
            break
          }
          case 'compress_tensor_surface':
          case 'compress_tensor_surface_batch': {
            const settings = n.op === 'compress_tensor_surface'
              ? {source: needSurface(n.input)}
              : {surfaces: [n.input, ...n.inputs].map(needSurface), patch_index: n.patch_index}
            const {surface, ...report} = callGeometryRust<{surface: NurbsSurface; workUsed: number}>(
              n.op, {...settings, max_work: n.max_work},
            )
            constructionReports[key] = {measurement: {operation: n.op, report}}
            result = {kind: 'surface', data: surface}
            break
          }
          case 'surface_gap_pair_distance': {
            const {op, id: _id, input, target_geometry, ...settings} = n
            const surface = needSurface(input)
            const target = needSurface(target_geometry)
            const report = callGeometryRust<object>(op, {surface, target, ...settings})
            constructionReports[key] = {measurement: {operation: op, report}}
            result = {kind: 'surface', data: surface}
            break
          }
          case 'surface_gap_world_polyline': {
            const {op,id:_id,input,return_field,...settings}=n
            const surface=needSurface(input)
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve|null;worldErrorUpperMm:number[]|null}>(op,{surface,...settings,return_field})
            constructionReports[key]={measurement:{operation:op,report}}
            if(return_field && !curve) throw new Error('The selected cell has no certified world polyline')
            result=return_field && curve ? {kind:'curve',data:curve} : {kind:'surface',data:surface}
            break
          }
          case 'surface_umbilic_cells':
          case 'surface_umbilic_regions':
          case 'surface_gap_comparison':
          case 'surface_gap_ranges':
          case 'surface_gap_regions':
          case 'surface_gap_edge_roots':
          case 'surface_gap_contour_graph':
          case 'surface_gap_contour_samples':
          case 'surface_gap_world_length':
          case 'surface_gap_plane_section':
          case 'surface_gap_world_distance':
          case 'surface_gap_world_bounds':
          case 'surface_gap_uv_polyline':
          case 'surface_gap_adaptive_polyline':
          case 'surface_gap_second_derivative':
          case 'surface_gap_second_order_polyline':
          case 'surface_gap_contour_slope':
          case 'surface_cell_interpolation_error':
          case 'surface_triangle_interpolation_error':
          case 'surface_grid_interpolation_error': {
            const {op,input,id:_id,...settings}=n
            const surface=needSurface(input)
            const report=callGeometryRust<object>(op,{surface,...settings})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'surface_quadric_support': {
            const surface = needSurface(n.input)
            const report = callGeometryRust<object>(n.op,{surface,quadric:n.quadric,max_work:n.max_work,max_cells:n.max_cells})
            constructionReports[key] = {measurement:{operation:n.op,report}}
            result = {kind:'surface',data:surface}
            break
          }
          case 'surface_self_contacts':
          case 'surface_span_regularity': {
            const surface = needSurface(n.input)
            const report = n.op === 'surface_self_contacts'
              ? callGeometryRust<object>('nurbs_surface_self_intersection_audit',{surface,maxSpans:n.max_spans})
              : callGeometryRust<object>(n.op,{surface,max_cells:n.max_cells})
            constructionReports[key] = {measurement:{operation:n.op,report}}
            result = {kind:'surface',data:surface}
            break
          }
          case 'surface_ray_hits': {
            const {id:_id,op,input,...parameters} = n
            const surface = needSurface(input)
            const report = callGeometryRust<object>(op, {surface,...parameters})
            constructionReports[key] = {measurement:{operation:op,report}}
            result = {kind:'surface',data:surface}
            break
          }
          case 'curve_plane_roots':
          case 'curve_pair_crossings':
          case 'curve_plane_chart_crossings': {
            const curve = needCurve(n.input)
            const parameters = n.op === 'curve_plane_roots'
              ? {plane_normal:n.plane_normal, plane_offset_mm:n.plane_offset_mm, parameter_tolerance:n.parameter_tolerance}
              : {target:n.op==='curve_pair_crossings' ? needCurve(n.target_geometry) : needSurface(n.target_geometry), tolerance_mm:n.tolerance_mm}
            const report = callGeometryRust<object>(n.op,{curve,...parameters,max_cells:n.max_cells})
            constructionReports[key] = {measurement:{operation:n.op,report}}
            result = {kind:'curve',data:curve}
            break
          }
          case 'minimum_sphere_gap':
          case 'minimum_box_gap':
          case 'minimum_oriented_box_gap':
          case 'minimum_capsule_gap':
          case 'sphere_path_clearance':
          case 'capsule_path_clearance':
          case 'box_path_clearance':
          case 'oriented_box_path_clearance':
          case 'first_sphere_contact':
          case 'minimum_sphere_partition_gap':
          case 'point_workspace':
          case 'point_obstacles':
          case 'ball_obstacles':
          case 'ball_workspace':
          case 'capsule_workspace':
          case 'hull_workspace':
          case 'hull_obstacles':
          case 'partition_axis_position':
          case 'partition_axis_positions':
          case 'partition_trajectory_bounds':
          case 'check_timed_chain':
          case 'inspect_knot_turns':
          case 'inspect_join_turn':
          case 'inspect_chain_turns':
          case 'inspect_knot_jumps':
          case 'inspect_timed_knot_jumps':
          case 'inspect_motion_join':
          case 'inspect_motion_chain': {
            const curve=needCurve(n.input)
            let report:object
            if('inputs' in n){
              const {id:_id,input:_input,inputs,op,...parameters}=n
              report=callGeometryRust<object>(op,{curves:[curve,...inputs.map(id=>needCurve(id))],...parameters})
            }else{
              const {id:_id,input:_input,op,...parameters}=n
              report=callGeometryRust<object>(op,{curve,...parameters})
            }
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'curve_curvature_torsion': {
            const curve = needCurve(n.input)
            const report = callGeometryRust<object>(n.op, {curve, parameter:n.parameter, query_side:n.query_side ?? 'automatic'})
            constructionReports[key] = { measurement:{operation:n.op,report} }
            result = {kind:'curve',data:curve}
            break
          }
          case 'surface_principal_curvatures': {
            const surface = needSurface(n.input)
            const report = callGeometryRust<object>(n.op, {surface, uv:n.uv, u_side:n.u_side ?? 'automatic', v_side:n.v_side ?? 'automatic'})
            constructionReports[key] = { measurement:{operation:n.op,report} }
            result = {kind:'surface',data:surface}
            break
          }
          case 'store_periodic_curve':
          case 'rotate_periodic_knot':
          case 'insert_periodic_seam':
          case 'open_periodic_curve':
          case 'trim_periodic_curve':
          case 'trim_periodic_turns':
          case 'partition_periodic_curve': {
            const {id:_id,op,input,...parameters}=n
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve;domain:number[];workUsed:number}>(op,{source:needCurve(input),...parameters})
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'join_affine_c1':
          case 'join_affine_c2':
          case 'join_projective_c2':
          case 'join_stationary_c2':
          case 'join_mixed_degree':
          case 'compact_join_seam':
          case 'compare_parametric_seam':
          case 'compare_geometric_seam': {
            const left=needCurve(n.input)
            const {curve,...report}=callGeometryRust<{curve?:NurbsCurve;workUsed:number}>(n.op,{left,right:needCurve(n.target_geometry),max_work:n.max_work})
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'curve',data:curve??left}
            break
          }
          case 'join_projective_chain':
          case 'compact_projective_chain':
          case 'close_projective_chain':
          case 'merge_weighted_curves':
          case 'merge_weighted_chain': {
            const ids = n.op === 'merge_weighted_curves' ? [n.input,n.target_geometry] : [n.input,...n.inputs]
            const {curve,...report} = callGeometryRust<{curve:NurbsCurve;domain:number[];workUsed:number}>(n.op,{curves:ids.map(id=>needCurve(id)),max_work:n.max_work})
            constructionReports[key] = {measurement:{operation:n.op,report}}
            result = {kind:'curve',data:curve}
            break
          }
          case 'merge_curve_exact': {
            const {curve,...report}=callGeometryRust<{curve:NurbsCurve;seam:number}>(n.op,{left:needCurve(n.input),right:needCurve(n.target_geometry)})
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'curve',data:curve}
            break
          }
          case 'merge_surface_exact': {
            const {surface,...report}=callGeometryRust<{surface:NurbsSurface;seam:number}>(n.op,{left:needSurface(n.input),right:needSurface(n.target_geometry),direction:n.direction})
            constructionReports[key]={measurement:{operation:n.op,report}}
            result={kind:'surface',data:surface}
            break
          }
          case 'curve_nearest_point':
          case 'surface_nearest_point':
          case 'curve_pair_distance':
          case 'curve_surface_distance':
          case 'surface_pair_distance': {
            const surfaceInput = n.op === 'surface_nearest_point' || n.op === 'surface_pair_distance'
            const source = surfaceInput
              ? { kind:'surface' as const, data:needSurface(n.input) }
              : { kind:'curve' as const, data:needCurve(n.input) }
            const options = { toleranceMm:n.tolerance_mm, maxCells:n.max_cells }
            const report = 'probe_point' in n
              ? callGeometryRust<object>(n.op, { geometry:source.data, point:n.probe_point, ...options })
              : callGeometryRust<object>(n.op === 'curve_pair_distance' ? 'curve_distance' :
                  n.op === 'surface_pair_distance' ? 'surface_distance' : n.op, {
                    a:source.data, b:n.op === 'curve_pair_distance' ? needCurve(n.target_geometry) : needSurface(n.target_geometry), ...options,
                  })
            constructionReports[key] = { measurement:{operation:n.op,report} }
            result = source
            break
          }
          case 'surface_area_bounds': {
            const surface = needSurface(n.input)
            const report = callGeometryRust<object>(n.op, { surface, tolerance_mm2:n.tolerance_mm2, max_cells:n.max_cells })
            constructionReports[key] = { measurement: { operation:n.op, report } }
            result = { kind:'surface', data:surface }
            break
          }
          case 'curve_length_bounds':
          case 'curve_station_at_length':
          case 'curve_divide_by_length': {
            const { id: _id, op, input, ...options } = n
            const curve = needCurve(input)
            const report = callGeometryRust<object>(op, {curve, ...options})
            constructionReports[key] = { measurement: { operation:op, report } }
            result = { kind:'curve', data:curve }
            break
          }
          case 'curve_measure_directional_travel':
          case 'curve_measure_directional_travel_batch': {
            const curve = needCurve(n.input)
            const batch = n.op === 'curve_measure_directional_travel_batch'
            const report = callGeometryRust<object>(
              batch ? 'nurbs_measure_curve_directional_travel_batch' : 'nurbs_measure_curve_directional_travel',
              {
                curve, direction: n.axis_vector, maxCells: n.max_cells, toleranceMm: n.tolerance_mm,
                ...(batch ? { parameterIntervals: n.intervals } : { parameterInterval: n.interval }),
                ...(n.reverse_limit === undefined ? {} : { reverseLimitMm: n.reverse_limit }),
              },
            )
            constructionReports[key] = { measurement: { operation: n.op, report } }
            result = { kind: 'curve', data: curve }
            break
          }
          case 'curve_reparameterize':
            result = { kind: 'curve', data: reparameterizeNurbsCurveExact(needCurve(n.input), n.domain).curve }
            break
          case 'curve_split':
            result = { kind: 'curve', data: splitNurbsCurve(needCurve(n.input), n.parameter)[n.part] }
            break
          case 'curve_bezier_span': {
            const span = decomposeNurbsCurve(needCurve(n.input))[n.index]
            if (!span) throw new Error('Bezier span index outside curve')
            result = { kind: 'curve', data: span.curve }
            break
          }
          case 'curve_edit': {
            let c = needCurve(n.input)
            for (const knot of n.insert_knots ?? []) c = insertNurbsKnot(c, knot.value, knot.count)
            if (n.degree !== undefined) c = elevateNurbsCurve(c, n.degree)
            if (n.trim) c = trimNurbsCurve(c, ...n.trim)
            if (n.reverse) c = reverseNurbsCurve(c)
            result = { kind: 'curve', data: c }
            break
          }
          case 'surface_edit': {
            let s = needSurface(n.input)
            for (const k of n.insert_knots_u ?? []) s = insertNurbsSurfaceKnot(s, 'u', k.value, k.count)
            for (const k of n.insert_knots_v ?? []) s = insertNurbsSurfaceKnot(s, 'v', k.value, k.count)
            if (n.degree_u !== undefined) s = elevateNurbsSurface(s, 'u', n.degree_u)
            if (n.degree_v !== undefined) s = elevateNurbsSurface(s, 'v', n.degree_v)
            if (n.trim) s = trimNurbsSurface(s, ...n.trim)
            if (n.reverse_u) s = reverseNurbsSurface(s, 'u')
            if (n.reverse_v) s = reverseNurbsSurface(s, 'v')
            result = { kind: 'surface', data: s }
            break
          }
          case 'iso_curve':
            result = { kind: 'curve', data: isoNurbsCurve(needSurface(n.input), n.direction, n.parameter) }
            break
          case 'natural_spline_curve':
            result = { kind: 'curve', data: naturalSplineNurbsCurve(n.points, n.parameters) }
            break
          case 'hermite_curve':
            result = { kind: 'curve', data: hermiteNurbsCurve(n.points, n.tangents, n.parameters) }
            break
          case 'clothoid_curve': {
            const approximation = approximateClothoidNurbsCurve(
              n.center,
              n.length,
              n.start_curvature,
              n.end_curvature,
              n.phase_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'spherical_spiral_curve': {
            const approximation = approximateSphericalSpiralNurbsCurve(
              n.center,
              n.radius,
              n.longitude_turns,
              n.latitude_turns,
              n.longitude_phase_degrees,
              n.latitude_phase_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'toroidal_spiral_curve': {
            const approximation = approximateToroidalSpiralNurbsCurve(
              n.center,
              n.major_radius,
              n.minor_radius,
              n.major_turns,
              n.minor_turns,
              n.major_phase_degrees,
              n.minor_phase_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'torus_knot_curve': {
            const approximation = approximateTorusKnotNurbsCurve(
              n.center,
              n.major_radius,
              n.minor_radius,
              n.p,
              n.q,
              n.major_phase_degrees,
              n.minor_phase_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'helicoid_patches': {
            const approximation = approximateHelicoidNurbsPatches(
              n.center,
              n.inner_radius,
              n.outer_radius,
              n.height,
              n.turns,
              n.phase_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = {
              kind: 'patches',
              data: { patches: approximation.patches, faceIds: approximation.patches.map((_, i) => i) },
            }
            break
          }
          case 'circle_rectangle_transition': {
            const patches = circleRectangleTransitionNurbsPatches(
              { center: n.circle_center, normal: n.circle_normal, seam: n.circle_seam, radius: n.circle_radius },
              { center: n.rectangle_center, axisU: n.rectangle_axis_u, axisV: n.rectangle_axis_v },
            )
            result = { kind: 'patches', data: { patches, faceIds: patches.map((_, i) => i) } }
            break
          }
          case 'ellipse_transition_surface':
            result = {
              kind: 'surface',
              data: ellipseTransitionNurbsSurface(
                { center: n.start_center, axisU: n.start_axis_u, axisV: n.start_axis_v },
                { center: n.end_center, axisU: n.end_axis_u, axisV: n.end_axis_v },
              ),
            }
            break
          case 'circle_transition_surface':
            result = {
              kind: 'surface',
              data: circleTransitionNurbsSurface(
                { center: n.start_center, normal: n.start_normal, seam: n.start_seam, radius: n.start_radius },
                { center: n.end_center, normal: n.end_normal, seam: n.end_seam, radius: n.end_radius },
              ),
            }
            break
          case 'helicoid_surface': {
            const approximation = approximateHelicoidNurbsSurface(
              n.center,
              n.inner_radius,
              n.outer_radius,
              n.height,
              n.turns,
              n.phase_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'surface', data: approximation.surface }
            break
          }
          case 'catenoid_patches': {
            const approximation = approximateCatenoidNurbsPatches(
              n.center,
              n.scale,
              n.start_z,
              n.end_z,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = {
              kind: 'patches',
              data: { patches: approximation.patches, faceIds: approximation.patches.map((_, i) => i) },
            }
            break
          }
          case 'superellipsoid_patches': {
            const { patches, report } = superellipsoidPatches(n.center, n.radii, n.exponents, n.tolerance, n.max_cells)
            constructionReports[key] = report
            result = { kind: 'patches', data: { patches, faceIds: patches.map((_, i) => i) } }
            break
          }
          case 'corrugated_bellows': {
            const patches = corrugatedBellows(n.center, n.mean_radius, n.amplitude, n.height, n.radial_thickness, n.periods)
            result = { kind: 'patches', data: { patches, faceIds: patches.map((_, i) => i) } }
            break
          }
          case 'optical_dome':
            result = { kind: 'surface', data: opticalDome(n.center, n.base_axes, n.height) }
            break
          case 'optical_lens': {
            const patches = opticalLens(n.center, n.base_axes, n.sags, n.vertex_thickness)
            result = { kind: 'patches', data: { patches, faceIds: patches.map((_, i) => i) } }
            break
          }
          case 'cubic_wave_surface':
            result = { kind: 'surface', data: cubicWaveNurbsSurface(n.origin, n.width, n.length, n.amplitude, n.periods) }
            break
          case 'catenoid_surface': {
            const approximation = approximateCatenoidNurbsSurface(
              n.center,
              n.scale,
              n.start_z,
              n.end_z,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'surface', data: approximation.surface }
            break
          }
          case 'catenary_curve': {
            const approximation = approximateCatenaryNurbsCurve(n.center, n.scale, n.start_x, n.end_x, n.max_deviation)
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'archimedean_spiral_curve': {
            const approximation = approximateArchimedeanSpiralNurbsCurve(
              n.center,
              n.start_radius,
              n.end_radius,
              n.start_degrees,
              n.end_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'epicycloid_curve': {
            const approximation = approximateEpicycloidNurbsCurve(
              n.center,
              n.fixed_radius,
              n.rolling_radius,
              (n.start_degrees * Math.PI) / 180,
              (n.end_degrees * Math.PI) / 180,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'hypocycloid_curve': {
            const approximation = approximateHypocycloidNurbsCurve(
              n.center,
              n.fixed_radius,
              n.rolling_radius,
              (n.start_degrees * Math.PI) / 180,
              (n.end_degrees * Math.PI) / 180,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'trochoid_curve': {
            const approximation = approximateTrochoidNurbsCurve(
              n.center,
              n.rolling_radius,
              n.tracing_radius,
              (n.start_degrees * Math.PI) / 180,
              (n.end_degrees * Math.PI) / 180,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'cycloid_curve': {
            const approximation = approximateCycloidNurbsCurve(
              n.center,
              n.radius,
              (n.start_degrees * Math.PI) / 180,
              (n.end_degrees * Math.PI) / 180,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'lissajous_curve': {
            const approximation = approximateLissajousNurbsCurve(
              n.center,
              n.amplitudes,
              n.frequencies,
              n.phases_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'logarithmic_spiral_curve': {
            const approximation = approximateLogarithmicSpiralNurbsCurve(
              n.center,
              n.radius,
              n.growth,
              (n.start_degrees * Math.PI) / 180,
              (n.end_degrees * Math.PI) / 180,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'involute_curve': {
            const approximation = approximateInvoluteNurbsCurve(
              n.center,
              n.radius,
              (n.start_degrees * Math.PI) / 180,
              (n.end_degrees * Math.PI) / 180,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'helix_curve': {
            const approximation = approximateHelixNurbsCurve(
              n.center,
              n.radius,
              n.height,
              n.turns,
              n.phase_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'elliptic_helix_curve': {
            const approximation = approximateEllipticHelixNurbsCurve(
              n.center,
              n.radius_x,
              n.radius_y,
              n.height,
              n.turns,
              n.phase_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'variable_pitch_helix_curve': {
            const approximation = approximateVariablePitchHelixNurbsCurve(
              n.center,
              n.radius,
              n.height,
              n.turns,
              n.start_pitch,
              n.end_pitch,
              n.phase_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'conical_helix_curve': {
            const approximation = approximateConicalHelixNurbsCurve(
              n.center,
              n.start_radius,
              n.end_radius,
              n.height,
              n.turns,
              n.phase_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = { kind: 'curve', data: approximation.curve }
            break
          }
          case 'formula_surface':
            result = { kind: 'surface', data: formulaNurbsSurface(n.domain, n.expressions) }
            break
          case 'formula_curve':
            result = { kind: 'curve', data: formulaNurbsCurve(n.domain, n.expressions) }
            break
          case 'coons_patch':
            result = { kind: 'surface', data: coonsNurbsPatch(n.inputs.map(needCurve)) }
            break
          case 'ruled_surface':
            result = { kind: 'surface', data: loftNurbsCurves(n.inputs.map(needCurve)) }
            break
          case 'polygon_profile':
            result = { kind: 'profile', data: { outer: n.outer, holes: n.holes } }
            break
          case 'extrude': {
            const v = get(n.input)
            const vector = [0, 0, n.height]
            if (v.kind === 'curve') result = { kind: 'surface', data: extrudeNurbsCurve(v.data, vector) }
            else if (v.kind === 'profile') result = { kind: 'mesh', data: extrudePolygonProfile(v.data, vector) }
            else throw new Error('Extrude expects a NURBS curve or polygon profile')
            break
          }
          case 'revolve': {
            const v = get(n.input)
            if (v.kind === 'curve')
              result = { kind: 'surface', data: revolveNurbsCurve(v.data, [0, 0, 0], [0, 0, 1], n.angle) }
            else if (v.kind === 'profile') {
              if (v.data.holes?.length) throw new Error('Revolve with profile holes is not supported')
              result = { kind: 'mesh', data: revolvePolygonProfile(v.data.outer, n.angle, n.segments) }
            } else throw new Error('Revolve expects a NURBS curve or polygon profile')
            break
          }
          case 'polygon_extrude': {
            const v = get(n.input)
            if (v.kind !== 'profile') throw new Error('Expected polygon profile')
            result = { kind: 'mesh', data: extrudePolygonProfile(v.data, n.vector) }
            break
          }
          case 'polygon_sweep': {
            const v = get(n.input)
            if (v.kind !== 'profile') throw new Error('Expected polygon profile')
            if (v.data.holes?.length) throw new Error('Sweep with profile holes is not supported')
            result = { kind: 'mesh', data: sweepPolygonProfile(v.data.outer, n.path, n.up) }
            break
          }
          case 'polygon_loft':
            result = { kind: 'mesh', data: loftPolygonSections(n.sections) }
            break
          case 'surface_sweep':
            result = { kind: 'surface', data: sweepNurbsCurve(needCurve(n.inputs[0]), needCurve(n.inputs[1])) }
            break
          case 'twist_sweep':
            result = {
              kind: 'surface',
              data: twistSweepNurbsCurve(
                needCurve(n.inputs[0]),
                needCurve(n.inputs[1]),
                n.origin,
                n.axis,
                n.start_degrees,
                n.sweep_degrees,
              ),
            }
            break
          case 'two_guide_sweep':
            result = {
              kind: 'surface',
              data: twoGuideSweepNurbsCurve(
                needCurve(n.inputs[0]),
                needCurve(n.inputs[1]),
                needCurve(n.inputs[2]),
                n.width,
                n.axis_y,
                n.axis_z,
              ),
            }
            break
          case 'scaled_sweep':
            result = {
              kind: 'surface',
              data: scaledSweepNurbsCurve(needCurve(n.inputs[0]), needCurve(n.inputs[1]), n.scale, n.origin),
            }
            break
          case 'progressive_sweep': {
            const builtSweep = buildProgressiveSweepNode(n, needCurve, resolveSweep)
            constructionReports[key] = {
              ...builtSweep.report,
              levels: builtSweep.levels,
              profilePatchRanges: builtSweep.profilePatchRanges,
            }
            result = {
              kind: 'patches',
              data: { patches: builtSweep.patches!, faceIds: builtSweep.patches!.map((_, i) => i) },
            }
            break
          }
          case 'profile_sweep': {
            const sweep = checkedProfileSweepNurbsSurface(
              needCurve(n.inputs[0]),
              needCurve(n.inputs[1]),
              n.scale,
              n.normal,
              n.sections,
              n.max_deviation,
            )
            if (!sweep.report.accepted || !sweep.surface)
              throw new Error(
                `Profile sweep continuous deviation is unproved within ${sweep.report.budget}mm (${sweep.report.continuousCertificate?.reason ?? 'unresolved'}); refine sections or split the path`,
              )
            constructionReports[key] = sweep.report
            result = { kind: 'surface', data: sweep.surface }
            break
          }
          case 'framed_sweep': {
            const sweep = framedSweepNurbsCurve(
              needCurve(n.inputs[0]),
              needCurve(n.inputs[1]),
              n.normal,
              n.sections,
              n.max_deviation,
            )
            if (!sweep.report.accepted || !sweep.surface)
              throw new Error(
                `Framed sweep sampled refinement ${sweep.report.sampledControlDeviation}mm exceeds ${sweep.report.budget}mm; refine sections or split the path`,
              )
            constructionReports[key] = sweep.report
            result = { kind: 'surface', data: sweep.surface }
            break
          }
          case 'control_tangent_loft_surface':
            result = {
              kind: 'surface',
              data: controlTangentLoftNurbsCurves(
                n.inputs.map(needCurve),
                n.parameters,
                n.start_tangents,
                n.end_tangents,
              ),
            }
            break
          case 'auto_guided_loft_surface': {
            if (n.construction === 'cartesian') {
              const curves = n.inputs.map(needCurve),
                guides = n.guides.map(needCurve)
              const built =
                n.start_tangents && n.end_tangents
                  ? createAutoControlTangentCartesianGuidedLoft(
                      curves,
                      n.parameters,
                      guides,
                      n.start_tangents,
                      n.end_tangents,
                      n.budget,
                      n.parameter_tolerance,
                      n.max_cells,
                      n.max_map_evaluations,
                      n.section_mappings,
                    )
                  : createAutoCartesianGuidedLoft(
                      curves,
                      n.parameters,
                      guides,
                      n.budget,
                      n.parameter_tolerance,
                      n.max_cells,
                      n.max_map_evaluations,
                      n.section_mappings,
                    )
              constructionReports[key] = {
                ...built.certificate,
                guideParameters: built.guide_parameters,
                guideOrder: built.guide_order,
                reversed: built.reversed,
                sectionErrorUpper: built.section_error_upper,
                guideErrorUpper: built.guide_error_upper,
                sectionMappingCertificates: built.section_mapping_certificates,
                originalSectionCertificates: built.original_section_certificates,
              }
              result = { kind: 'surface', data: built.surface }
              break
            }
            const built = autoGuidedLoftNurbsCurves(
              n.inputs.map(needCurve),
              n.parameters,
              n.guides.map(needCurve),
              n.budget,
              n.parameter_tolerance,
              n.section_mappings,
            )
            constructionReports[key] = {
              method: 'automatic-loft-alignment',
              guideParameters: built.guide_parameters,
              guideOrder: built.guide_order,
              reversed: built.reversed,
              sectionErrorUpper: built.section_error_upper,
              guideErrorUpper: built.guide_error_upper,
              sectionMappingCertificates: built.section_mapping_certificates,
              originalSectionCertificates: built.original_section_certificates,
            }
            result = { kind: 'surface', data: built.surface }
            break
          }
          case 'loft_match_surface': {
            const start = n.start_reference
              ? {
                  reference: needSurface(n.start_reference),
                  boundary: n.start_boundary,
                  order: n.order as 1 | 2,
                  scale: n.start_scale,
                  reverse: n.start_reverse,
                }
              : undefined
            const end = n.end_reference
              ? {
                  reference: needSurface(n.end_reference),
                  boundary: n.end_boundary,
                  order: n.order as 1 | 2,
                  scale: n.end_scale,
                  reverse: n.end_reverse,
                }
              : undefined
            const built = matchNurbsLoftEnds(
              needSurface(n.input),
              n.sections.map(needCurve),
              n.parameters,
              n.budget,
              start,
              end,
              n.guides.map(needCurve),
              n.guide_parameters,
            )
            constructionReports[key] = {
              method: 'loft-end-jets',
              seams: built.seams,
              sectionErrorUpper: built.section_error_upper,
              guideErrorUpper: built.guide_error_upper,
            }
            result = { kind: 'surface', data: built.surface }
            break
          }
          case 'brep_progressive_miter_sweep': {
            if (
              n.cap_correction_tolerance === undefined &&
              (n.cap_correction_quantum !== undefined ||
                n.cap_correction_max_work !== undefined ||
                n.cap_correction_authored_frame === true)
            )
              throw new Error('Miter cap correction requires a displacement tolerance')
            if (
              n.circle_correction_tolerance === undefined &&
              (n.circle_correction_quantum !== undefined || n.circle_correction_max_work !== undefined)
            )
              throw new Error('Miter circle correction requires a displacement tolerance')
            const args: MiterArguments = [
              n.loops.map(wire => wire.map(needCurve)),
              n.points,
              n.scale,
              n.twist,
              {
                ...(n.circle_correction_tolerance === undefined
                  ? {}
                  : {
                      circleCorrection: {
                        tolerance: n.circle_correction_tolerance,
                        quantum: n.circle_correction_quantum ?? 2 ** -40,
                        maxWork: n.circle_correction_max_work ?? 1000000,
                      },
                    }),
                retainedWallMaxInjectivityCells: n.retained_wall_max_injectivity_cells,
                ...(n.cap_correction_tolerance === undefined
                  ? {}
                  : {
                      capCorrection: {
                        tolerance: n.cap_correction_tolerance,
                        quantum: n.cap_correction_quantum ?? 2 ** -40,
                        maxWork: n.cap_correction_max_work ?? 1000000,
                        authoredFrame: n.cap_correction_authored_frame,
                      },
                    }),
                ...(n.orientation_guide ? { orientationGuide: needCurve(n.orientation_guide) } : {}),
                frameAxis: n.frame_axis,
                frameNormal: n.frame_normal,
                axisScale: n.axis_scale,
                centerLaw: n.center_law,
                normal: n.normal,
                closed: n.closed,
                miterLimit: n.miter_limit,
                initialSteps: n.initial_steps,
                maxSteps: n.max_steps,
                maxDeviation: n.max_deviation,
              },
            ]
            const body = resolveMiter ? resolveMiter(key, args) : createProgressiveMiterBrepProfileBody(...args)
            miterSources.set(key, body)
            constructionReports[key] = {
              ...body.approximation.report,
              sweepMiterReplay: body.sweepMiterReplay,
              continuousBound: body.boundaryCertificate.continuousBound,
              profileSmoothness: body.profileSmoothness,
              boundaryCertificate: body.boundaryCertificate,
              wallRegularityCertified: body.retainedWallCharts.allChartsCertified,
              levels: body.approximation.levels,
              sectionCorrection: body.sectionCorrection,
              retainedWallErrorUpper: body.retainedWallErrorUpper,
              capProjection: body.capProjection,
              capParallelism: body.capParallelism,
              retainedCapDecomposition: body.retainedCapDecomposition,
              retainedDecomposition: body.retainedDecomposition,
              idealCapDomains: body.idealCapDomains,
              filledCapErrorUpper: body.filledCapErrorUpper,
              boundaryErrorUpper: body.boundaryErrorUpper,
              boundaryErrorWithinBudget: body.boundaryErrorWithinBudget,
              wallAudit: body.wallAudit,
              retainedCorrespondence: body.retainedCorrespondence,
              retainedCaps: body.retainedCaps,
              retainedWallCharts: body.retainedWallCharts,
              capDomains: body.capDomains,
              capContacts: body.capContacts,
              capPairs: body.capPairs,
              embedding: body.embedding,
              volume: body.volume,
              globalEmbeddingCertified: false,
            }
            if (
              n.circle_correction_tolerance !== undefined ||
              n.cap_correction_tolerance !== undefined ||
              body.sectionCorrection?.reason === 'automatic-bounded-cap-planarity' ||
              body.sectionCorrection?.reason === 'bounded-periodic-profile-interpolation' ||
              body.sectionCorrection?.reason === 'bounded-rational-periodic-profile-interpolation'
            )
              exactMiterPlacements.set(key, {
                body,
                quantum: n.circle_correction_quantum ?? n.cap_correction_quantum ?? 2 ** -40,
                maxWork: n.circle_correction_max_work ?? n.cap_correction_max_work ?? 1000000,
              })
            result = { kind: 'brep', data: body.model }
            break
          }
          case 'brep_smooth_sweep_stations': {
            needBrep(n.input)
            const source = progressiveBodySources.get(n.input)
            if (!source) throw new Error('Missing owned progressive station source')
            const args: StationArguments = [
              source,
              {
                quantum: n.quantum,
                wallTolerance: n.wall_tolerance,
                maxWork: n.max_work,
                smoothnessMaxWork: n.smoothness_max_work,
              },
            ]
            const body = resolveStations ? resolveStations(key, args) : reconstructCertifiedProgressiveStations(...args)
            constructionReports[key] = body
            result = { kind: 'brep', data: body.model }
            break
          }
          case 'brep_smooth_miter_stations': {
            needBrep(n.input)
            const source = miterSources.get(n.input)
            if (!source)
              throw new Error('Station reconstruction requires a progressive miter source with owned retained sections')
            const body = reconstructCertifiedMiterStations(source, {
              quantum: n.quantum,
              maxWork: n.max_work,
              wallTolerance: n.wall_tolerance,
              maxDeviation: n.max_deviation,
            })
            constructionReports[key] = body
            exactMiterPlacements.set(key, { body, quantum: n.quantum, maxWork: n.max_work })
            result = { kind: 'brep', data: body.model }
            break
          }
          case 'brep_miter_sweep': {
            const { body, placement } = buildDirectMiterNode(n, needCurve)
            if (placement) {
              miterSources.set(key, placement.body)
              exactMiterPlacements.set(key, placement)
            }
            constructionReports[key] = body.report
            result = { kind: 'brep', data: body.model }
            break
          }
          case 'brep_rational_loft':
            result = { kind: 'brep', data: createRationalBrepSectionLoft(n.sections.map(section =>
              section.map(loop => loop.map(needCurve)))) }
            break
          case 'brep_natural_loft':
            result = {
              kind: 'brep',
              data: createNaturalBrepSectionLoft(
                n.sections.map(s => s.map(r => r.map(needCurve))),
                n.parameters,
              ),
            }
            break
          case 'brep_capped_loft': {
            const start = n.start.map(r => r.map(needCurve)),
              end = n.end.map(r => r.map(needCurve)),
              sides = n.sides.map(r => r.map(needSurface))
            if (n.cap_surfaces && n.cap_trims) {
              result = {
                kind: 'brep',
                data: createCappedBrepLoftWithCaps(
                  start,
                  end,
                  sides,
                  [
                    { surface: needSurface(n.cap_surfaces[0]), trims: n.cap_trims[0].map(r => r.map(needCurve)) },
                    { surface: needSurface(n.cap_surfaces[1]), trims: n.cap_trims[1].map(r => r.map(needCurve)) },
                  ],
                  n.embedding_limits,
                  n.tolerance_uv,
                ),
              }
              constructionReports[key] = {
                method: 'authored-cap-loft',
                boundaryEmbedding: 'accepted',
                globalEmbedding: 'accepted',
                exact: false,
              }
            } else result = { kind: 'brep', data: createCappedBrepLoftSurfaces(start, end, sides) }
            break
          }
          case 'guided_loft_surface': {
            if (n.construction === 'cartesian') {
              const curves = n.inputs.map(needCurve),
                guides = n.guides.map(needCurve)
              const built =
                n.start_tangents && n.end_tangents
                  ? createControlTangentCartesianGuidedLoft(
                      curves,
                      n.parameters,
                      guides,
                      n.guide_parameters,
                      n.start_tangents,
                      n.end_tangents,
                      n.error_budget,
                      n.max_cells,
                      n.max_map_evaluations,
                      n.section_mappings,
                    )
                  : createCartesianGuidedLoft(
                      curves,
                      n.parameters,
                      guides,
                      n.guide_parameters,
                      n.error_budget,
                      n.max_cells,
                      n.max_map_evaluations,
                      n.section_mappings,
                    )
              constructionReports[key] = {
                ...built.certificate,
                sectionMappingCertificates: built.section_mapping_certificates,
                originalSectionCertificates: built.original_section_certificates,
              }
              result = { kind: 'surface', data: built.surface }
              break
            }
            result = {
              kind: 'surface',
              data: guidedLoftNurbsCurves(
                n.inputs.map(needCurve),
                n.parameters,
                n.guides.map(needCurve),
                n.guide_parameters,
                n.start_tangents,
                n.end_tangents,
              ),
            }
            break
          }
          case 'clamped_loft_surface':
            result = {
              kind: 'surface',
              data: clampedLoftNurbsCurves(n.inputs.map(needCurve), n.parameters, n.start_tangent, n.end_tangent),
            }
            break
          case 'ribbon_surface': {
            const pipe = checkedRibbonNurbsSurface(
              needCurve(n.input),
              needCurve(n.width_law),
              n.normal,
              n.sections,
              n.max_deviation,
            )
            if (!pipe.report.accepted || !pipe.surface)
              throw new Error(
                `Ribbon sampled refinement ${pipe.report.sampledControlDeviation}mm exceeds ${pipe.report.budget}mm; refine sections or split the path`,
              )
            constructionReports[key] = pipe.report
            result = { kind: 'surface', data: pipe.surface }
            break
          }
          case 'variable_pipe_surface': {
            const pipe = checkedVariablePipeNurbsSurface(
              needCurve(n.input),
              needCurve(n.radius_law),
              n.normal,
              n.sections,
              n.max_deviation,
            )
            if (!pipe.report.accepted || !pipe.surface)
              throw new Error(
                `Variable pipe sampled refinement ${pipe.report.sampledControlDeviation}mm exceeds ${pipe.report.budget}mm; refine sections or split the path`,
              )
            constructionReports[key] = pipe.report
            result = { kind: 'surface', data: pipe.surface }
            break
          }
          case 'pipe_surface': {
            const pipe = checkedPipeNurbsSurface(needCurve(n.input), n.radius, n.normal, n.sections, n.max_deviation)
            if (!pipe.report.accepted || !pipe.surface)
              throw new Error(
                `Pipe sampled refinement ${pipe.report.sampledControlDeviation}mm exceeds ${pipe.report.budget}mm; refine sections or split the path`,
              )
            constructionReports[key] = pipe.report
            result = { kind: 'surface', data: pipe.surface }
            break
          }
          case 'screw_surface': {
            const approximation = approximateScrewNurbsSurfaces(
              needCurve(n.input),
              n.origin,
              n.axis,
              n.height,
              n.turns,
              n.phase_degrees,
              n.max_deviation,
            )
            constructionReports[key] = approximation.report
            result = {
              kind: 'patches',
              data: { patches: approximation.patches, faceIds: approximation.patches.map((_, i) => i) },
            }
            break
          }
          case 'surface_extrude_patches': {
            const patches = extrudeNurbsCurvePatches(needCurve(n.input), n.vector)
            result = { kind: 'patches', data: { patches, faceIds: patches.map((_, i) => i) } }
            break
          }
          case 'boundary_fill': {
            const patches = boundaryFillNurbsSurfaces(n.inputs.map(needCurve), n.center)
            result = { kind: 'patches', data: { patches, faceIds: patches.map((_, i) => i) } }
            break
          }
          case 'triangular_patch':
            result = {
              kind: 'surface',
              data: triangularNurbsPatch(needCurve(n.inputs[0]), needCurve(n.inputs[1]), needCurve(n.inputs[2])),
            }
            break
          case 'gordon_surface':
            result = {
              kind: 'surface',
              data: gordonNurbsSurface(
                n.u_curves.map(needCurve),
                n.v_curves.map(needCurve),
                n.parameters_u,
                n.parameters_v,
              ),
            }
            break
          case 'closed_loft_surface':
            result = { kind: 'surface', data: closedLoftNurbsCurves(n.inputs.map(needCurve), n.parameters) }
            break
          case 'natural_loft_surface': {
            if (n.section_mappings) {
              const built = createMappedNaturalLoft(n.inputs.map(needCurve), n.parameters, n.section_mappings)
              constructionReports[key] = {
                ...built.certificate,
                sectionMappingCertificates: built.section_mapping_certificates,
                originalSectionCertificates: built.original_section_certificates,
              }
              result = { kind: 'surface', data: built.surface }
            } else result = { kind: 'surface', data: naturalLoftNurbsCurves(n.inputs.map(needCurve), n.parameters) }
            break
          }
          case 'surface_loft':
            result = { kind: 'surface', data: loftAlignedNurbsCurves(n.inputs.map(needCurve)) }
            break
          case 'triangle_mesh': {
            const mesh = {
              positions: Float64Array.from(n.vertices.flat()),
              indices: Uint32Array.from(n.triangles.flat()),
            }
            result = { kind: 'mesh', data: { ...mesh, report: inspectPolygonMesh(mesh) } }
            break
          }
          case 'mesh_to_nurbs_brep':
            result = { kind: 'brep', data: meshToNurbsBrep(needMesh(n.input)) }
            break
          case 'mesh_to_sdf':
            result = { kind: 'sdf', data: meshToSdf(needMesh(n.input)) }
            break
          case 'mesh_to_subdivision': {
            const fitted = meshToSubdivision(needMesh(n.input), n.iterations)
            const { cage, ...report } = fitted
            reconstructionReports[key] = report
            result = { kind: 'subdivision', data: cage }
            break
          }
          case 'subdivision_tessellate': {
            const v = get(n.input)
            if (v.kind !== 'subdivision') throw new Error('Expected subdivision cage')
            result = { kind: 'mesh', data: tessellateSubdivision(v.data, n.levels) }
            break
          }
          case 'mesh_to_nurbs':
            result = { kind: 'patches', data: meshToNurbs(needMesh(n.input)) }
            break
          case 'mesh_fit_nurbs':
            result = { kind: 'patches', data: meshToNurbs(needMesh(n.input), 'point_normal', n.max_deviation) }
            break
          case 'nurbs_patches_tessellate': {
            const v = get(n.input)
            if (v.kind !== 'patches') throw new Error('Expected NURBS patch set')
            result = { kind: 'mesh', data: tessellateNurbsPatches(v.data, n.segments) }
            break
          }
          case 'subdivision':
            result = { kind: 'mesh', data: tessellateSubdivision({ vertices: n.vertices, faces: n.faces }, n.levels) }
            break
          case 'sdf_sphere':
            result = { kind: 'sdf', data: { kind: 'sphere', center: n.center, radius: n.radius } }
            break
          case 'sdf_box':
            result = { kind: 'sdf', data: { kind: 'box', center: n.center, half_size: n.half_size } }
            break
          case 'sdf_torus':
            result = {
              kind: 'sdf',
              data: { kind: 'torus', center: n.center, major_radius: n.major_radius, minor_radius: n.minor_radius },
            }
            break
          case 'sdf_union':
          case 'sdf_intersection':
          case 'sdf_difference':
            result = {
              kind: 'sdf',
              data: {
                kind: n.op === 'sdf_union' ? 'union' : n.op === 'sdf_intersection' ? 'intersection' : 'difference',
                a: needSdf(n.inputs[0]),
                b: needSdf(n.inputs[1]),
              },
            }
            break
          case 'sdf_smooth_union':
            result = {
              kind: 'sdf',
              data: { kind: 'smooth_union', a: needSdf(n.inputs[0]), b: needSdf(n.inputs[1]), radius: n.radius },
            }
            break
          case 'sdf_offset':
            result = { kind: 'sdf', data: { kind: 'offset', input: needSdf(n.input), distance: n.distance } }
            break
          case 'sdf_translate':
            result = { kind: 'sdf', data: { kind: 'translate', input: needSdf(n.input), vector: n.vector } }
            break
          case 'sdf_tessellate':
            result = { kind: 'mesh', data: tessellateSdf(needSdf(n.input), { min: n.min, max: n.max, cells: n.cells }) }
            break
          case 'brep_box':
            result = { kind: 'brep', data: createBrepBox(n.min, n.max) }
            break
          case 'brep_sphere':
            result = { kind: 'brep', data: createBrepSphere(n.radius) }
            break
          case 'radial_cam_profile':
          case 'radial_cam_walls': {
            const {id:_id,op,...args}=n
            const {curves,...report}=callGeometryRust<{curves:NurbsCurve[];spans:number;stop:string;maxSampledErrorMm:number;maxSampledPressureAngleRadians:number;sampledUndercut:boolean}>('radial_cam_profile',args)
            constructionReports[key]={measurement:{operation:op,report:{...report,pieces:curves.length}}}
            if (n.op==='radial_cam_profile') {
              const curve=curves[n.piece]
              if (!curve) throw new Error('Cam profile piece outside the generated range')
              result={kind:'curve',data:curve}
            } else {
              const patches=curves.map(curve=>extrudeNurbsCurve(curve,n.vector))
              result={kind:'patches',data:{patches,faceIds:patches.map((_,i)=>i)}}
            }
            break
          }
          case 'rack_profile':
          case 'brep_rack': {
            const {id:_id,op,...spec}=n
            const {model,curve,...report}=callGeometryRust<{model:NurbsBrep|null;curve:NurbsCurve;pitchMm:number;widthMm:number;toothThicknessAtPitchMm:number;toothCentersMm:number[]}>(op,spec)
            constructionReports[key]={measurement:{operation:op,report}}
            if (op==='rack_profile') result={kind:'curve',data:curve}
            else {
              if (!model) throw new Error('Missing extruded rack body')
              result={kind:'brep',data:model}
            }
            break
          }
          case 'finite_za_worm':
          case 'finite_za_wheel': {
            const {id:_id,op,...spec}=n
            const {model,...report} = op === 'finite_za_wheel' && resolveWheel
              ? resolveWheel(key, spec)
              : callGeometryRust<{model:NurbsBrep;wallErrorUpperMm:number;solidGeometryCertified:boolean}>(op, spec)
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'brep',data:model}
            break
          }
          case 'finite_external_thread':
          case 'finite_internal_thread': {
            const {id:_id,op,...spec}=n
            const {model,...report}=callGeometryRust<{model:NurbsBrep;patchErrorUpperMm:number;axialErrorBandUpperMm:number;boundaryAgreement:boolean;originalWallCount:number}>(op,spec)
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'brep',data:model}
            break
          }
          case 'verified_bevel_gear': {
            const {id:_id,op,...spec}=n
            const {model,...report}=callGeometryRust<{model:NurbsBrep;worldErrorUpperMm:number;solidGeometryCertified:boolean}>(op,spec)
            constructionReports[key]={measurement:{operation:op,report}}
            result={kind:'brep',data:model}
            break
          }
          case 'brep_gear': {
            const spec = {
                module: n.module,
                teeth: n.teeth,
                height: n.height,
                pressureAngle: n.pressure_angle,
                helixAngle: n.helix_angle,
                herringbone: n.herringbone,
                bore: n.bore,
                internal: n.internal,
                rimWidth: n.rim_width,
                clearance: n.clearance,
                backlash: n.backlash,
            }
            const inspect = n.inspect_spans || n.span_count !== undefined || n.measured_span_mm !== undefined
            const span = inspect ? callGeometryRust<{counts:number[]; nominalWidthMm:[number,number]|null; inferredThinningMm:[number,number]|null}>('brep_nurbs_gear_span', {
              ...spec,
              ...(n.span_count !== undefined && {spanCount:n.span_count}),
              ...(n.measured_span_mm && {measuredSpanMm:n.measured_span_mm}),
            }) : undefined
            if (n.verified_profile) {
              const {model,...report}=callGeometryRust<{model:NurbsBrep;profileErrorUpperMm:number;idealWallErrorUpperMm:number;profileWinding:number[];solidGeometryCertified:boolean}>('brep_nurbs_verified_gear',spec)
              result={kind:'brep',data:model}
              constructionReports[key]={measurement:{operation:'brep_nurbs_verified_gear',report:{...report,...span}}}
            } else {
              result={kind:'brep',data:createBrepGear(spec)}
              if (span) constructionReports[key]={measurement:{operation:'brep_nurbs_gear_span',report:span}}
            }
            break
          }
          case 'brep_torus':
            result = { kind: 'brep', data: createBrepTorus(n.major_radius, n.minor_radius) }
            break
          case 'brep_cylinder':
            result = { kind: 'brep', data: createBrepCylinder(n.radius, n.height) }
            break
          case 'brep_frustum':
            result = { kind: 'brep', data: createBrepFrustum(n.bottom_radius, n.top_radius, n.height) }
            break
          case 'brep_tube':
            result = { kind: 'brep', data: createBrepTube(n.outer_radius, n.inner_radius, n.height) }
            break
          case 'brep_revolve': {
            const profile = get(n.input)
            if (profile.kind !== 'profile' || profile.data.holes?.length)
              throw new Error('Exact revolve requires one polygon profile without holes')
            result = { kind: 'brep', data: revolveBrepProfile(profile.data.outer as [number, number][], n.angle) }
            break
          }
          case 'brep_extrude': {
            const profile = get(n.input)
            if (profile.kind !== 'profile') throw new Error('Expected a polygon profile')
            result = {
              kind: 'brep',
              data: extrudeBrepPolygon(
                profile.data.outer as [number, number][],
                Math.min(0, n.height),
                Math.max(0, n.height),
                profile.data.holes as [number, number][][],
              ),
            }
            break
          }
          case 'brep_progressive_sweep': {
            const corrections = readSweepCorrections(n)
            if (n.orientation === 'authored' && (!n.frame_axis || !n.frame_normal))
              throw new Error('Authored progressive body requires frame_axis and frame_normal')
            if (
              (n.contact_parameter !== undefined && !n.orientation_guide) ||
              (n.contact_profile !== undefined && n.contact_parameter === undefined)
            )
              throw new Error('Contact anchor requires orientation_guide and contact_parameter')
            const frameOptions =
              n.orientation === 'authored'
                ? { orientation: 'authored' as const, frameAxis: n.frame_axis!, frameNormal: n.frame_normal! }
                : { orientation: n.orientation }
            const makeBody = (...args: BodyArguments) =>
              resolveBody ? resolveBody(key, args) : createProgressiveBrepProfileBody(...args)
            const body = makeBody(
              n.loops.map(wire => wire.map(needCurve)),
              needCurve(n.inputs[0]),
              n.scale,
              n.twist,
              {
                annularCaps: { automatic: true },
                ...corrections,
                ...(n.orientation_guide
                  ? {
                      orientationGuide: needCurve(n.orientation_guide),
                      ...(n.contact_parameter !== undefined
                        ? { contactAnchor: { profileIndex: n.contact_profile ?? 0, parameter: n.contact_parameter } }
                        : {}),
                    }
                  : {}),
                axisScale: n.axis_scale,
                centerLaw: n.center_law,
                normal: n.normal,
                ...frameOptions,
                spacing: n.spacing,
                initialSections: n.initial_sections,
                maxSections: n.max_sections,
                maxDeviation: n.max_deviation,
                lengthTolerance: n.length_tolerance,
                lengthMaxCells: n.length_max_cells,
                rmfTransportSteps: n.rmf_transport_steps,
                errorMaxCells: n.error_max_cells,
                errorMaxProducts: n.error_max_products,
              },
            )
            progressiveBodySources.set(key, body)
            constructionReports[key] = {
              ...body.approximation.report,
              levels: body.approximation.levels,
              profilePatchRanges: body.approximation.profilePatchRanges,
              globalEmbeddingCertified: body.globalEmbeddingCertified,
              bodyBoundaryEvidence: {
                capSelectionExactWork: body.capSelectionExactWork,
                capRepresentation: body.capRepresentation,
                capFaceGroups: body.capFaceGroups,
                capFaces: body.capFaces,
                planarCapFaces: body.planarCapFaces,
                retainedAnnularFilling: body.retainedAnnularFilling,
                retainedSmoothness: body.retainedSmoothness,
                profileCorrectionErrorUpper: body.profileCorrectionErrorUpper,
                profileSmoothness: body.profileSmoothness,
                capCorrectionErrorUpper: body.capCorrectionErrorUpper,
                retainedCaps: body.retainedCaps,
                retainedWalls: body.retainedWalls,
                capProjection: body.capProjection,
                bodyDecompositionErrorUpper: body.bodyDecompositionErrorUpper,
                bodyDecompositionProducts: body.bodyDecompositionProducts,
                filledCapErrorUpper: body.filledCapErrorUpper,
                boundaryErrorUpper: body.boundaryErrorUpper,
                boundaryErrorWithinBudget: body.boundaryErrorWithinBudget,
                boundaryContinuousBound: body.boundaryContinuousBound,
                boundaryErrorScope: body.boundaryErrorScope,
                budget: body.approximation.report.budget,
                closedPath: body.approximation.report.closedPath,
                globalEmbeddingCertified: false,
              },
            }
            result = { kind: 'brep', data: body.model }
            break
          }
          case 'brep_extrude_curves':
            result = {
              kind: 'brep',
              data: extrudeBrepCurves(
                n.loops.map(wire => wire.map(needCurve)),
                n.z_min,
                n.z_max,
              ),
            }
            break
          case 'brep_boolean':
            result = { kind: 'brep', data: booleanNurbsBrep(needBrep(n.inputs[0]), needBrep(n.inputs[1]), n.operation) }
            break
          case 'brep_chamfer': {
            const model = needBrep(n.input)
            result = { kind: 'brep', data: chamferNurbsBrepEdges(model, selectBrepEdges(model, n.edges), n.size) }
            break
          }
          case 'brep_fillet': {
            const model = needBrep(n.input)
            result = {
              kind: 'brep',
              data: filletNurbsBrepEdges(model, selectBrepEdges(model, n.edges), n.radius, n.segments),
            }
            break
          }
          case 'brep_full_round_exact': {
            // Rust constructs and proves the exact body; only its display mesh enters the graph.
            const artifact = callGeometryRust<Readonly<Record<string, unknown>>>('cad_full_round_exact', {
              source: needBrep(n.input),
              faces: n.faces,
            })
            const display = prepareExactFullRoundDisplay(artifact, n.segments)
            result = {
              kind: 'mesh',
              data: normalizePolygonMesh({
                positions: display.displayVertices,
                indices: display.displayIndices,
                report: display.report,
                faceIds: display.faceIds,
                topologyFaceIds: display.topologyFaceIds,
              }),
            }
            break
          }
          case 'brep_tessellate': {
            const input = get(n.input)
            if (input.kind !== 'brep') throw new Error('Expected a B-rep model')
            result = { kind: 'mesh', data: input.displayMesh?.segments === n.segments ? input.displayMesh.mesh : tessellateNurbsBrep(input.data, n.segments) }
            break
          }
          case 'surface_extrude':
            result = { kind: 'surface', data: extrudeNurbsCurve(needCurve(n.input), n.vector) }
            break
          case 'surface_revolve':
            result = { kind: 'surface', data: revolveNurbsCurve(needCurve(n.input), n.origin, n.axis, n.angle) }
            break
          case 'transform': {
            const m = n.matrix
            const v = get(n.input)
            if (v.kind === 'curve') result = { kind: 'curve', data: transformNurbsCurve(v.data, m) }
            else if (v.kind === 'surface') result = { kind: 'surface', data: transformNurbsSurface(v.data, m) }
            else if (v.kind === 'patches')
              result = {
                kind: 'patches',
                data: { patches: transformNurbsSurfacePatches(v.data.patches, m), faceIds: [...v.data.faceIds] },
              }
            else if (v.kind === 'brep') {
              const source = exactMiterPlacements.get(n.input)
              if (source) {
                const placed = transformCertifiedMiterBody(source.body, m, {
                  quantum: source.quantum,
                  maxWork: source.maxWork,
                  maxDeviation: source.body.boundaryCertificate.budget,
                })
                constructionReports[key] = placed
                exactMiterPlacements.set(key, { ...source, body: placed })
                result = { kind: 'brep', data: placed.model }
              } else result = { kind: 'brep', data: transformNurbsBrep(v.data, m) }
            } else throw new Error('Transform spline control data before tessellation.')
            break
          }
          case 'tessellate': {
            if (n.trim && n.trim_curves) throw new Error('Use only one trim representation.')
            const sample = (id: string) => {
              const c = needCurve(id)
              if (c.controlPoints[0]!.length !== 2) throw new Error('UV trim requires a 2D curve.')
              const a = c.knots[c.degree]!,
                b = c.knots[c.controlPoints.length]!
              const params = [
                a,
                b,
                ...Array.from({ length: n.trim_segments }, (_, i) => a + ((b - a) * i) / n.trim_segments),
              ]
              const pts = evaluateNurbsCurvePoints(c, params),
                first = pts[0]!,
                last = pts[1]!
              if (Math.hypot(...first.map((v, i) => v - last[i]!)) > 1e-9)
                throw new Error('UV trim curve must be closed.')
              return pts.slice(2)
            }
            const surface = needSurface(n.input)
            const rings = transferredTrimLoops.get(n.input)
            const sampleRing = (ring: NurbsCurve[]) => ring.flatMap(curve => {
              const start = curve.knots[curve.degree]!
              const end = curve.knots[curve.controlPoints.length]!
              const spans = curve.knots.slice(curve.degree, curve.controlPoints.length)
                .filter(t => t >= start && t < end)
              const parameters = new Set(spans)
              if (curve.degree > 1) {
                for (let i = 0; i < n.trim_segments; i++) {
                  parameters.add(start + (end - start) * i / n.trim_segments)
                }
              }
              return evaluateNurbsCurvePoints(curve, [...parameters].sort((a,b) => a-b))
            })
            const trim = n.trim_curves
              ? { outer: sample(n.trim_curves.outer), holes: n.trim_curves.holes.map(sample) }
              : n.trim ?? (rings ? {outer:sampleRing(rings[0]!),holes:rings.slice(1).map(sampleRing)} : undefined)
            result = {
              kind: 'mesh',
              data: tessellateNurbsSurface(surface, {
                segmentsU: n.segments_u,
                segmentsV: n.segments_v,
                ...(trim ? { trim } : {}),
              }),
            }
            break
          }
          case 'mesh_boolean':
            result = {
              kind: 'mesh',
              data: booleanPolygonMeshes(
                needMesh(n.inputs[0]),
                needMesh(n.inputs[1]),
                n.operation,
                n.relative_tolerance === undefined ? {} : { relativeTolerance: n.relative_tolerance },
              ),
            }
            break
          case 'thicken':
            result = { kind: 'mesh', data: thickenNurbsMesh(needMesh(n.input), n.vector) }
            break
          default:
            throw new Error(`Unsupported native geometry operation ${n.op}`)
        }
    } catch (error) {
      if (
        error instanceof PendingWheel ||
        error instanceof PendingProgressiveStations ||
        error instanceof PendingProgressiveSweep ||
        error instanceof PendingProgressiveBody ||
        error instanceof PendingProgressiveMiter
      )
        throw error
      throw new Error(`Node ${key}: ${error instanceof Error ? error.message : 'NURBS operation failed.'}`, {
        cause: error,
      })
    }
    if (result.kind === 'sdf') evaluateSdf(result.data, [0, 0, 0])
    if (result.kind === 'mesh' && (meshTriangles += result.data.indices.length / 3) > 60000)
      throw new Error('NURBS graph mesh work exceeds 60000 triangles.')
    cache.set(key, result)
    return result
  }
  return { get, cache, constructionReports, reconstructionReports, exactMiterPlacements, transferredTrimLoops }
}
