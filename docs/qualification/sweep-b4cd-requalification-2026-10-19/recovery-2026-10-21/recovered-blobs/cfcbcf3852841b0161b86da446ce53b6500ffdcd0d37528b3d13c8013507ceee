/** Bounded fixed or variable bit pairs; reject extra words and nonzero padding. */
const alphabet=Array.from({length:94},(_,i)=>String.fromCharCode(i+33)).filter(c=>!"'\\`".includes(c)).join('')
const alphabet92=Array.from({length:94},(_,i)=>String.fromCharCode(i+33)).filter(c=>!"'\\".includes(c)).join('')
const digits94='\t '+alphabet92
const digits108=String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+digits94
const digits112=digits108+String.fromCharCode(22,23,24,25)
const formats: [string,string,number?][]=[['91',alphabet],['9v',alphabet],['92',alphabet92],['93',' '+alphabet92],['94',digits94],['95','\x01'+digits94],['Ax',digits108],['Bx',digits112],['Cx',digits112+String.fromCharCode(26)],['Dx',digits112+String.fromCharCode(26),3],['Ex',digits112+String.fromCharCode(26,28),3],['Hx',digits112+String.fromCharCode(26,28,29,30,31),3],['Ix',digits112+String.fromCharCode(26,28,29,30,31,127),3],['Jx',digits112+String.fromCharCode(26,28,29,30,31,127),4],['Kx',digits112+String.fromCharCode(26,28,29,30,31,127),7]]
const ranks=new Map(formats.map(([tag,digits,width=2])=>{
 const rank=new Int16Array(128).fill(-1)
 for(let i=0;i<digits.length;i++)rank[digits.charCodeAt(i)]=i
 return [tag,{base:digits.length,rank,width}] as const
}))
export function decodeBase91(text:string,maximum=4*1024*1024+4):Uint8Array<ArrayBuffer>{
 const format=ranks.get(text.slice(1,3))
 const width=format?.width??2,wordBits=width===7?48:width===4?27:width===3?20:13,mask=2**wordBits-1
 if(!format||!/^b(?:91|9v|92|93|94|95|Ax|Bx|Cx|Dx|Ex|Hx|Ix|Jx|Kx):[0-9a-f]{8}/.test(text)||text.length>12+width*Math.ceil(maximum*8/wordBits))throw Error('Base91 size limit')
 const {base,rank:digits}=format,threshold=base**width-mask-2,variable=!text.startsWith('b91:')
 const size=parseInt(text.slice(4,12),16)
 if(size>maximum||(text.length-12)%width!==0||(!variable&&text.length!==12+width*Math.ceil(size*8/wordBits)))throw Error('Base91 size mismatch')
 const out=new Uint8Array(size)
 let bits=0,value=0,offset=0
 for(let i=12;i<text.length;i+=width){
  let word=0,multiplier=1
  for(let j=0;j<width;j++){
   const digit=digits[text.charCodeAt(i+j)]??-1
   if(digit<0)throw Error('Invalid base91 word')
   word+=multiplier*digit;multiplier*=base
  }
  if((!variable&&word>mask)||offset===size)throw Error('Invalid base91 word')
  if(width===7){
   // Each word fits49 bits. Append <=26 bits at a time so the byte
   // accumulator stays below2^33, even across unaligned word boundaries.
   let remaining=word%2**wordBits<=threshold?wordBits+1:wordBits
   while(remaining>0){
    const take=Math.min(26,remaining),part=word%2**take
    word=Math.floor(word/2**take);remaining-=take
    if(offset===size){if(part!==0)throw Error('Base91 padding');continue}
    value+=part*2**bits;bits+=take
    while(bits>=8&&offset<size){out[offset++]=value%256;value=Math.floor(value/256);bits-=8}
   }
  }else if(width===4){
   // Four digits can exceed the 32-bit bitwise accumulator; all integers
   // remain below 2^35 and therefore exactly representable as Numbers.
   value+=word*2**bits;bits+=(word%2**wordBits)<=threshold?wordBits+1:wordBits
   while(bits>=8&&offset<size){out[offset++]=value%256;value=Math.floor(value/256);bits-=8}
  }else{
   value|=word<<bits;bits+=variable&&((word&mask)<=threshold)?wordBits+1:wordBits
   while(bits>=8&&offset<size){out[offset++]=value&255;value>>>=8;bits-=8}
  }
 }
 if(offset!==size||value!==0)throw Error('Noncanonical base91 padding')
 return out
}
