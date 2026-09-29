import {mkdir,copyFile,readFile,writeFile} from 'node:fs/promises'
import {spawnSync} from 'node:child_process'
import path from 'node:path'
import assert from 'node:assert/strict'
const fixtures=path.resolve(process.argv[2]??'/tmp/solid-blender-qualification')
const output=path.resolve(process.argv[3]??'/tmp/solid-interaction-matrix')
const python=process.argv[4]??'/tmp/cad-roadmap-ocp/bin/python'
await mkdir(output,{recursive:true})
const results=[]
for(const theme of ['dark','light','nord','solarized'])for(const interaction of ['mouse','keyboard']){
 const directory=path.join(output,`${theme}-${interaction}`)
 await mkdir(directory,{recursive:true})
 for(const width of [2,7])await copyFile(path.join(fixtures,`width-${width}.json`),path.join(directory,`width-${width}.json`))
 const args=['scripts/check-solid-download-browser.mjs',directory,`--theme=${theme}`,...(interaction==='keyboard'?['--keyboard']:[])]
 let run=spawnSync(process.execPath,args,{encoding:'utf8',timeout:120000})
 assert.equal(run.status,0,run.stdout+run.stderr)
 run=spawnSync(python,['scripts/verify-cad-step-occt.py',directory],{encoding:'utf8',timeout:120000})
 assert.equal(run.status,0,run.stdout+run.stderr)
 const browser=JSON.parse(await readFile(path.join(directory,'browser-download-verification.json'),'utf8'))
 const oracle=JSON.parse(await readFile(path.join(directory,'occt-report.json'),'utf8'))
 results.push({theme,interaction,browser,oracle})
 console.log(`${theme}/${interaction}: browser download and independent STEP check passed`)
}
await writeFile(path.join(output,'interaction-matrix.json'),JSON.stringify({cases:results.length,results},null,2)+'\n')
