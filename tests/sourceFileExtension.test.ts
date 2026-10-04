import { expect, it } from 'vitest'
import { sourceFileExtension, withSourceExtension } from '../src/services/modelGraphTextDetect'

it('names ModelGraph Text documents .mg and OpenSCAD documents .scad', () => {
  expect(sourceFileExtension('// @modelgraph-text/1\nshow box(1mm,1mm,1mm)')).toBe('.mg')
  expect(sourceFileExtension('cube(10);')).toBe('.scad')
  expect(withSourceExtension('spinner', '// @modelgraph-text/1\n')).toBe('spinner.mg')
  expect(withSourceExtension('spinner.scad', '// @modelgraph-text/1\n')).toBe('spinner.scad')
  expect(withSourceExtension('part.MG', 'cube(1);')).toBe('part.MG')
  expect(withSourceExtension('part', 'cube(1);')).toBe('part.scad')
})


it('recognizes Rush headers and preserves .r filenames', () => {
  expect(sourceFileExtension('// @rush/1\nshow sphere(10mm)')).toBe('.r')
  expect(sourceFileExtension('// @rush\nshow sphere(10mm)')).toBe('.r')
  expect(sourceFileExtension('// @rush/2\n')).toBe('.scad')
  expect(sourceFileExtension('// @rush-other\n')).toBe('.scad')
  expect(withSourceExtension('part', '// @rush/1\n')).toBe('part.r')
  expect(withSourceExtension('part.R', '// @rush/1\n')).toBe('part.R')
})
