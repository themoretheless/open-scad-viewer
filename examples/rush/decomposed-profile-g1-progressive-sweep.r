// @rush/1
// Internal profile joins have G1; alternating curvature prevents G2.
part0 = bezier_curve(points: [[0mm,0mm,0mm],[0.5mm,0mm,0mm],[1mm,0.125mm,0mm]])
part1 = bezier_curve(points: [[1mm,0.125mm,0mm],[1.5mm,0.25mm,0mm],[2mm,0.25mm,0mm]])
part2 = bezier_curve(points: [[2mm,0.25mm,0mm],[2.5mm,0.25mm,0mm],[3mm,0.375mm,0mm]])
part3 = bezier_curve(points: [[3mm,0.375mm,0mm],[3.5mm,0.5mm,0mm],[4mm,0.5mm,0mm]])
part4 = bezier_curve(points: [[4mm,0.5mm,0mm],[4.5mm,0.5mm,0mm],[5mm,0.625mm,0mm]])
part5 = bezier_curve(points: [[5mm,0.625mm,0mm],[5.5mm,0.75mm,0mm],[6mm,0.75mm,0mm]])
part6 = bezier_curve(points: [[6mm,0.75mm,0mm],[6.5mm,0.75mm,0mm],[7mm,0.875mm,0mm]])
part7 = bezier_curve(points: [[7mm,0.875mm,0mm],[7.5mm,1mm,0mm],[8mm,1mm,0mm]])
part8 = bezier_curve(points: [[8mm,1mm,0mm],[8.5mm,1mm,0mm],[9mm,1.125mm,0mm]])
part9 = bezier_curve(points: [[9mm,1.125mm,0mm],[9.5mm,1.25mm,0mm],[10mm,1.25mm,0mm]])
part10 = bezier_curve(points: [[10mm,1.25mm,0mm],[10.5mm,1.25mm,0mm],[11mm,1.375mm,0mm]])
part11 = bezier_curve(points: [[11mm,1.375mm,0mm],[11.5mm,1.5mm,0mm],[12mm,1.5mm,0mm]])
part12 = bezier_curve(points: [[12mm,1.5mm,0mm],[12.5mm,1.5mm,0mm],[13mm,1.625mm,0mm]])
part13 = bezier_curve(points: [[13mm,1.625mm,0mm],[13.5mm,1.75mm,0mm],[14mm,1.75mm,0mm]])
part14 = bezier_curve(points: [[14mm,1.75mm,0mm],[14.5mm,1.75mm,0mm],[15mm,1.875mm,0mm]])
part15 = bezier_curve(points: [[15mm,1.875mm,0mm],[15.5mm,2mm,0mm],[16mm,2mm,0mm]])
profile = curve_compose(part0,part1,part2,part3,part4,part5,part6,part7,part8,part9,part10,part11,part12,part13,part14,part15)
path = line_curve(start: [0mm,0mm,0mm],end: [0mm,0mm,4mm])
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [1,0,0],orientation: "fixed",initial_sections: 2,max_sections: 2,max_deviation: 0.01mm
).nurbs_patches_tessellate(segments: 2)
