/** Localize offset failures with the selected curve and an available next action. */
export function curveOffsetErrorMessage(error:unknown,locale:string,name?:string):string {
 const message=error instanceof Error?error.message:String(error)
 const label=(ru:string,en:string)=>locale==='ru'?ru:en
 const prefix=name?name+': ':''
 let detail:string
 if(message.includes('explicit profile join'))detail=label('В кривой есть излом. Выберите Bevel или обрезку в поле «Соединения».','The curve has a corner. Choose Bevel or a trim mode under Joins.')
 else if(/XY plane|constant Z|XY planar/.test(message))detail=label('Нужна кривая в плоскости XY с постоянной Z. Выберите плоскую кривую.','Select a curve in an XY plane with constant Z.')
 else if(/empty|contains no material|Reduce the offset/.test(message))detail=label('Смещение не оставляет заполненной области. Уменьшите модуль расстояния или смените правило заполнения.','Offset leaves no filled region. Reduce the distance magnitude or change the fill rule.')
 else if(/budget|limit|exceeds.*(vertices|edges|curves)/i.test(message))detail=label('Предел вычисления достигнут. Увеличьте допуск или разделите кривую.','Calculation limit reached. Increase tolerance or split the curve.')
 else if(/closed|not connected|disconnected/.test(message))detail=label('Для обрезки нужен связный замкнутый профиль. Соедините концы или выберите Bevel.','Trimming requires a connected closed profile. Join its endpoints or choose Bevel.')
 else if(/crossing|Intersection order|angular order|Contact could not|unresolved pairs|degeneracy|collapsed represented|Boundary area|witness/i.test(message))detail=label('Построение области не подтверждено в месте пересечения или касания. Разделите профиль либо выберите Bevel и проверьте цепочку.','Region construction is unproven at a crossing or contact. Split the profile or choose Bevel and inspect the chain.')
 else if(/tolerance|exceeds tolerance/.test(message))detail=label('Отклонение не удалось ограничить заданным допуском. Увеличьте допуск или уменьшите смещение.','Deviation could not be bounded by the requested tolerance. Increase tolerance or reduce offset.')
 else detail=label('Не удалось построить смещение. Повторите вычисление; если ошибка повторяется, разделите профиль.','Offset construction failed. Retry the calculation; if it fails again, split the profile.')
 return prefix+detail
}
