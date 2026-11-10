/** Import-specific guidance; kernel errors outside import keep their own diagnostics. */
export function solidImportErrorMessage(error:unknown,locale:'ru'|'en'):string {
 const message=(error instanceof Error?error.message:String(error)).replace(/^Error:\s*/,'')
 const known:Record<string,[string,string]>={
  'Invalid body mesh.':['Сетка тела повреждена: проверьте координаты вершин и индексы треугольников в JSON или экспортируйте проект заново.','The body mesh is invalid: check vertex coordinates and triangle indices in the JSON, or export the project again.'],
  'Invalid direct modeling document.':['Неверная структура проекта Solid. Откройте JSON, экспортированный из Solid.','Invalid Solid project structure. Open JSON exported from Solid.'],
  'Invalid body material.':['Материал тела повреждён. Проверьте цвет и значения материала в JSON.','Invalid body material. Check its color and material values in the JSON.'],
  'Invalid sketch.':['Эскиз повреждён. Проверьте точки и признак замкнутого контура в JSON.','Invalid sketch. Check its points and closed-profile flag in the JSON.'],
  'Invalid sketch workplane.':['Плоскость эскиза повреждена. Проверьте начало координат и ортонормированные оси в JSON.','Invalid sketch workplane. Check its origin and orthonormal axes in the JSON.'],
 }
 const detail=error instanceof SyntaxError?(locale==='ru'?'Некорректный JSON. Исправьте синтаксис файла и повторите импорт.':'Invalid JSON. Correct the file syntax and import again.'):known[message]?.[locale==='ru'?0:1]??message
 return (locale==='ru'?'Проект не открыт. ':'Project was not opened. ')+detail
}
