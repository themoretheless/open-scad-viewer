// @rush/1
// Original periodic spatial RMF with simultaneous rational laws and a cavity.
profile = circle_curve(center: [0.41666666666666663mm,0.8333333333333333mm,0.5mm],normal: [-0.75,0.5,0.5],radius: 0.05mm)
hole = circle_curve(center: [0.41666666666666663mm,0.8333333333333333mm,0.5mm],normal: [0.75,-0.5,-0.5],radius: 0.02mm)
path = nurbs_curve(degree: 3, knots: [0,1,2,3,4,5,6,7,8,9,10,11,12],control_points: [[1mm,0mm,0mm],[0.5mm,1mm,0.5mm],[-0.5mm,1mm,1mm],[-1mm,0mm,0mm],[-0.5mm,-1mm,0.25mm],[0.5mm,-1mm,-0.5mm],[1mm,0mm,0mm],[0.5mm,1mm,0.5mm],[-0.5mm,1mm,1mm]],weights: [1,1,1,1,1,1,1,1,1],periodic: true)
show brep_progressive_sweep([[profile],[hole]],path,
 scale: {degree: 2,knots: [0,0,0,1,1,1],values: [1.1,1.2,1.1],weights: [1,2,1]},
 twist: {degree: 2,knots: [0,0,0,1,1,1],values: [7.16197243913529deg,11.459155902616464deg,7.16197243913529deg],weights: [1,2,1]},
 axis_scale: {degree: 2,knots: [0,0,0,1,1,1],values: [[1,1.25,0.75],[1.2,1.1,0.8],[1,1.25,0.75]],weights: [1,2,1]},
 center_law: {degree: 2,knots: [0,0,0,1,1,1],values: [[0.01mm,-0.02mm,0.03mm],[0.02mm,0mm,0.01mm],[0.01mm,-0.02mm,0.03mm]],weights: [1,2,1]},
 normal: [1,0,0],orientation: "rmf",rmf_transport_steps: 4096,error_max_cells: 100000,error_max_products: 1000000,spacing: "parameter",initial_sections: 129,max_sections: 129,length_tolerance: 0.0001mm,length_max_cells: 100000,max_deviation: 0.25mm
).brep_tessellate(segments: 2)
