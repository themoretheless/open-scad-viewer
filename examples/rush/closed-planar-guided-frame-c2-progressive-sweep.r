// @rush/1
// Original closed guided-frame C2; retained joins and Solid remain separate.
profile = bezier_curve(points: [[0,0,0.1mm],[0,0,0.2mm]])
path = nurbs_curve(degree: 7,knots: [2,2,2,2,2,2,2,2,5,5,5,5,5,5,5,5],control_points: [[0,0,0],[1mm,0,0],[2mm,1mm,0],[3mm,3mm,0],[-3mm,3mm,0],[-2mm,1mm,0],[-1mm,0,0],[0,0,0]],weights: [1,1,1,1,1,1,1,1])
rail = nurbs_curve(degree: 7,knots: [-3,-3,-3,-3,-3,-3,-3,-3,7,7,7,7,7,7,7,7],control_points: [[0,0,1mm],[1mm,0,1mm],[2mm,1mm,1mm],[3mm,3mm,1mm],[-3mm,3mm,1mm],[-2mm,1mm,1mm],[-1mm,0,1mm],[0,0,1mm]],weights: [1,1,1,1,1,1,1,1])
show progressive_sweep(profile,path,
 orientation_guide: rail,orientation: "rmf",normal: [0,0,1],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 5,knots: [7,7,7,7,7,7,9,9,9,9,9,9],values: [0deg,8deg,16deg,-16deg,-8deg,0deg],weights: [1,1,1,1,1,1]},
 initial_sections: 5,max_sections: 65,max_deviation: 2mm
).nurbs_patches_tessellate(segments: 8)
