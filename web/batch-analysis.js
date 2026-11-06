// Isolated engine session: batch work never loads games into the active board.
export function parseBatchInfo(line,fen) {
  if(!line.startsWith('info ')||/\b(?:lowerbound|upperbound)\b/.test(line))return null;
  const score=/\bscore (cp|mate) (-?\d+)\b/.exec(line),multi=/\bmultipv (\d+)/.exec(line);
  if(!score||(multi&&multi[1]!=='1'))return null;
  const sign=fen.split(' ')[1]==='w'?1:-1;
  const number=key=>Number(new RegExp(`\\b${key} (\\d+)`).exec(line)?.[1]||0);
  return {eval_cp:score[1]==='cp'?Number(score[2])*sign:null,mate:score[1]==='mate'?Number(score[2])*sign:null,depth:number('depth'),nodes:number('nodes'),best_move:null,pv:/\bpv (.+)$/.exec(line)?.[1]||''};
}

export async function createBatchEngine(engine,signal) {
  let worker,adapter,waiting,failure;
  const command=line=>adapter?adapter.command(line):worker.postMessage({type:'command',command:line});
  const fail=error=>{failure=error instanceof Error?error:Error(String(error));waiting?.reject(failure);};
  const emit=line=>waiting?.line(line);
  const wait=(handler,timeout=30000)=>new Promise((resolve,reject)=>{
    if(signal.aborted)return reject(new DOMException('Cancelled','AbortError'));
    if(failure)return reject(failure);
    const timer=setTimeout(()=>finish(reject,Error('Engine response timed out')),timeout);
    const abort=()=>finish(reject,new DOMException('Cancelled','AbortError'));
    const finish=(done,value)=>{clearTimeout(timer);signal.removeEventListener('abort',abort);waiting=null;done(value);};
    waiting={reject:error=>finish(reject,error),line:line=>handler(line,value=>finish(resolve,value))};
    signal.addEventListener('abort',abort,{once:true});
  });
  const close=async()=>{waiting?.reject(new DOMException('Cancelled','AbortError'));if(adapter)await adapter.release();if(worker){command('quit');worker.terminate();}};
  try {
    const ready=wait((line,done)=>{if(line==='uciok')done();},180000);
    ready.catch(()=>{});
    if(engine.identity){
      const [{CompanionEngineAdapter},{companionEngineConfiguration}]=await Promise.all([import('./companion-engine-adapter.js'),import('./companion-connection.js')]);
      if(window.ironwoodCompanionFeatureEnabled?.()!==true)throw Error('Companion is disabled');
      adapter=new CompanionEngineAdapter(emit,fail);
      await adapter.connect(companionEngineConfiguration(engine.identity));
      if(adapter.details.options.some(option=>option.name==='UCI_LimitStrength'))adapter.overrides.UCI_LimitStrength='false';
    }else{
      worker=new Worker(new URL('./engine-worker.js',import.meta.url));
      worker.onmessage=({data})=>data.type==='uci'?emit(data.line):data.type==='error'?fail(data.message):null;
      worker.onerror=event=>fail(event.message||'Engine worker failed');
      worker.postMessage({type:'init',threads:2,hash:64,limitStrength:false});
    }
    await ready;
    command('setoption name MultiPV value 1');
    command('setoption name UCI_LimitStrength value false');
    return {newGame:()=>command('ucinewgame'),close,async search(fen,milliseconds=1000,chess960=false){
      if(chess960&&adapter&&!adapter.details.options.some(option=>option.name==='UCI_Chess960'&&option.kind==='check'))throw Error('Select a Chess960-compatible engine for this game');
      let latest=null;
      const result=wait((line,done)=>{
        const info=parseBatchInfo(line,fen);if(info)latest=info;
        if(line.startsWith('bestmove ')){
          if(!latest){fail(Error('Engine returned no evaluation'));return;}
          const best=line.split(/\s+/)[1];latest.best_move=['0000','(none)'].includes(best)?null:best;done(latest);
        }
      });
      command(`setoption name UCI_Chess960 value ${chess960}`);
      command(`position fen ${fen}`);command(`go movetime ${milliseconds}`);return result;
    }};
  }catch(error){await close();throw error;}
}

export async function runBatchAnalysis({ids,engine,read,save,prepare,normalize,signal,onProgress,verify=true,identifyCritical=()=>[],createEngine=createBatchEngine}) {
  const result={completed:0,partial:0,failed:0,errors:[],cancelled:false};
  let session;
  try {
    session=await createEngine(engine,signal);
    for(let gameIndex=0;gameIndex<ids.length&&!signal.aborted;gameIndex++) {
      let record,saved,positions,values=[],targets=[],verified=[];
      try {
        record=await read(ids[gameIndex]);if(!record?.json)throw Error('Saved game is unavailable');
        saved=JSON.parse(record.json);positions=prepare(record.json).positions;
        if(!positions?.length)throw Error('No positions to analyze');
        values=Array(positions.length).fill(null);await session.newGame();
        for(let index=0;index<positions.length&&!signal.aborted;index++) {
          onProgress({gameIndex,total:ids.length,pass:1,position:index,positions:positions.length,title:record.title||`Game ${gameIndex+1}`});
          const position=positions[index];
          values[index]=position.terminal?{eval_cp:position.terminal==='draw'?0:null,mate:position.terminal==='mate'?0:null,depth:0,nodes:0,best_move:null,pv:''}:await session.search(position.fen,1000,position.chess960);
        }
        if(verify&&values.every(Boolean)&&!signal.aborted){
          targets=identifyCritical(JSON.stringify({...saved,game_analysis:values}));
          if(!Array.isArray(targets)||targets.some(index=>!Number.isInteger(index)||index<0||index>=positions.length))throw Error('Invalid critical analysis targets');
          targets=[...new Set(targets)].sort((a,b)=>a-b);verified=Array(positions.length).fill(null);
          for(let step=0;step<targets.length&&!signal.aborted;step++){
            const index=targets[step];onProgress({gameIndex,total:ids.length,pass:2,position:step,positions:targets.length,title:record.title||`Game ${gameIndex+1}`});
            verified[index]=positions[index].terminal?values[index]:await session.search(positions[index].fen,5000,positions[index].chess960);
          }
          if(targets.every(index=>verified[index]))for(const index of targets)values[index]=verified[index];
        }
      }catch(error){
        if(error.name!=='AbortError'){result.failed++;result.errors.push(`${record?.title||ids[gameIndex]}: ${error.message}`);}
      }
      if(values.some(Boolean)) {
        try {
          const config={...saved.analysis_config,opponent:engine.identity?'Companion':'Stockfish',companion_identity:engine.identity||'',companion_name:engine.label,companion_options:{},uci_options:{},threads:2,hash_mib:64,limit_strength:false,multipv:1};
          const settings={...saved.full_game_analysis_config,budget:'Seconds',seconds_per_position:1};
          const run={id:crypto.randomUUID(),created_at:Date.now(),engine_name:engine.label,engine_identity:engine.identity||'builtin:stockfish',engine_options:{MultiPV:'1',UCI_LimitStrength:'false'},settings,settings_label:verify?'1 second per position · critical positions: 5 seconds':'1 second per position',config_key:JSON.stringify([config,settings,verify]),verification:targets.length>0,automatic_verification:verify,parent_id:saved.analysis_run?.id||null,position_fens:positions.map(p=>p.fen)};
          const updated={...saved,analysis_config:config,full_game_analysis_config:settings,analysis_run:run,game_analysis:values,game_analysis_running:false,game_analysis_paused:false,verification_targets:targets,verification_results:verified,verification_nodes:0,verification_time_ms:targets.length?5000:0};
          await save(record,normalize(JSON.stringify(updated)));
          if(values.every(Boolean)&&targets.every(index=>verified[index]))result.completed++;else result.partial++;
        }catch(error){result.failed++;result.errors.push(`${record.title||ids[gameIndex]}: Save failed: ${error.message}`);}
      }
      onProgress({gameIndex:gameIndex+1,total:ids.length,position:0,positions:0,title:'Saving progress',result});
    }
  }finally{await session?.close();}
  result.cancelled=signal.aborted;return result;
}
