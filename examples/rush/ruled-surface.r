// @rush/1
// Compatible rational boundaries with equal weights; straight generators.
a = bezier_curve(points: [[-10mm,0mm,0mm],[0mm,8mm,0mm],[10mm,0mm,0mm]], weights: [1,2,1])
b = bezier_curve(points: [[-8mm,0mm,12mm],[0mm,-6mm,18mm],[8mm,0mm,12mm]], weights: [1,2,1])
show ruled_surface(a,b).tessellate(segments_u: 16, segments_v: 32)
