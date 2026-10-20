// @rush/1
// Closed original C1 spatial Bishop transport with antipodal holonomy.
// Both principal-angle branches are enclosed; fresh Solid needs independent global proof.
profile = circle_curve(center: [0,0,0],normal: [1,0,0],radius: 0.05mm)
hole = circle_curve(center: [0,0,0],normal: [-1,0,0],radius: 0.02mm)
path = nurbs_curve(degree: 2,knots: [0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 10],control_points: [[0mm,0mm,0mm],[1mm,0mm,0mm],[1mm,1mm,0mm],[1mm,2mm,0mm],[1mm,2mm,1mm],[1mm,2mm,2mm],[2mm,2mm,2mm],[3mm,2mm,2mm],[3mm,3mm,2mm],[3mm,4mm,2mm],[2mm,4mm,2mm],[1mm,4mm,2mm],[1mm,4mm,1mm],[1mm,4mm,0mm],[1mm,3mm,0mm],[1mm,2mm,0mm],[0mm,2mm,0mm],[-1mm,2mm,0mm],[-1mm,1mm,0mm],[-1mm,0mm,0mm],[0mm,0mm,0mm]],weights: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1])
show brep_progressive_sweep([[profile],[hole]],path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1.1,1.1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [7.16197243913529deg,7.16197243913529deg],weights: [1,1]},
 axis_scale: {degree: 1,knots: [0,0,1,1],values: [[1,1.25,0.75],[1,1.25,0.75]],weights: [1,1]},
 center_law: {degree: 1,knots: [0,0,1,1],values: [[0.01mm,-0.02mm,0.03mm],[0.01mm,-0.02mm,0.03mm]],weights: [1,1]},
 normal: [0,0,1],orientation: "rmf",rmf_transport_steps: 4096,error_max_cells: 100000,error_max_products: 1000000,spacing: "arc_length",initial_sections: 65,max_sections: 65,length_tolerance: 0.001mm,length_max_cells: 100000,max_deviation: 1mm
).brep_tessellate(segments: 2)
