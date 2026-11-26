// @rush/1
part = brep_gear(module:1mm,teeth:8,height:2mm)
result = part.compact_body_step()
show result.brep_tessellate(3)
