// @rush/1
// Same-degree curves meet at one shared endpoint. The seam guarantees C0.
a = bezier_curve(points: [[0mm,0mm,0mm],[10mm,8mm,0mm],[20mm,0mm,0mm]], weights: [1,2,1])
b = bezier_curve(points: [[20mm,0mm,0mm],[30mm,-8mm,0mm],[40mm,0mm,0mm]], weights: [4,1,2])
show curve_compose(a,b).surface_extrude([0mm,0mm,12mm]).tessellate(segments_u: 16, segments_v: 32)
