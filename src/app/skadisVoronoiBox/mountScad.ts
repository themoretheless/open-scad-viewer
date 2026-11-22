import type { SkadisVoronoiBoxParams } from './schema'

export function emitMountVariables(p: SkadisVoronoiBoxParams, locale: 'ru' | 'en' = 'ru'): string[] {
  const ru = locale === 'ru'
  if (p.mount_system === 'skadis_dovetail') {
    return [
      `/* [SKADIS Dovetail Mount] */`,
      `skadis_part = ${p.skadis_part}; // [0, 1, 2, 3] ${ru ? 'Деталь: 0 — сборка, 1 — корпус, 2 — крючок лёжа, 3 — печатный стол' : 'Output: 0 = assembly, 1 = box only, 2 = flat hook, 3 = print plate'}`,
      `hook_count = ${p.hook_count}; // [0:1:8] ${ru ? 'Число крючков SKÅDIS (0 — авто по ширине коробки, 2–8 — вручную)' : 'Number of SKÅDIS hooks (0 = auto by box width, 2-8 = manual)'}`,
      `skadis_spacing_steps = ${p.skadis_spacing_steps}; // [1:1:6] ${ru ? 'Шаг между крючками в ячейках SKÅDIS по 40 мм' : 'Hook spacing in 40 mm SKÅDIS pitch steps'}`,
      `skadis_top_offset = ${p.skadis_top_offset}; // [8:1:40] ${ru ? 'Отступ крючков от верхнего края коробки, мм' : 'Hook offset from top rim, mm'}`,
      `skadis_back_thickness = ${p.skadis_back_thickness}; // [7:0.5:16] ${ru ? 'Толщина колонн с пазом «ласточкин хвост» на задней стенке, мм' : 'Back-wall dovetail column thickness, mm'}`,
      `skadis_fit = ${p.skadis_fit}; // [0.1:0.05:0.5] ${ru ? 'Монтажный зазор паза «ласточкин хвост», мм' : 'Dovetail fit clearance, mm'}`,
      `skadis_friction_ridge = ${p.skadis_friction_ridge}; // ${ru ? 'Фиксирующий выступ натяга внутри паза крючка' : 'Friction retention ridge inside dovetail slot'}`,
    ]
  }
  if (p.mount_system === 'skadis_integral') {
    return [
      `/* [SKADIS Integral Hooks] */`,
      `hook_count = ${p.hook_count}; // [0:1:8] ${ru ? 'Число цельных крючков SKÅDIS (0 — авто по ширине, 1–8 — вручную)' : 'Number of integral SKÅDIS hooks (0 = auto by width, 1-8 = manual)'}`,
      `skadis_spacing_steps = ${p.skadis_spacing_steps}; // [1:1:6] ${ru ? 'Шаг между крючками в ячейках SKÅDIS по 40 мм' : 'Hook spacing in 40 mm SKÅDIS pitch steps'}`,
      `skadis_top_offset = ${p.skadis_top_offset}; // [8:1:40] ${ru ? 'Отступ крючков от верхнего края коробки, мм' : 'Hook offset from top rim, mm'}`,
      `hook_chamfer = ${p.hook_chamfer}; // ${ru ? 'Косынка 45° под крючком для печати без поддержек' : '45-degree support-free gusset under hook'}`,
    ]
  }
  if (p.mount_system === 'none') {
    return [
      `hook_count = 0; // ${ru ? 'Без навесного крепления (настольный органайзер)' : 'No wall mount hooks (desk organizer)'}`,
    ]
  }
  const hookMount = p.mount_system === 'rail_t_slot' ? 1 : p.mount_system === 'rail_dovetail' ? 2 : 0
  return [
    `/* [Rail Hooks] */`,
    `hook_count = ${p.hook_count}; // [0:1:8] ${ru ? 'Число крючков на рейку (0 — авто по ширине, 1–8 — вручную)' : 'Number of rail hooks (0 = auto by width, 1-8 = manual)'}`,
    `hook_width = ${p.hook_width}; // [2:0.5:40] ${ru ? 'Ширина каждого крючка, мм' : 'Width of each hook, mm'}`,
    `hook_thickness = ${p.hook_thickness}; // [1.5:0.2:10] ${ru ? 'Толщина стенки крючка, мм' : 'Hook wall thickness, mm'}`,
    `hook_gap = ${p.hook_gap}; // [2:0.5:30] ${ru ? 'Зазор крючка под толщину рейки, мм' : 'Gap for mounting rail thickness, mm'}`,
    `hook_drop = ${p.hook_drop}; // [4:1:80] ${ru ? 'Длина вертикального зацепа крючка, мм' : 'Vertical hook drop length, mm'}`,
    `hook_spacing = ${p.hook_spacing}; // [0:1:300] ${ru ? 'Шаг между крючками (0 — авто по точкам Бесселя), мм' : 'Hook center spacing (0 = auto Bessel points), mm'}`,
    `hook_chamfer = ${p.hook_chamfer}; // ${ru ? 'Косынка 45° под полкой крючка' : '45-degree chamfer gusset under hook'}`,
    `hook_mount = ${hookMount}; // [0, 1, 2] ${ru ? 'Тип крючка: 0 — монолит, 1 — Т-прорезь, 2 — салазки ласточкин хвост' : 'Mount type: 0 = monolithic, 1 = T-slot, 2 = dovetail slide'}`,
    `hook_tolerance = ${p.hook_tolerance}; // [0.05:0.05:0.8] ${ru ? 'Монтажный зазор съёмного крючка, мм' : 'Clearance tolerance for detachable hooks, mm'}`,
  ]
}

export function emitSkadisDovetailModules(): string {
  return `mount_shift = height - skadis_top_offset - 66;

module skadis_male_section() { polygon([[-2.2, 0], [2.2, 0], [6.2, 4], [-2.2, 4]]); }
module skadis_female_section() { offset(delta = skadis_fit) skadis_male_section(); }
module skadis_under_roof() {
  translate([-20, 0, 0]) rotate([90, 0, 90]) linear_extrude(40)
    polygon([[-3, -2], [8, -2], [8, 66 + mount_shift], [-3, 77 + mount_shift]]);
}
module skadis_channel() {
  intersection() {
    translate([0, 0, -1]) linear_extrude(height + 2) skadis_female_section();
    skadis_under_roof();
  }
  translate([-2.2 - skadis_fit, -3, -1]) cube([4.4 + 2*skadis_fit, 3.1, max(0.5, 67.3 + mount_shift)]);
}
module skadis_blade_profile() {
  offset(delta = -0.7) offset(r = 0.7) union() {
    translate([-9.6, 62]) square([12.6, 4]);
    translate([-9.6, 54]) square([4, 12]);
    translate([0, 42]) square([3, 24]);
  }
}
module skadis_friction_ridge() {
  hull() {
    translate([4.2, 2, 46]) cube([0.01, 0.6, 0.1]);
    translate([4.2, 2, 48]) cube([0.65, 0.6, 4]);
    translate([4.2, 2, 54]) cube([0.01, 0.6, 0.1]);
  }
}
module skadis_hook() {
  union() {
    if (skadis_friction_ridge) translate([0, 0, mount_shift]) skadis_friction_ridge();
    intersection() {
      translate([0, 0, 42 + mount_shift]) linear_extrude(36) skadis_male_section();
      translate([0, 0, -skadis_fit]) skadis_under_roof();
    }
    translate([-2.2, 0, mount_shift]) multmatrix([[0,0,1,0],[1,0,0,0],[0,1,0,0],[0,0,0,1]])
      linear_extrude(hook_width_skadis) skadis_blade_profile();
  }
}
module skadis_print_hook() {
  translate([10, 0, 2.2]) rotate([0, -90, 0]) translate([0, 0, -42 - mount_shift]) skadis_hook();
}
module skadis_body() {
  floor_slab();
  front_panel();
  left_panel();
  right_panel();
  back_panel();
  corner_posts();
  if (dividers > 0) for (i = [1:dividers])
    translate([divider_margin + i*divider_step - wall/2, back_wall, 0]) divider_slab();
}`
}

export function emitSkadisIntegralHooks(): string {
  return `module skadis_integral_hook(hook_w, gap, thick, lip, top_z) {
  union() {
    translate([0, -gap - thick, top_z - thick]) cube([hook_w, gap + thick + 0.3, thick]);
    translate([0, -gap - thick, top_z - thick - lip]) cube([hook_w, thick, lip + thick]);
    if (hook_chamfer) translate([0, 0, top_z - thick]) rotate([90, 0, 90]) linear_extrude(hook_w)
      polygon([[0, 0], [-gap, 0], [0, -min(lip, gap)]]);
  }
}
module skadis_integral_hooks() {
  hook_w = 4.4;
  gap = 5.4;
  thick = 3.2;
  lip = 10;
  top_z = height - skadis_top_offset;
  if (eff_hook_count > 0) for (i = [0:eff_hook_count - 1])
    translate([hook_first_x + i*hook_spacing_skadis - hook_w/2, 0, 0])
      skadis_integral_hook(hook_w, gap, thick, lip, top_z);
}`
}

export function emitRailMountModules(p: SkadisVoronoiBoxParams): string {
  if (p.mount_system === 'none') {
    return `module mount_sockets() {}
module attached_mounts() {}
module detached_mounts() {}`
  }
  return `module mount_socket_at(px) {
  slot_h = max(10, min(wall_height - wall*1.5, hook_drop + hook_thickness));
  slot_z = height - wall - slot_h;
  dove_d = max(1.2, wall*0.6);
  base_w = hook_width + 2*hook_tolerance;
  neck_w = max(1.8, base_w - 2*dove_d);
  translate([px, 0, slot_z]) linear_extrude(slot_h)
    polygon([[-neck_w/2, -0.1], [neck_w/2, -0.1], [base_w/2, dove_d], [-base_w/2, dove_d]]);
}
module mount_sockets() {
  if (hook_mount == 2 && eff_hook_count > 0) for (i = [0:eff_hook_count - 1])
    mount_socket_at(hook_first_x + i*eff_hook_step);
}
module attached_mounts() {
  if (hook_mount == 0 && eff_hook_count > 0) {
    hook_r = min(hook_thickness*0.85, hook_gap*0.35, hook_drop*0.35);
    for (i = [0:eff_hook_count - 1]) translate([hook_first_x + i*eff_hook_step - hook_width/2, 0, 0]) union() {
      translate([0, -hook_gap - hook_thickness, height - hook_thickness]) cube([hook_width, hook_gap + hook_thickness + wall, hook_thickness]);
      translate([0, -hook_gap - hook_thickness, height - hook_drop - hook_thickness]) cube([hook_width, hook_thickness, hook_drop + hook_thickness]);
      translate([0, 0, height - hook_drop - hook_thickness]) rotate([90, 0, 90]) linear_extrude(hook_width) union() {
        if (hook_chamfer) {
          polygon([[0, hook_drop], [-hook_gap, hook_drop], [0, hook_drop - min(hook_drop*0.95, hook_gap)]]);
          polygon([[-hook_gap - hook_thickness, 0], [-hook_gap, 0], [-hook_gap, -hook_thickness]]);
        } else {
          difference() { translate([-hook_r, hook_drop - hook_r]) square([hook_r + 0.02, hook_r + 0.02]); translate([-hook_r, hook_drop - hook_r]) circle(r = hook_r, $fn = 8); }
          difference() { translate([-hook_gap - 0.02, hook_drop - hook_r]) square([hook_r + 0.02, hook_r + 0.02]); translate([-hook_gap + hook_r, hook_drop - hook_r]) circle(r = hook_r, $fn = 8); }
        }
      }
    }
  }
}
module detached_mounts() {
  if (hook_mount > 0 && eff_hook_count > 0) {
    slot_h1 = max(hook_thickness*1.3, 3.6);
    stem_drop = max(wall*1.2, hook_thickness*1.2);
    t_h = max(hook_thickness*1.2, 3.2);
    slot_h2 = max(10, min(wall_height - wall*1.5, hook_drop + hook_thickness));
    dove_d = max(1.2, wall*0.6);
    base_w = hook_width;
    neck_w = max(1.8, base_w - 2*dove_d);
    tenon_d = max(0.4, dove_d - hook_tolerance);
    pitch_x = max(hook_gap + 3*hook_thickness + wall + 14, base_w + 14);
    base_y = -(hook_gap + hook_thickness + 14);
    for (i = [0:eff_hook_count - 1]) translate([(width - (eff_hook_count - 1)*pitch_x)/2 + i*pitch_x, base_y, 0]) {
      if (hook_mount == 1) {
        linear_extrude(max(1, hook_width - 2*hook_tolerance)) union() {
          translate([-hook_gap - hook_thickness, -hook_drop]) square([hook_thickness, hook_drop + hook_thickness]);
          translate([-hook_gap - hook_thickness, 0]) square([hook_gap + 2*hook_thickness + wall + hook_tolerance, hook_thickness]);
          translate([0, -stem_drop - slot_h1]) square([wall + hook_tolerance + hook_thickness, hook_thickness]);
          translate([wall + hook_tolerance, -stem_drop - slot_h1 - t_h]) square([hook_thickness, stem_drop + slot_h1 + t_h]);
          translate([wall + hook_tolerance + hook_thickness, -stem_drop - slot_h1 - t_h]) square([hook_thickness*0.9, t_h]);
        }
      } else {
        union() {
          linear_extrude(hook_width) union() {
            translate([-hook_gap - hook_thickness, -hook_drop]) square([hook_thickness, hook_drop + hook_thickness]);
            translate([-hook_gap - hook_thickness, 0]) square([hook_gap + hook_thickness, hook_thickness]);
            translate([-hook_thickness, -slot_h2 + hook_thickness]) square([hook_thickness, slot_h2]);
          }
          hull() {
            translate([-0.02, -slot_h2 + hook_thickness + hook_tolerance/2, (hook_width - neck_w)/2]) cube([0.04, max(2, slot_h2 - hook_tolerance), neck_w]);
            translate([tenon_d - 0.04, -slot_h2 + hook_thickness + hook_tolerance/2, 0]) cube([0.04, max(2, slot_h2 - hook_tolerance), base_w]);
          }
        }
      }
    }
  }
}`
}
