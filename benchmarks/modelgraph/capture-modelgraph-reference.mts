import {readFileSync,writeFileSync} from 'node:fs'
import {compileModelGraphText} from './modelgraph-text-typescript-reference'
const paths=['examples/skadis-box/skadis-dovetail.modelgraph.scad','examples/modelgraph-text/generic-functions.scad','examples/modelgraph-text/range-pattern.scad','examples/modelgraph-text/nurbs-boolean.scad']
const cases=paths.map(path=>{const source=readFileSync(path,'utf8');const result=compileModelGraphText(source);return {path,source,document:result.document,customizer:result.customizer}})
writeFileSync('tests/fixtures/modelgraph-text-compatibility.json',JSON.stringify(cases,null,2)+'\n')
