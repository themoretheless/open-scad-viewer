import { z } from 'zod/v4'

export const BOX_DEFAULTS = {
  width: 120, depth: 75, height: 80, wall: 3, floor: 4,
  bend_radius: 0, bend_reverse: false, bend_segments: 24, corner_radius: 0, front_angle: 0,
  porous: true, organic: true, print_optimized: true, overhang_angle: 45, cell_size: 40, rib: 2.4,
  stress_gradient: 1, mesh_density: 1, pattern_seed: 1,
  hole_diameter: 6, hole_pitch: 16,
  floor_mesh: true, floor_cell_size: 20, floor_rib: 2.4,
  dividers: 0, drain: false, hooks: true, hook_count: 2,
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
  organic: z.boolean().default(true), cell_size: dimension.default(40), rib: dimension.default(2.4),
  stress_gradient: z.number().finite().min(0).max(2).default(1),
  mesh_density: z.number().finite().min(0.5).max(2).default(1),
  print_optimized: z.boolean().default(true), overhang_angle: z.number().finite().min(10).max(80).default(45),
  pattern_seed: z.number().int().min(0).max(100000).default(1),
  hole_diameter: dimension, hole_pitch: dimension,
  floor_mesh: z.boolean().default(false), floor_cell_size: dimension.default(20), floor_rib: dimension.default(2.4),
  dividers: z.number().int().min(0).max(8), drain: z.boolean(),
  hooks: z.boolean(), hook_count: z.number().int().min(1).max(6),
  hook_width: dimension, hook_thickness: dimension.default(3), hook_gap: dimension, hook_drop: dimension,
  hook_spacing: z.number().finite().min(0).max(500).default(0), hook_chamfer: z.boolean().default(false),
}).strict()

/** Dimensions are external; organic walls retain a solid border and hook pads. */
export function generateBox(input: unknown = BOX_DEFAULTS): string {
  const p = schema.parse(input)
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
  if (p.organic && p.porous && p.cell_size / p.mesh_density < 4*p.rib) throw Error('Размер ячейки с учётом плотности должен быть минимум в 4 раза больше перемычки / Effective cell size must be at least four times the rib width')
  if(p.floor_mesh && p.floor_cell_size / p.mesh_density < 2*p.floor_rib) throw Error('Размер ячейки дна с учётом плотности должен быть минимум вдвое больше перемычки / Effective floor cell size must be at least twice the rib width')
  if(p.floor_mesh && p.floor_cell_size / p.mesh_density > Math.min(w,d)-2*Math.max(t,p.floor_rib*1.5)) throw Error('Ячейка дна не помещается в коробку: уменьшите её размер или увеличьте плотность / Floor cell does not fit; reduce its size or increase density')
  const topDepth = d+(h-f)*Math.tan(p.front_angle*Math.PI/180)
  if (topDepth <= 4*t) throw Error('Наклон закрывает коробку: увеличьте глубину или уменьшите угол / Tilt leaves insufficient opening depth')
  if (2*p.corner_radius >= Math.min(w,d,topDepth)) throw Error('Радиус не помещается в габариты коробки / Corner radius does not fit the box dimensions')
  if(p.bend_radius>0) {
    if(w/p.bend_radius>=2*Math.PI) throw Error('Дуга замыкается: увеличьте радиус изгиба / Increase bend radius to avoid wrapping around')
    if(p.bend_radius <= (p.bend_reverse?Math.max(d,topDepth):(p.hooks?p.hook_gap+2*p.hook_thickness:0))) throw Error('Изгиб пересекает ось: увеличьте радиус / Bend reaches its axis; increase radius')
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
    [ox+(ix+1)*cw/2+((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*cw*0.17,
      oy+(1.29*floor((iy+1)/3)+((iy+1)-3*floor((iy+1)/3)>1.5 ? 1 : 0.16))*ch*0.42+((sin((ix+1)*269.5+(iy+1)*183.3+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+1)*269.5+(iy+1)*183.3+pattern_seed*91.3+83)*43758.5453))-0.5)*ch*0.036+sin((ix+1)*25+floor((iy+1)/3)*103+pattern_seed*53)*ch*0.04],
    [ox+(ix+1)*cw/2+((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*cw*0.17,
      oy+(1.29*floor((iy-1)/3)+((iy-1)-3*floor((iy-1)/3)>1.5 ? 1 : 0.16))*ch*0.42+((sin((ix+1)*269.5+(iy-1)*183.3+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+1)*269.5+(iy-1)*183.3+pattern_seed*91.3+83)*43758.5453))-0.5)*ch*0.032+sin((ix+1)*25+floor((iy-1)/3)*103+pattern_seed*53)*ch*0.04],
    [ox+ix*cw/2+((sin(ix*127.1+(iy-2)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin(ix*127.1+(iy-2)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*cw*0.17,
      oy+(1.29*floor((iy-2)/3)+((iy-2)-3*floor((iy-2)/3)>1.5 ? 1 : 0.16))*ch*0.42+((sin(ix*269.5+(iy-2)*183.3+pattern_seed*91.3+83)*43758.5453)-floor((sin(ix*269.5+(iy-2)*183.3+pattern_seed*91.3+83)*43758.5453))-0.5)*ch*0.036+sin(ix*25+floor((iy-2)/3)*103+pattern_seed*53)*ch*0.04],
    [ox+(ix-1)*cw/2+((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*cw*0.17,
      oy+(1.29*floor((iy-1)/3)+((iy-1)-3*floor((iy-1)/3)>1.5 ? 1 : 0.16))*ch*0.42+((sin((ix-1)*269.5+(iy-1)*183.3+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-1)*269.5+(iy-1)*183.3+pattern_seed*91.3+83)*43758.5453))-0.5)*ch*0.032+sin((ix-1)*25+floor((iy-1)/3)*103+pattern_seed*53)*ch*0.04],
    [ox+(ix-1)*cw/2+((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*cw*0.17,
      oy+(1.29*floor((iy+1)/3)+((iy+1)-3*floor((iy+1)/3)>1.5 ? 1 : 0.16))*ch*0.42+((sin((ix-1)*269.5+(iy+1)*183.3+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-1)*269.5+(iy+1)*183.3+pattern_seed*91.3+83)*43758.5453))-0.5)*ch*0.036+sin((ix-1)*25+floor((iy+1)/3)*103+pattern_seed*53)*ch*0.04]
  ]);
}
module printable_pores(a,b,z_offset=0,slope=0,reserve_hooks=false) {
  frame=max(wall,rib*1.3);
  usable_h=b-z_offset-2*frame;
  usable_w=a+min((z_offset+frame)*slope,(b-frame)*slope)-2*frame;
  // Side-wall roof motion is radial; front/rear roof motion is circumferential.
  local_ratio=abs(slope)>0 ? (tan(overhang_angle)-abs(slope))*0.9 : roof_ratio;
  if(usable_h>rib*2 && usable_w>rib*2 && local_ratio>0) {
    rows=max(1,ceil(usable_h*mesh_density/(cell_size*0.65)));
    ch=usable_h/(0.542*rows+0.28);
    cw=ch*0.56*local_ratio;
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
module organic_pores(a,b,z_offset=0,slope=0,reserve_hooks=false) {
  if(print_optimized) printable_pores(a,b,z_offset,slope,reserve_hooks);
  else {
  r = cell_size/(2*mesh_density);
  frame = max(wall,rib*1.3);
  rounding = min(rib*0.65,cell_size/(10*mesh_density));
  if(a>2*frame && b-z_offset>2*frame) intersection() {
    offset(delta=-frame) intersection() { outline(a,b,slope); translate([0,z_offset]) square([max(a,a+b*slope),b-z_offset]); }
    union() for(col=[-1:ceil(max(a,a+b*slope)/(1.5*r))+1]) for(row=[-1:ceil(b/(sqrt(3)*r))+1]) {
      ix=3*col; iy=2*row+(abs(col)%2);
      local_rib=rib*max(0.45,1+stress_gradient*(0.12-0.24*min(1,max(0,(iy*sqrt(3)*r/2-z_offset)/max(1,b-z_offset)))));
      offset(r=rounding) offset(delta=-rounding-local_rib/2)
        polygon([
          [(ix+(2))*r/2+((sin((ix+(2))*127.1+(iy+(0))*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+(2))*127.1+(iy+(0))*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.3,
            (iy+(0))*sqrt(3)*r/2+((sin((ix+(2))*127.1+(iy+(0))*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+(2))*127.1+(iy+(0))*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*r*0.3],
          [(ix+(1))*r/2+((sin((ix+(1))*127.1+(iy+(1))*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+(1))*127.1+(iy+(1))*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.3,
            (iy+(1))*sqrt(3)*r/2+((sin((ix+(1))*127.1+(iy+(1))*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+(1))*127.1+(iy+(1))*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*r*0.3],
          [(ix+(-1))*r/2+((sin((ix+(-1))*127.1+(iy+(1))*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+(-1))*127.1+(iy+(1))*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.3,
            (iy+(1))*sqrt(3)*r/2+((sin((ix+(-1))*127.1+(iy+(1))*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+(-1))*127.1+(iy+(1))*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*r*0.3],
          [(ix+(-2))*r/2+((sin((ix+(-2))*127.1+(iy+(0))*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+(-2))*127.1+(iy+(0))*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.3,
            (iy+(0))*sqrt(3)*r/2+((sin((ix+(-2))*127.1+(iy+(0))*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+(-2))*127.1+(iy+(0))*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*r*0.3],
          [(ix+(-1))*r/2+((sin((ix+(-1))*127.1+(iy+(-1))*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+(-1))*127.1+(iy+(-1))*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.3,
            (iy+(-1))*sqrt(3)*r/2+((sin((ix+(-1))*127.1+(iy+(-1))*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+(-1))*127.1+(iy+(-1))*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*r*0.3],
          [(ix+(1))*r/2+((sin((ix+(1))*127.1+(iy+(-1))*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+(1))*127.1+(iy+(-1))*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.3,
            (iy+(-1))*sqrt(3)*r/2+((sin((ix+(1))*127.1+(iy+(-1))*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+(1))*127.1+(iy+(-1))*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*r*0.3]
        ]);
    }
  }
}
}
module apertures(a,b,perforated,z_offset=0,organic_panel=false,hook_pads=false,slope=0) {
    if(perforated && organic_panel) {
      if(print_optimized) organic_pores(a,b,z_offset,slope,hook_pads);
      else difference() {
        organic_pores(a,b,z_offset,slope);
        if(hook_pads && hooks) for(i=[0:hook_count-1])
          translate([(width-hook_step*(hook_count-1))/2+i*hook_step-radius-hook_width/2-rib,b-hook_drop-hook_thickness-rib])
            square([hook_width+2*rib,hook_drop+hook_thickness+rib]);
      }
    }
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
module panel(a,b,thickness,perforated,z_offset=0,organic_panel=false,hook_pads=false,slope=0) {
  linear_extrude(thickness) difference() {
    outline(a,b,slope);
    apertures(a,b,perforated,z_offset,organic_panel,hook_pads,slope);
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
  r=floor_cell_size*0.58/mesh_density;
  ox=width/2; oy=depth/2;
  nc=ceil((width-2*frame)/(3*r))+1;
  nr=ceil((depth-2*frame)/(sqrt(3)*r))+1;
  difference() {
    intersection() {
      offset(delta=-frame) footprint();
      union() for(col=[-nc:nc]) for(row=[-nr:nr]) {
        ix=3*col; iy=2*row+(abs(col)%2);
        // Plate-stress grading: thicker ribs near the perimeter wall junction, lighter in the center span.
        local_floor_rib=floor_rib*max(0.45,1+stress_gradient*(-0.18+0.32*min(1,max(abs(ix*r)/width,abs(iy*sqrt(3)*r)/depth))));
        offset(delta=-local_floor_rib/2)
          polygon([
            [ox+(ix+2)*r/2+((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+0)*sqrt(3)*r/2+((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*r*0.28],
            [ox+(ix+1)*r/2+((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+1)*sqrt(3)*r/2+((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*r*0.28],
            [ox+(ix-1)*r/2+((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+1)*sqrt(3)*r/2+((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-1)*127.1+(iy+1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*r*0.28],
            [ox+(ix-2)*r/2+((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy+0)*sqrt(3)*r/2+((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-2)*127.1+(iy+0)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*r*0.28],
            [ox+(ix-1)*r/2+((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy-1)*sqrt(3)*r/2+((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix-1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*r*0.28],
            [ox+(ix+1)*r/2+((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453)-floor((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+17)*43758.5453))-0.5)*r*0.28,
              oy+(iy-1)*sqrt(3)*r/2+((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453)-floor((sin((ix+1)*127.1+(iy-1)*311.7+pattern_seed*91.3+83)*43758.5453))-0.5)*r*0.28]
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
      apertures(width-2*radius,wall_height,porous && organic,0,organic,true);
    translate([radius,depth+1,floor]) multmatrix([[1,0,0,0],[0,1,slope,0],[0,0,1,0],[0,0,0,1]])
      rotate([90,0,0]) linear_extrude(wall+2) apertures(width-2*radius,wall_height,porous,0,organic);
    translate([-1,radius,floor]) rotate([90,0,90]) linear_extrude(wall+2)
      apertures(depth-2*radius,wall_height,porous,0,organic,false,slope);
    translate([width-wall-1,radius,floor]) rotate([90,0,90]) linear_extrude(wall+2)
      apertures(depth-2*radius,wall_height,porous,0,organic,false,slope);
  }
}
module walls() {
  if(radius>0) rounded_wall_shell();
  else {
    translate([0,0,floor]) rotate([90,0,0]) translate([0,0,-wall])
      panel(width,wall_height,wall,porous && organic,0,organic,true);
    translate([0,depth,floor]) multmatrix([[1,0,0,0],[0,1,slope,0],[0,0,1,0],[0,0,0,1]])
      rotate([90,0,0]) panel(width,wall_height,wall,porous,0,organic);
    translate([0,0,floor]) rotate([90,0,90]) panel(depth,wall_height,wall,porous,0,organic,false,slope);
    translate([width-wall,0,floor]) rotate([90,0,90]) panel(depth,wall_height,wall,porous,0,organic,false,slope);
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
  if(hooks) for(i=[0:hook_count-1])
    translate([(width-hook_step*(hook_count-1))/2+i*hook_step-hook_width/2,0,0]) union() {
      translate([0,-hook_thickness,height-hook_drop-hook_thickness])
        cube([hook_width,wall+hook_thickness,hook_drop+hook_thickness]);
      translate([0,-hook_gap-2*hook_thickness,height-hook_thickness])
        cube([hook_width,hook_gap+2*hook_thickness+wall,hook_thickness]);
      translate([0,-hook_gap-2*hook_thickness,height-hook_drop-hook_thickness])
        cube([hook_width,hook_thickness,hook_drop+hook_thickness]);
      if(hook_chamfer) {
        // Self-supporting 45-degree hook profile:
        // 1) 45-degree rear-wall root ramp
        translate([0,0,height-hook_drop-hook_thickness]) rotate([90,0,90]) linear_extrude(hook_width)
          polygon([[-hook_thickness,0],[0,0],[0,-hook_thickness]]);
        // 2) Full 45-degree inner rail-slot corner gusset across the entire rail gap (eliminates horizontal bridge ceiling)
        translate([0,0,height-hook_drop-hook_thickness]) rotate([90,0,90]) linear_extrude(hook_width)
          polygon([
            [-hook_thickness,hook_drop],
            [-hook_thickness-hook_gap,hook_drop],
            [-hook_thickness,hook_drop-min(hook_drop*0.95,hook_gap)]
          ]);
        // 3) 45-degree lower bevel under the outer downturned lip
        translate([0,0,height-hook_drop-hook_thickness]) rotate([90,0,90]) linear_extrude(hook_width)
          polygon([[-hook_gap-2*hook_thickness,0],[-hook_gap-hook_thickness,0],[-hook_gap-hook_thickness,-hook_thickness]]);
      }
    }
} }
module straight_box() { assembled_box(); }
// viewer_bend is a native viewer extension; change these settings to regenerate the curved mesh.
if(bend_radius>0) viewer_bend(radius=bend_radius,center=width/2,step=width/bend_segments,reverse=bend_reverse) straight_box();
else straight_box();
`
}
