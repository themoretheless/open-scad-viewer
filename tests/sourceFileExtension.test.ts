import { expect, it } from 'vitest'
import { sourceFileExtension, withSourceExtension } from '../src/services/rushFrontendDetect'

it('names Rush documents .r and OpenSCAD documents .scad', () => {
  expect(sourceFileExtension('// @rush/1\nshow box(1mm,1mm,1mm)')).toBe('.r')
  expect(sourceFileExtension('cube(10);')).toBe('.scad')
  expect(withSourceExtension('spinner', '// @rush/1\n')).toBe('spinner.r')
  expect(withSourceExtension('spinner.scad', '// @rush/1\n')).toBe('spinner.scad')
  expect(withSourceExtension('part.R', 'cube(1);')).toBe('part.R')
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
