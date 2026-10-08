// @rush/1
outer = bezier_curve(points: [[2mm,0,0],[2mm,1mm,0]])
inner = bezier_curve(points: [[1mm,0,0],[1mm,0.5mm,0]],weights: [1,2])
path = bezier_curve(points: [[0,0,0],[0,0,10mm]])
rail = bezier_curve(points: [[2mm,0,0],[0,4mm,10mm]])
show progressive_sweep(outer,inner,path,
 orientation_guide: rail,contact_profile: 0,contact_parameter: 0,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,3],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [1,0,0],initial_sections: 3,max_sections: 129,max_deviation: 0.001mm
).nurbs_patches_tessellate(segments: 4)
