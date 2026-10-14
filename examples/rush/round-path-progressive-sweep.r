// @rush/1
profile = bezier_curve(points: [[0.5mm,0,0],[1mm,0,0]])
path = round_polyline_curve(points: [[0,0,0],[0,0,10mm],[10mm,0,10mm],[10mm,10mm,10mm]], radius: 2mm)
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [1,0,0],orientation: "rmf",initial_sections: 5,max_sections: 1025,max_deviation: 0.01mm
).nurbs_patches_tessellate(4)
