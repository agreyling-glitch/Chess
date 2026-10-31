import { addCompanionEngine } from './companion-add-engine.js';
import { manageCompanionEngines } from './companion-manage-engines.js';
import { companionEngineConnections, onCompanionConnectionsChange } from './companion-connection.js';
import { CompanionEngineAdapter } from './companion-engine-adapter.js';

let change = '', allowed = window.ironwoodCompanionAllowed ?? true;
let state = {engines:[], busy:false, status:''}, revision = 0, connection, catalogRequested = false;
// Session-only capabilities. Connections and explicit refreshes invalidate them;
// credentials and capabilities are never persisted by this cache.
const capabilityCache = new Map();
const enabled = () => window.ironwoodCompanionFeatureEnabled?.() !== false;
const sameConnection = shared => {
  return companionEngineConnections().some(current => current.endpoint === shared.endpoint && current.credential === shared.credential);
};
onCompanionConnectionsChange(() => {
  revision++; connection?.close(); connection = null;
  capabilityCache.clear();
  change = '';
  catalogRequested = false;
  state = {engines:[], busy:false, status:'Connection changed. Refresh the engine list.'};
});
window.ironwoodCompanionPollChange = () => { const value = change; change = ''; return value; };
window.ironwoodCompanionSetAllowed = value => { allowed = value; window.ironwoodCompanionAllowed = value; };
window.ironwoodCompanionEngineCatalog = () => {
  if (enabled() && !catalogRequested) void window.ironwoodRefreshCompanionEngines();
  return JSON.stringify(enabled() ? {...state, has_engine_provider: companionEngineConnections().length > 0} : {engines:[],busy:false,status:''});
};

window.ironwoodRefreshCompanionEngines = async () => {
  if (!enabled() || state.busy) return;
  capabilityCache.clear();
  catalogRequested = true;
  const providers = companionEngineConnections(), turn = ++revision;
  if (!providers.length) { state = {engines:[],busy:false,status:'Connect using Game → Companion connection… and enable an engine provider.'}; return; }
  state = {...state,busy:true,status:'Reading installed engines…'};
  const results = await Promise.all(providers.map(async shared => {
    try {
      const response = await fetch(new URL('/api/v1/engines', shared.endpoint), {
        headers:{Authorization:`Bearer ${shared.credential}`,'X-Ironwood-Protocol':'1'},
        credentials:'omit', redirect:'error', cache:'no-store', signal:AbortSignal.timeout(8000)
      });
      if (!response.ok) throw new Error(`Could not read engines (${response.status})`);
      const engines = await response.json();
      return {shared,engines:engines.map(engine => ({...engine,engine_label:engine.label,label:`${engine.label} — ${shared.name}`,provider_name:shared.name,provider_endpoint:shared.endpoint,identity:`${shared.endpoint}/${engine.id}`}))};
    } catch (error) { return {shared,engines:[],error:`${shared.name}: ${error.message}`}; }
  }));
  if (turn !== revision || !enabled()) return;
  const valid = results.filter(result => sameConnection(result.shared));
  state = {engines:valid.flatMap(result => result.engines),busy:false,status:valid.filter(result => result.error).map(result => result.error).join('; ')};
};
window.ironwoodSelectCompanionEngine = async identity => {
  if (!enabled() || !allowed || state.busy) return;
  if (!identity) { change = JSON.stringify({provider:'builtin'}); state.status = ''; return; }
  const engine = state.engines.find(engine => engine.identity === identity);
  const shared = companionEngineConnections().find(item => item.endpoint === engine?.provider_endpoint);
  if (!shared || !engine) { state.status = 'Refresh the engine list and select an engine.'; return; }
  const cached = capabilityCache.get(identity);
  if (cached) {
    change = JSON.stringify({provider:'companion',name:cached.name,identity,options:{},uci_options:cached.options});
    state = {...state,pending_identity:null,status:''};
    return;
  }
  const turn = ++revision;
  state = {...state,busy:true,pending_identity:identity,status:`Preparing ${engine.engine_label}…`};
  const adapter = new CompanionEngineAdapter(() => {}, () => {});
  connection = adapter;
  try {
    const details = await adapter.connect({...shared,engineId:engine.id});
    await adapter.release();
    if (turn !== revision || !enabled() || !allowed || !sameConnection(shared)) return;
    capabilityCache.set(identity, structuredClone({name:details.name,options:details.options}));
    change = JSON.stringify({provider:'companion',name:details.name,identity,options:{},uci_options:details.options});
    state.status = '';
  } catch (error) { if (turn === revision) state.status = error.message; }
  finally {
    adapter.close();
    if (connection === adapter) connection = null;
    if (turn === revision) { state.busy = false; state.pending_identity = null; }
  }
};
window.ironwoodAddCompanionEngine = async () => {
  if (!enabled() || !allowed || state.busy) return;
  const providers = companionEngineConnections();
  let shared = providers[0];
  if (providers.length > 1) {
    if (!document.querySelector('#companion-connection-style')) {
      const style = document.createElement('link'); style.id = 'companion-connection-style'; style.rel = 'stylesheet'; style.href = '/companion-archive.css'; document.head.append(style);
    }
    shared = await new Promise(resolve => {
      const dialog = document.createElement('dialog'); dialog.id = 'companion-engine-provider';
      const title = document.createElement('h2'); title.textContent = 'Add engine to Companion'; dialog.append(title);
      for (const provider of providers) {
        const button = document.createElement('button'); button.type = 'button'; button.textContent = provider.name;
        button.onclick = () => { resolve(provider); dialog.close(); }; dialog.append(button);
      }
      const cancel = document.createElement('button'); cancel.textContent = 'Cancel'; cancel.onclick = () => dialog.close(); dialog.append(cancel);
      dialog.addEventListener('close', () => { dialog.remove(); resolve(null); }, {once:true});
      document.body.append(dialog); dialog.showModal();
    });
  }
  if (shared && sameConnection(shared)) await addCompanionEngine(shared);
  else if (!providers.length) state.status = 'Add an engine provider in Companion connections first.';
  await window.ironwoodRefreshCompanionEngines();
};
window.ironwoodManageCompanionEngines=async(identity='')=>{
  if(!enabled()||!allowed||state.busy)return;
  const providers=companionEngineConnections();
  if(!providers.length){state.status='Connect an engine provider first.';return;}
  await manageCompanionEngines(providers,{identity,isCurrent:sameConnection,onSaved:()=>void window.ironwoodRefreshCompanionEngines()});
  await window.ironwoodRefreshCompanionEngines();
};
