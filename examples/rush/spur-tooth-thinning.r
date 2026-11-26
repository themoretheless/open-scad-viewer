// @rush/1
gear = brep_gear(module:1mm,teeth:20,height:2mm,pressure_angle:20,clearance:0.25mm,backlash:0.03mm)
result = gear.spur_tooth_thinning(measuring_radius_mm:10mm,measured_chord_mm:[1.53,1.55])
show result.brep_tessellate(3)
