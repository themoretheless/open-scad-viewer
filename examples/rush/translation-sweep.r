// @rush/1
// Fixed profile orientation: P(u)+Q(v)-Q(0). This surface has no end caps.
profile = bezier_curve(points: [[-4mm,0mm,0mm],[0mm,0mm,5mm],[4mm,0mm,0mm]], weights: [1,2,1])
path = bezier_curve(points: [[0mm,0mm,0mm],[0mm,10mm,2mm],[12mm,20mm,6mm]], weights: [1,3,2])
show surface_sweep(profile,path).tessellate(segments_u: 16, segments_v: 32)
