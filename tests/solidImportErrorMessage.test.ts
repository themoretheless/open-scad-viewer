import {expect,it} from 'vitest'
import {solidImportErrorMessage} from '../src/services/solidImportErrorMessage'
it('localizes invalid mesh imports with concrete recovery guidance',()=>{
 expect(solidImportErrorMessage(new Error('Invalid body mesh.'),'ru')).toContain('индексы треугольников')
 expect(solidImportErrorMessage('Error: Invalid body mesh.','ru')).not.toContain('Invalid body mesh')
 expect(solidImportErrorMessage(new Error('Invalid body mesh.'),'en')).toContain('export the project again')
})
it('explains malformed JSON and preserves unrecognized diagnostic details',()=>{
 expect(solidImportErrorMessage(new SyntaxError('unexpected token'),'ru')).toContain('Исправьте синтаксис')
 expect(solidImportErrorMessage(new Error('BREP specific defect'),'en')).toBe('Project was not opened. BREP specific defect')
})
