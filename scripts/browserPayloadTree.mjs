import {createHash} from 'node:crypto'
import {lstatSync,readFileSync,readlinkSync,readdirSync} from 'node:fs'
import {dirname,join,relative,resolve,sep} from 'node:path'

const sha256=bytes=>createHash('sha256').update(bytes).digest('hex')
export function verifyBrowserPayloadTree(directory,includeManifest=false,ignoredMarkers=[]) {
  const ignored=new Set()
  for(const marker of ignoredMarkers) {
    if(marker.path!=='DEPENDENCIES_VALIDATED'||marker.byteLength!==0||marker.sha256!==sha256(''))
      throw new Error('Unsupported installer marker exclusion')
    ignored.add(marker.path)
  }
  const entries=[]
  function walk(current) {
    for(const name of readdirSync(current).sort()) {
      const path=join(current,name),stat=lstatSync(path)
      const relativePath=relative(directory,path).split(sep).join('/')
      if(ignored.has(relativePath)) {
        if(!stat.isFile()||stat.size!==0||readFileSync(path).length!==0)
          throw new Error('Installer marker must be an empty regular file')
        continue
      }
      if(stat.isDirectory())walk(path)
      else if(stat.isFile())entries.push(`F\0${relativePath}\0${sha256(readFileSync(path))}\n`)
      else if(stat.isSymbolicLink()) {
        const target=readlinkSync(path),absoluteTarget=resolve(dirname(path),target)
        if(absoluteTarget!==directory&&!absoluteTarget.startsWith(`${directory}${sep}`))
          throw new Error('Browser tree symlink escapes revision')
        entries.push(`L\0${relativePath}\0${target}\n`)
      }else throw new Error('Browser tree contains a non-file entry')
    }
  }
  walk(directory)
  const manifest=entries.sort().join(''),bytes=Buffer.from(manifest,'utf8')
  return {value:sha256(bytes),byteLength:bytes.byteLength,entryCount:entries.length,
    ...(includeManifest?{manifest}:{})}
}
