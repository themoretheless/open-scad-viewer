import {readFileSync,writeFileSync} from 'node:fs'
import {compileRushFrontend} from './rush-frontend-typescript-reference'
const paths=['examples/skadis-box/skadis-dovetail.rush.scad','examples/rush-frontend/generic-functions.scad','examples/rush-frontend/range-pattern.scad','examples/rush-frontend/nurbs-boolean.scad']
const cases=paths.map(path=>{const source=readFileSync(path,'utf8');const result=compileRushFrontend(source);return {path,source,document:result.document,customizer:result.customizer}})
writeFileSync('tests/fixtures/rush-frontend-compatibility.json',JSON.stringify(cases,null,2)+'\n')
