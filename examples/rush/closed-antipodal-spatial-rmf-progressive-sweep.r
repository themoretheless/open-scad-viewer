// @rush/1
// Original closed C1 quadratic path with exact antipodal Bishop holonomy.
// Full principal-angle enclosure; source C1 does not certify retained G1/G2.
profile = bezier_curve(points: [[0.01mm,0,0],[0.02mm,0,0]])
path = nurbs_curve(degree: 2,knots: [0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 10],control_points: [[0mm,0mm,0mm],[1mm,0mm,0mm],[1mm,1mm,0mm],[1mm,2mm,0mm],[1mm,2mm,1mm],[1mm,2mm,2mm],[2mm,2mm,2mm],[3mm,2mm,2mm],[3mm,3mm,2mm],[3mm,4mm,2mm],[2mm,4mm,2mm],[1mm,4mm,2mm],[1mm,4mm,1mm],[1mm,4mm,0mm],[1mm,3mm,0mm],[1mm,2mm,0mm],[0mm,2mm,0mm],[-1mm,2mm,0mm],[-1mm,1mm,0mm],[-1mm,0mm,0mm],[0mm,0mm,0mm]],weights: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1])
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [0,0,1],orientation: "rmf",spacing: "arc_length",
 initial_sections: 65,max_sections: 65,length_tolerance: 0.001mm,length_max_cells: 100000,
 rmf_transport_steps: 4096,error_max_cells: 100000,error_max_products: 1000000,max_deviation: 1mm
).nurbs_patches_tessellate(segments: 4)
