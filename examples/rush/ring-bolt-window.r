// @rush/1
gear = brep_gear(module:1.5mm,teeth:40,internal:true,rim_width:13.4mm,height:2mm,clearance:0.1mm)
result = gear.ring_bolt_window(outer_radius_mm:45mm,hole_diameter_mm:4mm,hole_count:6,minimum_web_mm:1mm)
show result.brep_tessellate(3)
