// @rush/1
part = brep_gear(module:1mm,teeth:8,height:2mm)
result = part.compact_product_step(product_name:"Spinner",material_name:"Сталь",face_index:0,layer_name:"Контроль",colour_rgb:[0.2,0.4,0.8])
show result.brep_tessellate(3)
