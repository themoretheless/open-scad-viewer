// @rush/1
// Closed rational spatial C1 Bishop transport with unequal source spans.
profile = circle_curve(center: [1mm,0,0],normal: [0,1,1],radius: 0.05mm)
hole = circle_curve(center: [1mm,0,0],normal: [0,-1,-1],radius: 0.02mm)
path = nurbs_curve(degree: 3, knots: [0,0,0,0,0.125,0.125,0.125,0.375,0.375,0.375,0.75,0.75,0.75,1,1,1,1],control_points: [[1mm,0,0],[1mm,0.125mm,0.125mm],[0.125mm,1mm,1.125mm],[0,1mm,1mm],[-0.25mm,1mm,0.75mm],[-1mm,0.25mm,-0.25mm],[-1mm,0,0],[-1mm,-0.375mm,0.375mm],[-0.375mm,-1.1875mm,0.875mm],[0,-1mm,0.5mm],[0.25mm,-0.875mm,0.25mm],[1mm,-0.25mm,-0.25mm],[1mm,0,0]],weights: [1,1.25,1.25,1,1.25,1.25,1,1.25,1.25,1,1.25,1.25,1])
show brep_progressive_sweep([[profile],[hole]],path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1.1,1.1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [7.16197243913529deg,7.16197243913529deg],weights: [1,1]},
 axis_scale: {degree: 1,knots: [0,0,1,1],values: [[1,1.25,0.75],[1,1.25,0.75]],weights: [1,1]},
 center_law: {degree: 1,knots: [0,0,1,1],values: [[0.01mm,-0.02mm,0.03mm],[0.01mm,-0.02mm,0.03mm]],weights: [1,1]},
 normal: [1,0,0],orientation: "rmf",rmf_transport_steps: 4096,error_max_cells: 100000,error_max_products: 1000000,spacing: "arc_length",initial_sections: 65,max_sections: 65,length_tolerance: 0.001mm,length_max_cells: 100000,max_deviation: 0.4mm
).brep_tessellate(segments: 2)
