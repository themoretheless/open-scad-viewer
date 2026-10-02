// @rush/1
a = line_curve(start: [0mm,0mm,0mm],end: [20mm,0mm,0mm])
b = line_curve(start: [0mm,0mm,20mm],end: [20mm,0mm,20mm])
c = line_curve(start: [0mm,0mm,40mm],end: [20mm,0mm,40mm])
g1 = line_curve(start: [5mm,0mm,0mm],end: [5mm,0mm,40mm])
g2 = line_curve(start: [15mm,0mm,40mm],end: [15mm,0mm,0mm])
show auto_guided_loft_surface(a,b,c,parameters: [0,0.25,1],guides: [g2,g1],budget: 0.000001mm).tessellate(segments_u: 24,segments_v: 32)
