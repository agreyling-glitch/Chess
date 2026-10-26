// Read-only Scid conversion. File contents never leave this device.
const maxBytes=64*1024*1024;

export async function readScidFiles(files, createDatabase, {yieldToUi=()=>new Promise(resolve=>setTimeout(resolve,0)),cancelled=()=>false,onProgress=()=>{}}={}) {
  files=Array.from(files);
  if(files.some(file=>/\.(?:si|sn|sg)[35]$/i.test(file.name)))throw Error('Only Scid .si4 databases are supported. Export older or .si5 databases to PGN first.');
  const parts=new Map();let base=null,total=0;
  for(const file of files){
    const match=/^(.*)\.(si4|sn4|sg4)$/i.exec(file.name||'');
    if(!match)throw Error('Select only the three matching .si4, .sn4 and .sg4 files together.');
    const stem=match[1].toLowerCase(),extension=match[2].toLowerCase();
    if(base!==null&&stem!==base)throw Error('Choose Scid files with the same database name.');
    if(parts.has(extension))throw Error('Choose one .si4, one .sn4 and one .sg4 file.');
    base=stem;total+=file.size??file.bytes?.byteLength??0;
    if(total>maxBytes)throw Error('The three Scid files together exceed the 64 MiB import limit.');
    parts.set(extension,file);
  }
  if(parts.size!==3)throw Error('Select the .si4 index together with its matching .sn4 names and .sg4 games files.');
  const data=[];total=0;
  for(const extension of ['si4','sn4','sg4']){
    const file=parts.get(extension),bytes=new Uint8Array(ArrayBuffer.isView(file.bytes)?file.bytes:await file.arrayBuffer());
    total+=bytes.byteLength;if(total>maxBytes)throw Error('The three Scid files together exceed the 64 MiB import limit.');
    data.push(bytes);
  }
  if(cancelled())throw Error('Scid import cancelled.');
  const database=createDatabase(...data),texts=[];let expanded=0;
  try {
    for(let i=0;i<database.game_count();i++){
      if(i%8===0){onProgress(i,database.game_count());await yieldToUi();if(cancelled())throw Error('Scid import cancelled.');}
      const pgn=database.game_pgn(i);if(!pgn)continue; // Scid deletion flag.
      expanded+=new TextEncoder().encode(pgn).byteLength+(texts.length?2:0);
      if(expanded>maxBytes)throw Error('Converted Scid games exceed the 64 MiB PGN import limit. Export a smaller selection to PGN.');
      texts.push(pgn);
    }
    if(!texts.length)throw Error('This Scid database contains no games available to import.');
    return texts.join('\n\n');
  } finally { database.free(); }
}
