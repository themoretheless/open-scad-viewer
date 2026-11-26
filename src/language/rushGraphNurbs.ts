export { compileRushGraphNurbs, RushGraphNurbsError, hashNurbsDocument } from './rushGraphNurbsCompiler'
import { z } from 'zod/v4'
const id = z.string().regex(/^[A-Za-z][A-Za-z0-9_]{0,31}$/)
const number = z.number().finite().min(-1e6).max(1e6)
const scalar = z.union([number, z.object({ param: id }).strict()])
const vector = z.tuple([scalar, scalar, scalar]),
  uv = z.tuple([scalar, scalar]),
  interval = z.tuple([scalar, scalar])
const exactUnsignedRatio = z.string().regex(/^[0-9]{1,1234}\/[0-9]{1,1234}$/)
const finiteVector = z.tuple([z.number(), z.number(), z.number()])
/** Edge query evaluated against the input B-rep: every given filter must match. */
const brepFaceQuery = z
  .object({
    normal: finiteVector.optional(),
    angle_tolerance: z.number().min(0).max(90).default(1),
    type: z.enum(['planar', 'curved']).optional(),
    min_area: z.number().min(0).optional(),
    max_area: z.number().min(0).optional(),
  })
  .strict()
const brepEdgeQuery = z
  .object({
    direction: finiteVector.optional(),
    angle_tolerance: z.number().min(0).max(90).default(1),
    min_length: z.number().min(0).optional(),
    max_length: z.number().min(0).optional(),
    convexity: z.enum(['convex', 'concave', 'smooth']).optional(),
    face: brepFaceQuery.optional(),
  })
  .strict()
const brepEdgeSelection = z.union([z.array(z.number().int().min(0)).min(1).max(256), brepEdgeQuery])
const cornerJets = z.tuple([z.tuple([vector, vector]), z.tuple([vector, vector])])
const knots = z.array(scalar).min(4).max(512),
  degree = z.number().int().min(1).max(25)
// Scalar map data is dimensionless; it is not bounded as model coordinates.
const mapScalar = z.union([z.number().finite(), z.object({ param: id }).strict()])
type MapScalar = z.infer<typeof mapScalar>
type LoftParameterMap =
  | {
      pieces: Array<{
        domain: [MapScalar, MapScalar]
        range: [MapScalar, MapScalar]
        controlValues: MapScalar[]
        weights: MapScalar[]
      }>
    }
  | { composition: LoftParameterMap[] }
function parameterMap(depth: number): z.ZodType<LoftParameterMap> {
  const pair = z.tuple([mapScalar, mapScalar])
  const leaf = z
    .object({
      pieces: z
        .array(
          z
            .object({
              domain: pair,
              range: pair,
              controlValues: z.array(mapScalar).min(2).max(26),
              weights: z.array(mapScalar).min(2).max(26),
            })
            .strict(),
        )
        .min(1)
        .max(256),
    })
    .strict()
  return depth === 0
    ? leaf
    : z.union([
        leaf,
        z
          .object({
            composition: z
              .array(parameterMap(depth - 1))
              .min(1)
              .max(256),
          })
          .strict(),
      ])
}
const sectionMappings = {
  section_mappings: z
    .array(z.union([z.null(), parameterMap(8)]))
    .min(2)
    .max(86)
    .optional(),
}
const loftConstruction = {
  construction: z.enum(['homogeneous', 'cartesian']).optional(),
  error_budget: scalar.optional(),
  max_cells: z.number().int().min(1).max(1000000).optional(),
  max_map_evaluations: z.number().int().min(1).max(1000000).optional(),
}
const insert = z.array(z.object({ value: scalar, count: z.number().int().min(1).max(25) }).strict()).max(32)
const sectionCoordinate = z.union([z.number(),z.string().min(1).max(5000)])
const sectionQueryPoint = z.tuple([sectionCoordinate,sectionCoordinate,sectionCoordinate])

export const rushGraphNurbsSchema = z
  .object({
    language: z.literal('rush/nurbs-1'),
    units: z.literal('mm'),
    parameters: z
      .array(z.object({ id, value: number }).strict())
      .max(64)
      .default([]),
    nodes: z
      .array(
        z.discriminatedUnion('op', [
          z.object({ id, op: z.literal('sphere_surface'), center: vector, radius: scalar }).strict(),
          z.object({ id, op: z.literal('cylinder_surface'), center: vector, radius: scalar, height: scalar }).strict(),
          z.object({ id, op: z.literal('cone_surface'), center: vector, radius: scalar, height: scalar }).strict(),
          z
            .object({
              id,
              op: z.literal('formula_surface'),
              domain: z.tuple([scalar, scalar, scalar, scalar]),
              expressions: z.tuple([
                z.union([
                  z
                    .array(z.union([scalar, z.enum(['u', 'v', 'neg', '+', '-', '*', '/'])]))
                    .min(1)
                    .max(64),
                  z.string().min(1).max(1024),
                ]),
                z.union([
                  z
                    .array(z.union([scalar, z.enum(['u', 'v', 'neg', '+', '-', '*', '/'])]))
                    .min(1)
                    .max(64),
                  z.string().min(1).max(1024),
                ]),
                z.union([
                  z
                    .array(z.union([scalar, z.enum(['u', 'v', 'neg', '+', '-', '*', '/'])]))
                    .min(1)
                    .max(64),
                  z.string().min(1).max(1024),
                ]),
              ]),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('formula_curve'),
              domain: z.tuple([scalar, scalar]),
              expressions: z.tuple([
                z.union([
                  z
                    .array(z.union([scalar, z.enum(['t', 'neg', '+', '-', '*', '/'])]))
                    .min(1)
                    .max(64),
                  z.string().min(1).max(1024),
                ]),
                z.union([
                  z
                    .array(z.union([scalar, z.enum(['t', 'neg', '+', '-', '*', '/'])]))
                    .min(1)
                    .max(64),
                  z.string().min(1).max(1024),
                ]),
                z.union([
                  z
                    .array(z.union([scalar, z.enum(['t', 'neg', '+', '-', '*', '/'])]))
                    .min(1)
                    .max(64),
                  z.string().min(1).max(1024),
                ]),
              ]),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('closed_spline_curve'),
              points: z.array(vector).min(4).max(86),
              parameters: z.array(scalar).min(4).max(86),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('clamped_spline_curve'),
              points: z.array(vector).min(2).max(86),
              parameters: z.array(scalar).min(2).max(86),
              start_tangent: vector,
              end_tangent: vector,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('natural_spline_curve'),
              points: z.array(vector).min(2).max(86),
              parameters: z.array(scalar).min(2).max(86),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('hermite_curve'),
              points: z.array(vector).min(2).max(86),
              tangents: z.array(vector).min(2).max(86),
              parameters: z.array(scalar).min(2).max(86),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('clothoid_curve'),
              center: vector,
              length: scalar,
              start_curvature: scalar,
              end_curvature: scalar,
              phase_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('spherical_spiral_curve'),
              center: vector,
              radius: scalar,
              longitude_turns: scalar,
              latitude_turns: scalar,
              longitude_phase_degrees: scalar,
              latitude_phase_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('toroidal_spiral_curve'),
              center: vector,
              major_radius: scalar,
              minor_radius: scalar,
              major_turns: scalar,
              minor_turns: scalar,
              major_phase_degrees: scalar,
              minor_phase_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('torus_knot_curve'),
              center: vector,
              major_radius: scalar,
              minor_radius: scalar,
              p: scalar,
              q: scalar,
              major_phase_degrees: scalar,
              minor_phase_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('helicoid_patches'),
              center: vector,
              inner_radius: scalar,
              outer_radius: scalar,
              height: scalar,
              turns: scalar,
              phase_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('circle_rectangle_transition'),
              circle_center: vector,
              circle_normal: vector,
              circle_seam: vector,
              circle_radius: scalar,
              rectangle_center: vector,
              rectangle_axis_u: vector,
              rectangle_axis_v: vector,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('ellipse_transition_surface'),
              start_center: vector,
              start_axis_u: vector,
              start_axis_v: vector,
              end_center: vector,
              end_axis_u: vector,
              end_axis_v: vector,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('circle_transition_surface'),
              start_center: vector,
              start_normal: vector,
              start_seam: vector,
              start_radius: scalar,
              end_center: vector,
              end_normal: vector,
              end_seam: vector,
              end_radius: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('helicoid_surface'),
              center: vector,
              inner_radius: scalar,
              outer_radius: scalar,
              height: scalar,
              turns: scalar,
              phase_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('catenoid_patches'),
              center: vector,
              scale: scalar,
              start_z: scalar,
              end_z: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z.object({
            id, op: z.literal('superellipsoid_patches'), center: vector, radii: vector,
            exponents: z.tuple([scalar, scalar]), tolerance: scalar, max_cells: scalar,
          }).strict(),
          z.object({
            id, op: z.literal('corrugated_bellows'), center: vector, mean_radius: scalar,
            amplitude: scalar, height: scalar, radial_thickness: scalar, periods: scalar,
          }).strict(),
          z.object({
            id, op: z.literal('optical_dome'), center: vector, base_axes: z.tuple([scalar, scalar]), height: scalar,
          }).strict(),
          z.object({
            id, op: z.literal('optical_lens'), center: vector, base_axes: z.tuple([scalar, scalar]),
            sags: z.tuple([scalar, scalar]), vertex_thickness: scalar,
          }).strict(),
          z.object({
            id, op: z.literal('cubic_wave_surface'), origin: vector,
            width: scalar, length: scalar, amplitude: scalar, periods: scalar,
          }).strict(),
          z
            .object({
              id,
              op: z.literal('catenoid_surface'),
              center: vector,
              scale: scalar,
              start_z: scalar,
              end_z: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('catenary_curve'),
              center: vector,
              scale: scalar,
              start_x: scalar,
              end_x: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('archimedean_spiral_curve'),
              center: vector,
              start_radius: scalar,
              end_radius: scalar,
              start_degrees: scalar,
              end_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('epicycloid_curve'),
              center: vector,
              fixed_radius: scalar,
              rolling_radius: scalar,
              start_degrees: scalar,
              end_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('hypocycloid_curve'),
              center: vector,
              fixed_radius: scalar,
              rolling_radius: scalar,
              start_degrees: scalar,
              end_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('trochoid_curve'),
              center: vector,
              rolling_radius: scalar,
              tracing_radius: scalar,
              start_degrees: scalar,
              end_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('cycloid_curve'),
              center: vector,
              radius: scalar,
              start_degrees: scalar,
              end_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('lissajous_curve'),
              center: vector,
              amplitudes: vector,
              frequencies: vector,
              phases_degrees: vector,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('logarithmic_spiral_curve'),
              center: vector,
              radius: scalar,
              growth: scalar,
              start_degrees: scalar,
              end_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('involute_curve'),
              center: vector,
              radius: scalar,
              start_degrees: scalar,
              end_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('helix_curve'),
              center: vector,
              radius: scalar,
              height: scalar,
              turns: scalar,
              phase_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('elliptic_helix_curve'),
              center: vector,
              radius_x: scalar,
              radius_y: scalar,
              height: scalar,
              turns: scalar,
              phase_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('variable_pitch_helix_curve'),
              center: vector,
              radius: scalar,
              height: scalar,
              turns: scalar,
              start_pitch: scalar,
              end_pitch: scalar,
              phase_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('conical_helix_curve'),
              center: vector,
              start_radius: scalar,
              end_radius: scalar,
              height: scalar,
              turns: scalar,
              phase_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z.object({ id, op: z.literal('plane_patch'), origin: vector, axis_u: vector, axis_v: vector }).strict(),
          z
            .object({
              id,
              op: z.literal('grid_spline_surface'),
              points: z.array(z.array(vector).min(2).max(11)).min(2).max(11),
              parameters_u: z.array(scalar).min(2).max(11),
              parameters_v: z.array(scalar).min(2).max(11),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('hermite_patch'),
              corners: cornerJets,
              tangent_u: cornerJets,
              tangent_v: cornerJets,
              twist: cornerJets,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('bilinear_patch'),
              corners: z.tuple([z.tuple([vector, vector]), z.tuple([vector, vector])]),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('bezier_surface'),
              points: z.array(z.array(vector).min(2).max(26)).min(2).max(26),
              weights: z.array(z.array(scalar).min(2).max(26)).min(2).max(26).optional(),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('bezier_curve'),
              points: z.array(vector).min(2).max(26),
              weights: z.array(scalar).min(2).max(26).optional(),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('transition_polyline_curve'),
              points: z.array(vector).min(2).max(17),
              setback: scalar,
              closed: z.boolean().default(false),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('round_polyline_curve'),
              points: z.array(vector).min(2).max(17),
              radius: scalar,
              closed: z.boolean().default(false),
            })
            .strict(),
          z.object({ id, op: z.literal('curve_compose'), inputs: z.array(id).min(2).max(32) }).strict(),
          z.object({ id, op: z.literal('rounded_rectangle_profile'), center: vector, width: scalar, height: scalar, radius: scalar }).strict(),
          z.object({ id, op: z.literal('capsule_profile'), center: vector, straight: scalar, radius: scalar }).strict(),
          z.object({ id, op: z.literal('annular_sector_surface'), center: vector, normal: vector, inner_radius: scalar, outer_radius: scalar, start_degrees: scalar, sweep_degrees: scalar }).strict(),
          z.object({ id, op: z.literal('keyway_profile'), mouth: vector, width: scalar, depth: scalar }).strict(),
          z.object({ id, op: z.literal('t_slot_profile'), mouth: vector, neck_width: scalar, flange_width: scalar, neck_depth: scalar, total_depth: scalar }).strict(),
          z.object({ id, op: z.literal('dovetail_profile'), mouth: vector, mouth_width: scalar, base_width: scalar, depth: scalar }).strict(),
          z.object({ id, op: z.literal('poly_v_belt_profile'), center: vector, ribs: scalar, pitch: scalar, back_thickness: scalar, rib_height: scalar }).strict(),
          z.object({ id, op: z.literal('toothed_belt_profile'), center: vector, teeth: scalar, pitch: scalar, base_width: scalar, tip_width: scalar, back_thickness: scalar, tooth_height: scalar }).strict(),
          z.object({ id, op: z.literal('o_ring_groove_profile'), mouth: vector, width: scalar, depth: scalar, radius: scalar }).strict(),
          z.object({ id, op: z.literal('retaining_ring_profile'), center: vector, inner_radius: scalar, outer_radius: scalar, gap_degrees: scalar }).strict(),
          z.object({ id, op: z.literal('hex_fastener_profile'), center: vector, across_flats: scalar }).strict(),
          z.object({ id, op: z.literal('square_fastener_profile'), center: vector, across_flats: scalar }).strict(),
          z.object({ id, op: z.literal('exact_ellipse'), center: vector, axis_u: vector, axis_v: vector, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('exact_ellipsoid'), center: vector, axes: z.tuple([vector, vector, vector]), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('exact_ring_torus'), center: vector, axes: z.tuple([vector, vector, vector]), major_radius: scalar, minor_radius: scalar, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('exact_frustum_side'), center: vector, axes: z.tuple([vector, vector, vector]), radii: interval, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('exact_planar_region'), center: vector, axes: z.tuple([vector, vector]), radii: interval, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('line_curve'), start: vector, end: vector }).strict(),
          z
            .object({
              id,
              op: z.literal('polyline_curve'),
              points: z.array(vector).min(2).max(256),
              closed: z.boolean().default(false),
            })
            .strict(),
          z.object({ id, op: z.literal('circle_curve'), center: vector, normal: vector, radius: scalar }).strict(),
          z
            .object({
              id,
              op: z.literal('circle_arc'),
              center: vector,
              normal: vector,
              radius: scalar,
              start_degrees: scalar,
              sweep_degrees: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('hyperboloid_one_sheet'),
              center: vector,
              radii: vector,
              start: scalar,
              end: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('hyperboloid_two_sheet'),
              center: vector,
              radii: vector,
              start: scalar,
              end: scalar,
              lower: z.boolean().default(false),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('polynomial_graph'),
              bounds: z.tuple([scalar, scalar, scalar, scalar]),
              coefficients: z.array(z.array(scalar).min(1).max(13)).min(1).max(13),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('polynomial_curve'),
              domain: interval,
              coefficients: z.array(vector).min(1).max(13),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('polynomial_surface'),
              domain: z.tuple([scalar, scalar, scalar, scalar]),
              coefficients: z.array(z.array(vector).min(1).max(13)).min(1).max(13),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('rational_polynomial_curve'),
              domain: interval,
              coefficients: z
                .array(z.tuple([scalar, scalar, scalar, scalar]))
                .min(1)
                .max(13),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('rational_polynomial_surface'),
              domain: z.tuple([scalar, scalar, scalar, scalar]),
              coefficients: z
                .array(
                  z
                    .array(z.tuple([scalar, scalar, scalar, scalar]))
                    .min(1)
                    .max(13),
                )
                .min(1)
                .max(13),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('parabola_curve'),
              center: vector,
              axis_u: vector,
              axis_v: vector,
              start: scalar,
              end: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('hyperbola_curve'),
              center: vector,
              axis_u: vector,
              axis_v: vector,
              start: scalar,
              end: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('elliptic_cylinder_surface'),
              center: vector,
              radius_x: scalar,
              radius_y: scalar,
              height: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('cone_frustum_surface'),
              center: vector,
              bottom_radius: scalar,
              top_radius: scalar,
              height: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('quadratic_patch'),
              bounds: z.tuple([scalar, scalar, scalar, scalar]),
              coefficients: z.tuple([scalar, scalar, scalar, scalar, scalar, scalar]),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('ellipse_arc'),
              center: vector,
              axis_u: vector,
              axis_v: vector,
              start_degrees: scalar.default(0),
              sweep_degrees: scalar.default(360),
            })
            .strict(),
          z.object({ id, op: z.literal('ellipsoid_surface'), center: vector, radii: vector }).strict(),
          z
            .object({
              id,
              op: z.literal('torus_surface'),
              center: vector,
              major_radius: scalar,
              radial_radius: scalar,
              axial_radius: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('polygon_profile'),
              outer: z.array(uv).min(3).max(128),
              holes: z.array(z.array(uv).min(3).max(128)).max(16).default([]),
            })
            .strict(),
          z.object({ id, op: z.literal('extrude'), input: id, height: scalar }).strict(),
          z
            .object({
              id,
              op: z.literal('revolve'),
              input: id,
              angle: scalar,
              segments: z.number().int().min(3).max(64).default(32),
            })
            .strict(),
          z.object({ id, op: z.literal('polygon_extrude'), input: id, vector }).strict(),
          z
            .object({ id, op: z.literal('polygon_sweep'), input: id, path: z.array(vector).min(2).max(64), up: vector })
            .strict(),
          z
            .object({
              id,
              op: z.literal('polygon_loft'),
              sections: z.array(z.array(vector).min(3).max(128)).min(2).max(64),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('framed_sweep'),
              inputs: z.tuple([id, id]),
              normal: vector,
              sections: z.number().int().min(2).max(32),
              max_deviation: scalar,
            })
            .strict(),
          z.object({ id, op: z.literal('surface_sweep'), inputs: z.tuple([id, id]) }).strict(),
          z
            .object({
              id,
              op: z.literal('twist_sweep'),
              inputs: z.tuple([id, id]),
              origin: vector,
              axis: vector,
              start_degrees: scalar,
              sweep_degrees: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('two_guide_sweep'),
              inputs: z.tuple([id, id, id]),
              width: scalar,
              axis_y: vector,
              axis_z: vector,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('scaled_sweep'),
              inputs: z.tuple([id, id]),
              origin: vector,
              scale: z
                .object({
                  degree,
                  knots,
                  values: z.array(scalar).min(2).max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict(),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('profile_sweep'),
              inputs: z.tuple([id, id]),
              normal: vector,
              sections: z.number().int().min(2).max(32),
              max_deviation: scalar,
              scale: z
                .object({
                  degree,
                  knots,
                  values: z.array(scalar).min(2).max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict(),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('progressive_sweep'),
              rmf_transport_steps: z
                .literal([2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384])
                .optional(),
              error_max_cells: z.number().int().min(0).max(100000).optional(),
              error_max_products: z.number().int().min(0).max(1000000).optional(),
              contact_parameter: scalar.optional(),
              contact_profile: z.number().int().min(0).max(63).optional(),
              orientation_guide: id.optional(),
              inputs: z.array(id).min(2).max(65),
              axis_scale: z
                .object({
                  degree,
                  knots,
                  values: z
                    .array(z.tuple([scalar, scalar, scalar]))
                    .min(2)
                    .max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict()
                .optional(),
              center_law: z
                .object({
                  degree,
                  knots,
                  values: z
                    .array(z.tuple([scalar, scalar, scalar]))
                    .min(2)
                    .max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict()
                .optional(),
              frame_axis: z
                .object({
                  degree,
                  knots,
                  values: z
                    .array(z.tuple([scalar, scalar, scalar]))
                    .min(2)
                    .max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict()
                .optional(),
              frame_normal: z
                .object({
                  degree,
                  knots,
                  values: z
                    .array(z.tuple([scalar, scalar, scalar]))
                    .min(2)
                    .max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict()
                .optional(),
              normal: vector,
              orientation: z
                .enum(['rmf', 'fixed', 'fixed_normal', 'frenet', 'corrected_frenet', 'authored'])
                .default('rmf'),
              spacing: z.enum(['parameter', 'arc_length']).default('parameter'),
              initial_sections: z.number().int().min(2).max(1025).default(5),
              max_sections: z.number().int().min(2).max(1025).default(257),
              max_deviation: scalar,
              length_tolerance: scalar.default(0.001),
              length_max_cells: z.number().int().min(1).max(100000).default(100000),
              scale: z
                .object({
                  degree,
                  knots,
                  values: z.array(scalar).min(2).max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict(),
              twist: z
                .object({
                  degree,
                  knots,
                  values: z.array(scalar).min(2).max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict(),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('brep_progressive_miter_sweep'),
              circle_correction_tolerance: scalar.optional(),
              circle_correction_quantum: scalar.optional(),
              circle_correction_max_work: z.number().int().min(0).max(1000000).optional(),
              retained_wall_max_injectivity_cells: z.number().int().min(0).max(100000).optional(),
              cap_correction_tolerance: scalar.optional(),
              cap_correction_quantum: scalar.optional(),
              cap_correction_max_work: z.number().int().min(0).max(1000000).optional(),
              cap_correction_authored_frame: z.boolean().optional(),
              orientation_guide: id.optional(),
              frame_axis: z
                .object({
                  degree,
                  knots,
                  values: z
                    .array(z.tuple([scalar, scalar, scalar]))
                    .min(2)
                    .max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict()
                .optional(),
              frame_normal: z
                .object({
                  degree,
                  knots,
                  values: z
                    .array(z.tuple([scalar, scalar, scalar]))
                    .min(2)
                    .max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict()
                .optional(),
              axis_scale: z
                .object({
                  degree,
                  knots,
                  values: z
                    .array(z.tuple([scalar, scalar, scalar]))
                    .min(2)
                    .max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict()
                .optional(),
              center_law: z
                .object({
                  degree,
                  knots,
                  values: z
                    .array(z.tuple([scalar, scalar, scalar]))
                    .min(2)
                    .max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict()
                .optional(),
              closed: z.boolean().default(false),
              loops: z.array(z.array(id).min(1).max(64)).min(1).max(16),
              points: z.array(vector).min(2).max(17),
              normal: vector,
              miter_limit: scalar.default(4),
              initial_steps: z.number().int().min(1).max(1024).default(1),
              max_steps: z.number().int().min(1).max(1024).default(64),
              max_deviation: scalar,
              scale: z
                .object({
                  degree,
                  knots,
                  values: z.array(scalar).min(2).max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict(),
              twist: z
                .object({
                  degree,
                  knots,
                  values: z.array(scalar).min(2).max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict(),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('brep_miter_sweep'),
              max_deviation: scalar.optional(),
              circle_correction_tolerance: scalar.optional(),
              circle_correction_quantum: scalar.optional(),
              circle_correction_max_work: z.number().int().min(0).max(1000000).optional(),
              closed: z.boolean().default(false),
              loops: z.array(z.array(id).min(1).max(64)).min(1).max(16),
              points: z.array(vector).min(2).max(17),
              normal: vector,
              miter_limit: scalar.default(4),
              cap_correction_tolerance: scalar.optional(),
              cap_correction_quantum: scalar.optional(),
              cap_correction_max_work: z.number().int().min(0).max(1000000).optional(),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('brep_progressive_sweep'),
              circle_correction_tolerance: scalar.optional(),
              circle_correction_quantum: scalar.optional(),
              circle_correction_max_work: z.number().int().min(0).max(1000000).optional(),
              rmf_transport_steps: z
                .literal([2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384])
                .optional(),
              error_max_cells: z.number().int().min(0).max(100000).optional(),
              error_max_products: z.number().int().min(0).max(1000000).optional(),
              cap_correction_tolerance: scalar.optional(),
              cap_correction_quantum: scalar.optional(),
              cap_correction_max_work: z.number().int().min(0).max(1000000).optional(),
              contact_parameter: scalar.optional(),
              contact_profile: z.number().int().min(0).max(63).optional(),
              orientation_guide: id.optional(),
              inputs: z.tuple([id]),
              loops: z.array(z.array(id).min(1).max(64)).min(1).max(16),
              axis_scale: z
                .object({
                  degree,
                  knots,
                  values: z
                    .array(z.tuple([scalar, scalar, scalar]))
                    .min(2)
                    .max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict()
                .optional(),
              center_law: z
                .object({
                  degree,
                  knots,
                  values: z
                    .array(z.tuple([scalar, scalar, scalar]))
                    .min(2)
                    .max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict()
                .optional(),
              frame_axis: z
                .object({
                  degree,
                  knots,
                  values: z
                    .array(z.tuple([scalar, scalar, scalar]))
                    .min(2)
                    .max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict()
                .optional(),
              frame_normal: z
                .object({
                  degree,
                  knots,
                  values: z
                    .array(z.tuple([scalar, scalar, scalar]))
                    .min(2)
                    .max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict()
                .optional(),
              normal: vector,
              orientation: z
                .enum(['rmf', 'fixed', 'fixed_normal', 'frenet', 'corrected_frenet', 'authored'])
                .default('rmf'),
              spacing: z.enum(['parameter', 'arc_length']).default('parameter'),
              initial_sections: z.number().int().min(2).max(1025).default(5),
              max_sections: z.number().int().min(2).max(1025).default(257),
              max_deviation: scalar,
              length_tolerance: scalar.default(0.001),
              length_max_cells: z.number().int().min(1).max(100000).default(100000),
              scale: z
                .object({
                  degree,
                  knots,
                  values: z.array(scalar).min(2).max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict(),
              twist: z
                .object({
                  degree,
                  knots,
                  values: z.array(scalar).min(2).max(256),
                  weights: z.array(scalar).min(2).max(256),
                })
                .strict(),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('control_tangent_loft_surface'),
              inputs: z.array(id).min(2).max(11),
              parameters: z.array(scalar).min(2).max(11),
              start_tangents: z.array(vector).min(2).max(32),
              end_tangents: z.array(vector).min(2).max(32),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('auto_guided_loft_surface'),
              inputs: z.array(id).min(2).max(86),
              parameters: z.array(scalar).min(2).max(86),
              guides: z.array(id).min(1).max(84),
              budget: scalar,
              parameter_tolerance: scalar.optional(),
              start_tangents: z.array(vector).min(2).max(256).optional(),
              end_tangents: z.array(vector).min(2).max(256).optional(),
              ...loftConstruction,
              ...sectionMappings,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('loft_match_surface'),
              input: id,
              sections: z.array(id).min(2).max(11),
              parameters: z.array(scalar).min(2).max(11),
              guides: z.array(id).max(11).default([]),
              guide_parameters: z.array(scalar).max(11).default([]),
              budget: scalar,
              start_reference: id.optional(),
              end_reference: id.optional(),
              start_boundary: z.enum(['uMin', 'uMax', 'vMin', 'vMax']).default('vMax'),
              end_boundary: z.enum(['uMin', 'uMax', 'vMin', 'vMax']).default('vMin'),
              order: z.number().int().min(1).max(2).default(1),
              start_scale: scalar.default(1),
              end_scale: scalar.default(1),
              start_reverse: z.boolean().default(false),
              end_reverse: z.boolean().default(false),
            })
            .strict(),
          z.object({
            id, op: z.literal('brep_rational_loft'),
            sections: z.array(z.array(z.array(id).min(1).max(64)).min(1).max(16)).min(2).max(1025),
          }).strict(),
          z
            .object({
              id,
              op: z.literal('brep_natural_loft'),
              sections: z
                .array(z.array(z.array(id).min(1).max(64)).min(1).max(16))
                .min(2)
                .max(11),
              parameters: z.array(scalar).min(2).max(11),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('brep_capped_loft'),
              start: z.array(z.array(id).min(1).max(64)).min(1).max(16),
              end: z.array(z.array(id).min(1).max(64)).min(1).max(16),
              sides: z.array(z.array(id).min(1).max(64)).min(1).max(16),
              cap_surfaces: z.tuple([id, id]).optional(),
              cap_trims: z
                .tuple([
                  z.array(z.array(id).min(1).max(64)).min(1).max(16),
                  z.array(z.array(id).min(1).max(64)).min(1).max(16),
                ])
                .optional(),
              tolerance_uv: scalar.optional(),
              embedding_limits: z
                .object({
                  exactWork: z.number().int().min(1).max(1000000),
                  trimPairs: z.number().int().min(1).max(1000000),
                  trimCells: z.number().int().min(1).max(1000000),
                  trimDomainCells: z.number().int().min(1).max(1000000),
                  spans: z.number().int().min(1).max(1000000),
                  facePairs: z.number().int().min(1).max(1000000),
                  faceCells: z.number().int().min(1).max(1000000),
                  faceDomainCells: z.number().int().min(1).max(1000000),
                  faceCellsPerPair: z.number().int().min(1).max(1000000),
                  faceDomainCellsPerPair: z.number().int().min(1).max(1000000),
                })
                .strict()
                .optional(),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('guided_loft_surface'),
              inputs: z.array(id).min(2).max(86),
              parameters: z.array(scalar).min(2).max(86),
              guides: z.array(id).min(1).max(86),
              guide_parameters: z.array(scalar).min(1).max(86),
              start_tangents: z.array(vector).min(2).max(256).optional(),
              end_tangents: z.array(vector).min(2).max(256).optional(),
              ...loftConstruction,
              ...sectionMappings,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('clamped_loft_surface'),
              inputs: z.array(id).min(2).max(11),
              parameters: z.array(scalar).min(2).max(11),
              start_tangent: vector,
              end_tangent: vector,
            })
            .strict(),
          z.object({ id, op: z.literal('boundary_fill'), inputs: z.array(id).min(3).max(32), center: vector }).strict(),
          z.object({ id, op: z.literal('triangular_patch'), inputs: z.tuple([id, id, id]) }).strict(),
          z
            .object({
              id,
              op: z.literal('gordon_surface'),
              u_curves: z.array(id).min(2).max(11),
              v_curves: z.array(id).min(2).max(11),
              parameters_u: z.array(scalar).min(2).max(11),
              parameters_v: z.array(scalar).min(2).max(11),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('closed_loft_surface'),
              inputs: z.array(id).min(4).max(11),
              parameters: z.array(scalar).min(4).max(11),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('natural_loft_surface'),
              inputs: z.array(id).min(2).max(86),
              parameters: z.array(scalar).min(2).max(86),
              ...sectionMappings,
            })
            .strict(),
          z.object({ id, op: z.literal('surface_loft'), inputs: z.array(id).min(2).max(32) }).strict(),
          z.object({ id, op: z.literal('coons_patch'), inputs: z.tuple([id, id, id, id]) }).strict(),
          z
            .object({
              id,
              op: z.literal('triangle_mesh'),
              vertices: z.array(vector).min(3).max(6144),
              triangles: z
                .array(z.tuple([z.number().int().min(0), z.number().int().min(0), z.number().int().min(0)]))
                .min(1)
                .max(2048),
            })
            .strict(),
          z.object({ id, op: z.literal('mesh_to_nurbs_brep'), input: id }).strict(),
          z.object({ id, op: z.literal('mesh_to_sdf'), input: id }).strict(),
          z
            .object({
              id,
              op: z.literal('mesh_to_subdivision'),
              input: id,
              iterations: z.number().int().min(0).max(32).default(8),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('subdivision_tessellate'),
              input: id,
              levels: z.number().int().min(0).max(5).default(1),
            })
            .strict(),
          z.object({ id, op: z.literal('mesh_to_nurbs'), input: id }).strict(),
          z.object({ id, op: z.literal('mesh_fit_nurbs'), input: id, max_deviation: scalar }).strict(),
          z
            .object({
              id,
              op: z.literal('nurbs_patches_tessellate'),
              input: id,
              segments: z.number().int().min(1).max(16).default(2),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('subdivision'),
              vertices: z.array(vector).min(3).max(4096),
              faces: z
                .array(z.array(z.number().int().min(0).max(4095)).min(3).max(64))
                .min(1)
                .max(4096),
              levels: z.number().int().min(0).max(5).default(2),
            })
            .strict(),
          z.object({ id, op: z.literal('sdf_sphere'), center: vector, radius: scalar }).strict(),
          z.object({ id, op: z.literal('sdf_box'), center: vector, half_size: vector }).strict(),
          z
            .object({ id, op: z.literal('sdf_torus'), center: vector, major_radius: scalar, minor_radius: scalar })
            .strict(),
          z.object({ id, op: z.literal('sdf_union'), inputs: z.tuple([id, id]) }).strict(),
          z.object({ id, op: z.literal('sdf_intersection'), inputs: z.tuple([id, id]) }).strict(),
          z.object({ id, op: z.literal('sdf_difference'), inputs: z.tuple([id, id]) }).strict(),
          z.object({ id, op: z.literal('sdf_smooth_union'), inputs: z.tuple([id, id]), radius: scalar }).strict(),
          z.object({ id, op: z.literal('sdf_offset'), input: id, distance: scalar }).strict(),
          z.object({ id, op: z.literal('sdf_translate'), input: id, vector }).strict(),
          z
            .object({
              id,
              op: z.literal('sdf_tessellate'),
              input: id,
              min: vector,
              max: vector,
              cells: z.tuple([
                z.number().int().min(1).max(64),
                z.number().int().min(1).max(64),
                z.number().int().min(1).max(64),
              ]),
            })
            .strict(),
          z.object({ id, op: z.literal('brep_box'), min: vector, max: vector }).strict(),
          z.object({ id, op: z.literal('brep_sphere'), radius: scalar }).strict(),
          z.object({ id, op: z.literal('brep_torus'), major_radius: scalar, minor_radius: scalar }).strict(),
          z
            .object({
              id,
              op: z.literal('brep_gear'),
              module: scalar,
              teeth: z.number().int().min(3).max(256),
              height: scalar,
              pressure_angle: scalar.default(20),
              helix_angle: scalar.default(0),
              herringbone: z.boolean().default(false),
              bore: scalar.default(0),
              internal: z.boolean().default(false),
              rim_width: scalar.default(2),
              clearance: scalar.default(0.25),
              backlash: scalar.default(0),
              verified_profile: z.boolean().optional(),
              inspect_spans: z.boolean().optional(),
              span_count: z.number().int().min(1).max(256).optional(),
              measured_span_mm: z.tuple([scalar, scalar]).optional(),
            })
            .strict(),
          z.object({id,op:z.literal('radial_cam_profile'),center:z.tuple([scalar,scalar,scalar]),base_radius:scalar,roller_radius:scalar,phase:scalar,end_angles:z.array(scalar).min(1).max(64),end_lifts:z.array(scalar).min(1).max(64),laws:z.array(z.enum(['dwell','harmonic','cycloidal','polynomial345','polynomial4567'])).min(1).max(64),tolerance_mm:scalar,max_spans:z.number().int().min(1).max(4096),piece:z.number().int().nonnegative()}).strict(),
          z.object({id,op:z.literal('radial_cam_walls'),center:z.tuple([scalar,scalar,scalar]),base_radius:scalar,roller_radius:scalar,phase:scalar,end_angles:z.array(scalar).min(1).max(64),end_lifts:z.array(scalar).min(1).max(64),laws:z.array(z.enum(['dwell','harmonic','cycloidal','polynomial345','polynomial4567'])).min(1).max(64),tolerance_mm:scalar,max_spans:z.number().int().min(1).max(4096),vector:z.tuple([scalar,scalar,scalar])}).strict(),
          z.object({id,op:z.literal('rack_profile'),teeth:z.number().int().positive(),module:scalar,pressure_angle:scalar,backlash:scalar,addendum:scalar,dedendum:scalar,backing_height:scalar}).strict(),
          z.object({id,op:z.literal('brep_rack'),teeth:z.number().int().positive(),module:scalar,pressure_angle:scalar,backlash:scalar,addendum:scalar,dedendum:scalar,backing_height:scalar,z_min:scalar,z_max:scalar}).strict(),
          z.object({id,op:z.literal('finite_za_worm'),module:scalar,diameter_quotient:scalar,pressure_angle:scalar,dedendum_factor:scalar,backlash:scalar,starts:z.number().int().positive(),turns:z.number().int().positive(),left_handed:z.boolean(),tolerance_mm:scalar,z_min:scalar,z_max:scalar}).strict(),
          z.object({id,op:z.literal('finite_za_wheel'),module:scalar,diameter_quotient:scalar,pressure_angle:scalar,dedendum_factor:scalar,backlash:scalar,starts:z.number().int().positive(),turns:z.number().int().positive(),left_handed:z.boolean(),tolerance_mm:scalar,teeth:z.number().int().positive(),start_index:z.number().int().nonnegative(),axial_shift:scalar,y_breaks:z.array(scalar).min(2).max(257),stock_radius:scalar}).strict(),
          z.object({id,op:z.literal('finite_external_thread'),diameter:scalar,pitch:scalar,clearance:scalar,starts:z.number().int().positive(),turns:z.number().int().positive(),tolerance_mm:scalar,left_handed:z.boolean(),z_min:scalar,z_max:scalar}).strict(),
          z.object({id,op:z.literal('finite_internal_thread'),diameter:scalar,pitch:scalar,clearance:scalar,starts:z.number().int().positive(),turns:z.number().int().positive(),tolerance_mm:scalar,left_handed:z.boolean(),z_min:scalar,z_max:scalar,outer_radius:scalar}).strict(),
          z.object({id,op:z.literal('verified_bevel_gear'),teeth:z.number().int().positive(),module:scalar,pitch_cone_angle:scalar,pressure_angle:scalar,face_width:scalar,addendum:scalar,dedendum:scalar,backlash:scalar,tolerance_mm:scalar}).strict(),
          z.object({ id, op: z.literal('brep_cylinder'), radius: scalar, height: scalar }).strict(),
          z
            .object({ id, op: z.literal('brep_frustum'), bottom_radius: scalar, top_radius: scalar, height: scalar })
            .strict(),
          z
            .object({ id, op: z.literal('brep_tube'), outer_radius: scalar, inner_radius: scalar, height: scalar })
            .strict(),
          z.object({ id, op: z.literal('brep_revolve'), input: id, angle: scalar.default(360) }).strict(),
          z.object({ id, op: z.literal('brep_extrude'), input: id, height: scalar }).strict(),
          z
            .object({
              id,
              op: z.literal('brep_extrude_curves'),
              loops: z.array(z.array(id).min(1).max(254)).max(64),
              z_min: scalar,
              z_max: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('brep_boolean'),
              inputs: z.tuple([id, id]),
              operation: z.enum(['union', 'intersection', 'difference', 'xor']),
            })
            .strict(),
          z.object({ id, op: z.literal('brep_chamfer'), input: id, edges: brepEdgeSelection, size: scalar }).strict(),
          z
            .object({
              id,
              op: z.literal('brep_fillet'),
              input: id,
              edges: brepEdgeSelection,
              radius: scalar,
              segments: z.number().int().min(2).max(32).default(12),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('brep_full_round_exact'),
              input: id,
              faces: z.tuple([
                z.number().int().min(0).max(0xffffffff),
                z.number().int().min(0).max(0xffffffff),
                z.number().int().min(0).max(0xffffffff),
              ]),
              segments: z.number().int().min(1).max(32).default(4),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('brep_tessellate'),
              input: id,
              segments: z.number().int().min(1).max(32).default(4),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('brep_smooth_sweep_stations'),
              input: id,
              wall_tolerance: scalar,
              quantum: scalar.default(2 ** -40),
              max_work: z.number().int().min(0).max(1000000).default(1000000),
              smoothness_max_work: z.number().int().min(0).max(2000000).default(2000000),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('brep_smooth_miter_stations'),
              input: id,
              wall_tolerance: scalar,
              quantum: scalar.default(2 ** -40),
              max_work: z.number().int().min(0).max(1000000).default(1000000),
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('curve'),
              degree,
              knots,
              control_points: z
                .array(z.union([uv, vector]))
                .min(2)
                .max(256),
              weights: z.array(scalar).min(2).max(256),
              periodic: z.boolean().default(false),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('surface'),
              degree_u: degree,
              degree_v: degree,
              knots_u: knots,
              knots_v: knots,
              control_points: z.array(z.array(vector).min(2).max(32)).min(2).max(32),
              weights: z.array(z.array(scalar).min(2).max(32)).min(2).max(32),
              periodic_u: z.boolean().default(false),
              periodic_v: z.boolean().default(false),
            })
            .strict(),
          z.object({ id, op: z.literal('curve_remove_knot'), input: id, parameter: scalar, max_deviation: scalar }).strict(),
          z.object({ id, op: z.literal('curve_reduce_degree'), input: id, degree, max_deviation: scalar }).strict(),
          z.object({ id, op: z.literal('curve_rebuild'), input: id, degree, control_count: z.number().int().positive(), max_deviation: scalar }).strict(),
          z.object({ id, op: z.literal('surface_remove_knot'), input: id, direction: z.enum(['u', 'v']), parameter: scalar, max_deviation: scalar }).strict(),
          z.object({ id, op: z.literal('surface_reduce_degree'), input: id, direction: z.enum(['u', 'v']), degree, max_deviation: scalar }).strict(),
          z.object({ id, op: z.literal('surface_rebuild'), input: id, direction: z.enum(['u', 'v']), degree, control_count: z.number().int().positive(), max_deviation: scalar }).strict(),
          z.object({ id, op: z.literal('surface_split'), input: id, direction: z.enum(['u', 'v']), parameter: scalar, part: z.literal([0, 1]) }).strict(),
          z.object({ id, op: z.literal('surface_bezier_patch'), input: id, index: z.number().int().nonnegative() }).strict(),
          z.object({ id, op: z.literal('surface_transpose'), input: id }).strict(),
          z.object({ id, op: z.literal('curve_edit_weights'), input: id, edits: z.array(z.tuple([z.number().int().nonnegative(), scalar])).min(1).max(256) }).strict(),
          z.object({ id, op: z.literal('curve_relocate_seam'), input: id, parameter: scalar, max_deviation: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('surface_edit_weights'), input: id, edits: z.array(z.tuple([z.number().int().nonnegative(), z.number().int().nonnegative(), scalar])).min(1).max(1024) }).strict(),
          z.object({ id, op: z.literal('surface_relocate_seam'), input: id, direction: z.enum(['u','v']), parameter: scalar, max_deviation: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_exact_insert_knot'), input: id, parameter: scalar, count: z.number().int().positive(), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('curve_exact_remove_knot'), input: id, parameter: scalar, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('curve_exact_reduce_degree'), input: id, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('surface_exact_insert_knot'), input: id, direction: z.enum(['u','v']), parameter: scalar, count: z.number().int().positive(), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('surface_exact_remove_knot'), input: id, direction: z.enum(['u','v']), parameter: scalar, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('surface_exact_reduce_degree'), input: id, direction: z.enum(['u','v']), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('curve_normalize_weights'), input: id }).strict(),
          z.object({ id, op: z.literal('curve_cancel_factor'), input: id, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('surface_cancel_factor'), input: id, direction: z.enum(['u','v']), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('curve_exact_split'), input: id, parameter: scalar, part: z.literal([0, 1]), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('curve_exact_restrict'), input: id, domain: interval, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('curve_exact_partition'), input: id, cuts: z.array(scalar).max(256), part: z.number().int().nonnegative(), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('curve_exact_elevate'), input: id, degree, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('assemble_exact_periodic_surface_axis'), inputs: z.array(id).min(2).max(256), direction: z.enum(['u', 'v']), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('assemble_periodic_surface_axis_with_exact_weight_scaling'), inputs: z.array(id).min(2).max(256), direction: z.enum(['u', 'v']), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('assemble_periodic_surface_axis_with_exact_knot_union'), inputs: z.array(id).min(2).max(256), direction: z.enum(['u', 'v']), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('assemble_periodic_surface_axis_with_exact_basis_matching'), inputs: z.array(id).min(2).max(256), direction: z.enum(['u', 'v']), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('assemble_periodic_surface_axis_with_exact_degree_matching'), inputs: z.array(id).min(2).max(256), direction: z.enum(['u', 'v']), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('assemble_exact_doubly_periodic_surface'), inputs: z.array(id).min(1).max(256), columns: z.number().int().min(1).max(256), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('assemble_doubly_periodic_surface_with_exact_weight_scaling'), inputs: z.array(id).min(1).max(256), columns: z.number().int().min(1).max(256), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('rotate_periodic_surface_at_exact_knot'), input: id, direction: z.enum(['u', 'v']), parameter: scalar, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('insert_and_rotate_periodic_surface_seam'), input: id, direction: z.enum(['u', 'v']), parameter: scalar, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('open_exact_periodic_surface_axis'), input: id, direction: z.enum(['u', 'v']), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('restrict_exact_periodic_surface_axis'), input: id, direction: z.enum(['u', 'v']), interval: interval, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('restrict_exact_periodic_surface_over_turns'), input: id, direction: z.enum(['u', 'v']), interval: interval, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('restrict_exact_doubly_periodic_surface'), input: id, u: interval, v: interval, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('restrict_exact_doubly_periodic_surface_over_turns'), input: id, u: interval, v: interval, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('partition_exact_periodic_surface_axis'), input: id, direction: z.enum(['u', 'v']), seam: scalar, cuts: z.array(scalar).max(256), max_work: z.number().int().min(1).max(1000000), part: z.number().int().nonnegative() }).strict(),
          z.object({ id, op: z.literal('partition_exact_periodic_surface_over_turns'), input: id, direction: z.enum(['u', 'v']), interval: interval, cuts: z.array(scalar).max(256), max_work: z.number().int().min(1).max(1000000), part: z.number().int().nonnegative() }).strict(),
          z.object({ id, op: z.literal('partition_exact_doubly_periodic_surface'), input: id, seams: interval, u_cuts: z.array(scalar).max(256), v_cuts: z.array(scalar).max(256), max_work: z.number().int().min(1).max(1000000), u_index: z.number().int().nonnegative(), v_index: z.number().int().nonnegative() }).strict(),
          z.object({ id, op: z.literal('partition_exact_doubly_periodic_surface_over_turns'), input: id, u: interval, v: interval, u_cuts: z.array(scalar).max(256), v_cuts: z.array(scalar).max(256), max_work: z.number().int().min(1).max(1000000), u_index: z.number().int().nonnegative(), v_index: z.number().int().nonnegative() }).strict(),
          z.object({ id, op: z.literal('remove_exact_periodic_surface_knot'), input: id, direction: z.enum(['u', 'v']), knot: scalar, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('simplify_exact_periodic_surface_axis'), input: id, direction: z.enum(['u', 'v']), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('remove_exact_periodic_surface_seam_knot'), input: id, direction: z.enum(['u', 'v']), anchor: scalar, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('simplify_exact_doubly_periodic_surface'), input: id, max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('reduce_exact_periodic_surface_period'), input: id, direction: z.enum(['u', 'v']), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('repeat_exact_periodic_surface_period'), input: id, direction: z.enum(['u', 'v']), turns: z.number().int().positive(), max_work: z.number().int().min(1).max(1000000) }).strict(),
          z.object({ id, op: z.literal('curve_axis_motion_partition_speed'), input: id, interval, axis_vector: z.array(scalar).min(2).max(3), speed_limit: scalar, parameter_tolerance: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_axis_motion_partition_motion'), input: id, interval, directions: z.array(z.array(scalar).min(2).max(3)).min(1).max(1), limits: z.array(z.tuple([scalar,scalar,scalar])).min(1).max(64), parameter_tolerance: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_axis_motion_partition_axes'), input: id, interval, directions: z.array(z.array(scalar).min(2).max(3)).min(1).max(64), limits: z.array(z.tuple([scalar,scalar,scalar])).min(1).max(64), parameter_tolerance: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_axis_motion_limits'), input: id, interval, directions: z.array(z.array(scalar).min(2).max(3)).min(1).max(1), limits: z.array(z.tuple([scalar,scalar,scalar])).min(1).max(64), tolerances: z.tuple([scalar,scalar,scalar]), max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_axis_motion_slowdown'), input: id, interval, directions: z.array(z.array(scalar).min(2).max(3)).min(1).max(1), limits: z.array(z.tuple([scalar,scalar,scalar])).min(1).max(64), tolerances: z.tuple([scalar,scalar,scalar]), max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_axis_motion_multi_slowdown'), input: id, interval, directions: z.array(z.array(scalar).min(2).max(3)).min(1).max(64), limits: z.array(z.tuple([scalar,scalar,scalar])).min(1).max(64), tolerances: z.tuple([scalar,scalar,scalar]), max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_axis_motion_duration'), input: id, interval, directions: z.array(z.array(scalar).min(2).max(3)).min(1).max(64), limits: z.array(z.tuple([scalar,scalar,scalar])).min(1).max(64), tolerances: z.tuple([scalar,scalar,scalar]), max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_axis_motion_deadline'), input: id, interval, directions: z.array(z.array(scalar).min(2).max(3)).min(1).max(64), limits: z.array(z.tuple([scalar,scalar,scalar])).min(1).max(64), tolerances: z.tuple([scalar,scalar,scalar]), max_cells: z.number().int().min(1).max(100000), deadline: scalar }).strict(),
          z.object({ id, op: z.literal('curve_directional_travel_full'), input: id, axis_vector:z.array(scalar).min(2).max(3), tolerance_mm:scalar, max_cells:z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_directional_travel_signed'), input: id, axis_vector:z.array(scalar).min(2).max(3), tolerance_mm:scalar, max_cells:z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_check_backtracking'), input: id, axis_vector:z.array(scalar).min(2).max(3), tolerance_mm:scalar, max_cells:z.number().int().min(1).max(100000), interval, reverse_limit:scalar }).strict(),
          z.object({ id, op:z.literal('surface_trim_contour_audit'), input:id, contours:z.array(z.array(uv).min(3)).min(1), max_work:z.number().int().positive(), contour:z.number().int().nonnegative() }).strict(),
          z.object({ id, op:z.literal('surface_trim_contour_station'), input:id, contours:z.array(z.array(uv).min(3)).min(1), max_work:z.number().int().positive(), contour:z.number().int().nonnegative(), distance_mm:scalar, tolerance_mm:scalar, parameter_tolerance:scalar, max_cells:z.number().int().positive() }).strict(),
          z.object({ id, op:z.literal('surface_trim_contour_station_box'), input:id, contours:z.array(z.array(uv).min(3)).min(1), max_work:z.number().int().positive(), contour:z.number().int().nonnegative(), distance_mm:scalar, tolerance_mm:scalar, parameter_tolerance:scalar, max_cells:z.number().int().positive() }).strict(),
          z.object({ id, op:z.literal('surface_trim_contour_divide'), input:id, contours:z.array(z.array(uv).min(3)).min(1), max_work:z.number().int().positive(), contour:z.number().int().nonnegative(), divisions:z.number().int().positive(), tolerance_mm:scalar, parameter_tolerance:scalar, max_cells:z.number().int().positive() }).strict(),
          z.object({ id, op:z.literal('surface_trim_patch_area_vector'), input:id, contours:z.array(z.array(uv).min(3)).min(1), max_work:z.number().int().positive(), outer_contour:z.number().int().nonnegative(), tolerance_mm2:scalar, max_cells:z.number().int().positive() }).strict(),
          z.object({ id, op:z.literal('surface_trim_patch_projected_area'), input:id, contours:z.array(z.array(uv).min(3)).min(1), max_work:z.number().int().positive(), outer_contour:z.number().int().nonnegative(), normal:vector, tolerance_mm2:scalar, max_cells:z.number().int().positive() }).strict(),
          z.object({
            id,op:z.literal('surface_trim_range_length'),input:id,contours:z.array(z.array(uv).min(3)).min(1),
            contour:z.number().int().nonnegative(),segment:z.number().int().nonnegative(),range_start:exactUnsignedRatio,
            range_end:exactUnsignedRatio,tolerance_mm:scalar,max_work:z.number().int().positive(),max_cells:z.number().int().min(1).max(100000),
          }).strict(),
          z.object({id,op:z.literal('evaluate_exact_wrapped_surface_point'),input:id,uv,max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('evaluate_exact_wrapped_surface_points'),input:id,uv_points:z.array(uv).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('lift_exact_periodic_surface_path'),input:id,uv_points:z.array(uv).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('exact_periodic_surface_path_winding'),input:id,uv_points:z.array(uv).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('exact_periodic_surface_path_signed_area'),input:id,uv_points:z.array(uv).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('exact_periodic_surface_path_centroid'),input:id,uv_points:z.array(uv).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('exact_periodic_surface_path_inertia'),input:id,uv_points:z.array(uv).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('classify_exact_periodic_surface_path_point'),input:id,uv_points:z.array(uv).min(1), query_uv:uv,max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('classify_exact_periodic_surface_path_points'),input:id,uv_points:z.array(uv).min(1), query_points:z.array(uv).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('classify_exact_periodic_surface_region_points'),input:id,contours:z.array(z.array(uv).min(1)).min(1), query_points:z.array(uv).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('certify_exact_periodic_surface_path_simplicity'),input:id,uv_points:z.array(uv).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('check_exact_periodic_surface_path_contact'),input:id,first_path:z.array(uv).min(1), second_path:z.array(uv).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('relate_exact_periodic_surface_paths'),input:id,first_path:z.array(uv).min(1), second_path:z.array(uv).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('exact_periodic_surface_contour_forest'),input:id,contours:z.array(z.array(uv).min(1)).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('exact_periodic_surface_region_area'),input:id,contours:z.array(z.array(uv).min(1)).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('exact_periodic_surface_region_component_centroids'),input:id,contours:z.array(z.array(uv).min(1)).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('exact_periodic_surface_region_component_inertia'),input:id,contours:z.array(z.array(uv).min(1)).min(1),max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('measure_input_contour_length'),input:id,contours:z.array(z.array(uv).min(3)).min(1), contour:z.number().int().nonnegative(), max_work:z.number().int().positive(), tolerance_mm:scalar, max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('measure_input_segment_length'),input:id,contours:z.array(z.array(uv).min(3)).min(1), contour:z.number().int().nonnegative(), max_work:z.number().int().positive(), segment:z.number().int().nonnegative(), tolerance_mm:scalar, max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('measure_input_segment_range_length'),input:id,contours:z.array(z.array(uv).min(3)).min(1), contour:z.number().int().nonnegative(), max_work:z.number().int().positive(), segment:z.number().int().nonnegative(), range:interval, tolerance_mm:scalar, max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('locate_input_segment_at_length'),input:id,contours:z.array(z.array(uv).min(3)).min(1), contour:z.number().int().nonnegative(), max_work:z.number().int().positive(), segment:z.number().int().nonnegative(), distance_mm:scalar, tolerance_mm:scalar, parameter_tolerance:scalar, max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('measure_input_contour_prefix_lengths'),input:id,contours:z.array(z.array(uv).min(3)).min(1), contour:z.number().int().nonnegative(), max_work:z.number().int().positive(), tolerance_mm:scalar, max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('locate_input_contour_at_lengths'),input:id,contours:z.array(z.array(uv).min(3)).min(1), contour:z.number().int().nonnegative(), max_work:z.number().int().positive(), distances_mm:z.array(scalar).min(1), tolerance_mm:scalar, parameter_tolerance:scalar, max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('divide_input_contour_by_length'),input:id,contours:z.array(z.array(uv).min(3)).min(1), contour:z.number().int().nonnegative(), max_work:z.number().int().positive(), divisions:z.number().int().positive(), tolerance_mm:scalar, parameter_tolerance:scalar, max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('exact_input_contour_projected_area'),input:id,contours:z.array(z.array(uv).min(3)).min(1), contour:z.number().int().nonnegative(), max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('exact_input_contour_polynomial_projected_area'),input:id,contours:z.array(z.array(uv).min(3)).min(1), contour:z.number().int().nonnegative(), max_work:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('bound_input_contour_projected_area'),input:id,contours:z.array(z.array(uv).min(3)).min(1), contour:z.number().int().nonnegative(), max_work:z.number().int().positive(), tolerance_mm2:scalar, max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('bound_input_patch_max_projected_area'),input:id,contours:z.array(z.array(uv).min(3)).min(1), contour:z.number().int().nonnegative(), max_work:z.number().int().positive(), tolerance_mm2:scalar, max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('bound_input_patch_max_projection_direction'),input:id,contours:z.array(z.array(uv).min(3)).min(1), contour:z.number().int().nonnegative(), max_work:z.number().int().positive(), tolerance_mm2:scalar, max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('surface_trim_sheet_step'),input:id,contours:z.array(z.array(uv).min(3)).min(1),tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_trim_sheet_display'),input:id,segments:z.number().int().min(1).max(32),contours:z.array(z.array(uv).min(3)).min(1),tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_trim_sheet'),input:id,contours:z.array(z.array(uv).min(3)).min(1),tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_trim_sheet_edge_bindings'),input:id,contours:z.array(z.array(uv).min(3)).min(1),tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_trim_sheet_input_bindings'),input:id,contours:z.array(z.array(uv).min(3)).min(1),tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('locate_input_parameter'),input:id,contours:z.array(z.array(uv).min(3)).min(1),contour:z.number().int().nonnegative(),segment:z.number().int().nonnegative(),exact_parameter:exactUnsignedRatio,tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('locate_edge_parameter'),input:id,contours:z.array(z.array(uv).min(3)).min(1),edge_id:z.string().min(1).max(100),exact_parameter:exactUnsignedRatio,tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('locate_input_range'),input:id,contours:z.array(z.array(uv).min(3)).min(1),contour:z.number().int().nonnegative(),segment:z.number().int().nonnegative(),range_start:exactUnsignedRatio,range_end:exactUnsignedRatio,tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('extract_input_range_curves'),input:id,contours:z.array(z.array(uv).min(3)).min(1),contour:z.number().int().nonnegative(),segment:z.number().int().nonnegative(),range_start:exactUnsignedRatio,range_end:exactUnsignedRatio,tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000),piece:z.number().int().nonnegative().optional()}).strict(),
          z.object({id,op:z.literal('extract_input_range_curve_pairs'),input:id,contours:z.array(z.array(uv).min(3)).min(1),contour:z.number().int().nonnegative(),segment:z.number().int().nonnegative(),range_start:exactUnsignedRatio,range_end:exactUnsignedRatio,tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000),piece:z.number().int().nonnegative().optional(),use_pcurve:z.boolean().optional()}).strict(),
          z.object({id,op:z.literal('evaluate_input_point'),input:id,contours:z.array(z.array(uv).min(3)).min(1),contour:z.number().int().nonnegative(), segment:z.number().int().nonnegative(), exact_parameter:exactUnsignedRatio,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('evaluate_input_tangents'),input:id,contours:z.array(z.array(uv).min(3)).min(1),contour:z.number().int().nonnegative(), segment:z.number().int().nonnegative(), exact_parameter:exactUnsignedRatio,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('classify_input_tangent_join'),input:id,contours:z.array(z.array(uv).min(3)).min(1),contour:z.number().int().nonnegative(), segment:z.number().int().nonnegative(), exact_parameter:exactUnsignedRatio,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('classify_input_vertex_join'),input:id,contours:z.array(z.array(uv).min(3)).min(1),contour:z.number().int().nonnegative(), vertex:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('lift_exact_periodic_surface_region_components'),input:id,contours:z.array(z.array(uv).min(3)).min(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('build_exact_periodic_surface_trim_components'),input:id,contours:z.array(z.array(uv).min(3)).min(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('build_exact_periodic_surface_trim_patches'),input:id,contours:z.array(z.array(uv).min(3)).min(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('build_exact_periodic_surface_trim_edges'),input:id,contours:z.array(z.array(uv).min(3)).min(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('trimmed_surface_area_in_window'),input:id,loops:z.array(z.array(id).min(1)).min(1).optional(),contours:z.array(z.array(uv).min(3)).min(1).optional(),outer_contour:z.number().int().nonnegative().optional(),max_work:z.number().int().min(1).max(1000000).optional(),tolerance_uv:scalar,tolerance_mm2:scalar,max_surface_cells:z.number().int().min(1).max(100000),max_trim_cells:z.number().int().min(1).max(1000000),window:z.tuple([interval,interval])}).strict(),
          z.object({id,op:z.literal('trimmed_surface_centroid'),input:id,loops:z.array(z.array(id).min(1)).min(1).optional(),contours:z.array(z.array(uv).min(3)).min(1).optional(),outer_contour:z.number().int().nonnegative().optional(),max_work:z.number().int().min(1).max(1000000).optional(),tolerance_uv:scalar,tolerance_mm2:scalar,max_surface_cells:z.number().int().min(1).max(100000),max_trim_cells:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('trimmed_surface_area_inertia'),input:id,loops:z.array(z.array(id).min(1)).min(1).optional(),contours:z.array(z.array(uv).min(3)).min(1).optional(),outer_contour:z.number().int().nonnegative().optional(),max_work:z.number().int().min(1).max(1000000).optional(),tolerance_uv:scalar,tolerance_mm2:scalar,max_surface_cells:z.number().int().min(1).max(100000),max_trim_cells:z.number().int().min(1).max(1000000),origin:z.tuple([scalar,scalar,scalar])}).strict(),
          z.object({
            id, op: z.literal('surface_trim_area'), input:id,contours:z.array(z.array(z.tuple([scalar,scalar])).min(3)).min(1),
            tolerance_mm2:scalar, tolerance_uv:scalar, max_work:z.number().int().positive(),
            max_surface_cells:z.number().int().min(1).max(100000), max_trim_cells:z.number().int().min(1).max(1000000),
            outer_contour:z.number().int().nonnegative(),
          }).strict(),
          z.object({
            id, op: z.literal('surface_trim_mass_properties'), input:id,contours:z.array(z.array(z.tuple([scalar,scalar])).min(3)).min(1),
            tolerance_mm2:scalar, tolerance_uv:scalar, max_work:z.number().int().positive(),
            max_surface_cells:z.number().int().min(1).max(100000), max_trim_cells:z.number().int().min(1).max(1000000),
            outer_contour:z.number().int().nonnegative(), moment_tolerance:scalar,
          }).strict(),
          z.object({
            id, op: z.literal('surface_trim_window_properties'), input:id,contours:z.array(z.array(z.tuple([scalar,scalar])).min(3)).min(1),
            tolerance_mm2:scalar, tolerance_uv:scalar, max_work:z.number().int().positive(),
            max_surface_cells:z.number().int().min(1).max(100000), max_trim_cells:z.number().int().min(1).max(1000000),
            outer_contour:z.number().int().nonnegative(), moment_tolerance:scalar, uv_window:z.tuple([interval,interval]),
          }).strict(),
          z.object({
            id, op: z.literal('surface_trim_set_properties'), input:id,contours:z.array(z.array(z.tuple([scalar,scalar])).min(3)).min(1),
            tolerance_mm2:scalar, tolerance_uv:scalar, max_work:z.number().int().positive(),
            max_surface_cells:z.number().int().min(1).max(100000), max_trim_cells:z.number().int().min(1).max(1000000),
            moment_tolerance:scalar.optional(), axis_point:z.tuple([scalar,scalar,scalar]).optional(), axis_vector:z.tuple([scalar,scalar,scalar]).optional(), reference_point:z.tuple([scalar,scalar,scalar]).optional(), radius_of_gyration:z.boolean().optional(),
          }).strict(),
          z.object({
            id, op: z.literal('surface_measure_trimmed_mass'), input: id,
            contours: z.array(z.array(z.tuple([scalar,scalar])).min(3)).min(1),
            component_densities:z.array(z.tuple([z.number().int().nonnegative(), scalar, scalar])).min(1),
            tolerance_mm2:scalar, tolerance_uv:scalar, max_work:z.number().int().positive(),
            max_surface_cells:z.number().int().min(1).max(100000), max_trim_cells:z.number().int().min(1).max(1000000),
            plane_point:z.tuple([scalar,scalar,scalar]).optional(), plane_normal:z.tuple([scalar,scalar,scalar]).optional(),
            axis_point:z.tuple([scalar,scalar,scalar]).optional(), axis_vector:z.tuple([scalar,scalar,scalar]).optional(),
            reference_point:z.tuple([scalar,scalar,scalar]).optional(), moment_tolerance:scalar.optional(), radius_of_gyration:z.boolean().optional(),
            principal_axes:z.boolean().optional(), probe_point:z.tuple([scalar,scalar,scalar]).optional(), probe_direction:z.tuple([scalar,scalar,scalar]).optional(),
            balance_tolerance:scalar.optional(), static_unbalance:z.boolean().optional(), couple_unbalance:z.boolean().optional(),
            plane_offsets:z.tuple([scalar,scalar]).optional(),plane_tolerances:z.tuple([scalar,scalar]).optional(),
            weight_radii:z.tuple([scalar,scalar]).optional(),weight_mass_limits:z.tuple([scalar,scalar]).optional(),
            installed_weights:z.array(z.array(z.tuple([scalar,scalar])).length(4)).length(2).optional(),
          }).strict(),
          z.object({ id, op: z.literal('curve_axial_derivative_peak'), input: id, axis_vector: z.array(scalar).min(2).max(3), interval, order: z.number().int().min(1).max(3), tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000), parameter_tolerance: scalar.optional() }).strict(),
          z.object({id,op:z.literal('surface_highlight_lines'),input:id,lights:z.array(z.tuple([scalar,scalar,scalar])).min(1).max(64),view_direction:z.tuple([scalar,scalar,scalar]),level:scalar,grid:z.tuple([z.number().int().min(2).max(256),z.number().int().min(2).max(256)]),kink_angle_rad:scalar}).strict(),
          z.object({id,op:z.literal('surface_reflection_lines'),input:id,light:z.tuple([scalar,scalar,scalar]),view_direction:z.tuple([scalar,scalar,scalar]),level:scalar,grid:z.tuple([z.number().int().min(2).max(256),z.number().int().min(2).max(256)]),jump_threshold:scalar}).strict(),
          z.object({id,op:z.literal('surface_isophote_analysis'),input:id,light:z.tuple([scalar,scalar,scalar]),cos_levels:z.array(scalar).min(2).max(64),grid:z.tuple([z.number().int().min(2).max(256),z.number().int().min(2).max(256)]),uneven_tolerance:scalar}).strict(),
          z.object({id,op:z.literal('surface_dent_detection'),input:id,grid:z.tuple([z.number().int().min(2).max(256),z.number().int().min(2).max(256)]),sigma_cells:scalar,residual_per_mm2:scalar}).strict(),
          z.object({id,op:z.literal('fit_biarc_piece'),input:id,tolerance_mm:scalar,max_arcs:z.number().int().min(1).max(256),piece:z.number().int().nonnegative()}).strict(),
          z.object({id,op:z.literal('curve_from_curvature_samples'),stations_mm:z.array(scalar).min(3).max(4096),curvatures_per_mm:z.array(scalar).min(3).max(4096),origin:z.tuple([scalar,scalar]),start_angle_rad:scalar,tolerance_mm:scalar}).strict(),
          z.object({id,op:z.literal('simplify_linear_curve'),input:id,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('extend_terminal_curve'),input:id,source_end:z.enum(['start','end']),distance_mm:scalar,continuity_class:z.enum(['G1','G2','C2'])}).strict(),
          z.object({id,op:z.literal('extend_surface_boundary'),input:id,source_edge:z.enum(['u_min','u_max','v_min','v_max']),distance_mm:scalar,continuity_class:z.enum(['G1','G2','C2'])}).strict(),
          z.object({id,op:z.literal('affine_control_support'),input:id,max_work:z.number().int().min(0).max(Number.MAX_SAFE_INTEGER)}).strict(),
          z.object({id,op:z.literal('surface_trace_agreement'),input:id,inputs:z.tuple([id,id]),reverse:z.boolean(),tolerance_mm:scalar,max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('signed_shell_volume'),input:id,inputs:z.array(id).max(2047),reversed_faces:z.array(z.number().int().nonnegative()).max(2048),flux_axis:z.tuple([scalar,scalar,scalar]).optional(),tolerance_mm3:scalar,max_cells:z.number().int().min(1).max(200000)}).strict(),
          z.object({id,op:z.literal('geometry_bounds_union'),input:id,inputs:z.array(id).max(2047)}).strict(),
          z.object({id,op:z.literal('geometry_conditioning'),input:id}).strict(),
          z.object({id,op:z.literal('coordinate_frame_local'),input:id,origin:z.array(scalar).min(2).max(3),frame_scale:scalar}).strict(),
          z.object({id,op:z.literal('coordinate_frame_world'),input:id,origin:z.array(scalar).min(2).max(3),frame_scale:scalar}).strict(),
          z.object({id,op:z.literal('shared_patch_mesh'),input:id,inputs:z.array(id).max(15),seams:z.array(z.tuple([z.number().int().min(0).max(15),z.number().int().min(0).max(3),z.number().int().min(0).max(15),z.number().int().min(0).max(3),z.number().int().min(0).max(1)])).max(64),segments:z.number().int().min(1).max(128),tolerance_mm:scalar,max_cells:z.number().int().min(1).max(100000),max_agreement_cells:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_curve_chord'),input:id,tolerance_mm:scalar,max_cells:z.number().int().min(1).max(100000),piece:z.number().int().nonnegative()}).strict(),
          z.object({id,op:z.literal('bounded_surface_mesh'),input:id,tolerance_mm:scalar,max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('sample_plane_branch'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),transverse_ratios:z.array(z.tuple([z.number().int().nonnegative(),z.number().int().positive()])).min(1).max(1024),parameter_width:z.number().positive().max(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('plane_branch_uv_polyline'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),tolerance_uv:z.number().positive().max(1),max_segments:z.number().int().min(1).max(1024),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('store_plane_branch'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),tolerance_mm:z.number().positive(),max_segments:z.number().int().min(1).max(255),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('split_cell_by_plane'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),tolerance_mm:z.number().positive(),max_segments:z.number().int().min(1).max(255),gap_tolerance_mm:z.number().nonnegative(),positive_side:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('pair_plane_branch'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),tolerance_mm:z.number().positive(),max_segments:z.number().int().min(1).max(255),gap_tolerance_mm:z.number().nonnegative(),use_pcurve:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('stitch_plane_strip'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,solved_span:z.number().int().nonnegative(),parameter_width:z.number().positive().max(1),transverse_spans:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('close_plane_strip'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,solved_span:z.number().int().nonnegative(),parameter_width:z.number().positive().max(1),transverse_spans:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),tolerance_mm:z.number().positive(),max_segments_per_cell:z.number().int().min(1).max(1024),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('split_periodic_plane_strip'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,solved_span:z.number().int().nonnegative(),parameter_width:z.number().positive().max(1),transverse_spans:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),tolerance_mm:z.number().positive(),max_segments_per_cell:z.number().int().min(1).max(255),gap_tolerance_mm:z.number().nonnegative(),positive_side:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('store_closed_plane_piece'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,solved_span:z.number().int().nonnegative(),parameter_width:z.number().positive().max(1),transverse_spans:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),tolerance_mm:z.number().positive(),max_segments_per_cell:z.number().int().min(1).max(255),gap_tolerance_mm:z.number().nonnegative(),piece:z.number().int().nonnegative(),use_pcurve:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('trace_plane_strip'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,solved_span:z.number().int().nonnegative(),parameter_width:z.number().positive().max(1),transverse_spans:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),tolerance_mm:z.number().positive(),max_segments_per_cell:z.number().int().min(1).max(1024),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('plane_branch_xyz_polyline'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),tolerance_mm:z.number().positive(),max_segments:z.number().int().min(1).max(1024),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('audit_plane_side'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('refine_plane_side'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,max_depth:z.number().int().min(0).max(32),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('audit_plane_graphs'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('parallel_plane_sections'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offsets:z.array(scalar).min(1).max(1024),tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('parallel_section_piece'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offsets:z.array(scalar).min(1).max(1024),tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000),section_index:z.number().int().nonnegative(),piece:z.number().int().nonnegative()}).strict(),
          z.object({id,op:z.literal('clean_offset_boundary'),input:id,inputs:z.array(id).min(3).max(255),distance_mm:scalar,even_odd:z.boolean(),component:z.number().int().nonnegative(),piece:z.number().int().nonnegative(),max_pairs:z.number().int().positive(),max_edges:z.number().int().positive(),max_checks:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('planar_toolpath_edge'),input:id,inputs:z.array(id).max(255),tool_radius_mm:scalar,stock_mm:scalar,stepover_mm:scalar,passes:z.number().int().positive(),tolerance_mm:scalar,pass_index:z.number().int().nonnegative(),piece:z.number().int().nonnegative()}).strict(),
          z.object({id,op:z.literal('embed_xy_curve'),input:id,height:scalar}).strict(),
          z.object({id,op:z.literal('planar_offset_piece'),input:id,distance_mm:scalar,tolerance_mm:scalar,max_cells:z.number().int().min(1).max(65536),piece:z.number().int().nonnegative()}).strict(),
          z.object({id,op:z.literal('planar_knot_miter'),input:id,knot:scalar,distance_mm:scalar,extension_limit_mm:scalar,tolerance_mm:scalar}).strict(),
          z.object({id,op:z.literal('phase_aligned_section'),input:id,inputs:z.array(id).min(1).max(31),piece:z.number().int().min(0).max(31),landmarks:z.array(z.array(scalar).min(2).max(3)).min(2).max(32),search_tolerance_mm:scalar,shape_tolerance_mm:scalar,max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('compatible_section'),input:id,inputs:z.array(id).min(1).max(31),piece:z.number().int().min(0).max(31)}).strict(),
          z.object({id,op:z.literal('oriented_section'),input:id,inputs:z.array(id).min(1).max(31),piece:z.number().int().min(0).max(31),tolerance_mm:scalar}).strict(),
          z.object({ id, op:z.literal('uv_loop_closure'), input:id, loops:z.array(z.array(id).min(1).max(256)).min(1).max(1),  }).strict(),
          z.object({ id, op:z.literal('uv_loop_simplicity'), input:id, loops:z.array(z.array(id).min(1).max(256)).min(1).max(1), tolerance_uv:scalar, max_pairs:z.number().int().min(1).max(100000), max_cells:z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op:z.literal('uv_trim_location'), input:id, loops:z.array(z.array(id).min(1).max(256)).min(1).max(256), u:scalar, v:scalar, tolerance_uv:scalar, max_cells:z.number().int().min(1).max(100000) }).strict(),
          z.object({id,op:z.literal('naca_five_digit_profile'),profile_code:z.number().int().nonnegative(),chord_mm:scalar,closed:z.boolean(),samples:z.number().int().positive(),degree:z.number().int().positive(),control_count:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('cst_spline_profile'),chord_mm:scalar,n1:scalar,n2:scalar,upper_coefficients:z.array(scalar).min(1),lower_coefficients:z.array(scalar).min(1),samples:z.number().int().positive(),degree:z.number().int().positive(),control_count:z.number().int().positive()}).strict(),
          z.object({id,op:z.literal('fit_cst_sides'),upper_points:z.array(z.tuple([scalar,scalar,scalar])).min(3).max(4096),lower_points:z.array(z.tuple([scalar,scalar,scalar])).min(3).max(4096),origin:z.tuple([scalar,scalar,scalar]),chord_vector:z.tuple([scalar,scalar,scalar]),ordinate_direction:z.tuple([scalar,scalar,scalar]),order:z.number().int().min(1).max(11),n1:scalar,n2:scalar,lambda:scalar,samples:z.number().int(),degree:z.number().int(),control_count:z.number().int()}).strict(),
          z.object({id,op:z.literal('cst_section_integrals'),chord_mm:scalar,n1:scalar,n2:scalar,upper_coefficients:z.array(scalar).min(1),lower_coefficients:z.array(scalar).min(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('fit_sphere_cloud'),points:z.array(z.tuple([scalar,scalar,scalar])).min(4).max(65536)}).strict(),
          z.object({id,op:z.literal('fit_cylinder_cloud'),points:z.array(z.tuple([scalar,scalar,scalar])).min(5).max(65536),height:scalar}).strict(),
          z.object({id,op:z.literal('fit_cone_cloud'),points:z.array(z.tuple([scalar,scalar,scalar])).min(6).max(65536),height:scalar}).strict(),
          z.object({id,op:z.literal('fit_torus_cloud'),points:z.array(z.tuple([scalar,scalar,scalar])).min(6).max(65536)}).strict(),
          z.object({id,op:z.literal('fit_plane_cloud'),points:z.array(z.tuple([scalar,scalar,scalar])).min(3).max(65536),width:scalar,height:scalar}).strict(),
          z.object({ id, op:z.literal('surface_edit_deviation'), input:id, target_geometry:id }).strict(),
          z.object({ id, op:z.literal('curve_edit_deviation'), input:id, target_geometry:id, tolerance_mm:scalar, max_cells:z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op:z.literal('surface_seam_g0'), input:id, target_geometry:id, source_edge:z.enum(['u_min','u_max','v_min','v_max']), target_edge:z.enum(['u_min','u_max','v_min','v_max']), reverse:z.boolean(), tolerance_mm:scalar, max_cells:z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op:z.literal('surface_seam_g1'), input:id, target_geometry:id, source_edge:z.enum(['u_min','u_max','v_min','v_max']), target_edge:z.enum(['u_min','u_max','v_min','v_max']), reverse:z.boolean(), tolerance_mm:scalar, normal_tolerance:scalar, max_cells:z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op:z.literal('surface_seam_g2'), input:id, target_geometry:id, source_edge:z.enum(['u_min','u_max','v_min','v_max']), target_edge:z.enum(['u_min','u_max','v_min','v_max']), reverse:z.boolean(), tolerance_mm:scalar, normal_tolerance:scalar, curvature_tolerance_per_mm:scalar, segments:z.number().int().min(1).max(1024), max_cells:z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_join_continuity'), input: id, target_geometry: id,
            source_end: z.enum(['start','end']), target_end: z.enum(['start','end']),
            tolerance_mm:scalar, tangent_tolerance:scalar, curvature_tolerance_per_mm:scalar }).strict(),
          z.object({ id, op: z.literal('curve_self_contacts'), input: id, tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000), max_pairs: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('surface_self_contacts'), input: id, max_spans: z.number().int().min(1).max(100000) }).strict(),
          z.object({id,op:z.literal('bounded_surface_partial_field'),input:id,direction:z.enum(['u','v']),patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),field_error:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_partial_field'),input:id,direction:z.enum(['u','v']),patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_surface_area_normal_field'),input:id,patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),field_error:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_area_normal_field'),input:id,patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_surface_gaussian_field'),input:id,patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),field_error:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_surface_mean_squared_field'),input:id,patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),field_error:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_surface_principal_gap_squared_field'),input:id,patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),field_error:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_surface_metric_field'),input:id,patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),field_error:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gaussian_field'),input:id,patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_mean_squared_field'),input:id,patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_principal_gap_squared_field'),input:id,patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_metric_field'),input:id,patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_surface_mixed_partial_field'),input:id,orders:z.tuple([z.number().int().min(0).max(3),z.number().int().min(0).max(3)]),patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),field_error:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_mixed_partial_field'),input:id,orders:z.tuple([z.number().int().min(0).max(3),z.number().int().min(0).max(3)]),patch:z.number().int().nonnegative(),uv:z.tuple([scalar,scalar]),return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_curve_derivative_field'),input:id,order:z.number().int().min(1).max(3),piece:z.number().int().nonnegative(),parameter:scalar,return_field:z.boolean(),field_error:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('curve_derivative_field'),input:id,order:z.number().int().min(1).max(3),piece:z.number().int().nonnegative(),parameter:scalar,return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_curve_torsion_field'),input:id,piece:z.number().int().nonnegative(),parameter:scalar,return_field:z.boolean(),field_error:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_curve_speed_squared_field'),input:id,piece:z.number().int().nonnegative(),parameter:scalar,return_field:z.boolean(),field_error:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('curve_torsion_field'),input:id,piece:z.number().int().nonnegative(),parameter:scalar,return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('curve_speed_squared_field'),input:id,piece:z.number().int().nonnegative(),parameter:scalar,return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_curve_binormal_field'),input:id,piece:z.number().int().nonnegative(),parameter:scalar,return_field:z.boolean(),field_error:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('curve_binormal_field'),input:id,piece:z.number().int().nonnegative(),parameter:scalar,return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_curve_curvature_squared_field'),input:id,piece:z.number().int().nonnegative(),parameter:scalar,return_field:z.boolean(),field_error:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('curve_curvature_squared_field'),input:id,piece:z.number().int().nonnegative(),parameter:scalar,return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('projective_curve_exact'),input:id,coefficients:z.array(z.array(scalar).length(4)).length(4),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('projective_surface_exact'),input:id,coefficients:z.array(z.array(scalar).length(4)).length(4),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_cell_interpolation_error'),input:id,uv_domain:z.tuple([z.tuple([scalar,scalar]),z.tuple([scalar,scalar])]),max_cells:z.number().int().min(1).max(4096)}).strict(),
          z.object({id,op:z.literal('surface_triangle_interpolation_error'),input:id,triangle_uv:z.array(z.tuple([scalar,scalar])).length(3),vertices:z.array(z.tuple([scalar,scalar,scalar])).length(3),max_cells:z.number().int().min(1).max(4096)}).strict(),
          z.object({id,op:z.literal('surface_grid_interpolation_error'),input:id,u_lines:z.array(scalar).min(2).max(4097),v_lines:z.array(scalar).min(2).max(4097),vertices:z.array(z.tuple([scalar,scalar,scalar])).min(4).max(8194),max_cells:z.number().int().min(1).max(4096)}).strict(),
          z.object({id,op:z.literal('surface_umbilic_cells'),input:id,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_umbilic_regions'),input:id,max_depth:z.number().int().min(0).max(8),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_comparison'),input:id,gap_level_per_mm2:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_ranges'),input:id,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_regions'),input:id,gap_level_per_mm2:scalar,max_depth:z.number().int().min(0).max(8),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_edge_roots'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),source_edge:z.enum(['u_min','u_max','v_min','v_max']),gap_level_per_mm2:scalar,max_depth:z.number().int().min(0).max(16),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_contour_graph'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_contour_samples'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,transverse:z.array(scalar).min(1).max(256),max_depth:z.number().int().min(0).max(16),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_world_polyline'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,transverse:z.array(scalar).min(2).max(256),max_depth:z.number().int().min(0).max(16),return_field:z.boolean(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_world_length'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,transverse:z.array(scalar).min(2).max(256),max_depth:z.number().int().min(0).max(16),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('split_tensor_surface'),input:id,direction:z.enum(['u','v']),parameter:scalar,part:z.number().int().min(0).max(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('tile_tensor_surface'),input:id,cuts_u:z.array(scalar).max(254),cuts_v:z.array(scalar).max(254),patch_index:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('mobius_bezier_curve'),input:id,clock_factor:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('mobius_spline_curve'),input:id,clock_factor:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('fit_mobius_curve'),input:id,clock_pairs:z.tuple([scalar,scalar]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('mobius_bezier_surface'),input:id,clock_factor:z.tuple([scalar,scalar]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('mobius_spline_surface'),input:id,clock_factor:z.tuple([scalar,scalar]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('fit_mobius_surface'),input:id,clock_pairs:z.tuple([z.tuple([scalar,scalar]),z.tuple([scalar,scalar])]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('affine_clock_curve'),input:id,clock_pairs:z.tuple([scalar,scalar]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('piecewise_clock_curve'),input:id,span_factors:z.array(scalar).min(1).max(256),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('relayout_clock_curve'),input:id,span_factors:z.array(scalar).min(1).max(256),span_breaks:z.array(scalar).min(2).max(256),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('affine_clock_surface'),input:id,clock_pairs:z.tuple([z.tuple([scalar,scalar]),z.tuple([scalar,scalar])]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('piecewise_clock_surface'),input:id,u_factors:z.array(scalar).min(1).max(256),v_factors:z.array(scalar).min(1).max(256),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('relayout_clock_surface'),input:id,u_factors:z.array(scalar).min(1).max(256),v_factors:z.array(scalar).min(1).max(256),u_breaks:z.array(scalar).min(2).max(256),v_breaks:z.array(scalar).min(2).max(256),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('pullback_uv_clock'),input:id,uv_domain:z.tuple([uv,uv]),clock_factor:uv,piece:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('affine_trim_surface'),input:id,loops:z.array(z.array(id).min(1)).min(1),clock_pairs:z.tuple([uv,uv]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('transpose_trim_surface'),input:id,loops:z.array(z.array(id).min(1)).min(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('remap_trim_surface'),input:id,loops:z.array(z.array(id).min(1)).min(1),clock_pairs:z.tuple([uv,uv]),axis_order:z.number().int().min(0).max(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('reclock_trim_surface'),input:id,loops:z.array(z.array(id).min(1)).min(1),clock_factor:uv,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('affine_trim_piece'),input:id,loops:z.array(z.array(id).min(1)).min(1),clock_pairs:z.tuple([uv,uv]),loop_index:z.number().int().nonnegative(),piece:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('transpose_trim_piece'),input:id,loops:z.array(z.array(id).min(1)).min(1),loop_index:z.number().int().nonnegative(),piece:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('remap_trim_piece'),input:id,loops:z.array(z.array(id).min(1)).min(1),clock_pairs:z.tuple([uv,uv]),axis_order:z.number().int().min(0).max(1),loop_index:z.number().int().nonnegative(),piece:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('reclock_trim_piece'),input:id,loops:z.array(z.array(id).min(1)).min(1),clock_factor:uv,loop_index:z.number().int().nonnegative(),piece:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('certify_uv_chart'),input:id,uv_domain:z.tuple([uv,uv]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('classify_uv_seam'),input:id,direction:z.enum(['u','v']),seam_coordinate:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('isolate_uv_line'),input:id,line_coefficients:z.tuple([scalar,scalar,scalar]),parameter_width:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('isolate_uv_seam'),input:id,direction:z.enum(['u','v']),seam_coordinate:scalar,parameter_width:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('split_uv_seam'),input:id,direction:z.enum(['u','v']),seam_coordinate:scalar,piece:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('lift_uv_edge'),input:id,target_geometry:id,piece:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('transfer_uv_grid_piece'),input:id,old_u_breaks:z.array(scalar).min(2).max(256),old_v_breaks:z.array(scalar).min(2).max(256),u_breaks:z.array(scalar).min(2).max(256),v_breaks:z.array(scalar).min(2).max(256),u_factors:z.array(scalar).min(1).max(256),v_factors:z.array(scalar).min(1).max(256),piece:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('relayout_trim_surface'),input:id,loops:z.array(z.array(id).min(1)).min(1),u_breaks:z.array(scalar).min(2).max(256),v_breaks:z.array(scalar).min(2).max(256),u_factors:z.array(scalar).min(1).max(256),v_factors:z.array(scalar).min(1).max(256),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('relayout_trim_piece'),input:id,loops:z.array(z.array(id).min(1)).min(1),u_breaks:z.array(scalar).min(2).max(256),v_breaks:z.array(scalar).min(2).max(256),u_factors:z.array(scalar).min(1).max(256),v_factors:z.array(scalar).min(1).max(256),loop_index:z.number().int().nonnegative(),piece:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('verify_uv_edge'),input:id,target_geometry:id,inputs:z.tuple([id]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('compare_seam_orientations'),input:id,target_geometry:id,inputs:z.tuple([id,id]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('compare_uv_traces'),input:id,target_geometry:id,inputs:z.tuple([id,id]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('compare_uv_tangents'),input:id,target_geometry:id,inputs:z.tuple([id,id]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('check_seam_gap'),input:id,target_geometry:id,inputs:z.tuple([id,id]),tolerance_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('check_seam_angle'),input:id,target_geometry:id,inputs:z.tuple([id,id]),min_cosine_squared:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('isolate_seam_gap'),input:id,target_geometry:id,inputs:z.tuple([id,id]),tolerance_mm:scalar,parameter_width:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('isolate_seam_angle'),input:id,target_geometry:id,inputs:z.tuple([id,id]),min_cosine_squared:scalar,parameter_width:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('exact_wire_area'),input:id,inputs:z.array(id).max(31),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_wire_moments'),input:id,inputs:z.array(id).max(31),loops:z.array(z.array(id).min(1)).max(64),chart_origin:uv,moment_tolerances:z.tuple([scalar,scalar,scalar]),max_cells:z.number().int().min(1).max(100000)}).strict(),
          z.object({id,op:z.literal('cap_section_wire'),input:id,inputs:z.array(id).max(255),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,boundary_tolerance_mm:z.number().nonnegative(),support_tolerance_mm:z.number().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('attach_expanded_rims'),input:id,shell:z.number().int().nonnegative(),rim_addresses:z.array(z.array(z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()])).min(1).max(256)).min(1).max(64),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('attach_section_rims'),input:id,shell:z.number().int().nonnegative(),rim_addresses:z.array(z.array(z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()])).min(1).max(256)).min(1).max(64),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('discover_expanded_rims'),input:id,shell:z.number().int().nonnegative(),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('discover_section_rims'),input:id,shell:z.number().int().nonnegative(),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('attach_bounded_rims'),input:id,shell:z.number().int().nonnegative(),rim_addresses:z.array(z.array(z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()])).min(1).max(256)).min(1).max(64),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,support_tolerance_mm:z.number().nonnegative(),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('build_expanded_plane_solid'),input:id,volume_tolerance_mm3:z.number().positive(),tolerance_uv:z.number().positive(),max_volume_cells:z.number().int().positive().max(100000),planes:z.array(z.tuple([scalar,scalar,scalar,scalar])).min(1).max(64),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('build_outward_plane_solid'),input:id,volume_tolerance_mm3:z.number().positive(),tolerance_uv:z.number().positive(),max_volume_cells:z.number().int().positive().max(100000),planes:z.array(z.tuple([scalar,scalar,scalar,scalar])).min(1).max(64),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('build_section_solid'),input:id,volume_tolerance_mm3:z.number().positive(),tolerance_uv:z.number().positive(),max_volume_cells:z.number().int().positive().max(100000),planes:z.array(z.tuple([scalar,scalar,scalar,scalar])).min(1).max(64),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('infer_rim_planes'),input:id,shell:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('close_inferred_rims'),input:id,shell:z.number().int().nonnegative(),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('close_bounded_inferred_rims'),input:id,shell:z.number().int().nonnegative(),support_tolerance_mm:z.number().nonnegative(),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('inspect_inferred_solid'),input:id,support_tolerance_mm:z.number().nonnegative(),tolerance_uv:z.number().positive(),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('build_outward_inferred_solid'),input:id,volume_tolerance_mm3:z.number().positive(),tolerance_uv:z.number().positive(),max_verification_cells:z.number().int().min(1).max(100000),max_volume_cells:z.number().int().positive().max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('build_expanded_solid'),input:id,volume_tolerance_mm3:z.number().positive(),tolerance_uv:z.number().positive(),max_verification_cells:z.number().int().min(1).max(100000),max_volume_cells:z.number().int().positive().max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('build_inferred_solid'),input:id,support_tolerance_mm:z.number().nonnegative(),volume_tolerance_mm3:z.number().positive(),tolerance_uv:z.number().positive(),max_verification_cells:z.number().int().min(1).max(100000),max_volume_cells:z.number().int().positive().max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('close_expanded_planes'),input:id,shell:z.number().int().nonnegative(),planes:z.array(z.tuple([scalar,scalar,scalar,scalar])).min(1).max(64),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('project_polyhedral_body'),input:id,direction:z.tuple([scalar,scalar,scalar]),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_polyhedral_frame'),input:id,origin_mm:z.tuple([scalar,scalar,scalar]),frame_basis:z.tuple([z.tuple([scalar,scalar,scalar]),z.tuple([scalar,scalar,scalar]),z.tuple([scalar,scalar,scalar])]),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_polyhedral_axis_gap'),input:id,target_geometry:id,direction:z.tuple([scalar,scalar,scalar]),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('section_polyhedral_faces'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('assemble_polyhedral_section_loops'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_polyhedral_section_area'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_polyhedral_section_centroid'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_polyhedral_section_perimeter'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,max_width_mm:z.number().positive(),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('clip_polyhedral_segment'),input:id,query_path_mm:z.tuple([sectionQueryPoint,sectionQueryPoint]),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('locate_polyhedral_point'),input:id,query_point_mm:sectionQueryPoint,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_polyhedral_boundary_distance'),input:id,query_point_mm:sectionQueryPoint,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_polyhedral_signed_distance'),input:id,query_point_mm:sectionQueryPoint,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('sample_polyhedral_signed_distances'),input:id,query_path_mm:z.array(sectionQueryPoint).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_swept_ball_gap'),input:id,query_path_mm:z.tuple([sectionQueryPoint,sectionQueryPoint]),query_radius_mm:sectionCoordinate,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_path_material_length'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_mm:z.number().positive(),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('clip_exterior_path'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('clip_interior_path'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('partition_solid_path'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_path_length_breakdown'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_mm:z.number().positive(),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('find_path_face_contacts'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('clip_common_path'),input:id,inputs:z.array(id).max(31),query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('clip_union_path'),input:id,inputs:z.array(id).max(31),query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('subtract_path_cutters'),input:id,inputs:z.array(id).max(31),query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('classify_path_membership'),input:id,inputs:z.array(id).max(31),query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_overlap_lengths'),input:id,inputs:z.array(id).max(31),query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_mm:z.number().positive(),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('recover_overlap_paths'),input:id,inputs:z.array(id).max(31),query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),min_count:z.number().int().nonnegative(),max_count:z.number().int().nonnegative(),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('recover_material_path'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('clip_path_to_material'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('find_path_boundary_events'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_swept_path_gap'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),query_radius_mm:sectionCoordinate,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('classify_swept_path_material'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),query_radius_mm:sectionCoordinate,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_swept_first_contact'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),query_radius_mm:sectionCoordinate,max_parameter_width:z.number().positive(),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_swept_face_ranges'),input:id,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),query_radius_mm:sectionCoordinate,max_parameter_width:z.number().positive(),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_polyhedral_ball_clearance'),input:id,query_point_mm:sectionQueryPoint,query_radius_mm:sectionCoordinate,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('locate_section_point'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,query_point_mm:sectionQueryPoint,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_section_contour_distance'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,query_point_mm:sectionQueryPoint,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_section_material_distance'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,query_point_mm:sectionQueryPoint,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('clip_section_segment'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,query_path_mm:z.tuple([sectionQueryPoint,sectionQueryPoint]),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('clip_section_polyline'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_section_path_length'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,query_path_mm:z.array(sectionQueryPoint).min(2).max(4096),max_width_mm:z.number().positive(),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('classify_polyhedral_plane'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('intersect_polyhedral_edges'),input:id,plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_polyhedral_volume'),input:id,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_polyhedral_centroid'),input:id,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_polyhedral_inertia'),input:id,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_polyhedral_face_areas'),input:id,tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_polyhedral_area'),input:id,max_width_mm2:z.number().positive(),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_polyhedral_edge_length'),input:id,max_width_mm:z.number().positive(),tolerance_uv:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('close_section_planes'),input:id,shell:z.number().int().nonnegative(),planes:z.array(z.tuple([scalar,scalar,scalar,scalar])).min(1).max(64),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('build_bounded_section_solid'),input:id,volume_tolerance_mm3:z.number().positive(),tolerance_uv:z.number().positive(),max_volume_cells:z.number().int().positive().max(100000),planes:z.array(z.tuple([scalar,scalar,scalar,scalar,z.number().nonnegative(),z.number().nonnegative()])).min(1).max(64),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('close_bounded_planes'),input:id,shell:z.number().int().nonnegative(),planes:z.array(z.tuple([scalar,scalar,scalar,scalar,z.number().nonnegative(),z.number().nonnegative()])).min(1).max(64),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('discover_bounded_rims'),input:id,shell:z.number().int().nonnegative(),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,support_tolerance_mm:z.number().nonnegative(),selection_tolerance_mm:z.number().nonnegative(),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('expanded_section_sheets'),input:id,inputs:z.array(id).max(255),loops:z.array(z.array(id).min(1)).max(64),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,tolerance_mm:z.number().min(1e-10).max(1e-2),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('section_brep_sheets'),input:id,inputs:z.array(id).max(255),loops:z.array(z.array(id).min(1)).max(64),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,tolerance_mm:z.number().min(1e-10).max(1e-2),max_verification_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bounded_section_brep_sheets'),input:id,inputs:z.array(id).max(255),loops:z.array(z.array(id).min(1)).max(64),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,tolerance_mm:z.number().min(1e-10).max(1e-2),max_verification_cells:z.number().int().min(1).max(100000),support_tolerance_mm:z.number().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('cap_section_holes'),input:id,inputs:z.array(id).max(255),loops:z.array(z.array(id).min(1)).max(64),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,boundary_tolerance_mm:z.number().nonnegative(),support_tolerance_mm:z.number().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('arrange_section_caps'),input:id,inputs:z.array(id).max(255),loops:z.array(z.array(id).min(1)).max(64),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,boundary_tolerance_mm:z.number().nonnegative(),support_tolerance_mm:z.number().nonnegative(),region:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('exact_wire_mass'),input:id,inputs:z.array(id).max(31),loops:z.array(z.array(id).min(1)).max(64),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('exact_wire_inertia'),input:id,inputs:z.array(id).max(31),loops:z.array(z.array(id).min(1)).max(64),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_world_face_mass'),input:id,loops:z.array(z.array(id).min(1)).min(1),chart_origin:uv,moment_tolerances:z.tuple([scalar,scalar,scalar]),max_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_principal_face_moments'),input:id,loops:z.array(z.array(id).min(1)).min(1),chart_origin:uv,moment_tolerances:z.tuple([scalar,scalar,scalar]),max_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_principal_face_directions'),input:id,loops:z.array(z.array(id).min(1)).min(1),chart_origin:uv,moment_tolerances:z.tuple([scalar,scalar,scalar]),max_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('world_face_mass'),input:id,loops:z.array(z.array(id).min(1)).min(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('principal_face_moments'),input:id,loops:z.array(z.array(id).min(1)).min(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('principal_face_directions'),input:id,loops:z.array(z.array(id).min(1)).min(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('coplanar_ray_contacts'),input:id,loops:z.array(z.array(id).min(1)).min(1),query_point:z.tuple([scalar,scalar,scalar]),ray_direction:z.tuple([scalar,scalar,scalar]),parameter_width:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('coplanar_ray_partition'),input:id,loops:z.array(z.array(id).min(1)).min(1),query_point:z.tuple([scalar,scalar,scalar]),ray_direction:z.tuple([scalar,scalar,scalar]),parameter_width:z.number().positive(),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('cast_trimmed_face_ray'),input:id,loops:z.array(z.array(id).min(1)).min(1),query_point:z.tuple([scalar,scalar,scalar]),ray_direction:z.tuple([scalar,scalar,scalar]),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('planar_face_perimeter'),input:id,loops:z.array(z.array(id).min(1)).min(1),tolerance_mm:z.number().positive(),max_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('planar_boundary_distance'),input:id,loops:z.array(z.array(id).min(1)).min(1),query_point:z.tuple([scalar,scalar,scalar]),tolerance_mm:z.number().positive(),max_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('planar_material_distance'),input:id,loops:z.array(z.array(id).min(1)).min(1),query_point:z.tuple([scalar,scalar,scalar]),tolerance_mm:z.number().positive(),max_cells:z.number().int().min(1).max(100000),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('signed_planar_face_area'),input:id,loops:z.array(z.array(id).min(1)).min(1),tolerance_mm2:z.number().positive(),max_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('project_planar_wire'),input:id,loops:z.array(z.array(id).min(1)).min(1),inputs:z.array(id).max(253),contour_counts:z.array(z.number().int().min(1).max(64)).min(1).max(64),patch_index:z.number().int().min(0).max(63),loop_index:z.number().int().nonnegative(),piece:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('lift_planar_wire'),input:id,loops:z.array(z.array(id).min(1)).min(1),loop_index:z.number().int().nonnegative(),piece:z.number().int().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('active_surface_hull'),input:id,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('project_plane_chart'),input:id,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('store_section_edge'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),parameter_width:z.number().positive(),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),edge_index:z.number().int().nonnegative(),tolerance_mm:z.number().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('optimize_section_edge'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),parameter_width:z.number().positive(),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),edge_index:z.number().int().nonnegative(),tolerance_mm:z.number().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('store_section_edges'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),parameter_width:z.number().positive(),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),edge_index:z.number().int().nonnegative(),tolerance_mm:z.number().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('partition_face_events'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('enclose_face_events'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),xyz_tolerance:z.number().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('match_quadratic_contacts'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('shared_boundary_spans'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('shared_boundary_contacts'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('refine_boundary_section'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),parameter_width:z.number().positive().max(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('resolve_boundary_section'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),xyz_tolerance:z.number().positive(),max_passes:z.number().int().min(1).max(54),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('bound_boundary_section'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),parameter_width:z.number().positive().max(1),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('classify_section_gaps'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('assemble_section_material'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('build_material_segments'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),endpoint_tolerance:z.number().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('material_segment_curve'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),endpoint_tolerance:z.number().nonnegative(),edge_index:z.number().int().nonnegative(),tolerance_mm:z.number().nonnegative(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('measure_section_material'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),endpoint_tolerance:z.number().positive(),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('classify_face_section'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),parameter_width:z.number().positive(),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('clip_plane_section'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),parameter_width:z.number().positive(),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('build_plane_section_segments'),input:id,target_geometry:id,loops:z.array(z.array(id).min(1)).min(2),left_loop_count:z.number().int().min(1).max(63),parameter_width:z.number().positive(),tolerance_uv:z.number().positive(),max_winding_cells:z.number().int().min(1).max(100000),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('intersect_support_planes'),input:id,target_geometry:id,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('lift_plane_chart'),input:id,target_geometry:id,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('shared_plane_chart'),input:id,inputs:z.array(id).max(253),patch_index:z.number().int().min(0).max(253),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('compress_tensor_surface'),input:id,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('merge_patch_strip'),input:id,inputs:z.array(id).max(31),direction:z.enum(['u','v']),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('merge_patch_grid'),input:id,inputs:z.array(id).max(31),columns:z.number().int().min(2).max(16),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('inspect_patch_grid'),input:id,inputs:z.array(id).max(31),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('compress_tensor_surface_batch'),input:id,inputs:z.array(id).max(31),patch_index:z.number().int().min(0).max(31),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_pair_distance'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,transverse:z.array(scalar).min(2).max(256),max_depth:z.number().int().min(0).max(16),target_geometry:id,target_knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),target_direction:z.enum(['u','v']),target_gap_level_per_mm2:scalar,target_transverse:z.array(scalar).min(2).max(256),target_max_depth:z.number().int().min(0).max(16),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_plane_section'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,transverse:z.array(scalar).min(2).max(256),max_depth:z.number().int().min(0).max(16),plane_normal:z.tuple([scalar,scalar,scalar]),plane_offset_mm:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_world_distance'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,transverse:z.array(scalar).min(2).max(256),max_depth:z.number().int().min(0).max(16),probe_point:z.tuple([scalar,scalar,scalar]),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_world_bounds'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,transverse:z.array(scalar).min(2).max(256),max_depth:z.number().int().min(0).max(16),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_uv_polyline'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,transverse:z.array(scalar).min(2).max(256),max_depth:z.number().int().min(0).max(16),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_adaptive_polyline'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,uv_error:scalar,max_vertices:z.number().int().min(2).max(256),max_depth:z.number().int().min(0).max(16),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_second_derivative'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_second_order_polyline'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,uv_error:scalar,max_vertices:z.number().int().min(2).max(256),max_depth:z.number().int().min(0).max(16),max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_gap_contour_slope'),input:id,knot_span:z.tuple([z.number().int().nonnegative(),z.number().int().nonnegative()]),direction:z.enum(['u','v']),gap_level_per_mm2:scalar,max_work:z.number().int().min(1).max(1000000)}).strict(),
          z.object({id,op:z.literal('surface_quadric_support'),input:id,quadric:z.array(z.array(scalar).length(4)).length(4),max_work:z.number().int().min(0).max(16000000),max_cells:z.number().int().min(0).max(100000)}).strict(),
          z.object({ id, op: z.literal('surface_span_regularity'), input: id, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_plane_roots'), input: id, plane_normal: vector, plane_offset_mm: scalar, parameter_tolerance: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_pair_crossings'), input: id, target_geometry: id, tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_plane_chart_crossings'), input: id, target_geometry: id, tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('surface_ray_hits'), input: id, ray_origin: vector, ray_direction: vector, parameter_tolerance: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_curvature_torsion'), input: id, parameter: scalar,
            query_side: z.enum(['automatic','left','right']).optional() }).strict(),
          z.object({ id, op: z.literal('surface_principal_curvatures'), input: id, uv: z.tuple([scalar,scalar]),
            u_side: z.enum(['automatic','left','right']).optional(), v_side: z.enum(['automatic','left','right']).optional() }).strict(),
          z.object({ id, op: z.literal('curve_nearest_point'), input: id, probe_point: z.array(scalar).min(2).max(3), tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('surface_nearest_point'), input: id, probe_point: vector, tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({id,op:z.literal('merge_curve_exact'),input:id,target_geometry:id}).strict(),
          z.object({id,op:z.literal('merge_surface_exact'),input:id,target_geometry:id,direction:z.enum(['u','v'])}).strict(),
          z.object({ id, op: z.literal('curve_pair_distance'), input: id, target_geometry: id, tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_surface_distance'), input: id, target_geometry: id, tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('surface_pair_distance'), input: id, target_geometry: id, tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_length_bounds'), input: id, tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_station_at_length'), input: id, distance_mm: scalar, tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_divide_by_length'), input: id, segments: scalar, tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('surface_area_bounds'), input: id, tolerance_mm2: scalar, max_cells: z.number().int().min(1).max(100000) }).strict(),
          z.object({ id, op: z.literal('curve_measure_directional_travel'), input: id, axis_vector: z.array(scalar).min(2).max(3), interval, tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000), reverse_limit: scalar.optional() }).strict(),
          z.object({ id, op: z.literal('curve_measure_directional_travel_batch'), input: id, axis_vector: z.array(scalar).min(2).max(3), intervals: z.array(interval).min(1).max(256), tolerance_mm: scalar, max_cells: z.number().int().min(1).max(100000), reverse_limit: scalar.optional() }).strict(),
          z.object({ id, op: z.literal('curve_reparameterize'), input: id, domain: interval }).strict(),
          z.object({ id, op: z.literal('curve_split'), input: id, parameter: scalar, part: z.literal([0, 1]) }).strict(),
          z.object({ id, op: z.literal('curve_bezier_span'), input: id, index: z.number().int().nonnegative() }).strict(),
          z
            .object({
              id,
              op: z.literal('curve_edit'),
              input: id,
              insert_knots: insert.optional(),
              degree: degree.optional(),
              trim: interval.optional(),
              reverse: z.boolean().optional(),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('surface_edit'),
              input: id,
              insert_knots_u: insert.optional(),
              insert_knots_v: insert.optional(),
              degree_u: degree.optional(),
              degree_v: degree.optional(),
              trim: z.tuple([scalar, scalar, scalar, scalar]).optional(),
              reverse_u: z.boolean().optional(),
              reverse_v: z.boolean().optional(),
            })
            .strict(),
          z
            .object({ id, op: z.literal('iso_curve'), input: id, direction: z.enum(['u', 'v']), parameter: scalar })
            .strict(),
          z.object({ id, op: z.literal('ruled_surface'), inputs: z.array(id).min(2).max(32) }).strict(),
          z
            .object({
              id,
              op: z.literal('ribbon_surface'),
              input: id,
              width_law: id,
              normal: vector,
              sections: z.number().int().min(2).max(32),
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('variable_pipe_surface'),
              input: id,
              radius_law: id,
              normal: vector,
              sections: z.number().int().min(2).max(32),
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('pipe_surface'),
              input: id,
              radius: scalar,
              normal: vector,
              sections: z.number().int().min(2).max(32),
              max_deviation: scalar,
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('screw_surface'),
              input: id,
              origin: vector,
              axis: vector,
              height: scalar,
              turns: scalar,
              phase_degrees: scalar,
              max_deviation: scalar,
            })
            .strict(),
          z.object({ id, op: z.literal('surface_extrude_patches'), input: id, vector }).strict(),
          z.object({ id, op: z.literal('surface_extrude'), input: id, vector }).strict(),
          z
            .object({ id, op: z.literal('surface_revolve'), input: id, origin: vector, axis: vector, angle: scalar })
            .strict(),
          z
            .object({ id, op: z.literal('transform'), input: id, matrix: z.array(z.array(scalar).length(4)).length(4) })
            .strict(),
          z
            .object({
              id,
              op: z.literal('tessellate'),
              input: id,
              segments_u: z.number().int().min(1).max(64).default(16),
              segments_v: z.number().int().min(1).max(64).default(16),
              trim_segments: z.number().int().min(8).max(128).default(32),
              trim_curves: z
                .object({ outer: id, holes: z.array(id).max(8).default([]) })
                .strict()
                .optional(),
              trim: z
                .object({
                  outer: z.array(uv).min(3).max(128),
                  holes: z.array(z.array(uv).min(3).max(128)).max(8).default([]),
                })
                .strict()
                .optional(),
            })
            .strict(),
          z
            .object({
              id,
              op: z.literal('mesh_boolean'),
              inputs: z.tuple([id, id]),
              operation: z.enum(['union', 'intersection', 'difference']),
              relative_tolerance: z.number().finite().min(1e-12).max(1e-5).optional(),
            })
            .strict(),
          z.object({ id, op: z.literal('thicken'), input: id, vector }).strict(),
        ]),
      )
      .min(1)
      .max(128),
    root: id,
  })
  .strict()
export type RushGraphNurbs = z.infer<typeof rushGraphNurbsSchema>
type Resolved<T> = T extends {
  param: string
}
  ? number
  : T extends object
    ? {
        [K in keyof T]: Resolved<T[K]>
      }
    : T
export type ResolvedRushGraphNurbs = Resolved<RushGraphNurbs>
export type RushGraphNurbsCompilation = {
  document: RushGraphNurbs
  resolved_document: ResolvedRushGraphNurbs
  document_sha256: string
  execution_target: 'own-nurbs'
}
export const RUSH_GRAPH_NURBS_GUIDE = `RushGraph NURBS uses our own Rust numerical kernel, with no third-party spline or B-rep kernel. Select language:"rush/nurbs-1",units:"mm". Scalars are finite numbers or {param:"id"}; define parameters:[{id,value}],nodes and root. Read the accompanying schema. Graph IDs are unique, all nodes reachable, cycles forbidden. Maximum128nodes, depth32 and30000values. This contract uses own Rust geometry and the shared brep-topology library, without routing into Manifold.
curve nodes specify degree, expanded knots, control_points (all2D orall3D), positive weights and periodic. surface nodes specify degree_u/v, knots_u/v, control_points[u][v][xyz], weights[u][v], periodic_u/v. Expanded knots have controlCount+degree+1 entries, finite nondecreasing values and active domain[knots[degree],knots[controlCount]]. Periodic data uses explicitly wrapped control points and extended knots, not an implicit one-period kernel-specific convention. Evaluation is homogeneous rational B-spline evaluation with first and second derivatives. At insufficient-continuity knots derivatives can be unavailable. Degenerate surface normals/curvatures are null; inspect derivative_status.
curve_edit supports insert_knots:[{value,count}], degree elevation, trim:[a,b] and reverse, in that order. surface_edit supports the same peraxis and trim:[uMin,uMax,vMin,vMax]. The NURBS foundation API separately provides outward-rounded per-span positive-denominator/hull certificates, conservative regularity classifications, bounded approximation, analytic linear-span projection, certified insertion/elevation wrappers, and exact affine monotone reparameterization. RushGraph curve and surface root reports include these foundation certificates. A certificate can explicitly report unresolved; this is not proof of failure or regularity. General interval root isolation, nonplanar normal regularity, knot removal, degree reduction and nonlinear rational reparameterization are not implied. Elevation uses rational Bezier pieces and may increase interior knot multiplicities while preserving the curve image; reduced encoded continuity can make knot derivatives unavailable even for a smooth image. Insertion, trimming and elevation of periodic data operate on the active period and return a clamped nonperiodic representation; exact affine reparameterization and reversing preserve wrapped periodic storage, but full periodic insertion/removal/elevation is not yet a product edit. curve_split(parameter,part) selects the first (0) or second (1) split curve. curve_bezier_span(index) selects a zero-based Bezier span in knot order. iso_curve fixes the named u or v parameter. transform accepts a finite affine4x4 matrix and transforms control points; no perspective matrices.
grid_spline_surface(points,parameters_u,parameters_v) interpolates rectangular 2..11 site grids per axis with tensor natural cubics on [0,1]^2. Sites indexed [u][v] use mm; parameters strictly increase and are dimensionless. Real-arithmetic C2 is not a binary64 certificate. No regularity or injectivity guarantee; max31 controls per axis, precision refusals inherited from natural spline.
hermite_patch(corners,tangent_u,tangent_v,twist) constructs a bicubic [0,1]^2 patch from four positions, first u/v derivatives and mixed uv derivatives indexed [u][v]. All jets use mm per dimensionless parameters. Degenerate patches are permitted; no regularity or injectivity certificate. Nonfinite or unrepresentable authored conditions are refused.
closed_spline_curve(points,parameters) interpolates 4..86 sites including an exactly repeated endpoint. A cyclic cubic moment system matches real-arithmetic C2 through all joins and seam. Output domain [0,1] uses clamped, not periodic knots; binary64 continuity is not certified, regularity and absence of self-intersections are not guaranteed.
clamped_spline_curve(points,parameters,start_tangent,end_tangent) interpolates sites with endpoint dP/dt tangents in mm per dimensionless authored parameter. Interior tangents are solved for real-arithmetic C2 continuity. Output domain [0,1] scales endpoint derivatives by total authored parameter span; binary64 continuity is not certified and one-sided trimmed jets are needed at joins.
natural_spline_curve(points,parameters) interpolates 2..86 sites with natural zero second derivatives at endpoints, and solves interior cubic moments. Parameters are dimensionless and strictly increasing; positions use mm. Output domain is [0,1]. Real-arithmetic C2 continuity is not a binary64 certificate; joins require trimmed one-sided jets. Singular paths and overshoot are possible.
hermite_curve(points,tangents,parameters) constructs piecewise cubic interpolation of2..86 matching3D sites and authored dP/dt tangents. Parameters strictly increase and are dimensionless; points and tangents use mm coordinates. Output parameter is normalized to[0,1], so derivatives scale by the authored parameter span. Interior C0-multiplicity knots require one-sided jets; regularity is not certified.
formula_surface(domain:[u0,u1,v0,v1],expressions) accepts three rational coordinate token lists with u/v in place of t, degree<=12 per axis, and normalized output domains[0,1]^2. It uses the same algebraic conversion and conservative denominator admission as formula_curve.
formula_curve(domain,expressions) accepts three infix strings or reverse Polish coordinate token lists of at most64 tokens each: finite numbers, t, neg, +, -, *, /. Domains and t are dimensionless; constants/results use mm coordinates. Native rational-polynomial conversion performs no sampled fitting; intermediate/final degree<=12 and conservative positive Bernstein denominator admission apply. Text permits arithmetic, parentheses and integer powers with magnitude1..12, at most1024 ASCII bytes and nesting24. No trigonometric/exponential operators or automatic factor cancellation. coons_patch(inputs:[bottom,top,left,right]) preserves compatible rational boundaries, requires exactly shared corners and compatible homogeneous corner weights, and refuses nonpositive interior weights.
brep_progressive_miter_sweep([[outer],[hole]],points:...,normal:...,scale:{degree:...,knots:...,values:...,weights:...},twist:{degree:...,knots:...,values:...deg,weights:...},max_deviation:...,closed:false,miter_limit:4,initial_steps:1,max_steps:64) retains polyline corners and distributes closed frame holonomy. Async wall previews precede rational B-rep construction. Acceptance requires resolved twist phase, certified frame transport, sampled error and an outward-certified retained section-interpolation error upper within max_deviation, plus whole-domain profile tangent and retained wall Jacobian regularity. Unresolved certificates refuse acceptance; previews may show deferred wall regularity. Global embedding and exact G1/G2 are independent obligations; sharp miter seams retain C0. A completed body reports continuousBound only when actual wall domains/decomposition and both filled-cap regions have complete same-request error proofs (no caps for a closed path). The full bound includes stored rounding, correction and decomposition and must fit max_deviation when available; a complete bound over budget refuses construction. Missing component proofs remain explicit and cannot produce a positive whole-boundary flag. Optional axis_scale (positive dimensionless XYZ law) and center_law (local XYZ offsets in mm) use independent knot domains and a certified affine interpolation bound. Uniform scale multiplies axis-scaled profile offsets; center is added before twist/miter projection. Optional frame_axis and frame_normal provide dimensionless authored longitudinal/transverse XYZ laws with independent knot domains; both are required together. Twist rotates around the authored longitudinal axis while miter projection uses the path tangent. Affine and authored-frame laws can be combined; center uses the authored frame axis for its axial component. Optional orientation_guide uses a spatial curve rail with normalized parameter traversal to orient the frame toward rail minus path. It combines with affine and authored frame laws: with both present, the normalized frame_axis controls the longitudinal axis and the projected guide offset controls the transverse normal; frame_normal remains a validated authored input. It does not fit profile contact to the rail. Optional circle_correction_tolerance (mm), circle_correction_quantum (power-of-two mm, default 2^-40) and circle_correction_max_work (shared section budget, at most 1000000) explicitly rebuild supported nine-pole circular sections with shared dyadic generators. Correction displacement is composed outward into the complete boundary bound; rebuilt wall regularity, cap regions and Solid certificates are mandatory independent gates. Circle reconstruction clears prior projected-plane claims. Optional cap_correction_tolerance (mm), cap_correction_quantum (mm, default 2^-40) and cap_correction_max_work (default 1000000) explicitly project open endpoint sections onto endpoint path planes. Correction refuses on exhausted work or unproved retained correspondence; its displacement is added outward to the certified original wall error, retained correspondence must be exact, and the sum must fit max_deviation. Filled-cap bounds additionally require original material domains, exact or bounded retained region correspondence and nonsingular endpoint plane projection. Optional retained_wall_max_injectivity_cells (default 1000, at most 100000) bounds shared work for actual retained wall charts; exhausted work cannot promote a regularity certificate. Closed paths have no caps and refuse cap projection. .brep_smooth_miter_stations(wall_tolerance: ..., quantum: ..., max_work: ..., max_deviation: ...) is an explicit modifier of a progressive miter body with constructor-owned final sections. It rebuilds supported canonical nine-pole profiles as bounded quintic station walls, including closed wrap, while preserving one-sided jets at original polyline vertices. All lengths are mm; quantum is a normal power of two, max_work is a shared construction budget at most 1000000. The full boundary budget adds reconstruction displacement to the original wall error and retains unchanged filled caps. Station/profile jets, retained chart injectivity, intersections, nesting and orientation are recomputed; missing source ownership, exhausted budgets or unproved material geometry refuse. Its reports describe the rebuilt body and do not inherit the source wall certificates. This sufficient implementation may refuse unsupported moving-frame/guide/affine combinations.
progressive_sweep(profile,path,scale:{degree:...,knots:...,values:...,weights:...},twist:{degree:...,knots:...,values:...deg,weights:...},normal:...,max_deviation:...) combines positive dimensionless scale and signed angular twist. Orientation is rmf/fixed/fixed_normal/frenet/corrected_frenet (default rmf). corrected_frenet uses sign-continuous sampled principal normals and double-reflection fallback at zero curvature; strict frenet refuses undefined principal normals. This is not a continuous frame certificate, spacing parameter/arc_length (default parameter). Initial/max sections default5/257; at most1025, arc_length at most257. Inverse-length residual uses length_tolerance (default0.001mm) and bounded length_max_cells (default100000). Refinement doubles intervals until the sampled budget is met or maximum sections is reached. Dense nonperiodic profiles retain knot-span patches; each patch has at most32 controls per axis. Use nurbs_patches_tessellate. Reports retain every level; continuousBound:false, no continuous error/injectivity/solid certificate. Closed paths require matching tangent, equal endpoint scale and whole-turn twist. profile_sweep(profile,path,scale:{degree:...,knots:...,values:...,weights:...},normal:...,sections:...,max_deviation:...) transports an arbitrary rational3D profile with rotation-minimizing frames and positive dimensionless scale. Profile coordinates are authored around path start; the first frame preserves orientation. Path/scale domains normalize independently. Closed paths require matching tangent and scale endpoints. Fourfold section refinement is sampled, continuousBound:false; budget failure refuses the surface. profile.screw_surface(origin:...,axis:...,height:...,turns:...,phase_degrees:...,max_deviation:...) rotates a retained rational3D profile about an arbitrary nonzero axis and translates by signed height. Angle and height are linear in normalized V; turns is signed dimensionless, phase degrees, origin/height/budget lengths. Height may be zero. Dense profiles and motion split at knots, up to2048 patches retaining U/V subdomains. Use nurbs_patches_tessellate. The profile radial hull chooses a shared Hermite estimate; binary64 rounding and solid topology remain uncertified. path.ribbon_surface(width_law:curve,normal:...,sections:...,max_deviation:...) creates a symmetric open strip; positive rational width controls [width,0,0], normalized law/path domains. Initial width direction projects perpendicular to path tangent. Closed endpoint widths must match. Authored stations preserve width; intervening loft has only sampled refinement, continuousBound:false. path.variable_pipe_surface(radius_law:curve,normal:...,sections:...,max_deviation:...) uses a positive rational radius law encoded by [radius,0,0] controls. Path/law domains normalize independently, not by arc length. Closed radius endpoints must match exactly. Same sampled refinement and uncertified continuous accuracy as pipe_surface. path.pipe_surface(radius:...,normal:...,sections:...,max_deviation:...) creates transported rational circular sections normal to a3D path. Radius/budget positive lengths, normal dimensionless direction. Up to32 stations, closed paths require at least4. The sampled fourfold frame refinement report is retained; budget failure refuses graph construction, never relaxes tolerance. Circular sections have authored radius, but their linear loft between stations is not an exact constant-radius or exact RMF surface. continuousBound:false; solid topology/self-intersection not certified. catenoid_patches uses the same center, scale, start_z, end_z and max_deviation as catenoid_surface. Dense catenary profiles split at existing knots into up to85 retained revolution patches with original U subdomains and V=[0,4]. No refitting or budget relaxation. Display using nurbs_patches_tessellate. Binary64 knot insertion/revolution and sewn solid topology are not certified. clothoid_curve uses center, length, start_curvature, end_curvature, phase_degrees, max_deviation. Length/budget are positive model lengths, curvatures signed inverse lengths (e.g. 2 / 1mm), phase angular degrees. Curvature changes linearly by arc length. Composite Simpson and cubic Hermite ideal estimates are separately reported and summed within budget; binary64 rounding remains uncertified. Up to4096 quadrature intervals/85 spans; unavailable budgets refuse. Straight/circle limits supported. spherical_spiral_curve uses center, radius, longitude_turns, latitude_turns, longitude_phase_degrees, latitude_phase_degrees, max_deviation. Longitude/latitude angles advance linearly on normalized t; signed fractional turns and pole crossings are allowed. Positive radius and budget are lengths, phases angular degrees, turns dimensionless and nonzero. This is not a geodesic or equal-area spacing solver. Ideal cubic Hermite remainder excludes binary64 rounding; periodic:false, no exact sphere constraint or closure certificate. surface_extrude_patches(vector) extrudes a 3D nonperiodic curve into a retained patch set without fitting or weakening its construction budget. Small curves retain one surface; dense curves split at active knot spans with original U domains, up to256 patches. Use nurbs_patches_tessellate(segments:1..16) to display. Knot-insertion rounding and sewn solid topology are not certified. toroidal_spiral_curve uses center, major_radius, minor_radius, major_turns, minor_turns, major_phase_degrees, minor_phase_degrees, max_deviation. Ring torus requires major_radius>minor_radius>0; radii are lengths, phases angular degrees, turns signed dimensionless. torus_knot_curve uses p,q instead of turns, requiring coprime integers2..32. It shares endpoint samples but keeps periodic:false; topology is not certified. Cubic Hermite remainder excludes binary64 rounding. Up to85 spans/256 curve controls; surface extrusion is independently limited to32 profile controls and refuses dense curves. helicoid_patches uses the same fields as helicoid_surface and retains dense angular curves as up to85 patches at existing knots with original V domains. No refitting or tolerance relaxation. Use nurbs_patches_tessellate; knot-insertion rounding and sewn topology remain uncertified. circle_rectangle_transition(circle_center:...,circle_normal:...,circle_seam:...,circle_radius:...,rectangle_center:...,rectangle_axis_u:...,rectangle_axis_v:...) retains four degreeU3/degreeV1 patches. Circle quarter0 maps to rectangle corner +axis_u+axis_v; corner order (+,+),(-,+),(-,-),(+,-). Rectangle axes are orthogonal half-edge vectors. Centers/radius/rectangle axes lengths; normal/seam scalar directions. Shared Bernstein weights preserve linear Cartesian V, original quarter U domains, G0 seams; no fitting, G1/G2, sewn topology or rounding-inclusive certificate. Display with nurbs_patches_tessellate. ellipse_transition_surface(start_center:...,start_axis_u:...,start_axis_v:...,end_center:...,end_axis_u:...,end_axis_v:...) retains two full affine ellipses and their authored angular correspondence. All centers/axis vectors are lengths; each pair of axes must be independent, skew axes permitted. DegreeU2/degreeV1,9x2 controls,U/V=[0,1], G0 ruled surface; no G1/G2 blend, rounding-inclusive or topology certificate. circle_transition_surface(start_center:...,start_normal:...,start_seam:...,start_radius:...,end_center:...,end_normal:...,end_seam:...,end_radius:...) is a compact rational G0 ruled transition. Centers/radii lengths; normals/seams dimensionless. Seams project into each circle plane and choose correspondence; positive radii, nonparallel directions, distinct profiles. DegreeU2/degreeV1,9x2 controls,U/V=[0,1]. No automatic twist minimization, G1/G2 blend, rounding-inclusive or topology certificate. helicoid_surface(center:...,inner_radius:...,outer_radius:...,height:...,turns:...,phase_degrees:...,max_deviation:...) creates a Z-axis radial ruled helicoid. Radii and height use lengths, turns is signed dimensionless, phase uses degrees. Requires 0<=inner<outer, nonzero height/turns. Linear angle and Z share normalized V; cubic Hermite approximation preserves the explicit uncertified remainder report. One surface supports32 angular controls; denser requests refuse. No solid/topology or rounding-inclusive error certificate. catenoid_surface(center:...,scale:...,start_z:...,end_z:...,max_deviation:...) revolves an approximate catenary radial profile about Z. Scale and local Z bounds use lengths. Radius is scale*cosh(z/scale); the ideal profile remainder is retained, continuousBound:false, roundingCertified:false. One surface allows32 controls per axis; denser requests refuse without relaxing budget. Full circular seam controls match but periodic_v=false; no solid/topology or minimal-surface certificate. catenary_curve(center:...,scale:...,start_x:...,end_x:...,max_deviation:...) constructs center+[x,scale*(cosh(x/scale)-1),0]. Scale and x bounds use lengths; scale positive and bounds increase. Center is the vertex. Stable near-vertex height uses sinh squared. Ideal Hermite estimate excludes binary64 rounding; no hanging-chain length/tension solver is included. archimedean_spiral_curve uses center, start_radius, end_radius, start_degrees, end_degrees, max_deviation. Endpoint radii are nonnegative lengths, at least one positive; angles strictly increase. Radius changes linearly by angle. Cubic Hermite approximation reports remain uncertified; max85spans/256controls. epicycloid_curve and hypocycloid_curve use center, fixed_radius, rolling_radius, start_degrees, end_degrees, max_deviation. Radii are positive lengths, angles increase; hypocycloid requires fixed_radius>rolling_radius. The center locates the fixed circle. Cubic Hermite approximation reports remain uncertified; cusps, exact closure, topology and rounding-inclusive error are not certified. trochoid_curve(center:...,rolling_radius:...,tracing_radius:...,start_degrees:...,end_degrees:...,max_deviation:...) creates XY straight-line rolling-circle trajectories. Rolling radius is positive, tracing radius nonnegative, angles strictly increase. cycloid_curve uses radius for both. The ideal cubic Hermite remainder excludes binary64 rounding; reports stay uncertified. Cusps, loops, regularity and solid topology are not certified. lissajous_curve(center:[...],amplitudes:[...],frequencies:[...],phases_degrees:[...],max_deviation:...) creates three-axis sinusoidal trajectories. Amplitudes are nonnegative lengths, frequencies signed dimensionless cycles on normalized t, phases angular degrees. At least one axis must vary. Up to85 cubic spans/256controls; continuousBound:false, roundingCertified:false. Integer frequencies do not imply an exact binary64 closed seam; periodic=false. logarithmic_spiral_curve(center:...,radius:...,growth:...,start_degrees:...,end_degrees:...,max_deviation:...) uses radius*exp(growth*(theta-start)) in XY. Growth is dimensionless per radian even when authored angles use degrees. It preserves the explicit uncertified approximation report, rejects overflow/underflow and unattainable budgets. involute_curve(center:...,radius:...,start_degrees:...,end_degrees:...,max_deviation:...) approximates the XY base-circle involute over strictly increasing angles. Radius/deviation are positive model lengths, angles use degrees. Up to85 cubic Hermite spans/256controls; zero angle has a zero tangent and no regularity certificate. The ideal fourth-derivative remainder excludes binary64 rounding: continuousBound:false, roundingCertified:false. This is a curve, not a complete tooth or gear. polygon_profile stores a planar outer ring and holes. extrude dispatches profiles to native polygon extrusion and curves to NURBS surface extrusion; revolve dispatches similarly (polygon profiles with holes are unsupported). polygon_extrude accepts a vector; polygon_sweep uses a polyline path and initial up vector; polygon_loft joins corresponding planar sections with caps. helix_curve(center:...,radius:...,height:...,turns:...,phase_degrees:...,max_deviation:...) approximates a Z-axis cylindrical helix with cubic Hermite spans. Center/radius/height/deviation use model lengths, turns is dimensionless and phase angular. Positive radius/budget and nonzero turns; up to85spans/256controls. The reported realArithmeticErrorEstimate limits the ideal interpolation remainder only: continuousBound:false, roundingCertified:false. Trigonometric, control and evaluation rounding are not certified. Dense curves may exceed the32-control surface budget for a single extrusion. elliptic_helix_curve replaces radius with positive radius_x/radius_y. conical_helix_curve uses nonnegative start_radius/end_radius, at least one positive, interpolated linearly by t. Both preserve approximation reports; conical estimates include the radius derivative term. Zero-radius apex is permitted without a regularity certificate. variable_pitch_helix_curve uses constant radius and cubic axial Hermite height with start_pitch/end_pitch in model length per signed revolution; endpoint dZ/dt is turns*pitch. Interior axial reversal is allowed. It preserves the uncertified XY approximation report; cubic Z introduces no real-arithmetic interpolation remainder. surface_sweep constructs an exact translational rational sweep with fixed profile orientation. twist_sweep(profile,path,origin:[...],axis:[...],start_degrees:...,sweep_degrees:...) rotates the profile about a fixed axis and adds C(v)-C(0). Origin uses model lengths, axis is dimensionless, angles use degrees. Rotation follows rational conic parameterization, not linear angle or arc length. Bernstein products avoid sampled fitting; max25 degreeV and32 controls per axis. No automatic RMF, regularity, self-intersection or solid topology certificate. two_guide_sweep(profile,a,b,width:...,axis_y:[...],axis_z:[...]) constructs A(v)+(x(u)/width)*(B(v)-A(v))+y(u)*axis_y+z(u)*axis_z from a local profile. Width is a positive model length; axes are authored independent dimensionless vectors and may be nonunit. Guides normalize independently to [0,1]. Rational products preserve the curves algebraically; no automatic RMF, orthogonality, section length preservation, regularity or self-intersection certificate. Degree V is the sum of guide degrees, maximum25, with32 controls per surface axis. scaled_sweep(profile,path,origin:[...],scale:{degree,knots,values,weights}) constructs origin+C(v)-C(0)+r(v)*(P(u)-origin). Scale values and knots are dimensionless, values and weights must be positive; origin uses model lengths. Profile orientation is fixed; path and scale domains normalize independently to [0,1]. Rational Bernstein products avoid sampled fitting. Maximum V degree25 and32 controls per surface axis. Ordinary binary64 rounding is allowed; continuous rounding error, regularity, self-intersection and solid topology are not certified. framed_sweep transports profile sections with discrete rotation-minimizing frames; normal selects the initial frame, sections is2..32, max_deviation is a sampled fourfold-refinement budget in mm. Its construction report declares continuousBound:false; exceeding the sampled budget refuses the surface. Closed paths require a matching seam tangent and use C0 closure. It does not certify distance to a continuous RMF surface, injectivity or solid topology. surface_loft aligns curve degrees, domains and knots before a piecewise-linear loft. natural_loft_surface(section1,section2,...,parameters:[...]) aligns 2..11 rational sections and interpolates homogeneous controls with natural cubics on normalized V. Stations strictly increase and are dimensionless. Weight scales affect intermediate shape; nonpositive interpolated weights are refused. Natural endpoint conditions apply to homogeneous coordinates. No continuous rounding, regularity or injectivity certificate. clamped_loft_surface(section1,section2,...,parameters:[...],start_tangent:[...],end_tangent:[...]) instead constrains constant Cartesian dP/dt vectors across each boundary section. Tangents use mm per authored dimensionless parameter; normalized V derivatives scale by the parameter span. Endpoint homogeneous weight derivatives are zero. It does not automatically match neighboring surfaces or prescribe U-dependent tangent fields. closed_loft_surface(section1,...,section1,parameters:[...]) interpolates 4..11 cyclic rational sections including a repeated endpoint. Aligned endpoint homogeneous controls and weights must match exactly. Real-arithmetic C2 includes the seam; clamped encoding keeps periodic_v:false and does not wrap V outside [0,1]. Closure is not solid topology or an injectivity certificate. gordon_surface(u_curves:[...],v_curves:[...],parameters_u:[...],parameters_v:[...]) builds a rational Gordon network from two families of2..11 nonperiodic curves. U curves occupy V stations, V curves occupy U stations. Homogeneous crossings must match exactly including weights. Natural homogeneous lofts combine as Su+Sv-Suv after degree/knot alignment. Positive intermediate and final weights are required; max32 aligned controls per axis. No snapping, automatic reorientation, crossing search or continuous rounding/regularity certificate. triangular_patch(base,side_a,side_b) preserves three rational boundaries oriented A->B,A->C,B->C using a weighted collapsed fourth Coons boundary. V=1 is an intentional singular apex with undefined normal. Endpoints must match exactly. Positive-weight and 32-control Coons budgets apply; no snapping, automatic orientation, boundary rounding or regularity certificate. boundary_fill(edge1,...,center:[...]) constructs a G0 fan of rational patches from3..32 edges forming an exact oriented cycle. Center uses mm and may be nonplanar. Result retains every patch with faceIds and is tessellated via nurbs_patches_tessellate(segments:1..16). No reconstruction error metrics are fabricated. G1/G2, interior center placement, boundary simplicity, nonoverlap and solid topology are not certified.
ruled_surface joins compatible3D curves with matching degree, knots and control count. This is a rational ruled construction, not automatic interpolation through arbitrary incompatible sections. surface_extrude translates a3D curve into a rational surface. surface_revolve uses exact rational quadratic arc representations for a nonzero angle within +/-360degrees about origin/axis; degree limits still apply.
tessellate converts a surface into a derived mesh with explicit segments_u/v and optional polygonal UV trim{outer,holes}. Trims may be concave but must be simple and nonintersecting. Alternatively trim_curves:{outer:curveId,holes:[curveId,...]} samples closed2D NURBS in the surface UV parameter space (dimensionless), using trim_segments perloop (8..128, default32). These retained rational trim definitions produce polygonal boundary approximations; neither their distance error nor topology correspondence is certified. Use only one of trim and trim_curves. Sampling reports are not certified global error bounds. thicken applies a fixed vector to a mesh, builds boundary walls and checks edge topology; this is not a general normal-offset operation. Curves/surfaces remain the authoritative definitions in the document. Build reports include evaluated definitions, bounds, mesh topology and preview images when available; evaluate returns numerical jets. STL (ASCII or stl_binary), 3MF and AMF export require a closed, consistently oriented mesh; OBJ, PLY and OFF also support open surfaces; self-intersections, wall thickness and printer suitability are not certified. JSON export retains the native NURBS document. Mesh formats: stl, stl_binary, 3mf, obj, ply, off, amf. Printing files contain geometry in millimeters; they do not include slicer profiles or textures. Exports are limited to 4 MiB and 100000 triangles. mesh_boolean takes inputs:[a,b] and operation:union|intersection|difference (a minus b), with optional relative_tolerance (1e-12..1e-5, default1e-9). It runs bounded numerical BSP CSG in the own Rust polygon kernel on closed oriented solids; invalid, ambiguous or non-manifold geometry and exhausted budgets return errors. Limits:10000 combined input triangles,20000 output triangles,8000000 work units. UV coordinates are discarded. Empty results have null bounds. NURBS definitions remain retained; mesh CSG does not reconstruct NURBS surfaces. Explicit reverse conversion: triangle_mesh(vertices,triangles) accepts indexed input. mesh_to_sdf() builds signed triangle distance for a closed oriented mesh; sdf_offset and sdf_tessellate work on the resulting field. mesh_to_subdivision(iterations:0..32) retains source topology and fits original-vertex samples at one refinement step; subdivision_tessellate(levels) emits its mesh. mesh_to_nurbs() creates exact faceted NURBS patches; mesh_fit_nurbs(max_deviation) creates approximate cubic point-normal patches, checking deviation on 9x9 samples per patch, not a certified bound. nurbs_patches_tessellate(segments) emits their mesh. mesh_to_nurbs_brep() builds exact planar trimmed NURBS topology, limited to 256 triangles, and works with brep_tessellate. These are explicit reconstruction choices, not recovery of unknown original CAD or coarse subdivision controls. Mesh sources are bounded; signed distance sources at most4096 triangles, NURBS/subdivision at most2048, sampled reconstruction work8million. Subdivision: subdivision(vertices,faces,levels) uniformly refines a convex polygon cage using own Rust Catmull-Clark (levels 0..5), emits a mesh and retains original face IDs. This is finite refinement, not exact limit evaluation; crease weights and adaptive subdivision are absent. Implicit fields: sdf_sphere(center,radius), sdf_box(center,half_size), sdf_torus(center,major_radius,minor_radius); sdf_union(a,b), sdf_intersection(a,b), sdf_difference(a,b), sdf_smooth_union(a,b,radius:3mm); pipe fields through sdf_offset(distance), sdf_translate(vector), sdf_tessellate(min,max,cells). Cells is a 3-vector of integers 1..64. Bounds must enclose the surface with positive boundary samples. CSG fields are not generally exact signed distances; extraction can miss sub-cell features. Field trees are bounded to 256 nodes/depth32. Both libraries exchange meshes with polygon-core; mesh CSG does not reconstruct control cages or implicit fields. brep_box creates a topological NURBS body from min/max vectors. brep_sphere(radius) uses eight regular rational patches without collapsed pole edges; brep_torus(major_radius,minor_radius) uses sixteen rational patches and requires major_radius>minor_radius. brep_cylinder(radius,height), brep_frustum(bottom_radius,top_radius,height), and brep_tube(outer_radius,inner_radius,height) create exact rational round solids. Cylinder and tube radii are positive. Frustum accepts one zero radius for an explicit apex cone. polygon_profile(outer).brep_revolve() revolves a simple CCW (radius,z) section through a signed angle (default360°, nonzero up to±360°) (3..64 vertices, no profile holes, nonnegative radii with explicit axis poles); concave turning profiles are supported. polygon_profile(...).brep_extrude(height) constructs a planar B-rep with concave outlines and holes. brep_extrude_curves(loops,z_min,z_max) extrudes retained 2D NURBS curve loops into a closed B-rep; JSON loops are nested curve-node IDs, text uses nested curve values, e.g. brep_extrude_curves([[outer],[hole]],-2mm,3mm). Each loop is connected and closed with material left: CCW outer boundaries, CW holes; disconnected outer loops and a single multi-span circle are supported. Only degree-one rational lines and rational quadratic circular arcs are admitted; general noncircular NURBS fail explicitly. Curves and cap trims remain rational, without sampled profile reconstruction. Limits:64 loops,254 curve references and254 active knot spans in total,8192 control points and256 final faces. Each nonempty loop has at least one curve; loops:[] yields an empty B-rep. z_min/z_max are millimeters within +/-1000000 and z_max-z_min must be at least0.00001mm. brep_boolean(inputs:[a,b],operation) supports union, intersection, difference and xor. Text uses a.brep_union(b), a.brep_subtract(b), a.brep_intersection(b), a.brep_xor(b). Bounded planar CSG supports concave regions and holes. Curved CSG supports parallel extrusions of lines/circular arcs: equal axial intervals for all operations, arbitrary axial overlap for intersection, through cutters for difference and equal-footprint interval union/XOR/difference along the axis. A covering cutter footprint can remove a partial axial interval and leave zero, one or two separate bodies. Rational trimmed arcs and ruled surfaces remain authoritative; binary64 intersections and rigid-frame roundoff normalization are numerical, not certified. Empty results have no topology and tessellate to an empty mesh with null bounds. Strictly separated curved carrier hulls support independent bodies. Bounded stepped line/circle prisms support unequal axial intervals and repeated Boolean use after serialization, with at most8 layers,16 profiles and32 planar cells plus existing B-rep budgets. General sphere/torus/NURBS intersections remain unsupported. brep_chamfer(edges,size) and brep_fillet(edges,radius,segments) operate on connected convex planar edge chains; edges is an index array or an edge query {direction?:[x,y,z],angle_tolerance?:1deg,min_length?,max_length?,convexity?:convex|concave|smooth,face?:{normal?,angle_tolerance?,type?:planar|curved,min_area?,max_area?}} resolved on the input B-rep (face selects edges bounding a matching face; lengths in mm, areas in mm^2; an empty match is an error); fillets are faceted approximations. brep_full_round_exact(faces,segments) runs the native exact Full Round on three distinct original face indices of an admitted extrusion body; Rust constructs and proves the rational body, and the graph receives only its uniformly sampled display mesh (segments 1..32, default4, error not certified), so later B-rep operations cannot consume it. Unresolved curved Boolean cases and curved blends are rejected without mesh fallback. brep_tessellate samples it with segments:1..32 and preserves faceIds. Shared B-rep topology includes vertices, edges, oriented coedges, loops, faces, shells and bodies. Geometry agreement is sampled, not certified; shell containment and self-intersections are not certified. Exact B-rep roots support AP242 STEP export via format:"step"; mesh/curve/surface STEP conversion, general curved B-rep booleans, exact surface/surface trimming, analytic fillets and offset-shell construction are not implemented. Multi-profile progressive_sweep(profile1,profile2,...,path,scale:...,twist:...,normal:...,max_deviation:...) accepts 1..64 ordered profile curves followed by one path. All curves refine on common stations and share one acceptance budget; output is walls only, without contour nesting, caps or sewing. Direct API retains profilePatchRanges. brep_progressive_sweep([[outer],[hole,...]],path,scale:...,twist:...,normal:...,max_deviation:...) builds rational swept walls with audited planar cap trims and shared B-rep incidence. Profiles are world-space 3D curves. Open paths have audited planar caps; closed paths reuse periodic seam edges and preserve hole contours as inner shells. Refinement retains one aggregate sampled budget, constrained by the B-rep face budget. Cap nesting/orientation/simplicity must be proven; unresolved caps refuse. Side regularity and global self-intersections are unproven. Use brep_tessellate for display; Solid retains the authoritative body.`
export const RUSH_GRAPH_NURBS_EXAMPLE: RushGraphNurbs = {
  language: 'rush/nurbs-1',
  units: 'mm',
  parameters: [],
  nodes: [
    {
      id: 'arc',
      op: 'curve',
      degree: 2,
      knots: [0, 0, 0, 1, 1, 1],
      control_points: [
        [10, 0, 0],
        [10, 10, 0],
        [0, 10, 0],
      ],
      weights: [1, Math.SQRT1_2, 1],
      periodic: false,
    },
  ],
  root: 'arc',
}
export const RUSH_GRAPH_NURBS_SURFACE_EXAMPLE: RushGraphNurbs = {
  language: 'rush/nurbs-1',
  units: 'mm',
  parameters: [{ id: 'height', value: 8 }],
  nodes: [
    {
      id: 'patch',
      op: 'surface',
      degree_u: 2,
      degree_v: 2,
      knots_u: [0, 0, 0, 1, 1, 1],
      knots_v: [0, 0, 0, 1, 1, 1],
      control_points: [
        [
          [0, 0, 0],
          [0, 10, 0],
          [0, 20, 0],
        ],
        [
          [10, 0, 0],
          [10, 10, { param: 'height' }],
          [10, 20, 0],
        ],
        [
          [20, 0, 0],
          [20, 10, 0],
          [20, 20, 0],
        ],
      ],
      weights: [
        [1, 1, 1],
        [1, 2, 1],
        [1, 1, 1],
      ],
      periodic_u: false,
      periodic_v: false,
    },
    { id: 'skin', op: 'tessellate', input: 'patch', segments_u: 16, segments_v: 16, trim_segments: 32 },
    { id: 'plate', op: 'thicken', input: 'skin', vector: [0, 0, -2] },
  ],
  root: 'plate',
}
