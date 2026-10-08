// @rush/1
// Bottom/top run in +u; left/right run in +v. Shared corners coincide exactly.
bottom = bezier_curve(points: [[0mm,0mm,0mm],[10mm,0mm,5mm],[20mm,0mm,0mm]], weights: [1,2,1])
top = bezier_curve(points: [[0mm,16mm,0mm],[10mm,16mm,-3mm],[20mm,16mm,0mm]], weights: [1,2,1])
left = bezier_curve(points: [[0mm,0mm,0mm],[0mm,8mm,4mm],[0mm,16mm,0mm]], weights: [1,2,1])
right = bezier_curve(points: [[20mm,0mm,0mm],[20mm,8mm,-2mm],[20mm,16mm,0mm]], weights: [1,2,1])
show coons_patch(bottom,top,left,right).tessellate(segments_u: 16, segments_v: 32)
