import {
  emitMountVariables,
  emitRailMountModules,
  emitSkadisDovetailModules,
  emitSkadisIntegralHooks,
} from './mountScad'
import type { SkadisVoronoiBoxParams } from './schema'

function emitVoronoiFloorModule(p: SkadisVoronoiBoxParams): string {
  const hasSkadisStrips = p.mount_system === 'skadis_dovetail'
  const cavitySubtract = hasSkadisStrips ? `\n    if (eff_hook_count > 0) slot_strips_2d(0);` : ''
  const skadisObstacleLine = hasSkadisStrips ? `    if (eff_hook_count > 0) slot_strips_2d(floor_rib / 2);\n` : ''
  const uncoupledCore = `viewer_voronoi(min=[wall + floor_box_inset, wall + floor_box_inset], max=[width - wall - floor_box_inset, depth - wall - floor_box_inset], cell=eff_floor_cell, rib=floor_rib, pattern=floor_fill, seed=pattern_seed + 71, jitter=floor_jitter, overhang=0, gradient=stress_gradient, gradient_mode="radial", radius=(floor_frame && floor_clip_r > 0.1) ? 0 : eff_floor_radius, frame=0, max_sites=600, clip_out=floor_clips)`
  const coupledCore = `viewer_voronoi(min=[wall, wall], max=[width - wall, depth - wall], cell=eff_floor_cell, rib=floor_rib, pattern=floor_fill, seed=pattern_seed + 71, jitter=floor_jitter, overhang=0, gradient=stress_gradient, gradient_mode="radial", radius=eff_floor_radius, frame=0, max_sites=600, clip_out=floor_clips, couple_walls=couple_vec)`

  return `module floor_cavity_2d() {
  difference() {
    inner_cavity_2d();${cavitySubtract}
    if (dividers > 0) for (i = [1:dividers])
      translate([divider_margin + i*divider_step - wall/2, 0]) square([wall, depth]);
  }
}
module floor_obstacles_2d() {
  union() {
${skadisObstacleLine}    if (dividers > 0) for (i = [1:dividers])
      translate([divider_margin + i*divider_step - wall/2 - floor_rib/2, 0]) square([wall + floor_rib, depth]);
  }
}
module floor_open_domain_2d() {
  if (floor_rim_pad > 0.05) {
    difference() {
      offset(r = -floor_rim_pad, $fn = 32) inner_cavity_2d();
      if (has_floor_obstacles > 0) floor_obstacles_2d();
    }
  } else if (has_floor_obstacles > 0) {
    difference() { floor_cavity_2d(); floor_obstacles_2d(); }
  } else {
    floor_cavity_2d();
  }
}
module floor_lattice_raw_2d() {
  if (couple_floor > 0) {
    if (has_floor_obstacles > 0) {
      difference() {
        ${coupledCore};
        floor_obstacles_2d();
      }
    } else {
      ${coupledCore};
    }
  } else if (width - 2*(wall + floor_box_inset) > floor_rib * 1.6 && depth - 2*(wall + floor_box_inset) > floor_rib * 1.6) {
    if (floor_frame && floor_clip_r > 0.1) {
      offset(r = floor_clip_r, $fn = 12) offset(r = -floor_clip_r, $fn = 12) intersection() {
        floor_open_domain_2d();
        ${uncoupledCore};
      }
    } else {
      intersection() {
        floor_open_domain_2d();
        ${uncoupledCore};
      }
    }
  }
}
module floor_cutters_2d() {
  if (floor_fill == "circles") {
    na = max(0, floor((width - 4*wall - hole_diameter)/hole_pitch) + 1);
    nb = max(0, floor((depth - 4*wall - hole_diameter)/hole_pitch) + 1);
    if (na > 0 && nb > 0) intersection() {
      floor_open_domain_2d();
      union() for (i = [0:na-1]) for (j = [0:nb-1])
        translate([(width - (na-1)*hole_pitch)/2 + i*hole_pitch, (depth - (nb-1)*hole_pitch)/2 + j*hole_pitch]) circle(d = hole_diameter);
    }
  } else if (floor_fill != "solid") {
    floor_lattice_raw_2d();
  }
}
module floor_slab() {
  linear_extrude(floor) difference() {
    floor_cavity_2d();
    floor_cutters_2d();
  }
}`
}

function emitWallOpeningsHelper(): string {
  return `module apertures_circles(a, b, s) {
  na = max(0, floor((a - 2*wall - hole_diameter)/hole_pitch) + 1);
  nb = max(0, floor((b - wall - hole_diameter)/hole_pitch) + 1);
  tear_slope = max(0.35, min(1.5, abs(s) > 0 ? (tan(overhang_angle) - abs(s))*0.85 : tan(overhang_angle)/print_stretch*0.9));
  tear_apex = hole_diameter/(2*sin(atan(tear_slope)));
  if (na > 0 && nb > 0) intersection() {
    polygon([[0, 0], [a, 0], [a + max(0, b - wall)*s, max(0.1, b - wall)], [0, max(0.1, b - wall)]]);
    union() for (i = [0:na-1]) for (j = [0:nb-1])
      translate([(a - (na-1)*hole_pitch)/2 + i*hole_pitch, (max(0.1, b - wall) - (nb-1)*hole_pitch)/2 + j*hole_pitch])
        if (print_optimized) hull() { circle(d = hole_diameter); polygon([[0, tear_apex], [-hole_diameter*0.35, 0], [hole_diameter*0.35, 0]]); }
        else circle(d = hole_diameter);
  }
}
module wall_openings_2d(w, h, s, ovh, sd, keep = [], so = 0, sdir = 1, perim = 0, open_l = 0, open_r = 0, open_b = 0, pad_l = 0, pad_r = 0) {
  x0_v = open_l > 0 ? 0 : max(pad_l, 0.4);
  x1_v = open_r > 0 ? w : w - max(pad_r, 0.4);
  y0_trab = open_b > 0 ? 0 : wall_bot_pad;
  y1_trab = max(y0_trab + 0.1, h - wall_top_pad);
  y0_rust = open_b > 0 ? 0 : max(0, wall_bot_pad - rib/2);
  y1_rust = max(y0_rust + 0.5, h - wall_top_pad + rib/2);
  y_cut_bot = open_b > 0 ? -(floor + 0.2) : 0;
  so_v = so + sdir * x0_v;
  if (wall_fill == "circles") {
    apertures_circles(w, h, s);
  } else if (wall_fill == "trabecular") {
    if (x1_v - x0_v > rib * 1.6 && (h - wall_top_pad) - y0_trab > rib * 1.6) intersection() {
      polygon([[-open_l, y_cut_bot], [w + open_r, y_cut_bot], [w + h*s + open_r, h], [-open_l, h]]);
      viewer_voronoi(min=[x0_v, y0_trab], max=[x1_v, max(0.5, h - wall_top_pad)], cell=eff_cell, rib=rib, pattern="trabecular", seed=pattern_seed, jitter=voronoi_jitter, overhang=print_optimized ? ovh : 0, gradient=stress_gradient, gradient_mode="vertical", radius=eff_cell_radius, slope=s, frame=0, max_sites=600, keep_out=keep, seam_offset=so_v, seam_dir=sdir, perimeter=perim, open_left=open_l, open_right=open_r, open_bottom=open_b);
    }
  } else if (wall_fill != "solid") {
    if (x1_v - x0_v > rib * 1.6 && (h - wall_top_pad) - y0_trab > rib * 1.6) intersection() {
      polygon([[-open_l, y_cut_bot], [w + open_r, y_cut_bot], [w + h*s + open_r, h], [-open_l, h]]);
      viewer_voronoi(min=[x0_v, y0_rust], max=[x1_v, y1_rust], cell=eff_cell, rib=rib, pattern=wall_fill, seed=perim > 0 ? pattern_seed : sd, jitter=voronoi_jitter, overhang=print_optimized ? ovh : 0, gradient=stress_gradient, gradient_mode="vertical", radius=eff_cell_radius, slope=s, frame=0, max_sites=600, keep_out=keep, seam_offset=so_v, seam_dir=sdir, perimeter=perim, open_left=open_l, open_right=open_r, open_bottom=open_b);
    }
  }
}
module gusset_profile_2d() {
  difference() {
    square([r_bot, floor + r_bot]);
    translate([r_bot, floor + r_bot]) circle(r = r_bot, $fn = 16);
  }
}
module wall_gusset_local(len) {
  if (r_bot > 0.2 && len > 0.05) {
    rotate([90, 0, 90]) linear_extrude(len) gusset_profile_2d();
  }
}
module clip_gusset_floor(bx, by, bw, bh) {
  if (edge_frame || floor_frame || wall_fill == "solid") {
    children(0);
  } else difference() {
    children(0);
    translate([0, 0, -0.1]) linear_extrude(floor + r_bot + 0.3)
      intersection() {
        translate([bx, by]) square([bw, bh]);
        floor_cutters_2d();
      }
  }
}`
}

function emitStrippedBackPanel(useSlotStrip2d: boolean): string {
  const solidSingle = useSlotStrip2d
    ? `linear_extrude(height) slot_strip_2d(first_s0, last_s1);`
    : `translate([first_s0, 0, 0]) cube([last_s1 - first_s0, back_wall, height]);`
  const solidPerHook = useSlotStrip2d
    ? `linear_extrude(height) slot_strip_2d(s0, s1);`
    : `translate([s0, 0, 0]) cube([s1 - s0, back_wall, height]);`
  const singleCutter = useSlotStrip2d
    ? `translate([hook_first_x + i*strip_step, 0, 0]) skadis_channel();`
    : `mount_socket_at(hook_first_x + i*strip_step);`
  const perHookCutter = useSlotStrip2d ? `translate([px, 0, 0]) skadis_channel();` : `mount_socket_at(px);`

  return `module back_bay(x0, x1, sd) {
  bay_w = x1 - x0;
  if (bay_w > 0.05) {
    so = (x0 >= s_origin_x - 1e-4) ? (x0 - cx - s_shift) : (perim_s + x0 - cx - s_shift);
    open_l = (x0 <= cx + 1e-4) ? open_bl_seam : 0;
    open_r = (x1 >= width - cx - 1e-4) ? open_seam : 0;
    pad_l = (x0 <= cx + 1e-4) ? wall_side_pad : 0;
    pad_r = (x1 >= width - cx - 1e-4) ? wall_side_pad : 0;
    translate([x0, back_wall, 0]) rotate([90, 0, 0]) linear_extrude(back_wall) difference() {
      square([bay_w, height]);
      translate([0, floor]) wall_openings_2d(bay_w, wall_height, 0, eff_overhang, pattern_seed + sd, [], so, 1, perim_s, open_l, open_r, couple_floor, pad_l, pad_r);
    }
    if (r_bot > 0.2)
      clip_gusset_floor(x0 - 0.5, back_wall - 0.5, bay_w + 1, r_bot + 1)
        translate([x0, back_wall, 0])
          wall_gusset_local(bay_w);
  }
}
module mount_column_merged() {
  if (last_s1 > first_s0 + 0.05) difference() {
    ${solidSingle}
    for (i = [0:eff_hook_count - 1])
      ${singleCutter}
  }
}
module mount_column(s0, s1, px) {
  if (s1 > s0 + 0.05) difference() {
    ${solidPerHook}
    ${perHookCutter}
  }
}
module back_panel() {
  if (eff_hook_count <= 0) {
    back_bay(cx, width - cx, 47);
  } else {
    back_bay(cx, first_s0, 47);
    if (strips_merged == 0 && eff_hook_count > 1) {
      for (i = [0:eff_hook_count - 2]) {
        s1_c = (i == 0) ? first_s1 : min(width - cx, hook_first_x + i*strip_step + strip_pad_r);
        s0_n = max(cx, hook_first_x + (i + 1)*strip_step - strip_pad_l);
        back_bay(s1_c, s0_n, 64 + i*17);
      }
    }
    back_bay(last_s1, width - cx, 64 + max(0, eff_hook_count - 1)*17);
    if (strips_merged > 0) {
      mount_column_merged();
    } else {
      for (i = [0:eff_hook_count - 1]) {
        px = hook_first_x + i * strip_step;
        s0 = (i == 0) ? first_s0 : max(cx, px - strip_pad_l);
        s1 = (i == eff_hook_count - 1) ? last_s1 : min(width - cx, px + strip_pad_r);
        mount_column(s0, s1, px);
      }
    }
  }
}`
}

function emitGenericBackPanel(p: SkadisVoronoiBoxParams): string {
  const keepRects =
    p.mount_system === 'skadis_integral'
      ? Array.from(
          { length: 8 },
          (_, k) =>
            `eff_hook_count > ${k} ? [hook_first_x + ${k}*hook_spacing_skadis - cx - 5, max(0, wall_height - skadis_top_offset - 15 - rib), hook_first_x + ${k}*hook_spacing_skadis - cx + 5, wall_height] : [0, 0, 0, 0]`,
        )
      : p.mount_system === 'rail_monolithic' || p.mount_system === 'rail_t_slot'
        ? Array.from(
            { length: 8 },
            (_, k) =>
              `eff_hook_count > ${k} ? [hook_first_x + ${k}*eff_hook_step - cx - hook_width/2 - rib - 2, max(0, wall_height - hook_drop - hook_thickness - rib - 2), hook_first_x + ${k}*eff_hook_step - cx + hook_width/2 + rib + 2, wall_height] : [0, 0, 0, 0]`,
          )
        : []
  const keepExpr = `[${keepRects.join(', ')}]`
  const tSlots2d =
    p.mount_system === 'rail_t_slot'
      ? `    slot_w = hook_width + 2*hook_tolerance;
    slot_h = max(hook_thickness*1.3, 3.6);
    slot_y = floor + max(wall + 1, wall_height - max(wall*1.2, hook_thickness*1.2) - slot_h);
    if (eff_hook_count > 0) for (i = [0:eff_hook_count - 1])
      translate([hook_first_x + i*eff_hook_step - cx - slot_w/2, slot_y]) square([slot_w, slot_h]);`
      : ''
  const panelGeom = `  translate([cx, back_wall, 0]) rotate([90, 0, 0]) linear_extrude(back_wall) difference() {
    square([max(0.5, width - 2*cx), height]);
    translate([0, floor]) wall_openings_2d(max(0.5, width - 2*cx), wall_height, 0, eff_overhang, pattern_seed + 47, ${keepExpr}, 0, 1, perim_s, open_bl_seam, open_seam, couple_floor, wall_side_pad, wall_side_pad);
${tSlots2d}
  }
  if (r_bot > 0.2)
    clip_gusset_floor(cx - 0.5, back_wall - 0.5, max(0.5, width - 2*cx) + 1, r_bot + 1)
      translate([cx, back_wall, 0])
        wall_gusset_local(max(0.5, width - 2*cx));`
  return `module back_panel() {
${panelGeom}
}`
}

function emitPanelsAndCorners(p: SkadisVoronoiBoxParams): string {
  const backMod =
    p.mount_system === 'skadis_dovetail'
      ? emitStrippedBackPanel(true)
      : p.mount_system === 'rail_dovetail'
        ? emitStrippedBackPanel(false)
        : emitGenericBackPanel(p)
  return `module front_panel() {
  span = max(0.5, width - 2*cx);
  if (abs(slope) < 1e-6) {
    translate([cx, depth, 0]) rotate([90, 0, 0]) linear_extrude(wall) difference() {
      square([span, height]);
      translate([0, floor]) wall_openings_2d(span, wall_height, 0, front_overhang, pattern_seed, [], 2*span_x - s_shift + 2*corner_arc + side_span, -1, perim_s, open_seam, open_seam, couple_floor, wall_side_pad, wall_side_pad);
    }
    if (r_bot > 0.2)
      clip_gusset_floor(cx - 0.5, depth - wall - r_bot - 0.5, span + 1, r_bot + 1)
        translate([cx, depth - wall, 0]) mirror([0, 1, 0])
          wall_gusset_local(span);
  } else {
    union() {
      translate([cx, depth - wall, 0]) cube([span, wall, floor]);
      translate([cx, depth, floor]) multmatrix([[1,0,0,0],[0,1,slope,0],[0,0,1,0],[0,0,0,1]]) rotate([90, 0, 0])
        linear_extrude(wall) difference() {
          square([span, wall_height]);
          wall_openings_2d(span, wall_height, 0, front_overhang, pattern_seed, [], 2*span_x - s_shift + 2*corner_arc + side_span, -1, perim_s, 0, 0, 0, wall_side_pad, wall_side_pad);
        }
    }
  }
}
module side_panel(sd, so = 0, sdir = 1, open_l = 0, open_r = 0, open_b = 0, pad_l = 0, pad_r = 0) {
  linear_extrude(wall) difference() {
    polygon([[0, 0], [side_span, 0], [side_span, floor], [side_span + wall_height*slope, height], [0, height]]);
    translate([0, floor]) wall_openings_2d(side_span, wall_height, slope, eff_overhang, sd, [], so, sdir, perim_s, open_l, open_r, open_b, pad_l, pad_r);
  }
}
module left_panel() {
  if (side_span > 0.5) {
    translate([0, cy_back, 0]) rotate([90, 0, 90]) side_panel(pattern_seed + 13, 2*span_x - s_shift + 3*corner_arc + 2*side_span, -1, open_seam, open_seam, couple_floor, wall_side_pad, wall_side_pad);
    if (r_bot > 0.2)
      clip_gusset_floor(wall - 0.5, cy_back - 0.5, r_bot + 1, side_span + 1)
        translate([wall, cy_back, 0]) rotate([0, 0, 90]) mirror([0, 1, 0])
          wall_gusset_local(side_span);
  }
}
module right_panel() {
  if (side_span > 0.5) {
    translate([width - wall, cy_back, 0]) rotate([90, 0, 90]) side_panel(pattern_seed + 29, span_x - s_shift + corner_arc, 1, open_seam, open_seam, couple_floor, wall_side_pad, wall_side_pad);
    if (r_bot > 0.2)
      clip_gusset_floor(width - wall - r_bot - 0.5, cy_back - 0.5, r_bot + 1, side_span + 1)
        translate([width - wall, cy_back, 0]) rotate([0, 0, 90])
          wall_gusset_local(side_span);
  }
}
${backMod}
module front_corner_pair(side_x) {
  if (abs(slope) < 1e-6) {
    linear_extrude(height) difference() {
      intersection() { rr(width, depth, corner_radius); translate([side_x, depth - cy_front]) square([cx, cy_front]); }
      inner_cavity_2d();
    }
  } else {
    union() {
      linear_extrude(floor) difference() {
        intersection() { rr(width, depth, corner_radius); translate([side_x, depth - cy_front]) square([cx, cy_front]); }
        inner_cavity_2d();
      }
      translate([0, 0, floor]) multmatrix([[1,0,0,0],[0,1,slope,0],[0,0,1,0],[0,0,0,1]])
        linear_extrude(wall_height) difference() {
          intersection() { rr(width, depth, corner_radius); translate([side_x, depth - cy_front]) square([cx, cy_front]); }
          inner_cavity_2d();
        }
    }
  }
}
module bent_corner(so, open_l = 1, open_r = 1) {
  viewer_bend(radius = corner_radius, center = 0, step = corner_arc / 16, reverse = true) {
    translate([0, wall, 0]) rotate([90, 0, 0]) linear_extrude(wall) difference() {
      square([corner_arc, height]);
      translate([0, floor]) wall_openings_2d(corner_arc, wall_height, 0, eff_overhang, pattern_seed, [], so, 1, perim_s, open_l, open_r, couple_floor);
    }
  }
}
module solid_corner_gusset(xc, yc, ang) {
  translate([xc, yc, 0]) rotate([0, 0, ang])
    rotate_extrude(angle = 90, $fn = 32)
      translate([inner_r, 0]) mirror([1, 0]) gusset_profile_2d();
}
module corner_posts() {
  if (use_bent_corners) {
    translate([width - corner_radius, 0, 0]) bent_corner(span_x - s_shift, 1, 1);
    translate([width, depth - corner_radius, 0]) rotate([0, 0, 90]) bent_corner(span_x - s_shift + corner_arc + side_span, 1, 1);
    translate([corner_radius, depth, 0]) rotate([0, 0, 180]) bent_corner(2*span_x - s_shift + 2*corner_arc + side_span, 1, 1);
    translate([0, corner_radius, 0]) rotate([0, 0, 270]) bent_corner(2*span_x - s_shift + 3*corner_arc + 2*side_span, 1, open_bl_seam);
  } else {
    linear_extrude(height) difference() {
      intersection() {
        rr(width, depth, corner_radius);
        union() { square([cx, cy_back]); translate([width - cx, 0]) square([cx, cy_back]); }
      }
      inner_cavity_2d();
    }
    front_corner_pair(0);
    front_corner_pair(width - cx);
  }
  if (r_bot > 0.2 && inner_r > r_bot + 0.05) {
    clip_gusset_floor(width - corner_radius - 0.5, depth - corner_radius - 0.5, corner_radius + 1, corner_radius + 1)
      solid_corner_gusset(width - corner_radius, depth - corner_radius, 0);
    clip_gusset_floor(0, depth - corner_radius - 0.5, corner_radius + 0.5, corner_radius + 1)
      solid_corner_gusset(corner_radius, depth - corner_radius, 90);
    clip_gusset_floor(0, 0, corner_radius + 0.5, corner_radius + 0.5)
      solid_corner_gusset(corner_radius, corner_radius, 180);
    clip_gusset_floor(width - corner_radius - 0.5, 0, corner_radius + 1, corner_radius + 0.5)
      solid_corner_gusset(width - corner_radius, corner_radius, 270);
  }
}
module divider_slab() {
  div_len = max(1, depth - back_wall - wall);
  div_pad = edge_frame ? rib * 0.5 : 0;
  rotate([90, 0, 90]) linear_extrude(wall) difference() {
    polygon([[0, 0], [div_len, 0], [div_len, floor], [div_len + wall_height*slope, height], [0, height]]);
    translate([0, floor]) wall_openings_2d(div_len, wall_height, slope, eff_overhang, pattern_seed + 97, [], 0, 1, 0, 0, 0, 0, div_pad, div_pad);
  }
}`
}

function emitMountDerivedExpressions(mountSystem: SkadisVoronoiBoxParams['mount_system']): string[] {
  if (mountSystem === 'skadis_dovetail') {
    return [
      `board_pitch = 40 * 1;`,
      `hook_width_skadis = 4.4 * 1;`,
      `skadis_step_raw = board_pitch * skadis_spacing_steps;`,
      `skadis_side_margin = max(16, corner_radius + 9);`,
      `max_fit_hooks = max(2, min(8, floor((width - 2*skadis_side_margin) / skadis_step_raw) + 1));`,
      `eff_hook_count = hook_count > 0 ? min(8, max(2, hook_count)) : max_fit_hooks;`,
      `hook_spacing_skadis = eff_hook_count > 1 ? skadis_step_raw : 0;`,
      `hook_span_skadis = max(0, eff_hook_count - 1) * hook_spacing_skadis;`,
      `hook_first_x = (width - hook_span_skadis) / 2;`,
      `strip_step = hook_spacing_skadis;`,
      `strip_pad_l = 5 * 1;`,
      `strip_pad_r = 9 * 1;`,
      `strips_merged = (eff_hook_count <= 1 || strip_step <= strip_pad_l + strip_pad_r + 2) ? 1 : 0;`,
      `raw_first_s0 = max(cx, hook_first_x - strip_pad_l);`,
      `first_s0 = (raw_first_s0 - cx <= 2) ? cx : raw_first_s0;`,
      `hook_last_x = hook_first_x + max(0, eff_hook_count - 1) * strip_step;`,
      `raw_last_s1 = min(width - cx, hook_last_x + strip_pad_r);`,
      `last_s1 = ((width - cx) - raw_last_s1 <= 2) ? (width - cx) : raw_last_s1;`,
      `first_s1 = strips_merged > 0 ? last_s1 : min(width - cx, hook_first_x + strip_pad_r);`,
      `s_origin_x = eff_hook_count > 0 ? (first_s0 + first_s1) / 2 : cx;`,
      `s_shift = max(0, min(span_x, s_origin_x - cx));`,
    ]
  }
  if (mountSystem === 'skadis_integral') {
    return [
      `board_pitch = 40 * 1;`,
      `skadis_step_raw = board_pitch * skadis_spacing_steps;`,
      `skadis_side_margin = max(10, corner_radius + 6);`,
      `max_fit_hooks = max(1, min(8, floor((width - 2*skadis_side_margin) / skadis_step_raw) + 1));`,
      `eff_hook_count = hook_count > 0 ? min(8, hook_count) : max_fit_hooks;`,
      `hook_spacing_skadis = eff_hook_count > 1 ? skadis_step_raw : 0;`,
      `hook_span_skadis = max(0, eff_hook_count - 1) * hook_spacing_skadis;`,
      `hook_first_x = (width - hook_span_skadis) / 2;`,
      `s_shift = 0 * 1;`,
    ]
  }
  if (mountSystem === 'rail_dovetail') {
    return [
      `eff_hook_count = hook_count > 0 ? min(8, hook_count) : max(2, min(8, floor(width / 90 + 0.5)));`,
      `hook_span_avail = width - 2*corner_radius;`,
      `hook_bessel_span = max(0, width - 2*max(corner_radius + hook_width/2, corner_radius + 0.2071*hook_span_avail));`,
      `eff_hook_step = hook_spacing > 0 ? hook_spacing : (eff_hook_count <= 1 ? 0 : (dividers == eff_hook_count ? divider_step : (hook_bessel_span >= (eff_hook_count - 1)*hook_width*1.15 ? hook_bessel_span/(eff_hook_count - 1) : hook_span_avail/(eff_hook_count + 1))));`,
      `hook_span_rail = max(0, eff_hook_count - 1) * eff_hook_step;`,
      `hook_first_x = (width - hook_span_rail) / 2;`,
      `strip_step = eff_hook_step;`,
      `strip_pad_l = hook_width/2 + hook_tolerance + max(2, wall);`,
      `strip_pad_r = strip_pad_l;`,
      `strips_merged = (eff_hook_count <= 1 || strip_step <= strip_pad_l + strip_pad_r + 2) ? 1 : 0;`,
      `raw_first_s0 = max(cx, hook_first_x - strip_pad_l);`,
      `first_s0 = (raw_first_s0 - cx <= 2) ? cx : raw_first_s0;`,
      `hook_last_x = hook_first_x + max(0, eff_hook_count - 1) * strip_step;`,
      `raw_last_s1 = min(width - cx, hook_last_x + strip_pad_r);`,
      `last_s1 = ((width - cx) - raw_last_s1 <= 2) ? (width - cx) : raw_last_s1;`,
      `first_s1 = strips_merged > 0 ? last_s1 : min(width - cx, hook_first_x + strip_pad_r);`,
      `s_origin_x = eff_hook_count > 0 ? (first_s0 + first_s1) / 2 : cx;`,
      `s_shift = max(0, min(span_x, s_origin_x - cx));`,
    ]
  }
  if (mountSystem === 'rail_monolithic' || mountSystem === 'rail_t_slot') {
    return [
      `eff_hook_count = hook_count > 0 ? min(8, hook_count) : max(2, min(8, floor(width / 90 + 0.5)));`,
      `hook_span_avail = width - 2*corner_radius;`,
      `hook_bessel_span = max(0, width - 2*max(corner_radius + hook_width/2, corner_radius + 0.2071*hook_span_avail));`,
      `eff_hook_step = hook_spacing > 0 ? hook_spacing : (eff_hook_count <= 1 ? 0 : (dividers == eff_hook_count ? divider_step : (hook_bessel_span >= (eff_hook_count - 1)*hook_width*1.15 ? hook_bessel_span/(eff_hook_count - 1) : hook_span_avail/(eff_hook_count + 1))));`,
      `hook_span_rail = max(0, eff_hook_count - 1) * eff_hook_step;`,
      `hook_first_x = (width - hook_span_rail) / 2;`,
      `s_shift = 0 * 1;`,
    ]
  }
  return [`eff_hook_count = 0 * 1;`, `s_shift = 0 * 1;`]
}

export function emitSkadisVoronoiScad(p: SkadisVoronoiBoxParams, locale: 'ru' | 'en' = 'ru'): string {
  const ru = locale === 'ru'
  const hasSkadisStrips = p.mount_system === 'skadis_dovetail'
  const wallFillValue = p.lattice_mode
  const bendComment =
    p.bend_radius > 0
      ? ru
        ? '// viewer_bend — расширение просмотрщика для цилиндрического изгиба сетки.'
        : '// viewer_bend is a viewer mesh extension.'
      : ''
  const header = [
    ru
      ? `// Генератор коробки SKÅDIS и решётки Вороного. Все размеры в мм.`
      : `// SKÅDIS & Voronoi Box Generator. Dimensions in mm.`,
    ru
      ? `// Единый движок Вороного и трабекулярной сетки (viewer_voronoi).`
      : `// Powered by unified Voronoi / trabecular lattice (viewer_voronoi).`,
    bendComment,
    `$fn = 48; // ${ru ? 'Число граней на дугах и скруглениях' : 'Curve tessellation segments for arcs and fillets'}`,
    `/* [Box dimensions] */`,
    `width = ${p.width}; // [50:1:300] ${ru ? 'Ширина коробки снаружи (X), мм' : 'Outer box width (X), mm'}`,
    `depth = ${p.depth}; // [30:1:200] ${ru ? 'Глубина коробки снаружи (Y), мм' : 'Outer box depth (Y), mm'}`,
    `height = ${p.height}; // [40:1:250] ${ru ? 'Высота коробки снаружи (Z), мм' : 'Outer box height (Z), mm'}`,
    `wall = ${p.wall}; // [1.2:0.2:6] ${ru ? 'Толщина всех вертикальных стенок, мм' : 'Wall thickness (all sides), mm'}`,
    `floor = ${p.floor}; // [1.2:0.2:8] ${ru ? 'Толщина дна, мм' : 'Floor thickness, mm'}`,
    `corner_radius = ${p.corner_radius}; // [0:0.5:20] ${ru ? 'Наружный радиус скругления углов, мм (0 — прямые углы)' : 'Outer vertical corner radius, mm (0 = sharp corners)'}`,
    `front_angle = ${p.front_angle}; // [-45:1:45] ${ru ? 'Наклон передней стенки наружу, °' : 'Front wall outward tilt, °'}`,
    `dividers = ${p.dividers}; // [0:1:8] ${ru ? 'Количество внутренних перегородок' : 'Number of internal dividers'}`,
    `bottom_fillet = ${p.bottom_fillet}; // ${ru ? 'Скругление стыка со дном: внутренняя галтель стенок и углов' : 'Bottom inner fillet: concave gusset along floor-to-wall and corner seams'}`,
    `edge_frame = ${p.edge_frame}; // ${ru ? 'Каркас стенок: сплошные перемычки по краям стенок и вертикальным углам' : 'Wall and corner frame: solid vertical corners and wall perimeter rims'}`,
    ...emitMountVariables(p, locale),
    `/* [Wall fill] */`,
    `wall_fill = "${wallFillValue}"; // ["trabecular", "voronoi", "honeycomb", "isogrid", "triangles", "grid", "circles", "solid"] ${ru ? 'Тип рисунка стенок' : 'Wall lattice pattern'}`,
    `print_optimized = ${p.print_optimized}; // ${ru ? 'Печать без поддержек: срезать своды ячеек по углу свеса' : 'Support-free upright FDM print: clip cell roofs to overhang angle'}`,
    `overhang_angle = ${p.overhang_angle}; // [15:1:65] ${ru ? 'Максимальный угол свеса от вертикали при print_optimized, °' : 'Maximum vertical overhang angle when print_optimized is true, °'}`,
    `cell_size = ${p.cell_size}; // [6:0.5:40] ${ru ? 'Базовый размер ячейки стенок, мм' : 'Wall lattice cell size, mm'}`,
    `cell_radius = ${p.cell_radius}; // [0:0.5:12] ${ru ? 'Радиус скругления внутренних углов ячеек стенок, мм' : 'Wall cell inner corner fillet radius, mm'}`,
    `rib = ${p.rib}; // [1:0.2:8] ${ru ? 'Толщина перемычек сетки стенок, мм' : 'Wall lattice rib thickness, mm'}`,
    `voronoi_jitter = ${p.voronoi_jitter}; // [0:0.05:1] ${ru ? 'Нерегулярность ячеек Вороного на стенках (0 — ровные, 1 — органика)' : 'Wall Voronoi jitter irregularity (0 = regular, 1 = organic)'}`,
    `stress_gradient = ${p.stress_gradient}; // [0:0.1:2] ${ru ? 'Градиент усиления перемычек к основанию (сопромат)' : 'Structural stress gradient thickening ribs near base'}`,
    `mesh_density = ${p.mesh_density}; // [0.5:0.1:2] ${ru ? 'Множитель плотности сетки (>1 — мельче ячейки)' : 'Lattice density multiplier (>1 = finer cells)'}`,
    `pattern_seed = ${p.pattern_seed}; // [1:1:1000] ${ru ? 'Случайное зерно генерации рисунка' : 'Random seed for lattice pattern generation'}`,
    `/* [Floor fill] */`,
    `floor_fill = "${p.floor_lattice_mode}"; // ["trabecular", "voronoi", "honeycomb", "isogrid", "triangles", "grid", "circles", "solid"] ${ru ? 'Тип рисунка дна' : 'Floor lattice pattern'}`,
    `floor_frame = ${p.floor_frame}; // ${ru ? 'Каркас дна: сплошные перемычки (рамка) по всему периметру дна' : 'Floor perimeter frame: solid border ribs around the floor cavity'}`,
    `floor_cell_size = ${p.floor_cell_size}; // [6:0.5:40] ${ru ? 'Базовый размер ячейки дна, мм' : 'Floor lattice cell size, mm'}`,
    `floor_cell_radius = ${p.floor_cell_radius}; // [0:0.5:12] ${ru ? 'Радиус скругления углов ячеек дна, мм' : 'Floor cell corner fillet radius, mm'}`,
    `floor_rib = ${p.floor_rib}; // [1:0.2:8] ${ru ? 'Толщина перемычек сетки дна, мм' : 'Floor lattice rib thickness, mm'}`,
    `floor_jitter = ${p.floor_jitter}; // [0:0.05:1] ${ru ? 'Нерегулярность ячеек Вороного на дне (0–1)' : 'Floor Voronoi jitter irregularity (0-1)'}`,
    `hole_diameter = ${p.hole_diameter}; // [2:0.5:20] ${ru ? 'Диаметр круглых отверстий (при режиме circles), мм' : 'Circular perforation hole diameter (circles mode), mm'}`,
    `hole_pitch = ${p.hole_pitch}; // [4:0.5:40] ${ru ? 'Шаг круглых отверстий (при режиме circles), мм' : 'Circular hole spacing pitch (circles mode), mm'}`,
    `/* [Arc bend] */`,
    `bend_radius = ${p.bend_radius}; // ${ru ? 'Радиус изгиба коробки по дуге, мм (0 — прямая коробка; не для SKÅDIS)' : 'Arc bend radius, mm (0 = straight box; non-SKÅDIS only)'}`,
    `bend_reverse = ${p.bend_reverse}; // ${ru ? 'Направление изгиба: true — вогнуть внутрь, false — наружу' : 'Invert arc bend direction (concave vs convex)'}`,
    `bend_segments = ${p.bend_segments}; // ${ru ? 'Число сегментов дискретизации дуги при изгибе' : 'Arc bend mesh tessellation segments'}`,
  ]
    .filter(Boolean)
    .join('\n')

  const skadisClipSlots = hasSkadisStrips
    ? [
        `(eff_hook_count >= 1 && first_s1 > first_s0 + 0.05) ? [first_s0 - floor_rib/2, 0, first_s1 + floor_rib/2, slot_wall + floor_rib/2] : [0, 0, 0, 0]`,
        ...Array.from({ length: 7 }, (_, idx) => {
          const k = idx + 1
          return `(strips_merged == 0 && eff_hook_count > ${k} && (${k} == eff_hook_count - 1 ? last_s1 : min(width - cx, hook_first_x + ${k}*strip_step + strip_pad_r)) > max(cx, hook_first_x + ${k}*strip_step - strip_pad_l) + 0.05) ? [max(cx, hook_first_x + ${k}*strip_step - strip_pad_l) - floor_rib/2, 0, (${k} == eff_hook_count - 1 ? last_s1 : min(width - cx, hook_first_x + ${k}*strip_step + strip_pad_r)) + floor_rib/2, slot_wall + floor_rib/2] : [0, 0, 0, 0]`
        }),
      ]
    : []
  const dividerClipSlots = Array.from({ length: 8 }, (_, idx) => {
    const d = idx + 1
    return `dividers >= ${d} ? [divider_margin + ${d}*divider_step - wall/2 - floor_rib/2, 0, divider_margin + ${d}*divider_step + wall/2 + floor_rib/2, depth] : [0, 0, 0, 0]`
  })
  const floorClipsExpr = `[${[...skadisClipSlots, ...dividerClipSlots].join(', ')}]`

  const derived = [
    `slope = tan(front_angle);`,
    `wall_height = height - floor;`,
    `back_wall = wall + 0;`,
    hasSkadisStrips ? `slot_wall = max(wall, skadis_back_thickness);` : `slot_wall = wall + 0;`,
    `inner_r = max(0, corner_radius - wall);`,
    `cx = corner_radius > 0 ? max(wall, corner_radius) : wall;`,
    `cy_back = corner_radius > 0 ? max(wall, corner_radius) : wall;`,
    `cy_front = corner_radius > 0 ? max(wall, corner_radius) : wall;`,
    `side_span = depth - cy_back - cy_front;`,
    `use_bent_corners = !edge_frame && (corner_radius >= wall + 1.2) && (abs(slope) < 1e-6) && (bend_radius == 0) && (wall_fill != "solid") && (wall_fill != "circles");`,
    `open_seam = !edge_frame && (use_bent_corners || corner_radius == 0) && (abs(slope) < 1e-6) && (wall_fill != "solid") && (wall_fill != "circles") ? 1 : 0;`,
    `corner_arc = use_bent_corners ? corner_radius * PI / 2 : (corner_radius > 0 ? max(0, corner_radius - wall) * PI / 2 : 0);`,
    `span_x = max(0.5, width - 2*cx);`,
    `perim_s = 2*span_x + 2*side_span + 4*corner_arc;`,
    `divider_margin = max(wall, corner_radius);`,
    `divider_step = (width - 2*divider_margin)/(dividers + 1);`,
    ...emitMountDerivedExpressions(p.mount_system),
    `open_bl_seam = (s_shift > 0 || wall_fill == "trabecular") ? open_seam : 0;`,
    `eff_cell = max(rib*2.1, cell_size/mesh_density);`,
    `eff_floor_cell = max(floor_rib*2.1, floor_cell_size/mesh_density, sqrt((width * depth) / 420));`,
    `print_stretch = bend_radius > 0 ? max(1, 1 + (bend_reverse ? 0 : depth + wall_height*max(0, slope))/bend_radius) : 1;`,
    `eff_overhang = print_optimized ? max(10, atan(tan(overhang_angle)/print_stretch)) : 0;`,
    `front_overhang = print_optimized ? max(10, atan(sqrt(max(0.01, tan(overhang_angle)*tan(overhang_angle) - slope*slope))/print_stretch)) : 0;`,
    `eff_cell_radius = min(cell_radius, eff_cell * 0.48);`,
    `eff_floor_radius = min(floor_cell_radius, eff_floor_cell * 0.48);`,
    `couple_floor = !edge_frame && !floor_frame && (floor_fill == wall_fill) && (abs(eff_floor_cell - eff_cell) < 0.05) && (abs(slope) < 1e-6) && (bend_radius == 0) && (wall_fill != "solid") && (wall_fill != "circles") ? 1 : 0;`,
    `couple_vec = couple_floor > 0 ? [perim_s, s_shift, span_x, side_span, corner_arc, inner_r, cx, cy_back, cy_front, print_optimized ? eff_overhang : 0, eff_cell, rib, voronoi_jitter, pattern_seed, max(0.5, wall_height - wall)] : [];`,
    `raw_r_bot = inner_r > 0 ? min(2.8, inner_r - 0.35) : min(2.8, max(wall, rib));`,
    `r_bot = (bottom_fillet && (couple_floor > 0 || edge_frame || floor_frame || wall_fill == "solid") && abs(slope) < 1e-6 && bend_radius == 0 && raw_r_bot > 0.2) ? raw_r_bot : 0;`,
    `wall_top_pad = edge_frame ? max(wall, rib) : wall;`,
    `wall_bot_pad = edge_frame ? (r_bot + max(wall, rib)) : ((floor_frame && r_bot > 0.2) ? (r_bot + 0.4) : 0);`,
    `wall_side_pad = edge_frame ? max(rib * 0.5, wall + rib - cx) : 0;`,
    `floor_rim_pad = floor_frame ? (r_bot + max(wall, floor_rib)) : ((r_bot > 0.2 && couple_floor == 0) ? (r_bot + 0.2) : 0);`,
    `floor_box_inset = floor_frame ? max(0, floor_rim_pad - floor_rib * 0.45) : 0;`,
    `floor_clip_r = min(eff_floor_radius, floor_rib * 0.4);`,
    `has_floor_obstacles = (${hasSkadisStrips ? 'eff_hook_count > 0 || dividers > 0' : 'dividers > 0'}) ? 1 : 0;`,
    `floor_clips = ${floorClipsExpr};`,
  ].join('\n')

  const strip2dHelper = hasSkadisStrips
    ? `\nmodule slot_strip_2d(s0, s1) {
  sw = s1 - s0;
  br = min(2, max(0, slot_wall - wall), sw/2 - 0.1);
  translate([s0, 0]) union() {
    square([sw, wall]);
    if (br > 0.1) rr(sw, slot_wall, br);
    else square([sw, slot_wall]);
  }
}
module slot_strips_raw_2d() {
  if (eff_hook_count > 0) {
    if (strips_merged > 0) {
      if (last_s1 > first_s0 + 0.05) slot_strip_2d(first_s0, last_s1);
    } else {
      for (i = [0:eff_hook_count - 1]) {
        px = hook_first_x + i * strip_step;
        s0 = (i == 0) ? first_s0 : max(cx, px - strip_pad_l);
        s1 = (i == eff_hook_count - 1) ? last_s1 : min(width - cx, px + strip_pad_r);
        if (s1 > s0 + 0.05) slot_strip_2d(s0, s1);
      }
    }
  }
}
module slot_strips_2d(pad = 0) {
  if (pad > 0) offset(r = pad, $fn = 32) slot_strips_raw_2d();
  else slot_strips_raw_2d();
}`
    : ''

  const rrHelper = `module rr(w, d, r) {
  if (r > 0) {
    cr = min(r, w/2 - 0.05, d/2 - 0.05);
    hull() for (x = [cr, w - cr]) for (y = [cr, d - cr]) translate([x, y]) circle(r = cr, $fn = 64);
  } else square([w, d]);
}
module inner_cavity_2d() {
  translate([wall, wall]) rr(width - 2*wall, depth - 2*wall, inner_r);
}${strip2dHelper}`

  const floorMod = emitVoronoiFloorModule(p)
  const wallHelper = emitWallOpeningsHelper()
  const panelsMod = emitPanelsAndCorners(p)

  if (p.mount_system === 'skadis_dovetail') {
    const dovetailMods = emitSkadisDovetailModules()
    const mainEmit = `if (skadis_part == 1) {
  color([0.18, 0.55, 0.64]) skadis_body();
} else if (skadis_part == 2) {
  color([0.95, 0.65, 0.22]) skadis_print_hook();
} else if (skadis_part == 3) {
  color([0.18, 0.55, 0.64]) skadis_body();
  for (i = [0:eff_hook_count - 1])
    color([0.95, 0.65, 0.22])
      translate([width/2 - (eff_hook_count - 1)*10 + i*20 - 2.8, -24, 0])
        rotate([0, 0, 90]) skadis_print_hook();
} else {
  color([0.18, 0.55, 0.64]) skadis_body();
  if (eff_hook_count > 0) for (i = [0:eff_hook_count - 1])
    color([0.95, 0.65, 0.22])
      translate([hook_first_x + i*hook_spacing_skadis, 0, 0]) skadis_hook();
}`
    return [header, derived, rrHelper, floorMod, wallHelper, panelsMod, dovetailMods, mainEmit].join('\n\n')
  }

  if (p.mount_system === 'skadis_integral') {
    const integralHooks = emitSkadisIntegralHooks()
    const body = `module skadis_box() {
  color([0.18, 0.55, 0.64]) {
    floor_slab();
    front_panel();
    left_panel();
    right_panel();
    back_panel();
    corner_posts();
    if (dividers > 0) for (i = [1:dividers])
      translate([divider_margin + i*divider_step - wall/2, back_wall, 0]) divider_slab();
    skadis_integral_hooks();
  }
}

skadis_box();`
    return [header, derived, rrHelper, floorMod, wallHelper, panelsMod, integralHooks, body].join('\n\n')
  }

  const railMods = emitRailMountModules(p)
  const straightBody = `module straight_box() {
  color([0.18, 0.55, 0.64]) {
    floor_slab();
    front_panel();
    left_panel();
    right_panel();
    back_panel();
    corner_posts();
    if (dividers > 0) for (i = [1:dividers])
      translate([divider_margin + i*divider_step - wall/2, back_wall, 0]) divider_slab();
    attached_mounts();
  }
}`
  const bendEmit = `bend_step = width/min(bend_segments, 14);
if (bend_radius > 0) viewer_bend(radius=bend_radius, center=width/2, step=bend_step, reverse=bend_reverse) straight_box();
else straight_box();
detached_mounts();`
  return [header, derived, rrHelper, floorMod, wallHelper, panelsMod, railMods, straightBody, bendEmit].join('\n\n')
}
