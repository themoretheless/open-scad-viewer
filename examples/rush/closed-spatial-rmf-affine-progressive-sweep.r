// @rush/1
// Closed spatial C1 Bishop transport with nonzero holonomy.
profile = bezier_curve(points: [[1.1mm,0,0],[1.2mm,0,0]])
path = nurbs_curve(degree: 3, knots: [0,0,0,0,0.25,0.25,0.25,0.5,0.5,0.5,0.75,0.75,0.75,1,1,1,1],control_points: [[1mm,0mm,0mm],[1mm,0.25mm,0.25mm],[0.25mm,1mm,1.25mm],[0mm,1mm,1mm],[-0.25mm,1mm,0.75mm],[-1mm,0.25mm,-0.25mm],[-1mm,0mm,0mm],[-1mm,-0.25mm,0.25mm],[-0.25mm,-1.125mm,0.75mm],[0mm,-1mm,0.5mm],[0.25mm,-0.875mm,0.25mm],[1mm,-0.25mm,-0.25mm],[1mm,0mm,0mm]],weights: [1,1,1,1,1,1,1,1,1,1,1,1,1])
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1.1,1.1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [7.16197243913529deg,7.16197243913529deg],weights: [1,1]},
 axis_scale: {degree: 1,knots: [0,0,1,1],values: [[1,1.25,0.75],[1,1.25,0.75]],weights: [1,1]},
 center_law: {degree: 1,knots: [0,0,1,1],values: [[0.01mm,-0.02mm,0.03mm],[0.01mm,-0.02mm,0.03mm]],weights: [1,1]},
 normal: [1,0,0],orientation: "rmf",spacing: "arc_length",initial_sections: 5,max_sections: 17,length_tolerance: 0.01mm,length_max_cells: 100000,max_deviation: 3mm
).nurbs_patches_tessellate(segments: 4)
