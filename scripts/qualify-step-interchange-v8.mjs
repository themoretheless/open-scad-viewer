import {createHash} from 'node:crypto'
import {existsSync,mkdirSync,readFileSync,readdirSync,rmSync,statSync,writeFileSync} from 'node:fs'
import {join,resolve} from 'node:path'
import {spawnSync} from 'node:child_process'

const root=resolve(import.meta.dirname,'..')
const output=resolve(process.env.STEP_V8_OUTPUT||join(root,'output/qualification/step-interchange-8-v1'))
const tools=resolve(process.env.STEP_V8_TOOL_ROOT||join(output,'toolchain'))
const source=join(tools,'stepcode-src'),build=join(tools,'stepcode-build')
const schema=join(tools,'ap242.exp'),binary=join(build,'bin/p21read_sdai_ap242')
const manifest=JSON.parse(readFileSync(join(root,'tools/stepcode-ap242-ed4-manifest.json'),'utf8'))
const pinned=manifest.validator
const sha256=path=>createHash('sha256').update(readFileSync(path)).digest('hex')
const run=(command,args,options={})=>{
  const result=spawnSync(command,args,{cwd:root,encoding:'utf8',...options})
  if(result.error||result.status!==0)throw new Error(`${command} ${args.join(' ')} failed (${result.status}):\n${result.stdout||''}${result.stderr||''}`)
  return result
}
const download=async(url,path)=>{
  const response=await fetch(url,{headers:{'user-agent':'open-scad-viewer-step-qualification/1'}})
  if(!response.ok)throw new Error(`download failed ${response.status}: ${url}`)
  writeFileSync(path,Buffer.from(await response.arrayBuffer()))
}

mkdirSync(tools,{recursive:true})
if(!existsSync(schema))await download(pinned.schema.url,schema)
if(sha256(schema)!==pinned.schema.sha256)throw new Error('AP242 schema digest mismatch')
if(!existsSync(join(source,'.git')))run('git',['clone',pinned.sourceUrl,source])
run('git',['-C',source,'fetch','--depth','1','origin',pinned.sourceCommit])
run('git',['-C',source,'checkout','--detach','--force',pinned.sourceCommit])
if(run('git',['-C',source,'rev-parse','HEAD']).stdout.trim()!==pinned.sourceCommit)throw new Error('STEPcode commit mismatch')
const macro=join(source,'cmake/SC_CXX_schema_macros.cmake')
let macroText=readFileSync(macro,'utf8')
macroText=macroText.replace('  P21_TESTS(${expFile})\n','  # Qualification build: fixture tests are driven externally.\n')
writeFileSync(macro,macroText)
if(sha256(macro)!==pinned.buildPatch.patchedFileSha256)throw new Error('STEPcode build-only patch digest mismatch')
run('cmake',['-S',source,'-B',build,`-DSC_BUILD_SCHEMAS=${schema}`,'-DCMAKE_BUILD_TYPE=Release','-DBUILD_TESTING=OFF'])
run('cmake',['--build',build,'--target','p21read_sdai_ap242','--parallel',process.env.STEP_V8_BUILD_JOBS||'4'])
if(!statSync(binary).isFile())throw new Error('STEPcode oracle executable unavailable')
const version=run(binary,['-v']).stdout.trim()
if(!version.endsWith(`build info: ${pinned.version}`))throw new Error(`unexpected STEPcode version: ${version}`)

const positives=['tests/fixtures/step-v5','tests/fixtures/step-v6','tests/fixtures/step-v8']
  .flatMap(directory=>{
    const absolute=join(root,directory)
    return existsSync(absolute)?readdirSync(absolute).filter(name=>/\.ste?p$/i.test(name)).map(name=>join(directory,name)):[]
  }).sort()
if(!positives.length)throw new Error('no positive STEP fixtures found')
const negativeDirectory=join(output,'negative-mutations')
rmSync(negativeDirectory,{recursive:true,force:true})
mkdirSync(negativeDirectory,{recursive:true})
const seed=readFileSync(join(root,positives[0]),'utf8')
const negatives=[
  ['duplicate-instance.step',seed.replace('ENDSEC;\nEND-ISO-10303-21;',"#1=CARTESIAN_POINT('',(0.,0.,0.));\nENDSEC;\nEND-ISO-10303-21;")],
  ['wrong-coordinate-type.step',seed.replace(/CARTESIAN_POINT\('([^']*)',\(([^)]+)\)\)/,
    "CARTESIAN_POINT('$1','not-an-aggregate')")],
  ['wrong-pcurve-reference-type.step',seed.replace('DEFINITIONAL_REPRESENTATION','REPRESENTATION')],
]
for(const [name,text] of negatives){
  if(text===seed)throw new Error(`negative mutation did not apply: ${name}`)
  writeFileSync(join(negativeDirectory,name),text)
}
const logs=join(output,'stepcode-raw-logs')
rmSync(logs,{recursive:true,force:true})
mkdirSync(logs,{recursive:true})
const environment={...process.env,STEP_AP242_VALIDATOR:binary,STEP_AP242_VALIDATOR_SHA256:sha256(binary),
  STEP_AP242_VALIDATOR_VERSION:pinned.version,STEP_AP242_LOG_DIR:logs}
for(const fixture of positives)run(process.execPath,['scripts/validate-step-ap242.mjs',fixture],{env:environment})
for(const [name] of negatives)run(process.execPath,['scripts/validate-step-ap242.mjs','--expect-reject',join(negativeDirectory,name)],{env:environment})
const dylib=join(build,'lib/libsdai_ap242.dylib')
const result={
  schema:'open-scad-viewer/step-interchange-v8-oracle-run',status:'pass',
  validator:{name:pinned.name,version:pinned.version,sourceUrl:pinned.sourceUrl,sourceCommit:pinned.sourceCommit,
    license:pinned.license,licenseSha256:pinned.licenseSha256,executable:binary,executableSha256:sha256(binary),
    schemaLibrarySha256:existsSync(dylib)?sha256(dylib):null},
  expressSchema:pinned.schema,positiveFixtures:positives,positiveCount:positives.length,
  negativeMutations:negatives.map(([name])=>join('negative-mutations',name)),negativeCount:negatives.length,
  rawLogDirectory:logs,
}
writeFileSync(join(output,'stepcode-oracle-result.json'),JSON.stringify(result,null,2)+'\n')
console.log(JSON.stringify(result,null,2))
