// @rush/1
profile = bezier_curve(points: [[0,0,1mm],[0,0,2mm]])
path = bezier_curve(points: [[0,0,0],[1mm,1mm,0],[2mm,-1mm,0],[3mm,0,0]])
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [0,0,1],orientation: "corrected_frenet",initial_sections: 5,max_sections: 257,max_deviation: 0.001mm
).nurbs_patches_tessellate(4)
