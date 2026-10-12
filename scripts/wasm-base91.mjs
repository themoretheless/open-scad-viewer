// Two ASCII digits encode 13 bits. Exclude JS string escapes and delimiters.
export const alphabet=Array.from({length:94},(_,i)=>String.fromCharCode(i+33)).filter(c=>!"'\\`".includes(c)).join('')
export function encodeBase91(bytes){
 if(bytes.length>0xffffffff)throw Error('Base91 size overflow')
 const parts=['b91:',bytes.length.toString(16).padStart(8,'0')]
 let bits=0,value=0
 for(const byte of bytes){value|=byte<<bits;bits+=8;if(bits>=13){const word=value&8191;parts.push(alphabet[word%91],alphabet[Math.floor(word/91)]);value>>>=13;bits-=13}}
 if(bits)parts.push(alphabet[value%91],alphabet[Math.floor(value/91)])
 return parts.join('')
}

// A backtick is safe inside the single-quoted generated literal.
export const alphabet92=Array.from({length:94},(_,i)=>String.fromCharCode(i+33)).filter(c=>!"'\\".includes(c)).join('')
export const encodeVariableBase91=bytes=>encodeVariable(bytes,alphabet,'b9v:')
export const encodeVariableBase92=bytes=>encodeVariable(bytes,alphabet92,'b92:')
// Literal tabs are valid in a single-quoted JS string and retained by esbuild.
export const encodeVariableBase94=bytes=>encodeVariable(bytes,'\t '+alphabet92,'b94:')
// SOH is a legal one-byte JS string character; emission is independently verified.
export const encodeVariableBase95=bytes=>encodeVariable(bytes,'\x01\t '+alphabet92,'b95:')
export const encodeVariableBase108=bytes=>encodeVariable(bytes,String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+'\t '+alphabet92,'bAx:')
// Additional literal control digits remain one byte after esbuild emission.
export const encodeVariableBase112=bytes=>encodeVariable(bytes,String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+'\t '+alphabet92+String.fromCharCode(22,23,24,25),'bBx:')
export const encodeVariableBase113=bytes=>encodeVariable(bytes,digits112+String.fromCharCode(26),'bCx:')
export const encodeVariableBase93=bytes=>encodeVariable(bytes,' '+alphabet92,'b93:')
// Three variable-width digits improve density without changing packed bytes.
export const encodeTripleBase113=bytes=>encodeTriple(bytes,digits112+String.fromCharCode(26),'bDx:')
export const encodeTripleBase114=bytes=>encodeTriple(bytes,digits112+String.fromCharCode(26,28),'bEx:')
export const encodeTripleBase117=bytes=>encodeTriple(bytes,digits112+String.fromCharCode(26,28,29,30,31),'bHx:')
// DEL is a legal literal ASCII digit; preserve the exact compressed payload.
export const encodeTripleBase118=bytes=>encodeTriple(bytes,digits112+String.fromCharCode(26,28,29,30,31,127),'bIx:')
function encodeTriple(bytes,digits,tag){
 if(bytes.length>0xffffffff)throw Error('Base91 size overflow')
 const base=digits.length,threshold=base**3-1048577
 const parts=[tag,bytes.length.toString(16).padStart(8,'0')]
 let bits=0,value=0
 const emit=word=>parts.push(digits[word%base],digits[Math.floor(word/base)%base],digits[Math.floor(word/(base*base))])
 for(const byte of bytes){
  value|=byte<<bits;bits+=8
  if(bits>20){let word=value&1048575;const width=word>threshold?20:21;if(width===21)word=value&2097151;emit(word);value>>>=width;bits-=width}
 }
 if(bits)emit(value)
 return parts.join('')
}
function encodeVariable(bytes,digits,tag){
 if(bytes.length>0xffffffff)throw Error('Base91 size overflow')
 const base=digits.length,threshold=base*base-8193
 const parts=[tag,bytes.length.toString(16).padStart(8,'0')]
 let bits=0,value=0
 for(const byte of bytes){
  value|=byte<<bits;bits+=8
  if(bits>13){let word=value&8191;const width=word>threshold?13:14;if(width===14)word=value&16383;parts.push(digits[word%base],digits[Math.floor(word/base)]);value>>>=width;bits-=width}
 }
 if(bits)parts.push(digits[value%base],digits[Math.floor(value/base)])
 return parts.join('')
}

const digits94='\t '+alphabet92
const digits108=String.fromCharCode(1,2,3,4,5,6,14,15,16,17,18,19,20,21)+digits94
const digits112=digits108+String.fromCharCode(22,23,24,25)
const formats=[['91',alphabet],['9v',alphabet],['92',alphabet92],['93',' '+alphabet92],['94',digits94],['95','\x01'+digits94],['Ax',digits108],['Bx',digits112],['Cx',digits112+String.fromCharCode(26)],['Dx',digits112+String.fromCharCode(26),3],['Ex',digits112+String.fromCharCode(26,28),3],['Hx',digits112+String.fromCharCode(26,28,29,30,31),3],['Ix',digits112+String.fromCharCode(26,28,29,30,31,127),3],['Jx',digits112+String.fromCharCode(26,28,29,30,31,127),4],['Kx',digits112+String.fromCharCode(26,28,29,30,31,127),7]]
const ranks=new Map(formats.map(([tag,digits,width=2])=>{
 const rank=new Int16Array(128).fill(-1)
 for(let i=0;i<digits.length;i++)rank[digits.charCodeAt(i)]=i
 return [tag,{base:digits.length,rank,width}]
}))
export function decodeBase91(text,maximum=4*1024*1024+4){
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

export function encodeQuadBase118(bytes){
 if(bytes.length>0xffffffff)throw Error('Base91 size overflow')
 const digits=digits112+String.fromCharCode(26,28,29,30,31,127)
 const threshold=118**4-2**27-1,parts=['bJx:',bytes.length.toString(16).padStart(8,'0')]
 let bits=0,value=0
 const emit=word=>{for(let j=0;j<4;j++){parts.push(digits[word%118]);word=Math.floor(word/118)}}
 for(const byte of bytes){
  value+=byte*2**bits;bits+=8
  if(bits>27){const width=value%2**27>threshold?27:28;emit(value%2**width);value=Math.floor(value/2**width);bits-=width}
 }
 if(bits)emit(value)
 return parts.join('')
}

/** Seven base118 digits retain48/49 bits; BigInt is build-time transport only. */
export function encodeSeptBase118(bytes){
 if(bytes.length>0xffffffff)throw Error('Base91 size overflow')
 const digits=digits112+String.fromCharCode(26,28,29,30,31,127)
 const threshold=118n**7n-2n**48n-1n,parts=['bKx:',bytes.length.toString(16).padStart(8,'0')]
 let bits=0,value=0n
 const emit=word=>{for(let j=0;j<7;j++){parts.push(digits[Number(word%118n)]);word/=118n}}
 for(const byte of bytes){
  value+=BigInt(byte)<<BigInt(bits);bits+=8
  if(bits>48){const width=value%(2n**48n)>threshold?48:49;emit(value%(2n**BigInt(width)));value>>=BigInt(width);bits-=width}
 }
 if(bits)emit(value)
 return parts.join('')
}
