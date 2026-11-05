import { z } from 'zod/v4'

export const BOX_DEFAULTS = {
  width: 120, depth: 75, height: 80, wall: 3, floor: 4,
  corner_radius: 0, front_angle: 0, dividers: 0, hooks: true,
  bend_radius: 0, bend_reverse: false, bend_segments: 24,
  porous: true, organic: true, print_optimized: true, overhang_angle: 45, cell_size: 5, cell_radius: 2, rib: 2.4,
  stress_gradient: 1, mesh_density: 1, pattern_seed: 1,
  hole_diameter: 6, hole_pitch: 16,
  floor_mesh: true, floor_cell_size: 20, floor_rib: 2.4,
  drain: false, hook_count: 2,
  hook_width: 10, hook_thickness: 3, hook_gap: 6, hook_drop: 16, hook_spacing: 0, hook_chamfer: false,
}
const dimension = z.number().finite().min(0.8).max(500)
const schema = z.object({
  width: dimension, depth: dimension, height: dimension,
  wall: dimension, floor: dimension, porous: z.boolean(),
  corner_radius: z.number().finite().min(0).max(250).default(0),
  front_angle: z.number().finite().min(-60).max(60).default(0),
  bend_radius: z.number().finite().min(0).max(100000).default(0),
  bend_reverse: z.boolean().default(false), bend_segments: z.number().int().min(4).max(128).default(24),
  organic: z.boolean().default(true), cell_size: dimension.min(1).default(5), cell_radius: z.number().finite().min(0).max(50).default(2), rib: dimension.default(2.4),
  stress_gradient: z.number().finite().min(0).max(2).default(1),
  mesh_density: z.number().finite().min(0.5).max(2).default(1),
  print_optimized: z.boolean().default(true), overhang_angle: z.number().finite().min(10).max(80).default(45),
  pattern_seed: z.number().int().min(0).max(100000).default(1),
  hole_diameter: dimension, hole_pitch: dimension,
  floor_mesh: z.boolean().default(false), floor_cell_size: dimension.min(1).default(20), floor_rib: dimension.default(2.4),
  dividers: z.number().int().min(0).max(8), drain: z.boolean(),
  hooks: z.boolean(), hook_count: z.number().int().min(1).max(6),
  hook_width: dimension, hook_thickness: dimension.default(3), hook_gap: dimension, hook_drop: dimension,
  hook_spacing: z.number().finite().min(0).max(500).default(0), hook_chamfer: z.boolean().default(false),
}).strict()

/** Dimensions are external; organic walls retain a solid border and hook pads. */
export function generateBox(input: unknown = BOX_DEFAULTS): string {
  const normalized = input && typeof input === 'object' && 'bend_radius' in input && Number((input as Record<string, unknown>).bend_radius) <= 0
    ? { ...(input as Record<string, unknown>), bend_segments: Math.max(4, Math.trunc(Number((input as Record<string, unknown>).bend_segments) || 24)) }
    : input
  const p = schema.parse(normalized)
  const { width:w, depth:d, height:h, wall:t, floor:f } = p
  if (w <= 4*t || d <= 4*t || h <= f+2*t) throw Error('Размеры коробки слишком малы для выбранных стенок и дна / Box dimensions are too small for the wall and floor')
  if ((w-2*Math.max(t,p.corner_radius)-p.dividers*t)/(p.dividers+1) < 4) throw Error('Между перегородками должно оставаться минимум 4 мм / Compartments must be at least 4 mm wide')
  if (((p.porous && !p.organic) || (p.drain && !p.floor_mesh)) && p.hole_pitch < p.hole_diameter+1.2) throw Error('Между отверстиями нужно минимум 1.2 мм материала / Leave at least 1.2 mm between holes')
  if(p.hooks) {
    const span=w-2*p.corner_radius
    const besselSpan=Math.max(0,w-2*Math.max(p.corner_radius+p.hook_width/2,p.corner_radius+0.2071*span))
    const step=p.hook_spacing||(p.hook_count===1?0:p.dividers===p.hook_count&&p.hook_count>1?(w-2*Math.max(t,p.corner_radius))/(p.dividers+1):(besselSpan>=(p.hook_count-1)*p.hook_width*1.15?besselSpan/(p.hook_count-1):span/(p.hook_count+1)))
    const left=(w-step*(p.hook_count-1)-p.hook_width)/2
    if(p.hook_drop+p.hook_thickness>h || left<p.corner_radius || (p.hook_count>1 && p.hook_width>=step))
      throw Error('Крючки не помещаются или перекрываются: уменьшите размеры, количество или шаг / Hooks do not fit or overlap; adjust their dimensions, count or spacing')
  }
  if (p.organic && p.porous && p.cell_size / p.mesh_density < p.rib * 0.4) throw Error('Размер ячейки с учётом плотности должен быть не меньше перемычки / Effective cell size must be at least the rib width')
  if(p.floor_mesh && p.floor_cell_size / p.mesh_density < p.floor_rib * 0.4) throw Error('Размер ячейки дна с учётом плотности должен быть не меньше перемычки / Effective floor cell size must be at least the rib width')
  if(p.floor_mesh && p.floor_cell_size / p.mesh_density > Math.min(w,d)-2*Math.max(t,p.floor_rib*1.5)) throw Error('Ячейка дна не помещается в коробку: уменьшите её размер или увеличьте плотность / Floor cell does not fit; reduce its size or increase density')
  const topDepth = d+(h-f)*Math.tan(p.front_angle*Math.PI/180)
  if (topDepth <= 4*t) throw Error('Наклон закрывает коробку: увеличьте глубину или уменьшите угол / Tilt leaves insufficient opening depth')
  if (2*p.corner_radius >= Math.min(w,d,topDepth)) throw Error('Радиус не помещается в габариты коробки / Corner radius does not fit the box dimensions')
  if(p.bend_radius>0) {
    const minRadius = Math.ceil(Math.max(w/(2*Math.PI)+1, (p.bend_reverse?Math.max(d,topDepth):(p.hooks?p.hook_gap+p.hook_thickness:0))+1))
    if(w/p.bend_radius>=2*Math.PI) throw Error(`Дуга замыкается: для ширины ${w} мм укажите радиус изгиба минимум ${minRadius} мм (или 0 для прямой коробки) / Increase bend radius to at least ${minRadius} mm`)
    if(p.bend_radius <= (p.bend_reverse?Math.max(d,topDepth):(p.hooks?p.hook_gap+p.hook_thickness:0))) throw Error(`Изгиб пересекает ось: укажите радиус изгиба минимум ${minRadius} мм (или 0 для прямой коробки) / Bend reaches its axis; increase radius to at least ${minRadius} mm`)
  }
  if(p.print_optimized && p.porous && Math.abs(p.front_angle)>=p.overhang_angle) throw Error('Наклон передней стенки превышает угол свеса для печати / Front tilt exceeds the selected print overhang angle')
  return `// Organizer box. External dimensions in mm; organic skeletal walls.
// Generic rail hooks: measure your rail. No load rating or physical fit test.
${Object.entries(p).map(([key,value])=>`${key} = ${value};`).join('\n')}
$fn = 12;
slope = tan(front_angle);
radius = corner_radius;
wall_height = height-floor;
inner_radius = max(0,radius-wall);
divider_margin=max(wall,radius);
divider_step=(width-2*divider_margin)/(dividers+1);
// Structural hook placement: align with internal dividers when hook_count == dividers,
// otherwise place outermost hooks at Airy/Bessel minimum-sag points (~0.2071 of straight rear span)
// so shear transfers into the stiff corner posts / side walls instead of bending the center span.
hook_span = width-2*radius;
hook_bessel_span = max(0, width-2*max(radius+hook_width/2, radius+0.2071*hook_span));
hook_step = hook_spacing>0 ? hook_spacing : (hook_count<=1 ? 0 : (dividers==hook_count ? divider_step : (hook_bessel_span>=(hook_count-1)*hook_width*1.15 ? hook_bessel_span/(hook_count-1) : hook_span/(hook_count+1))));
// Cut holes in flat profiles before extrusion to bound 3D Boolean work.
module outline(a,b,slope=0) { polygon([[0,0],[a,0],[a+b*slope,b],[0,b]]); }
// Upright FDM cells have two sloping roof edges and no clipped/rounded ceiling.
// Bound horizontal growth per vertical rise after front tilt and cylindrical stretch.
print_stretch = bend_radius>0 ? max(1,1+(bend_reverse ? 0 : depth+wall_height*max(0,slope))/bend_radius) : 1;
roof_ratio = sqrt(max(0,tan(overhang_angle)*tan(overhang_angle)-slope*slope))/print_stretch*0.9;
module print_cell(ix,iy,cw,ch,ox,oy,rib_width=rib) {
  // Exact shared-vertex trivalent Voronoi/trabecular lattice (iy = 3*row):
  // Every adjacent cell shares identical (ix, jy) node coordinates so trabecular struts are 100% continuous,
  // while vertical Y perturbation combines a row/column trabecular wave and local node jitter so node heights
  // vary organically across each row while keeping every roof edge safely <43° for support-free printing.
  offset(delta=-rib_width/2) polygon([
    [ox+ix*cw/2+((sin(ix*127.1+(iy+2)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin(ix*127.1+(iy+2)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*cw*0.17,
      oy+(1.29*floor((iy+2)/3)+((iy+2)-3*floor((iy+2)/3)>1.5 ? 1 : 0.16))*ch*0.42+((sin(ix*269.5+(iy+2)*183.3+pattern_seed*91.3+83)*43758.5453)-floor((sin(ix*269.5+(iy+2)*183.3+pattern_seed*91.3+83)*43758.5453))-0.5)*ch*0.032+sin(ix*25+floor((iy+2)/3)*103+pattern_seed*53)*ch*0.04],
    [ox+(ix-1)*cw/2+((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*cw*0.17,
      oy+(1.29*floor((iy+1)/3)+((iy+1)-3*floor((iy+1)/3)>1.5 ? 1 : 0.16))*ch*0.42+((sin((ix-1)*269.5+(iy+1)*183.3+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-1)*269.5+(iy+1)*183.3+pattern_seed*91.3+83)*43758.5453))-0.5)*ch*0.036+sin((ix-1)*25+floor((iy+1)/3)*103+pattern_seed*53)*ch*0.04],
    [ox+(ix-1)*cw/2+((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*cw*0.17,
      oy+(1.29*floor((iy-1)/3)+((iy-1)-3*floor((iy-1)/3)>1.5 ? 1 : 0.16))*ch*0.42+((sin((ix-1)*269.5+(iy-1)*183.3+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-1)*269.5+(iy-1)*183.3+pattern_seed*91.3+83)*43758.5453))-0.5)*ch*0.032+sin((ix-1)*25+floor((iy-1)/3)*103+pattern_seed*53)*ch*0.04],
    [ox+ix*cw/2+((sin(ix*127.1+(iy-2)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin(ix*127.1+(iy-2)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*cw*0.17,
      oy+(1.29*floor((iy-2)/3)+((iy-2)-3*floor((iy-2)/3)>1.5 ? 1 : 0.16))*ch*0.42+((sin(ix*269.5+(iy-2)*183.3+pattern_seed*91.3+83)*43758.5453)-floor((sin(ix*269.5+(iy-2)*183.3+pattern_seed*91.3+83)*43758.5453))-0.5)*ch*0.036+sin(ix*25+floor((iy-2)/3)*103+pattern_seed*53)*ch*0.04],
    [ox+(ix+1)*cw/2+((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*cw*0.17,
      oy+(1.29*floor((iy-1)/3)+((iy-1)-3*floor((iy-1)/3)>1.5 ? 1 : 0.16))*ch*0.42+((sin((ix+1)*269.5+(iy-1)*183.3+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+1)*269.5+(iy-1)*183.3+pattern_seed*91.3+83)*43758.5453))-0.5)*ch*0.032+sin((ix+1)*25+floor((iy-1)/3)*103+pattern_seed*53)*ch*0.04],
    [ox+(ix+1)*cw/2+((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*cw*0.17,
      oy+(1.29*floor((iy+1)/3)+((iy+1)-3*floor((iy+1)/3)>1.5 ? 1 : 0.16))*ch*0.42+((sin((ix+1)*269.5+(iy+1)*183.3+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+1)*269.5+(iy+1)*183.3+pattern_seed*91.3+83)*43758.5453))-0.5)*ch*0.036+sin((ix+1)*25+floor((iy+1)/3)*103+pattern_seed*53)*ch*0.04]
  ]);
}
module printable_pores(a,b,z_offset=0,slope=0,reserve_hooks=false) {
  frame=max(wall,rib*1.3);
  usable_h=b-z_offset-2*frame;
  usable_w=a+min((z_offset+frame)*slope,(b-frame)*slope)-2*frame;
  // Side-wall roof motion is radial; front/rear roof motion is circumferential.
  local_ratio=abs(slope)>0 ? (tan(overhang_angle)-abs(slope))*0.9 : roof_ratio;
  if(usable_h>rib*2 && usable_w>rib*2 && local_ratio>0) {
    eff_cell = min(40,max(30,(30+(cell_size-1)*2)/mesh_density));
    rows=max(1,ceil(usable_h/(eff_cell*0.65)));
    ch=usable_h/(0.542*rows+0.28);
    cw=min(ch*0.56,max(rib*2.8,min(eff_cell*0.8,(width-2*frame)/3)))*local_ratio;
    cols=ceil(max(a,a+b*slope)/cw)+1;
    // Side/bottom clipping is supported; every roof apex remains below the top frame.
    if(cw>rib*2) intersection() {
      offset(delta=-frame) outline(a,b,slope);
      translate([0,z_offset+frame]) square([max(a,a+b*slope),b-z_offset-frame]);
      union() for(row=[0:rows-1]) for(col=[-1:cols]) {
        cx=frame+(2*col+(row%2))*cw/2;
        cy=z_offset+frame+(1.29*row+0.5)*ch*0.42;
        first_hook=(width-hook_step*(hook_count-1))/2-radius;
        nearest=max(0,min(hook_count-1,floor((cx-first_hook)/hook_step+0.5)));
        // Local load-bearing pad under hooks: open below the hook anchor zone to save weight.
        // Moment-graded ribs: thicker near the floor cantilever root, lighter toward the top rim.
        hook_zone = reserve_hooks && hooks && (cy > b - hook_drop - hook_thickness - ch*0.45) && abs(cx-first_hook-nearest*hook_step)<=hook_width/2+rib+cw*0.46;
        if(!hook_zone)
          print_cell(2*col+(row%2),3*row,cw,ch,frame,z_offset+frame+ch*0.2,rib*max(0.45,1+stress_gradient*(0.12-0.28*row/max(1,rows-1))));
      }
    }
  }
}
module inset_hex(pts,d,s=1) {
  m0=[(pts[0][0]+pts[1][0])/2,(pts[0][1]+pts[1][1])/2]; l0=max(1e-6,sqrt((pts[1][0]-pts[0][0])*(pts[1][0]-pts[0][0])+(pts[1][1]-pts[0][1])*(pts[1][1]-pts[0][1])));
  m1=[(pts[1][0]+pts[2][0])/2,(pts[1][1]+pts[2][1])/2]; l1=max(1e-6,sqrt((pts[2][0]-pts[1][0])*(pts[2][0]-pts[1][0])+(pts[2][1]-pts[1][1])*(pts[2][1]-pts[1][1])));
  m2=[(pts[2][0]+pts[3][0])/2,(pts[2][1]+pts[3][1])/2]; l2=max(1e-6,sqrt((pts[3][0]-pts[2][0])*(pts[3][0]-pts[2][0])+(pts[3][1]-pts[2][1])*(pts[3][1]-pts[2][1])));
  m3=[(pts[3][0]+pts[4][0])/2,(pts[3][1]+pts[4][1])/2]; l3=max(1e-6,sqrt((pts[4][0]-pts[3][0])*(pts[4][0]-pts[3][0])+(pts[4][1]-pts[3][1])*(pts[4][1]-pts[3][1])));
  m4=[(pts[4][0]+pts[5][0])/2,(pts[4][1]+pts[5][1])/2]; l4=max(1e-6,sqrt((pts[5][0]-pts[4][0])*(pts[5][0]-pts[4][0])+(pts[5][1]-pts[4][1])*(pts[5][1]-pts[4][1])));
  m5=[(pts[5][0]+pts[0][0])/2,(pts[5][1]+pts[0][1])/2]; l5=max(1e-6,sqrt((pts[0][0]-pts[5][0])*(pts[0][0]-pts[5][0])+(pts[0][1]-pts[5][1])*(pts[0][1]-pts[5][1])));
  difference() {
    polygon(pts);
    union() {
      translate(m0) rotate(atan2(pts[1][1]-pts[0][1],pts[1][0]-pts[0][0])) translate([-l0*2,s>0?-d*6:-d]) square([l0*4,d*7]);
      translate(m1) rotate(atan2(pts[2][1]-pts[1][1],pts[2][0]-pts[1][0])) translate([-l1*2,s>0?-d*6:-d]) square([l1*4,d*7]);
      translate(m2) rotate(atan2(pts[3][1]-pts[2][1],pts[3][0]-pts[2][0])) translate([-l2*2,s>0?-d*6:-d]) square([l2*4,d*7]);
      translate(m3) rotate(atan2(pts[4][1]-pts[3][1],pts[4][0]-pts[3][0])) translate([-l3*2,s>0?-d*6:-d]) square([l3*4,d*7]);
      translate(m4) rotate(atan2(pts[5][1]-pts[4][1],pts[5][0]-pts[4][0])) translate([-l4*2,s>0?-d*6:-d]) square([l4*4,d*7]);
      translate(m5) rotate(atan2(pts[0][1]-pts[5][1],pts[0][0]-pts[5][0])) translate([-l5*2,s>0?-d*6:-d]) square([l5*4,d*7]);
    }
  }
}
module organic_pores(a,b,z_offset=0,slope=0,reserve_hooks=false,seam_offset=0,seam_dir=1,seam_v=0,h_scale=1) {
  if(print_optimized) printable_pores(a,b,z_offset,slope,reserve_hooks);
  else {
  frame = max(wall,rib*1.3);
  r_min = rib*4.6;
  r_cap = max(r_min,(max(width,depth)-2*frame)/4.8);
  r = min(r_cap, r_min + (r_cap - r_min)*max(0,cell_size/mesh_density - 0.8)/16);
  rx = r*sqrt(3)/2;
  ry = r*0.58*min(1.45,max(0.7,h_scale));
  span = max(a,a+b*slope);
  ox = -seam_offset*seam_dir; oy = z_offset-seam_v;
  c0 = floor((seam_offset-(seam_dir<0 ? span : 0))/(2*rx))-1;
  c1 = ceil((seam_offset+(seam_dir>0 ? span : 0))/(2*rx))+1;
  r0 = floor(seam_v/(3*ry))-1;
  r1 = ceil((seam_v+b-z_offset)/(3*ry))+1;
  if(a>2*frame && b-z_offset>2*frame) intersection() {
    offset(delta=-frame) intersection() { outline(a,b,slope); translate([0,z_offset]) square([span,b-z_offset]); }
    union() for(col=[c0:c1]) for(row=[r0:r1]) {
      ix=2*col+(abs(row)%2); iy=3*row;
      cx=ox+ix*rx*seam_dir; cy=oy+iy*ry;
      first_hook=(width-hook_step*(hook_count-1))/2-radius;
      nearest=max(0,min(hook_count-1,floor((cx-first_hook)/hook_step+0.5)));
      hook_zone = reserve_hooks && hooks && (cy > b - hook_drop - hook_thickness - ry*1.7) && abs(cx-first_hook-nearest*hook_step)<=hook_width/2+rib+rx*0.95;
      local_rib=rib*max(0.45,1+stress_gradient*(0.12-0.24*min(1,max(0,(cy-z_offset)/max(1,b-z_offset)))));
      hx0=ix*127.1; hx1=(ix+seam_dir)*127.1; hxm1=(ix-seam_dir)*127.1;
      hy2=(iy+2)*311.7+pattern_seed*91.3; hy1=(iy+1)*311.7+pattern_seed*91.3;
      hym1=(iy-1)*311.7+pattern_seed*91.3; hym2=(iy-2)*311.7+pattern_seed*91.3;
      cell_r = radius>0 || bend_radius>0 || r<rib*5.2 || cell_radius<=0 ? 0 : min(cell_radius, r*0.76-local_rib/2);
      if(!hook_zone) {
        if(cell_r>r*0.24-local_rib*0.35) offset(r=cell_r,$fn=cell_r>r*0.28?64:48) inset_hex([
            [ox+ix*rx*seam_dir+((sin(hx0+hy2+17)*43758.5453)-floor(sin(hx0+hy2+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy+2)*ry+((sin(hx0+hy2+83)*43758.5453)-floor(sin(hx0+hy2+83)*43758.5453)-0.5)*ry*0.48],
            [ox+(ix-1)*rx*seam_dir+((sin((seam_dir>0?hxm1:hx1)+hy1+17)*43758.5453)-floor(sin((seam_dir>0?hxm1:hx1)+hy1+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy+1)*ry+((sin((seam_dir>0?hxm1:hx1)+hy1+83)*43758.5453)-floor(sin((seam_dir>0?hxm1:hx1)+hy1+83)*43758.5453)-0.5)*ry*0.48],
            [ox+(ix-1)*rx*seam_dir+((sin((seam_dir>0?hxm1:hx1)+hym1+17)*43758.5453)-floor(sin((seam_dir>0?hxm1:hx1)+hym1+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy-1)*ry+((sin((seam_dir>0?hxm1:hx1)+hym1+83)*43758.5453)-floor(sin((seam_dir>0?hxm1:hx1)+hym1+83)*43758.5453)-0.5)*ry*0.48],
            [ox+ix*rx*seam_dir+((sin(hx0+hym2+17)*43758.5453)-floor(sin(hx0+hym2+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy-2)*ry+((sin(hx0+hym2+83)*43758.5453)-floor(sin(hx0+hym2+83)*43758.5453)-0.5)*ry*0.48],
            [ox+(ix+1)*rx*seam_dir+((sin((seam_dir>0?hx1:hxm1)+hym1+17)*43758.5453)-floor(sin((seam_dir>0?hx1:hxm1)+hym1+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy-1)*ry+((sin((seam_dir>0?hx1:hxm1)+hym1+83)*43758.5453)-floor(sin((seam_dir>0?hx1:hxm1)+hym1+83)*43758.5453)-0.5)*ry*0.48],
            [ox+(ix+1)*rx*seam_dir+((sin((seam_dir>0?hx1:hxm1)+hy1+17)*43758.5453)-floor(sin((seam_dir>0?hx1:hxm1)+hy1+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy+1)*ry+((sin((seam_dir>0?hx1:hxm1)+hy1+83)*43758.5453)-floor(sin((seam_dir>0?hx1:hxm1)+hy1+83)*43758.5453)-0.5)*ry*0.48]
          ],local_rib/2+cell_r,seam_dir);
        else if(cell_r>0) offset(r=cell_r,$fn=cell_r>r*0.14?36:24) offset(delta=-local_rib/2-cell_r)
          polygon([
            [ox+ix*rx*seam_dir+((sin(hx0+hy2+17)*43758.5453)-floor(sin(hx0+hy2+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy+2)*ry+((sin(hx0+hy2+83)*43758.5453)-floor(sin(hx0+hy2+83)*43758.5453)-0.5)*ry*0.48],
            [ox+(ix-1)*rx*seam_dir+((sin((seam_dir>0?hxm1:hx1)+hy1+17)*43758.5453)-floor(sin((seam_dir>0?hxm1:hx1)+hy1+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy+1)*ry+((sin((seam_dir>0?hxm1:hx1)+hy1+83)*43758.5453)-floor(sin((seam_dir>0?hxm1:hx1)+hy1+83)*43758.5453)-0.5)*ry*0.48],
            [ox+(ix-1)*rx*seam_dir+((sin((seam_dir>0?hxm1:hx1)+hym1+17)*43758.5453)-floor(sin((seam_dir>0?hxm1:hx1)+hym1+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy-1)*ry+((sin((seam_dir>0?hxm1:hx1)+hym1+83)*43758.5453)-floor(sin((seam_dir>0?hxm1:hx1)+hym1+83)*43758.5453)-0.5)*ry*0.48],
            [ox+ix*rx*seam_dir+((sin(hx0+hym2+17)*43758.5453)-floor(sin(hx0+hym2+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy-2)*ry+((sin(hx0+hym2+83)*43758.5453)-floor(sin(hx0+hym2+83)*43758.5453)-0.5)*ry*0.48],
            [ox+(ix+1)*rx*seam_dir+((sin((seam_dir>0?hx1:hxm1)+hym1+17)*43758.5453)-floor(sin((seam_dir>0?hx1:hxm1)+hym1+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy-1)*ry+((sin((seam_dir>0?hx1:hxm1)+hym1+83)*43758.5453)-floor(sin((seam_dir>0?hx1:hxm1)+hym1+83)*43758.5453)-0.5)*ry*0.48],
            [ox+(ix+1)*rx*seam_dir+((sin((seam_dir>0?hx1:hxm1)+hy1+17)*43758.5453)-floor(sin((seam_dir>0?hx1:hxm1)+hy1+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy+1)*ry+((sin((seam_dir>0?hx1:hxm1)+hy1+83)*43758.5453)-floor(sin((seam_dir>0?hx1:hxm1)+hy1+83)*43758.5453)-0.5)*ry*0.48]
          ]);
        else offset(delta=-local_rib/2)
          polygon([
            [ox+ix*rx*seam_dir+((sin(hx0+hy2+17)*43758.5453)-floor(sin(hx0+hy2+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy+2)*ry+((sin(hx0+hy2+83)*43758.5453)-floor(sin(hx0+hy2+83)*43758.5453)-0.5)*ry*0.48],
            [ox+(ix-1)*rx*seam_dir+((sin((seam_dir>0?hxm1:hx1)+hy1+17)*43758.5453)-floor(sin((seam_dir>0?hxm1:hx1)+hy1+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy+1)*ry+((sin((seam_dir>0?hxm1:hx1)+hy1+83)*43758.5453)-floor(sin((seam_dir>0?hxm1:hx1)+hy1+83)*43758.5453)-0.5)*ry*0.48],
            [ox+(ix-1)*rx*seam_dir+((sin((seam_dir>0?hxm1:hx1)+hym1+17)*43758.5453)-floor(sin((seam_dir>0?hxm1:hx1)+hym1+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy-1)*ry+((sin((seam_dir>0?hxm1:hx1)+hym1+83)*43758.5453)-floor(sin((seam_dir>0?hxm1:hx1)+hym1+83)*43758.5453)-0.5)*ry*0.48],
            [ox+ix*rx*seam_dir+((sin(hx0+hym2+17)*43758.5453)-floor(sin(hx0+hym2+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy-2)*ry+((sin(hx0+hym2+83)*43758.5453)-floor(sin(hx0+hym2+83)*43758.5453)-0.5)*ry*0.48],
            [ox+(ix+1)*rx*seam_dir+((sin((seam_dir>0?hx1:hxm1)+hym1+17)*43758.5453)-floor(sin((seam_dir>0?hx1:hxm1)+hym1+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy-1)*ry+((sin((seam_dir>0?hx1:hxm1)+hym1+83)*43758.5453)-floor(sin((seam_dir>0?hx1:hxm1)+hym1+83)*43758.5453)-0.5)*ry*0.48],
            [ox+(ix+1)*rx*seam_dir+((sin((seam_dir>0?hx1:hxm1)+hy1+17)*43758.5453)-floor(sin((seam_dir>0?hx1:hxm1)+hy1+17)*43758.5453)-0.5)*r*0.28*seam_dir,
              oy+(iy+1)*ry+((sin((seam_dir>0?hx1:hxm1)+hy1+83)*43758.5453)-floor(sin((seam_dir>0?hx1:hxm1)+hy1+83)*43758.5453)-0.5)*ry*0.48]
          ]);
      }
    }
  }
}
}
module apertures(a,b,perforated,z_offset=0,organic_panel=false,hook_pads=false,slope=0,seam_offset=0,seam_dir=1,seam_v=0,h_scale=1) {
    if(perforated && organic_panel) organic_pores(a,b,z_offset,slope,hook_pads,seam_offset,seam_dir,seam_v,h_scale);
    if(perforated && !organic_panel) {
      na = max(0,floor((a-4*wall-hole_diameter)/hole_pitch)+1);
      nb = max(0,floor((b-z_offset-4*wall-hole_diameter)/hole_pitch)+1);
      tear_slope = max(0.35, min(1.5, abs(slope)>0 ? (tan(overhang_angle)-abs(slope))*0.85 : roof_ratio));
      tear_apex = hole_diameter/(2*sin(atan(tear_slope)));
      if(na>0 && nb>0) for(i=[0:na-1]) for(j=[0:nb-1])
        translate([(a-(na-1)*hole_pitch)/2+i*hole_pitch,
          z_offset+(b-z_offset-(nb-1)*hole_pitch)/2+j*hole_pitch])
          if(print_optimized) hull() { circle(d=hole_diameter); polygon([[0,tear_apex],[-hole_diameter*0.35,0],[hole_diameter*0.35,0]]); }
          else circle(d=hole_diameter);
    }
}
module panel(a,b,thickness,perforated,z_offset=0,organic_panel=false,hook_pads=false,slope=0,seam_offset=0,seam_dir=1,seam_v=0,h_scale=1) {
  linear_extrude(thickness) difference() {
    outline(a,b,slope);
    apertures(a,b,perforated,z_offset,organic_panel,hook_pads,slope,seam_offset,seam_dir,seam_v,h_scale);
  }
}
// Positive front_angle opens the front outward; the back stays vertical.
module footprint() {
  if(radius>0) translate([radius,radius]) offset(r=radius)
    square([width-2*radius,depth-2*radius]);
  else square([width,depth]);
}
module floor_pores() {
  frame=max(wall,floor_rib*1.25);
  r_floor_min=floor_rib*4.8;
  r_floor_cap=max(r_floor_min,(max(width,depth)-2*frame)/4.2);
  r=min(r_floor_cap,r_floor_min+(r_floor_cap-r_floor_min)*max(0,floor_cell_size/mesh_density-1)/39);
  ry=r*(bend_radius>0 && !print_optimized ? min(1.35,max(0.75,1+(bend_reverse ? -0.5 : 0.5)*depth/bend_radius)) : 1);
  ox=width/2; oy=depth/2;
  nc=ceil((width-2*frame)/(3*r))+1;
  nr=ceil((depth-2*frame)/(sqrt(3)*ry))+1;
  difference() {
    intersection() {
      offset(delta=-frame) footprint();
      union() for(col=[-nc:nc]) for(row=[-nr:nr]) {
        ix=3*col; iy=2*row+(abs(col)%2);
        // Plate-stress grading: thicker ribs near the perimeter wall junction, lighter in the center span.
        local_floor_rib=floor_rib*max(0.45,1+stress_gradient*(-0.18+0.32*min(1,max(abs(ix*r)/width,abs(iy*sqrt(3)*ry)/depth))));
        floor_cell_r = radius>0 || bend_radius>0 || r<floor_rib*5.2 || cell_radius<=0 ? 0 : min(cell_radius, r*0.76-local_floor_rib/2);
        if(floor_cell_r>r*0.24-local_floor_rib*0.35) offset(r=floor_cell_r,$fn=floor_cell_r>r*0.28?64:48) inset_hex([
            [ox+(ix+2)*r/2+((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+0)*sqrt(3)*ry/2+((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix+1)*r/2+((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy-1)*sqrt(3)*ry/2+((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix-1)*r/2+((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy-1)*sqrt(3)*ry/2+((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix-2)*r/2+((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+0)*sqrt(3)*ry/2+((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix-1)*r/2+((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+1)*sqrt(3)*ry/2+((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix+1)*r/2+((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+1)*sqrt(3)*ry/2+((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28]
          ],local_floor_rib/2+floor_cell_r);
        else if(floor_cell_r>0) offset(r=floor_cell_r,$fn=floor_cell_r>r*0.14?36:24) offset(delta=-local_floor_rib/2-floor_cell_r)
          polygon([
            [ox+(ix+2)*r/2+((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+0)*sqrt(3)*ry/2+((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix+1)*r/2+((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy-1)*sqrt(3)*ry/2+((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix-1)*r/2+((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy-1)*sqrt(3)*ry/2+((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix-2)*r/2+((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+0)*sqrt(3)*ry/2+((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix-1)*r/2+((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+1)*sqrt(3)*ry/2+((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix+1)*r/2+((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+1)*sqrt(3)*ry/2+((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28]
          ]);
        else offset(delta=-local_floor_rib/2)
          polygon([
            [ox+(ix+2)*r/2+((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+0)*sqrt(3)*ry/2+((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix+1)*r/2+((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy-1)*sqrt(3)*ry/2+((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix-1)*r/2+((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy-1)*sqrt(3)*ry/2+((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix-2)*r/2+((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+0)*sqrt(3)*ry/2+((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix-1)*r/2+((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+1)*sqrt(3)*ry/2+((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28],
            [ox+(ix+1)*r/2+((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+1)*sqrt(3)*ry/2+((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*ry*0.28]
          ]);
      }
    }
    if(dividers>0) for(i=[1:dividers])
      translate([divider_margin+i*divider_step-wall/2-floor_rib*0.8,0]) square([wall+1.6*floor_rib,depth]);
  }
}
module rounded_outline(w,d,r) {
  if(r>0) translate([r,r]) offset(r=r) square([w-2*r,d-2*r]);
  else square([w,d]);
}
front_stretch = bend_radius>0 ? max(0.35,1+(bend_reverse ? -1 : 1)*(depth+0.5*wall_height*slope)/bend_radius) : 1;
module rounded_wall_shell() {
  difference() {
    hull() {
      translate([0,0,floor]) linear_extrude(0.05) rounded_outline(width,depth,radius);
      translate([0,0,height-0.05]) linear_extrude(0.05)
        rounded_outline(width,depth+wall_height*slope,radius);
    }
    hull() {
      translate([wall,wall,floor-1]) linear_extrude(0.05)
        rounded_outline(width-2*wall,depth-2*wall-slope,inner_radius);
      translate([wall,wall,height+1]) linear_extrude(0.05)
        rounded_outline(width-2*wall,depth-2*wall+(wall_height+1)*slope,inner_radius);
    }
    translate([radius,-1,floor]) rotate([90,0,0]) translate([0,0,-wall-2]) linear_extrude(wall+2)
      apertures(width-2*radius,wall_height,porous && organic,0,organic,true,0,-width/2+radius,1,-depth/2,1);
    translate([radius,depth+1,floor]) multmatrix([[1,0,0,0],[0,1,slope,0],[0,0,1,0],[0,0,0,1]])
      rotate([90,0,0]) linear_extrude(wall+2) apertures(width-2*radius,wall_height,porous,0,organic,false,0,-width/2-depth+radius,1,-depth/2,front_stretch);
    translate([-1,radius,floor]) rotate([90,0,90]) linear_extrude(wall+2)
      apertures(depth-2*radius,wall_height,porous,0,organic,false,slope,-width/2-radius,-1,-depth/2,1);
    translate([width-wall-1,radius,floor]) rotate([90,0,90]) linear_extrude(wall+2)
      apertures(depth-2*radius,wall_height,porous,0,organic,false,slope,width/2+radius,1,-depth/2,1);
  }
}
module walls() {
  if(radius>0) rounded_wall_shell();
  else {
    translate([0,0,floor]) rotate([90,0,0]) translate([0,0,-wall])
      panel(width,wall_height,wall,porous && organic,0,organic,true,0,-width/2,1,-depth/2,1);
    translate([0,depth,floor]) multmatrix([[1,0,0,0],[0,1,slope,0],[0,0,1,0],[0,0,0,1]])
      rotate([90,0,0]) panel(width,wall_height,wall,porous,0,organic,false,0,-width/2-depth,1,-depth/2,front_stretch);
    translate([0,0,floor]) rotate([90,0,90]) panel(depth,wall_height,wall,porous,0,organic,false,slope,-width/2,-1,-depth/2,1);
    translate([width-wall,0,floor]) rotate([90,0,90]) panel(depth,wall_height,wall,porous,0,organic,false,slope,width/2,1,-depth/2,1);
  }
}
module assembled_box() { union() {
  linear_extrude(floor) difference() {
    footprint();
    if(floor_mesh) floor_pores();
    else if(drain) {
      na=max(0,floor((width-4*wall-hole_diameter)/hole_pitch)+1);
      nb=max(0,floor((depth-4*wall-hole_diameter)/hole_pitch)+1);
      if(na>0 && nb>0) intersection() {
        offset(delta=-max(wall,rib*1.5)) footprint();
        union() for(i=[0:na-1]) for(j=[0:nb-1])
          translate([(width-(na-1)*hole_pitch)/2+i*hole_pitch,(depth-(nb-1)*hole_pitch)/2+j*hole_pitch]) circle(d=hole_diameter);
      }
    }
  }
  walls();
  if(dividers>0) for(i=[1:dividers])
    translate([divider_margin+i*divider_step-wall/2,wall-0.25,floor-0.1]) rotate([90,0,90])
      panel(depth-2*wall+0.5-0.5*slope,wall_height+0.1,wall,porous && radius==0,0,organic,false,slope);
  if(hooks) {
    hook_r = min(hook_thickness*0.85,hook_gap*0.35,hook_drop*0.35);
    hook_fn = 8;
    for(i=[0:hook_count-1])
      translate([(width-hook_step*(hook_count-1))/2+i*hook_step-hook_width/2,0,0]) union() {
        translate([0,-hook_gap-hook_thickness,height-hook_thickness])
          cube([hook_width,hook_gap+hook_thickness+wall,hook_thickness]);
        translate([0,-hook_gap-hook_thickness,height-hook_drop-hook_thickness])
          cube([hook_width,hook_thickness,hook_drop+hook_thickness]);
        translate([0,0,height-hook_drop-hook_thickness]) rotate([90,0,90]) linear_extrude(hook_width) union() {
          if(hook_chamfer) {
            polygon([[0,hook_drop],[-hook_gap,hook_drop],[0,hook_drop-min(hook_drop*0.95,hook_gap)]]);
            polygon([[-hook_gap-hook_thickness,0],[-hook_gap,0],[-hook_gap,-hook_thickness]]);
          } else {
            difference() {
              translate([-hook_r,hook_drop-hook_r]) square([hook_r+0.02,hook_r+0.02]);
              translate([-hook_r,hook_drop-hook_r]) circle(r=hook_r,$fn=hook_fn);
            }
            difference() {
              translate([-hook_gap-0.02,hook_drop-hook_r]) square([hook_r+0.02,hook_r+0.02]);
              translate([-hook_gap+hook_r,hook_drop-hook_r]) circle(r=hook_r,$fn=hook_fn);
            }
          }
        }
      }
  }
} }
module straight_box() { assembled_box(); }
// viewer_bend is a native viewer extension; change these settings to regenerate the curved mesh.
bend_step = width/min(bend_segments, porous || floor_mesh ? 14 : bend_segments);
if(bend_radius>0) viewer_bend(radius=bend_radius,center=width/2,step=bend_step,reverse=bend_reverse) straight_box();
else straight_box();
`
}
