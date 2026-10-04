// @rush/1
// Section degrees are aligned; sections interpolate at integer v stations.
a = bezier_curve(points: [[-10mm,0mm,0mm],[10mm,0mm,0mm]])
b = bezier_curve(points: [[-8mm,0mm,10mm],[0mm,8mm,12mm],[8mm,0mm,10mm]])
c = bezier_curve(points: [[-6mm,0mm,20mm],[0mm,-4mm,20mm],[6mm,0mm,20mm]])
show surface_loft(a,b,c).tessellate(segments_u: 16, segments_v: 32)
