export interface EditorBlock { start: number; end: number; column: number; depth: number }
export function editorBlocks(source: string): EditorBlock[] {
  // Mask strings/comments while keeping columns and line breaks.
  const masked = source.replace(/\/\*[\s\S]*?(?:\*\/|$)|\/\/[^\n]*|"(?:\\[\s\S]|[^"\\])*(?:"|$)/g, s => s.replace(/[^\n]/g, ' '))
  const lines = masked.split('\n'), blocks: EditorBlock[] = [], stack: {line:number; column:number; char:string}[] = []
  const indent = (s:string) => s.match(/^[ \t]*/)![0].replace(/\t/g, '  ').length
  lines.forEach((line, i) => {
    for (let col=0; col<line.length; col++) {
      const char=line[col]!
      if ('([{'.includes(char)) stack.push({line:i,column:indent(line),char})
      else if (')]}'.includes(char)) {
        const open=stack.at(-1)
        if (open && '([{'.indexOf(open.char)===')]}'.indexOf(char)) {
          stack.pop(); if(i>open.line) blocks.push({start:open.line,end:i,column:open.column,depth:0})
        }
      }
    }
  })
  const indentStack: {line:number;column:number}[]=[]
  let previous=-1
  lines.forEach((line,i)=>{
    if(!line.trim())return
    const column=indent(line)
    while(indentStack.length && column<=indentStack.at(-1)!.column){
      const open=indentStack.pop()!; blocks.push({start:open.line,end:previous,column:open.column,depth:0})
    }
    if(previous>=0 && column>indent(lines[previous]!))indentStack.push({line:previous,column:indent(lines[previous]!)})
    previous=i
  })
  for(const open of indentStack)blocks.push({start:open.line,end:previous,column:open.column,depth:0})
  const unique = new Map<number,EditorBlock>()
  for(const b of blocks)if(!unique.has(b.start)||unique.get(b.start)!.end<b.end)unique.set(b.start,b)
  const sorted=[...unique.values()].sort((a,b)=>a.start-b.start||b.end-a.end)
  const parents:EditorBlock[]=[]
  for(const b of sorted){while(parents.length&&parents.at(-1)!.end<=b.start)parents.pop();b.depth=parents.length;parents.push(b)}
  return sorted
}
export function indentSelection(source:string,start:number,end:number,outdent=false) {
  if(start===end&&!outdent){const column=source.slice(source.lastIndexOf('\n',start-1)+1,start).replace(/\t/g,'  ').length;const text=' '.repeat(2-column%2);return {start,end,text,selectionStart:start+text.length,selectionEnd:start+text.length}}
  const from=source.lastIndexOf('\n',start-1)+1
  const last=end>start&&source[end-1]==='\n'?end-1:end
  const newline=source.indexOf('\n',last),to=newline<0?source.length:newline
  const lines=source.slice(from,to).split('\n');let delta=0,firstDelta=0
  const text=lines.map((line,i)=>{const remove=outdent?(line.startsWith('\t')?1:Math.min(2,line.match(/^ */)![0].length)):0;const d=outdent?-remove:2;delta+=d;if(!i)firstDelta=d;return outdent?line.slice(remove):'  '+line}).join('\n')
  return {start:from,end:to,text,selectionStart:Math.max(from,start+firstDelta),selectionEnd:Math.max(from,end+delta)}
}

/** Guides occupy whitespace only, never a closing delimiter or other code. */
export function guideFitsIndent(line: string, column: number): boolean {
  if (!line.trim()) return true
  let width = 0
  for (const char of line) {
    if (char === ' ') width++
    else if (char === '\t') width += 2 - width % 2
    else break
  }
  return column < width
}
