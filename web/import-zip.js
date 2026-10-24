// ZIP files are read in memory with browser-native decompression; nothing is written to disk.
const crcTable=Uint32Array.from({length:256},(_,n)=>{for(let i=0;i<8;i++)n=(n&1)?0xedb88320^(n>>>1):n>>>1;return n>>>0;});
const invalid=()=>new Error('The ZIP file is damaged or uses an unsupported format.');
export async function readPgnZip(buffer,{maxBytes=64*1024*1024,maxEntries=1000}={}) {
  const bytes=new Uint8Array(buffer),view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
  if(bytes.length>64*1024*1024)throw Error('Choose a ZIP file smaller than 64 MiB.');
  let end=-1;
  for(let i=bytes.length-22;i>=Math.max(0,bytes.length-22-65535);i--){if(view.getUint32(i,true)===0x06054b50&&i+22+view.getUint16(i+20,true)===bytes.length){end=i;break;}}
  if(end<0)throw invalid();
  const count=view.getUint16(end+10,true),size=view.getUint32(end+12,true),start=view.getUint32(end+16,true);
  if(view.getUint16(end+4,true)||view.getUint16(end+6,true)||view.getUint16(end+8,true)!==count||count===65535||size===0xffffffff||start===0xffffffff)throw Error('Multi-part and ZIP64 archives are not supported. Choose a standard ZIP file.');
  if(count>maxEntries)throw Error(`The ZIP contains too many files (maximum ${maxEntries}).`);
  if(start+size!==end)throw invalid();
  let offset=start,total=0;const entries=[];
  for(let i=0;i<count;i++){
    if(offset+46>end||view.getUint32(offset,true)!==0x02014b50)throw invalid();
    const flags=view.getUint16(offset+8,true),method=view.getUint16(offset+10,true),crc=view.getUint32(offset+16,true),compressed=view.getUint32(offset+20,true),expanded=view.getUint32(offset+24,true),nameLength=view.getUint16(offset+28,true),extra=view.getUint16(offset+30,true),comment=view.getUint16(offset+32,true),disk=view.getUint16(offset+34,true),local=view.getUint32(offset+42,true);
    const next=offset+46+nameLength+extra+comment;if(next>end)throw invalid();
    const rawName=bytes.subarray(offset+46,offset+46+nameLength),name=new TextDecoder().decode(rawName);
    if(/\.pgn$/i.test(name)&&!name.startsWith('__MACOSX/')&&!/(^|\/)\._/.test(name)){
      if(flags&0x41)throw Error('Password-protected PGN files are not supported. Unzip the file first.');
      if(![0,8].includes(method))throw Error('This ZIP compression method is not supported. Unzip the PGN files first.');
      if(disk||compressed===0xffffffff||expanded===0xffffffff||local===0xffffffff)throw invalid();
      total+=expanded+(entries.length?2:0);if(total>maxBytes)throw Error('The PGN files inside the ZIP exceed the 64 MiB import limit.');
      if(local+30>start||view.getUint32(local,true)!==0x04034b50||view.getUint16(local+6,true)!==flags||view.getUint16(local+8,true)!==method)throw invalid();
      const localNameLength=view.getUint16(local+26,true),dataStart=local+30+localNameLength+view.getUint16(local+28,true);
      if(dataStart+compressed>start||localNameLength!==rawName.length||!rawName.every((value,j)=>value===bytes[local+30+j]))throw invalid();
      entries.push({method,crc,compressed,expanded,dataStart});
    }
    offset=next;
  }
  if(offset!==end)throw invalid();
  if(!entries.length)throw Error('The ZIP does not contain any PGN files. Choose an archive containing .pgn files.');
  const texts=[];
  for(const entry of entries){
    const data=bytes.subarray(entry.dataStart,entry.dataStart+entry.compressed);
    let stream=new Blob([data]).stream();
    if(entry.method===8){
      if(typeof DecompressionStream==='undefined')throw Error('ZIP decompression is unavailable in this browser. Unzip the PGN files first.');
      stream=stream.pipeThrough(new DecompressionStream('deflate-raw'));
    }
    const reader=stream.getReader(),decoder=new TextDecoder(),parts=[];let length=0,crc=0xffffffff;
    try {
      while(true){
        const {value,done}=await reader.read();if(done)break;
        length+=value.length;if(length>entry.expanded||length>maxBytes)throw Error('The ZIP expands beyond its declared size or the import limit.');
        for(const byte of value)crc=crcTable[(crc^byte)&255]^(crc>>>8);
        parts.push(decoder.decode(value,{stream:true}));
      }
    }catch(error){await reader.cancel().catch(()=>{});throw Error(`Could not read a PGN inside the ZIP: ${error.message}`);}
    finally{reader.releaseLock();}
    if(length!==entry.expanded||((crc^0xffffffff)>>>0)!==entry.crc)throw Error('A PGN file inside the ZIP failed its integrity check.');
    parts.push(decoder.decode());texts.push(parts.join('').replace(/^\uFEFF/,''));
  }
  return texts.join('\n\n');
}
