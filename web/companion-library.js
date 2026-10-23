import {CompanionArchiveClient,browserDocuments} from './companion-archive-client.js';
import {companionConnection,requireCompanionConnection} from './companion-connection.js';
export const ARCHIVE_IMPORT_BATCH_SIZE=128;
const IMPORT_BYTES=15*1024*1024;
const yieldImport=()=>new Promise(resolve=>setTimeout(resolve,0));
// One in-flight import per client, including retries and transfers from Browser.
const importQueues=new WeakMap(),importPacers=new WeakMap();
export class ImportPacer {
  constructor(now=Date.now,sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms))) {this.now=now;this.sleep=sleep;this.window=now();this.count=0;this.latency=0;}
  async wait(budget) {
    const now=this.now();let delay=0;
    if(budget && now<budget.resetAt) {
      const reserve=Math.min(budget.limit-1,Math.ceil(budget.limit/4));
      if(budget.remaining<=reserve)delay=budget.resetAt-now+50;
      else if(budget.busy)delay=Math.min(1000,Math.max(200,this.latency/2));
    } else {
      if(now-this.window>=60000){this.window=now;this.count=0;}
      if(this.count>=120)delay=Math.max(0,this.window+60050-now);
    }
    if(delay)await this.sleep(delay);
    if(this.now()-this.window>=60000){this.window=this.now();this.count=0;}
    this.count++;
  }
}
// This retry applies only to content-deduplicated archive imports, never to
// arbitrary edits/deletions. Keep the exact documents across uncertain replies.
export async function importWithRetry(storage, documents, sleep = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds)), onRetry) {
  return archiveRequestWithRetry(storage,'archive/import',{documents},sleep,onRetry);
}
// Only call this for reads or content-deduplicated imports, not edits/deletions.
export async function archiveRequestWithRetry(storage, path, body, sleep = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds)), onRetry, isActive = () => true) {
  for(let attempt=0;;attempt++) {
    if(!isActive())throw Error('Archive loading was cancelled.');
    try { const result=await storage.request(path,body);onRetry?.('');return result; }
    catch(error) {
      const transient=[408,429,500,502,503,504].includes(error.status)
        || (!error.status && (error instanceof TypeError || ['AbortError','TimeoutError'].includes(error.name)));
      if(!transient || attempt>=4)throw error;
      const delay=error.retryAfterMs || (error.status===429 ? (error.code==='busy'?Math.min(5000,500*2**attempt):60000) : [1000,4000,15000,45000][attempt]);
      onRetry?.(`${error.status===429 ? 'Companion is busy' : 'Connection interrupted'}. Retrying automatically in ${Math.ceil(delay/1000)} seconds…`);
      await sleep(delay);storage.check();
    }
  }
}
export class ArchiveLibrary {
  constructor(client,{details,prepare,exporter,receive,connection=companionConnection}) { Object.assign(this,{client,endpoint:client.endpoint,details,prepare,exporter,receive,connection}); }
  check() {
    const current=this.connection();
    if (globalThis.window?.ironwoodCompanionFeatureEnabled?.() === false || current?.endpoint!==this.endpoint || current?.credential!==this.client.credential) throw Error('Companion connection changed. Reconnect and refresh the archive.');
  }
  async request(path,body) {
    const perform=async()=>{
      this.check();
      if(!importPacers.has(this.client))importPacers.set(this.client,new ImportPacer());
      const pacer=importPacers.get(this.client);
      if(path==='archive/import')await pacer.wait(this.client.importBudget);
      this.check();const start=Date.now();
      try {const value=await this.client.request(path,body);this.check();return value;}
      finally {if(path==='archive/import')pacer.latency=Date.now()-start;}
    };
    if(path!=='archive/import')return perform();
    const pending=(importQueues.get(this.client)||Promise.resolve()).then(perform);
    importQueues.set(this.client,pending.catch(()=>{}));return pending;
  }
  record(game) {
    const record=game.document.record ? structuredClone(game.document.record) : {id:game.id,json:this.prepare(game.document.pgn),category:'imported',favorite:false,updatedAt:game.updated_at};
    return {...record,...(game.document.analysis_runs?.length?{archiveAnalysisRuns:game.document.analysis_runs}:{}),id:game.id,_archive:{endpoint:this.endpoint,id:game.id,revision:game.revision,document:game.document}};
  }
  async list(isActive=()=>true,onProgress=()=>{}) {
    const records=[], seen=new Set();let after='';
    const read=(path,body)=>archiveRequestWithRetry(this,path,body,undefined,message=>{if(isActive())onProgress(records.length,message);},isActive);
    const append=async batch=>{
      if(!batch.length||!isActive())return;
      let bundle;
      try {bundle=await read('archive/export',{ids:batch.map(game=>game.id),format:'json'});}
      catch(error) {
        if(batch.length>1 && (error.status===413 || (error.status===400 && /exceeds 16 MiB/i.test(error.message)))) {
          const middle=Math.ceil(batch.length/2);await append(batch.slice(0,middle));await append(batch.slice(middle));return;
        }
        throw error;
      }
      if(!isActive())return;
      if(bundle.documents?.length!==batch.length)throw Error('Incomplete archive response. Refresh and try again.');
      batch.forEach((game,index)=>records.push(this.record({...game,document:bundle.documents[index]})));
      onProgress(records.length,'');
    };
    do {
      if(!isActive())return [];
      const page=await read(`archive/games?limit=100&after=${encodeURIComponent(after)}`);
      await append(page.games);
      after=page.next || '';if(after&&seen.has(after))throw Error('Invalid archive pagination.');seen.add(after);
    } while(after);
    return records;
  }
  async page(filters, isActive=()=>true, {prefetch=false}={}) {
    this.check();this.libraryFacets ??= new Map();
    const category=filters.category||'all',cached=this.libraryFacets.get(category);
    const query=new URLSearchParams({...filters,limit:10,...(cached?.revision!==undefined?{facets_revision:cached.revision}:{})});
    for(const key of ['source','date','opening_choice'])if(!query.get(key)||query.get(key)==='*')query.delete(key);
    const path=`archive/library?${query}`;
    if(!isActive())throw Error('Archive loading was cancelled.');
    const result=prefetch?await this.request(path):await archiveRequestWithRetry(this,path,undefined,undefined,undefined,isActive);
    if(!isActive())throw Error('Archive loading was cancelled.');
    let facets;
    if(result.counts&&result.players&&result.profiles) {
      facets={revision:result.revision,counts:result.counts,players:result.players,profiles:result.profiles,sources:result.sources||[],dates:result.dates||[],openings:result.openings||[]};
      this.libraryFacets.delete(category);this.libraryFacets.set(category,facets);
      while(this.libraryFacets.size>8)this.libraryFacets.delete(this.libraryFacets.keys().next().value);
    } else if(cached&&cached.revision===result.revision)facets=cached;
    else throw Error('Archive filter choices changed. Refresh and try again.');
    return {...result,...facets,games:result.games.map(game=>({...game,_archive:{endpoint:this.endpoint,id:game.id,revision:game.revision}}))};
  }
  async selection(filters,isActive=()=>true,onProgress=()=>{}) {
    const games=[],seen=new Set();let total,revision,limit=this.selectionLimit||1000;
    for(let page=0;;page++) {
      const query=new URLSearchParams({...filters,selection:true,page,limit});
      for(const key of ['source','date','opening_choice'])if(!query.get(key)||query.get(key)==='*')query.delete(key);
      let result;
      try {result=await archiveRequestWithRetry(this,`archive/library?${query}`,undefined,undefined,undefined,isActive);}
      catch(error){
        // Earlier private-preview servers allow only 100 selection IDs per page.
        if(error.status===400&&page===0&&limit===1000){this.selectionLimit=limit=100;page=-1;continue;}
        throw error;
      }
      if(!isActive())throw Error('Archive loading was cancelled.');
      total ??= result.total;revision ??= result.revision;
      if(result.total!==total || (revision!==undefined&&result.revision!==revision) || result.games.some(game=>seen.has(game.id)))throw Error('Archive changed during selection. Refresh and select again.');
      for(const game of result.games){seen.add(game.id);games.push({...game,_archive:{endpoint:this.endpoint,id:game.id,revision:game.revision}});}
      if(games.length>total)throw Error('Archive changed during selection. Refresh and select again.');
      onProgress(games.length,total);
      if(games.length===total)return games;
      if(!result.games.length)throw Error('Archive changed during selection. Refresh and select again.');
    }
  }
  async stats() {const [stats,index,capabilities]=await Promise.all([this.request('archive/stats'),this.request('archive/explorer/status'),this.request('capabilities')]);return {...stats,index,server_version:capabilities.server_version};}
  async read(id) { return this.record(await archiveRequestWithRetry(this,`archive/games/${id}`)); }
  async readMany(ids,onProgress=()=>{}) {
    const records=[];
    const batch=async selected=>{
      let result;
      try {result=await archiveRequestWithRetry(this,'archive/export',{ids:selected,format:'json'});}
      catch(error){if(selected.length>1&&(error.status===413||error.status===400&&/exceeds 16 MiB/i.test(error.message))){const half=Math.ceil(selected.length/2);await batch(selected.slice(0,half));await batch(selected.slice(half));return;}throw error;}
      if(result.documents?.length!==selected.length)throw Error('Incomplete archive response.');
      result.documents.forEach((document,index)=>records.push(this.record({id:selected[index],document})));
      onProgress(records.length,ids.length);
    };
    for(let offset=0;offset<ids.length;offset+=100)await batch(ids.slice(offset,offset+100));
    return records;
  }
  async write(record) {
    const {_archive,id,...content}=record;
    if(!_archive || _archive.endpoint!==this.endpoint)throw Error('Archive game identity is missing. Refresh and try again.');
    const document=structuredClone(_archive.document);
    document.record={...content,id:document.record?.id || id};
    if(document.record.json!==_archive.document.record?.json){const regenerated=browserDocuments([document.record],this.exporter,this.details)[0];document.pgn=regenerated.pgn;document.metadata={...document.metadata,...regenerated.metadata};}
    const updated=await this.request(`archive/games/${id}/update`,{revision:_archive.revision,document});
    return this.record(updated);
  }
  async remove(records) {
    for(const record of records)await this.request(`archive/games/${record.id}/delete`,{revision:record._archive.revision,...(record._archive.analysis_runs?{analysis_runs:record._archive.analysis_runs}:{})});
  }
  async importRecords(records) {
    const ids=[];
    let batch=[],size=0;
    const commit=async()=>{
      if(!batch.length)return;
      const result=await this.request('archive/import',{documents:batch});
      if(result.ids?.length!==batch.length)throw Error('Archive did not confirm every destination game. Browser originals were kept.');
      ids.push(...result.ids);batch=[];size=0;
    };
    for(let index=0;index<records.length;index++){
      if(batch.length>=ARCHIVE_IMPORT_BATCH_SIZE)await commit();
      if(index%8===0)await yieldImport();
      const document=browserDocuments([records[index]],this.exporter,this.details)[0];
      const bytes=new TextEncoder().encode(JSON.stringify(document)).length;
      if(bytes>15*1024*1024)throw Error('A game exceeds the archive transfer limit. Browser originals were kept.');
      if(size+bytes>IMPORT_BYTES)await commit();batch.push(document);size+=bytes;
    }
    await commit();return ids;
  }
  async importBatchRecords(records,progress,onRetry) {
    let batch=[],size=0;
    const commit=async()=>{
      if(!batch.length)return;
      const result=await importWithRetry(this,batch,undefined,onRetry);
      if(result.imported+result.duplicates!==batch.length)throw Error('Archive did not confirm the imported games.');
      progress(result,batch.length);batch=[];size=0;
    };
    for(let index=0;index<records.length;index++){
      if(batch.length>=ARCHIVE_IMPORT_BATCH_SIZE)await commit();
      if(index%8===0)await yieldImport();
      const document=browserDocuments([records[index]],this.exporter,this.details)[0];
      const bytes=new TextEncoder().encode(JSON.stringify(document)).length;
      if(bytes>15*1024*1024)throw Error('A game exceeds the archive import limit.');
      if(size+bytes>IMPORT_BYTES)await commit();
      batch.push(document);size+=bytes;
    }
    await commit();
  }
  async moveToBrowser(ids) {
    const records=[];for(const id of ids){const record=await this.read(id);const result=await this.request(`archive/games/${id}/analysis-runs`);if(result.runs?.length){record._archive.document.analysis_runs=[];for(const run of result.runs)record._archive.document.analysis_runs.push(await this.request(`archive/games/${id}/analysis-runs/${run.id}`));}if(Array.isArray(result.runs))record._archive.analysis_runs=(record._archive.document.analysis_runs||[]).map(run=>[run.id,run.revision]);records.push(record);}
    // Browser commits (including exact duplicates) must succeed before any source deletion.
    const received=await this.receive(records.map(record=>record._archive.document),{returnIds:true});
    if(received.received+received.duplicates!==records.length)throw Error('Browser did not confirm every destination game. Archive originals were kept.');
    await this.remove(records);return received;
  }
}
if(typeof window!=='undefined')window.ironwoodCompanionLibrary=async(options={})=>{
  if(window.ironwoodCompanionFeatureEnabled?.()===false)throw Error('Companion testing is disabled.');
  const shared=options.interactive===false ? companionConnection() : await requireCompanionConnection();
  if(!shared)throw Error('Connect to Companion to access the archive.');
  return new ArchiveLibrary(new CompanionArchiveClient(shared.endpoint,shared.credential),options);
};
