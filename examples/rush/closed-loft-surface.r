// @rush/1
// Cyclic cubic ring through four sections; not an exact circular cylinder.
a = line_curve(start: [10mm,0mm,-2mm],end: [10mm,0mm,2mm])
b = line_curve(start: [0mm,10mm,-2mm],end: [0mm,10mm,2mm])
c = line_curve(start: [-10mm,0mm,-2mm],end: [-10mm,0mm,2mm])
d = line_curve(start: [0mm,-10mm,-2mm],end: [0mm,-10mm,2mm])
show closed_loft_surface(a,b,c,d,a,parameters: [0,1,2,3,4]).tessellate(segments_u: 8,segments_v: 48)
