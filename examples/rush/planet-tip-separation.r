// @rush/1
gear = brep_gear(module:1mm,teeth:20,height:2mm,pressure_angle:20,clearance:0.25mm,backlash:0.03mm)
mate = brep_gear(module:1mm,teeth:16,height:2mm,pressure_angle:20,internal:false,clearance:0.25mm,backlash:0.02mm)
result = gear.planet_tip_separation(target_geometry:mate,orbit_ranges_mm:[[31,31],[29,29]],separation_degrees:[180,180])
show result.brep_tessellate(3)
