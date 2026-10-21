export function primitiveCreationError(message:string,invalid:boolean,label:(ru:string,en:string)=>string,code=''):string {
return invalid?label('Проверьте внутренний радиус и уменьшите G2-переходы: сумма соседних размеров должна помещаться в общую толщину стенки или высоту.','Check the inner radius and reduce the G2 transition sizes: adjacent sizes must fit the shared wall thickness or height.'):
   (code==='BREP_G2_GLOBAL_UNPROVEN'||message.includes('BREP_G2_GLOBAL_UNPROVEN'))?label('Замкнутость и отсутствие пересечений G2-тела не подтверждены. Уменьшите переход и повторите создание.','The G2 body closure and absence of intersections remain unproved. Reduce the transition and retry.'):
   (code==='BREP_G2_UNPROVEN'||message.includes('BREP_G2_UNPROVEN'))?label('G2-гладкость для этих размеров не подтверждена. Измените размер перехода и повторите создание.','G2 smoothness is unproved for these dimensions. Change the transition size and retry.'):message
}
