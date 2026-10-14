// @rush/1
a = bezier_curve(points: [[0mm,0mm,0mm],[10mm,-3mm,2mm],[20mm,0mm,0mm]])
b = line_curve(start: [20mm,0mm,0mm],end: [25mm,12mm,1mm])
c = line_curve(start: [25mm,12mm,1mm],end: [10mm,24mm,2mm])
d = line_curve(start: [10mm,24mm,2mm],end: [-4mm,12mm,1mm])
e = line_curve(start: [-4mm,12mm,1mm],end: [0mm,0mm,0mm])
show boundary_fill(a,b,c,d,e,center: [10mm,10mm,5mm]).nurbs_patches_tessellate(segments: 12)
