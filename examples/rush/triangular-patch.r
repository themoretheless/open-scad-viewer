// @rush/1
base = bezier_curve(points: [[0mm,0mm,0mm],[10mm,-3mm,2mm],[20mm,0mm,0mm]])
left = bezier_curve(points: [[0mm,0mm,0mm],[-2mm,10mm,3mm],[8mm,20mm,5mm]])
right = bezier_curve(points: [[20mm,0mm,0mm],[22mm,10mm,-2mm],[8mm,20mm,5mm]])
show triangular_patch(base,left,right).tessellate(segments_u: 24,segments_v: 24)
