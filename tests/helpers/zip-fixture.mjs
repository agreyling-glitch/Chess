import {deflateRawSync} from 'node:zlib';
function crc(bytes){let value=0xffffffff;for(const byte of bytes){value^=byte;for(let i=0;i<8;i++)value=(value&1)?0xedb88320^(value>>>1):value>>>1;}return (value^0xffffffff)>>>0;}
export function zip(files){
 const local=[],central=[];let offset=0;
 for(const {name,text='',method=8,flags=0,declaredSize,checksum} of files){
  const filename=Buffer.from(name),raw=Buffer.from(text),data=method===8?deflateRawSync(raw):raw,check=checksum??crc(raw),expanded=declaredSize??raw.length;
  const header=Buffer.alloc(30);header.writeUInt32LE(0x04034b50);header.writeUInt16LE(20,4);header.writeUInt16LE(flags,6);header.writeUInt16LE(method,8);header.writeUInt32LE(check,14);header.writeUInt32LE(data.length,18);header.writeUInt32LE(expanded,22);header.writeUInt16LE(filename.length,26);local.push(header,filename,data);
  const entry=Buffer.alloc(46);entry.writeUInt32LE(0x02014b50);entry.writeUInt16LE(20,6);entry.writeUInt16LE(flags,8);entry.writeUInt16LE(method,10);entry.writeUInt32LE(check,16);entry.writeUInt32LE(data.length,20);entry.writeUInt32LE(expanded,24);entry.writeUInt16LE(filename.length,28);entry.writeUInt32LE(offset,42);central.push(entry,filename);offset+=header.length+filename.length+data.length;
 }
 const index=Buffer.concat(central),end=Buffer.alloc(22);end.writeUInt32LE(0x06054b50);end.writeUInt16LE(files.length,8);end.writeUInt16LE(files.length,10);end.writeUInt32LE(index.length,12);end.writeUInt32LE(offset,16);return Buffer.concat([...local,index,end]);
}
