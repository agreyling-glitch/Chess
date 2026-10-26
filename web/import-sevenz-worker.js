import init,{ironwoodSevenZipList,ironwoodSevenZipExtract} from './pkg/battle_chess.js';
let file,directory,manifest,handles=[];
const read=(offset,length)=>new Uint8Array(new FileReaderSync().readAsArrayBuffer(file.slice(offset,offset+length)));
const close=()=>{for(const handle of handles){try{handle?.close();}catch{}}handles=[];};
self.onmessage=async({data})=>{
  try{
    if(data.type==='open'){
      file=data.file;directory=data.directory;
      self.postMessage({type:'stage',message:'Loading the archive decoder…'});
      const versionResponse=await fetch('/app-version.json',{cache:'no-store'});
      if(!versionResponse.ok)throw Error('Could not check the archive decoder version. Reload Ironwood and try again.');
      const version=await versionResponse.json(),revision=version.package_version||version.version;
      if(!revision)throw Error('Missing archive decoder version. Reload Ironwood and try again.');
      // The unversioned WASM URL can be cached for a year. Match the main app's
      // versioned URL instead of initializing fresh glue with an old binary.
      const response=await fetch(`/pkg/battle_chess_bg.wasm?v=${encodeURIComponent(revision)}`,{cache:'reload'});
      if(!response.ok)throw Error('Could not load the archive decoder. Reload Ironwood and try again.');
      await init({module_or_path:response});
      self.postMessage({type:'stage',message:'Checking the archive contents…'});
      manifest=JSON.parse(ironwoodSevenZipList(file.size,read));self.postMessage({type:'manifest',manifest});
    }else if(data.type==='extract'){
      const plan=data.plan,index=new Map(plan.files.map((file,i)=>[file.name,i]));
      for(let i=0;i<plan.files.length;i++)handles.push(await (await directory.getFileHandle(`entry-${i}`,{create:true})).createSyncAccessHandle());
      let last=0;
      ironwoodSevenZipExtract(file.size,read,(name,offset,bytes)=>{const i=index.get(name);if(i!==undefined){const n=handles[i].write(bytes,{at:offset});if(n!==bytes.length)throw Error('Incomplete temporary file write');}},(done,total)=>{const now=performance.now();if(now-last>150||done===total){last=now;self.postMessage({type:'progress',done,total});}});
      for(let i=0;i<handles.length;i++){handles[i].truncate(plan.files[i].size);handles[i].flush();}close();self.postMessage({type:'ready',plan});
    }
  }catch(error){close();self.postMessage({type:'error',message:error.message||String(error)});}
};
