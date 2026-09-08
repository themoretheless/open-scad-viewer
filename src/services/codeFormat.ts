/** Conservative layout formatter: preserves expressions, comments and literal contents. */
export function formatCode(source: string): string {
  const protectedLines = new Set<number>(), commentedLines = new Set<number>()
  const tokens = /\/\*[\s\S]*?(?:\*\/|$)|\/\/[^\n]*|"(?:\\[\s\S]|[^"\\])*(?:"|$)/g
  for (const match of source.matchAll(tokens)) {
    const start = source.slice(0, match.index).split('\n').length - 1
    const count = match[0].split('\n').length
    if(match[0].startsWith('//')) commentedLines.add(start)
    if(count>1) for(let line=start;line<start+count;line++) protectedLines.add(line)
  }
  const levels = [0], output: string[] = []
  let previousMergeable = false
  source.split('\n').forEach((line, index) => {
    if(protectedLines.has(index)) { output.push(line); previousMergeable=false; return }
    const trimmed=line.trim()
    if(!trimmed) { output.push(''); previousMergeable=false; return }
    const leading=line.match(/^[ \t]*/)![0]
    let column=0
    for(const char of leading) column+=char==='\t'?2-column%2:1
    while(levels.length>1&&levels.at(-1)!>column) levels.pop()
    if(levels.at(-1)!<column) levels.push(column)
    const formatted='  '.repeat(levels.length-1)+line.slice(leading.length).trimEnd()
    if(/^\)+(?:[,;]?$|\.[A-Za-z_]\w*\s*\()/.test(trimmed)&&previousMergeable) {
      output[output.length-1]=output.at(-1)!.replace(/,\s*$/, '')+trimmed
    } else output.push(formatted)
    previousMergeable=!commentedLines.has(index)
  })
  return output.join('\n')
}
