import {
  BUSHING_DEFAULTS,
  bushingSchema,
  computeBushingParameterSuggestions,
  fdmRadialClearance,
  standardMetricPitch,
  validateBushingParameters,
  type BushingOptions,
  type BushingParameterSuggestion,
} from './bushingGeneratorSuggestions'

export {
  BUSHING_DEFAULTS,
  computeBushingParameterSuggestions,
  fdmRadialClearance,
  standardMetricPitch,
  type BushingOptions,
  type BushingParameterSuggestion,
}

const ANGLES_16 = Array.from({ length: 16 }, (_, i) => i * 22.5)

/**
 * Generates a parametric OpenSCAD (.scad) threaded bushing / sleeve / tube
 * supporting:
 * - mode 0: internal thread bushing / nut / sleeve
 * - mode 1: external thread hollow stud / fitting
 * - mode 2: dual internal + external reducer bushing
 * - mode 3: modular tube with bottom female ("мама") socket & top male ("папа") spigot,
 *           with standard metric pitch and FDM clearance auto-scaled by radius.
 */
export function generateBushing(input: unknown = BUSHING_DEFAULTS): string {
  const p = bushingSchema.parse(input)
  validateBushingParameters(p)

  const knExpr = (a: number) => {
    const u = `((${a} * knurl_count / 360) - floor(${a} * knurl_count / 360))`
    const w = `(1 - abs(2 * ${u} - 1))`
    const r = `(d / 2 - k_depth + k_depth * ${w})`
    return `[${r} * cos(${a}), ${r} * sin(${a})]`
  }
  const knurlPolygonPoints = ANGLES_16.map(knExpr).join(', ')

  return `// Parametric threaded bushing / sleeve / modular tube (mm).
// thread_mode: 0 = internal thread, 1 = external thread + smooth bore,
//              2 = internal + external reducer, 3 = tube (bottom female "мама" + top male "папа").
// auto_thread_std: automatically selects ISO pitch and FDM radial clearance from radius.
// fdm_thread_angle: uses 45-degree print-safe flank overhang for support-free vertical FDM printing.
thread_mode = ${p.thread_mode}; // [0, 1, 2, 3]
auto_thread_std = ${p.auto_thread_std};
fdm_thread_angle = ${p.fdm_thread_angle};
length = ${p.length}; // [4:0.5:300]
thread_section_length = ${p.thread_section_length}; // [2:0.5:120]
inner_diameter = ${p.inner_diameter}; // [1.6:0.2:200]
inner_pitch = ${p.inner_pitch}; // [0.25:0.05:10]
inner_starts = ${p.inner_starts}; // [1:1:4]
inner_left_handed = ${p.inner_left_handed};
inner_thread_depth = ${p.inner_thread_depth}; // [0:0.5:200]
outer_diameter = ${p.outer_diameter}; // [3:0.2:250]
outer_pitch = ${p.outer_pitch}; // [0.25:0.05:10]
outer_starts = ${p.outer_starts}; // [1:1:4]
outer_left_handed = ${p.outer_left_handed};
outer_shape = ${p.outer_shape}; // [0, 1, 2, 3, 4]
wrench_flat_depth = ${p.wrench_flat_depth}; // [0.2:0.1:30]
knurl_count = ${p.knurl_count}; // [4:1:64]
knurl_depth = ${p.knurl_depth}; // [0.1:0.05:5]
flange = ${p.flange};
flange_diameter = ${p.flange_diameter}; // [4:0.5:300]
flange_thickness = ${p.flange_thickness}; // [0.8:0.2:80]
flange_shape = ${p.flange_shape}; // [0, 1, 2]
flange_holes = ${p.flange_holes}; // [0:1:12]
flange_hole_diameter = ${p.flange_hole_diameter}; // [1:0.2:30]
blind_bottom = ${p.blind_bottom}; // [0:0.5:100]
chamfer = ${p.chamfer}; // [0:0.1:10]
clearance = ${p.clearance}; // [0:0.02:1.5]
cross_hole_diameter = ${p.cross_hole_diameter}; // [0:0.2:80]
cross_hole_z = ${p.cross_hole_z}; // [0:0.5:200]
split_slot_width = ${p.split_slot_width}; // [0:0.2:40]
segments = ${p.segments}; // [16:4:72]

$fn = segments;
depth_factor = fdm_thread_angle ? 0.375 : (5 * sqrt(3) / 16);
is_tube_mf = thread_mode == 3;
has_inner_thread = thread_mode == 0 || thread_mode == 2;
has_outer_thread = thread_mode == 1 || thread_mode == 2;

// Radius-dependent standard metric pitch and 3D-printing radial clearance
d_mid = (inner_diameter + outer_diameter) / 2;
wall_total = (outer_diameter - inner_diameter) / 2;
std_pitch_in = (is_tube_mf ? d_mid : inner_diameter) <= 3.2 ? 0.5 : ((is_tube_mf ? d_mid : inner_diameter) <= 5.2 ? 0.8 : ((is_tube_mf ? d_mid : inner_diameter) <= 6.5 ? 1.0 : ((is_tube_mf ? d_mid : inner_diameter) <= 9 ? 1.25 : ((is_tube_mf ? d_mid : inner_diameter) <= 12 ? 1.5 : ((is_tube_mf ? d_mid : inner_diameter) <= 16 ? 1.75 : ((is_tube_mf ? d_mid : inner_diameter) <= 22 ? 2.0 : ((is_tube_mf ? d_mid : inner_diameter) <= 30 ? 2.5 : ((is_tube_mf ? d_mid : inner_diameter) <= 42 ? 3.0 : 3.5))))))));
std_pitch_out = outer_diameter <= 5.2 ? 0.8 : (outer_diameter <= 6.5 ? 1.0 : (outer_diameter <= 9 ? 1.25 : (outer_diameter <= 12 ? 1.5 : (outer_diameter <= 16 ? 1.75 : (outer_diameter <= 22 ? 2.0 : (outer_diameter <= 30 ? 2.5 : (outer_diameter <= 42 ? 3.0 : 3.5)))))));
max_safe_pitch = max(0.5, (wall_total - 1.5) / 0.6);

eff_in_pitch = auto_thread_std ? min(std_pitch_in, max_safe_pitch) : inner_pitch;
eff_out_pitch = auto_thread_std ? min(std_pitch_out, max_safe_pitch) : outer_pitch;
eff_clearance = auto_thread_std ? min(0.45, max(0.16, 0.14 + 0.005 * d_mid)) : clearance;

r_bore = inner_diameter / 2;
r_in_major = inner_diameter / 2 + eff_clearance / 2;
r_in_minor = has_inner_thread ? max(0.3, r_in_major - depth_factor * eff_in_pitch) : (is_tube_mf ? r_bore : r_in_major);

r_out_major = max(r_in_major + 0.8, outer_diameter / 2 - (has_outer_thread ? eff_clearance / 2 : 0));
r_out_minor = has_outer_thread ? max(r_in_major + 0.6, r_out_major - depth_factor * eff_out_pitch) : r_out_major;

r_body_bound = has_outer_thread ? r_out_major : (outer_shape == 1 ? outer_diameter / sqrt(3) + 0.15 : (outer_shape == 3 ? outer_diameter / sqrt(2) + 0.15 : (outer_shape == 4 ? outer_diameter / 2 + 0.15 : outer_diameter / 2)));
flange_h = flange ? min(flange_thickness, length - 0.5) : 0;
flange_d = flange ? max(flange_diameter, outer_diameter + 0.4) : outer_diameter;
r_flange_bound = flange_shape == 1 ? flange_d / sqrt(3) + 0.15 : (flange_shape == 2 ? flange_d / sqrt(2) + 0.15 : flange_d / 2);

// Equalized-wall joint thread geometry for thread_mode == 3 (bottom female "мама" + top male "папа")
h_joint = depth_factor * eff_in_pitch;
r_joint_nom = (r_bore + outer_diameter / 2 + h_joint) / 2;
r_fem_major = r_joint_nom + eff_clearance / 2;
r_fem_minor = max(r_bore + 0.1, r_fem_major - h_joint);
r_male_major = r_joint_nom - eff_clearance / 2;
r_male_minor = max(r_bore + 0.6, r_male_major - h_joint);

sec_len = min(thread_section_length, length * 0.44);
fem_len = sec_len + 0.4;
male_len = sec_len;
z_male_start = length - male_len;
taper_h = max(0.4, r_fem_major - r_bore);

eff_chamfer = min(chamfer, length * 0.22, (r_out_minor - r_in_major) * 0.45);
fl_chamfer = min(eff_chamfer, flange_h * 0.3);

z_bot = blind_bottom > 0 ? min(length - 1, blind_bottom) : 0;
th_depth = inner_thread_depth > 0 ? min(length - z_bot, inner_thread_depth) : (length - z_bot);
z_th_start = length - th_depth;
has_3d_trims = (!has_outer_thread && outer_shape > 0) || (flange && flange_h > 0 && (flange_shape > 0 || flange_holes > 0)) || cross_hole_diameter > 0;

// Cut ISO / 45-deg FDM thread profiles and chamfers in 2D before revolution to bound 3D Boolean work.
module revolved_bushing_2d() {
  difference() {
    union() {
      translate([r_in_minor, 0])
        square([r_body_bound - r_in_minor, length]);
      if (flange && flange_h > 0) {
        translate([r_in_minor, 0])
          square([r_flange_bound - r_in_minor, flange_h]);
      }
      if (blind_bottom > 0) {
        square([r_in_minor + 0.1, z_bot]);
      }
    }

    // Mode 3: Bottom Female ("мама") socket + 45-deg support-free conical transition + Top Male ("папа") spigot
    if (is_tube_mf) {
      max_mf_teeth = has_3d_trims ? 3 : 9;
      // 1. Widen bottom bore to female minor radius with 45-deg self-supporting roof taper
      polygon([
        [r_bore - 0.1, z_bot - 0.1],
        [r_fem_minor, z_bot - 0.1],
        [r_fem_minor, fem_len],
        [r_bore - 0.1, fem_len + (r_fem_minor - r_bore + 0.1)]
      ]);

      // 2. Bottom Female ("мама") internal thread grooves (bounded for 128-vertex revolve limit)
      fem_step = max(eff_in_pitch, fem_len / max_mf_teeth);
      n_fem = ceil(fem_len / fem_step) + 1;
      fem_phase = (inner_left_handed ? 0.35 : 0.1) + (inner_starts - 1) * 0.12;
      intersection() {
        polygon([
          [r_fem_minor - 0.05, z_bot],
          [r_fem_major + 0.1, z_bot],
          [r_fem_major + 0.1, fem_len],
          [r_fem_minor - 0.05, fem_len + (r_fem_major - r_fem_minor + 0.15)]
        ]);
        union() {
          for (i = [-1 : n_fem]) {
            yc = z_bot + (i + fem_phase) * fem_step;
            polygon([
              [r_fem_minor - 0.05, yc - 0.375 * fem_step],
              [r_fem_major, yc - 0.0625 * fem_step],
              [r_fem_major, yc + 0.0625 * fem_step],
              [r_fem_minor - 0.05, yc + 0.375 * fem_step]
            ]);
          }
        }
      }

      // 3. Bottom Female entrance 45-deg lead-in countersink chamfer
      if (blind_bottom <= 0) {
        ch_fem = min(eff_chamfer, eff_in_pitch * 0.6);
        polygon([
          [r_bore - 0.1, -0.1],
          [r_fem_major + ch_fem, -0.1],
          [r_fem_minor - 0.1, ch_fem + (r_fem_major - r_fem_minor)]
        ]);
      }

      // 4. Step down outer diameter at top for Male ("папа") spigot
      translate([r_male_major, z_male_start])
        square([r_body_bound - r_male_major + 0.5, male_len + 0.2]);

      // 5. Top Male ("папа") external thread grooves (bounded for 128-vertex revolve limit)
      male_step = max(eff_in_pitch, male_len / max_mf_teeth);
      n_male = ceil(male_len / male_step) + 1;
      male_phase = (inner_left_handed ? 0.35 : 0.1) + (inner_starts - 1) * 0.12;
      intersection() {
        translate([r_male_minor - 0.1, z_male_start])
          square([r_male_major - r_male_minor + 0.2, male_len]);
        union() {
          for (j = [-1 : n_male]) {
            yc = z_male_start + (j + male_phase) * male_step;
            polygon([
              [r_male_major + 0.05, yc - 0.375 * male_step],
              [r_male_minor, yc - 0.0625 * male_step],
              [r_male_minor, yc + 0.0625 * male_step],
              [r_male_major + 0.05, yc + 0.375 * male_step]
            ]);
          }
        }
      }

      // 6. Top Male tip 45-deg lead-in bevel chamfer
      ch_male = min(eff_chamfer, eff_in_pitch * 0.6);
      polygon([
        [r_male_minor - ch_male * 0.3, length + 0.1],
        [r_body_bound + 0.2, length + 0.1],
        [r_body_bound + 0.2, length - (r_body_bound - r_male_minor + ch_male * 0.3)]
      ]);
    }

    // Modes 0 & 2: Full/partial Internal thread grooves in 2D
    if (has_inner_thread && th_depth > 0) {
      max_in_teeth = has_3d_trims ? (has_outer_thread ? 3 : 6) : (has_outer_thread ? 9 : 18);
      in_step = max(eff_in_pitch, th_depth / max_in_teeth);
      n_in = ceil(th_depth / in_step) + 1;
      in_phase = (inner_left_handed ? 0.35 : 0.1) + (inner_starts - 1) * 0.12;
      intersection() {
        translate([r_in_minor - 0.05, z_th_start])
          square([r_in_major - r_in_minor + 0.2, th_depth]);
        union() {
          for (i = [-1 : n_in]) {
            yc = z_th_start + (i + in_phase) * in_step;
            polygon([
              [r_in_minor - 0.05, yc - 0.375 * in_step],
              [r_in_major, yc - 0.0625 * in_step],
              [r_in_major, yc + 0.0625 * in_step],
              [r_in_minor - 0.05, yc + 0.375 * in_step]
            ]);
          }
        }
      }
    }

    // Modes 1 & 2: External thread grooves in 2D
    if (has_outer_thread && length > flange_h) {
      out_span = length - flange_h;
      max_out_teeth = has_3d_trims ? (has_inner_thread ? 3 : 6) : (has_inner_thread ? 9 : 18);
      out_step = max(eff_out_pitch, out_span / max_out_teeth);
      n_out = ceil(out_span / out_step) + 1;
      out_phase = (outer_left_handed ? 0.35 : 0.1) + (outer_starts - 1) * 0.12;
      intersection() {
        translate([r_out_minor - 0.1, flange_h])
          square([r_body_bound - r_out_minor + 0.2, out_span]);
        union() {
          for (j = [-1 : n_out]) {
            yc = flange_h + (j + out_phase) * out_step;
            polygon([
              [r_body_bound + 0.05, yc - 0.375 * out_step],
              [r_out_minor, yc - 0.0625 * out_step],
              [r_out_minor, yc + 0.0625 * out_step],
              [r_body_bound + 0.05, yc + 0.375 * out_step]
            ]);
          }
        }
      }
    }

    // Standard top/bottom chamfers for modes 0, 1, 2
    if (!is_tube_mf && eff_chamfer > 0) {
      polygon([
        [r_in_minor - 0.1, length - eff_chamfer],
        [r_in_major + eff_chamfer + 0.1, length + 0.1],
        [r_in_minor - 0.1, length + 0.1]
      ]);
      if (blind_bottom <= 0) {
        polygon([
          [r_in_minor - 0.1, -0.1],
          [r_in_major + eff_chamfer + 0.1, -0.1],
          [r_in_minor - 0.1, eff_chamfer]
        ]);
      }
      polygon([
        [r_body_bound - eff_chamfer, length + 0.1],
        [r_body_bound + 0.2, length + 0.1],
        [r_body_bound + 0.2, length - eff_chamfer]
      ]);
    }

    // Bottom outer bevel chamfer (when no flange)
    if (eff_chamfer > 0 && (!flange || flange_h <= 0)) {
      polygon([
        [r_body_bound - eff_chamfer, -0.1],
        [r_body_bound + 0.2, -0.1],
        [r_body_bound + 0.2, eff_chamfer]
      ]);
    }

    // Flange top and bottom bevel chamfers
    if (flange && flange_h > 0 && fl_chamfer > 0) {
      polygon([
        [r_flange_bound - fl_chamfer, -0.1],
        [r_flange_bound + 0.2, -0.1],
        [r_flange_bound + 0.2, fl_chamfer]
      ]);
      polygon([
        [r_flange_bound - fl_chamfer, flange_h + 0.05],
        [r_flange_bound + 0.2, flange_h + 0.05],
        [r_flange_bound + 0.2, flange_h - fl_chamfer]
      ]);
    }
  }
}

module unthreaded_outer_2d(d, shape) {
  if (shape == 1) {
    circle(r = d / sqrt(3), $fn = 6);
  } else if (shape == 2) {
    flat_span = max(2 * r_in_major + 1.2, d - 2 * wrench_flat_depth);
    intersection() {
      circle(d = d, $fn = segments);
      square([flat_span, d + 2], center = true);
    }
  } else if (shape == 3) {
    square([d, d], center = true);
  } else if (shape == 4) {
    k_depth = min(knurl_depth, max(0.1, (d / 2 - r_in_major - 0.8)));
    polygon([${knurlPolygonPoints}]);
  } else {
    circle(d = d, $fn = segments);
  }
}

module flange_2d(d, shape) {
  if (shape == 1) {
    circle(r = d / sqrt(3), $fn = 6);
  } else if (shape == 2) {
    square([d, d], center = true);
  } else {
    circle(d = d, $fn = segments);
  }
}

module exterior_trim_cutters() {
  r_outer_box = max(r_body_bound, r_flange_bound) * 1.5;
  trim_top_z = is_tube_mf ? z_male_start + 0.2 : length + 0.2;

  if (!has_outer_thread && outer_shape > 0) {
    z_start_trim = flange && flange_h > 0 ? flange_h + 0.05 : -0.2;
    if (trim_top_z > z_start_trim) {
      translate([0, 0, z_start_trim])
        linear_extrude(height = trim_top_z - z_start_trim)
          if (outer_shape == 2) {
            flat_half = max(r_in_major + 0.6, outer_diameter / 2 - wrench_flat_depth);
            union() {
              translate([flat_half, -r_outer_box])
                square([r_outer_box, 2 * r_outer_box]);
              translate([-flat_half - r_outer_box, -r_outer_box])
                square([r_outer_box, 2 * r_outer_box]);
            }
          } else {
            difference() {
              circle(r = r_outer_box, $fn = 12);
              unthreaded_outer_2d(outer_diameter, outer_shape);
            }
          }
    }
  }

  if (flange && flange_h > 0 && (flange_shape > 0 || flange_holes > 0)) {
    translate([0, 0, -0.1])
      linear_extrude(height = flange_h + 0.08)
        union() {
          if (flange_shape > 0) {
            difference() {
              circle(r = r_outer_box, $fn = 12);
              flange_2d(flange_d, flange_shape);
            }
          }
          if (flange_holes > 0) {
            bolt_circle_r = (flange_d + outer_diameter) / 4;
            for (k = [0 : flange_holes - 1]) {
              ang = k * 360 / flange_holes;
              translate([bolt_circle_r * cos(ang), bolt_circle_r * sin(ang)])
                circle(d = flange_hole_diameter, $fn = 10);
            }
          }
        }
  }

  if (cross_hole_diameter > 0) {
    z_c = cross_hole_z > 0 ? cross_hole_z : (flange ? (length + flange_h) / 2 : length / 2);
    translate([0, 0, z_c])
      rotate([0, 90, 0])
        cylinder(h = r_outer_box * 2.2, d = cross_hole_diameter, center = true, $fn = 10);
  }
}

has_3d_trims = (!has_outer_thread && outer_shape > 0) || (flange && flange_h > 0 && (flange_shape > 0 || flange_holes > 0)) || cross_hole_diameter > 0;
rev_fn = cross_hole_diameter > 0 ? min(segments, 10) : (has_3d_trims ? min(segments, 10) : segments);
slot_angle = split_slot_width > 0 ? min(120, max(4, 2 * asin(min(0.85, split_slot_width / (2 * r_in_major))))) : 0;
sweep_angle = 360 - slot_angle;

color([0.28, 0.56, 0.92])
if (has_3d_trims) {
  difference() {
    rotate([0, 0, slot_angle / 2])
      rotate_extrude(angle = sweep_angle, $fn = rev_fn)
        revolved_bushing_2d();
    exterior_trim_cutters();
  }
} else {
  rotate([0, 0, slot_angle / 2])
    rotate_extrude(angle = sweep_angle, $fn = rev_fn)
      revolved_bushing_2d();
}
`
}
