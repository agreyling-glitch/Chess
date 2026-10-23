import { companionConnection, manageCompanionConnection } from './companion-connection.js';
import { CompanionArchiveClient } from './companion-archive-client.js';
import { previewCompanionDocument } from './game-storage.js';

// One current query, polled for generation changes. Old connection/filter/position
// responses cannot publish into a newer view; credentials never enter cache keys.
export class ArchiveExplorer {
  constructor(connection, {createClient = (endpoint, credential) => new CompanionArchiveClient(endpoint, credential), clock = () => Date.now()} = {}) {
    this.connection = connection; this.createClient = createClient; this.clock = clock;
    this.filters = {}; this.cursor = ''; this.epoch = 0; this.notice = ''; this.refreshAt = 0;
  }
  configure() {
    const selected = this.connection();
    if (!selected) { this.client = null; this.identity = null; this.invalidate(); return null; }
    if (!this.identity || this.identity.endpoint !== selected.endpoint || this.identity.credential !== selected.credential) {
      this.identity = {...selected}; this.client = this.createClient(selected.endpoint, selected.credential); this.notice = ''; this.invalidate();
    }
    return this.client;
  }
  invalidate() { this.epoch++; this.pending = null; this.data = null; this.refreshAt = 0; }
  setFilters(filters) { this.filters = {...filters}; this.cursor = ''; this.invalidate(); }
  page(next) { this.cursor = next ? this.data?.next || '' : ''; this.invalidate(); }
  poll(fen) {
    let client;
    try { client = this.configure(); } catch (error) { return JSON.stringify({error:error.message}); }
    if (!client) return JSON.stringify({error:'Connect through Storage → Companion archive or Game → Companion engines to explore your archived games.'});
    if (this.fen !== fen) { this.fen = fen; this.cursor = ''; this.invalidate(); }
    if (!this.pending && this.clock() >= this.refreshAt) {
      const epoch = this.epoch, query = new URLSearchParams({...this.filters, fen, variant:'standard', game_after:this.cursor, game_limit:'10'});
      this.pending = client.request(`archive/explorer?${query}`).then(data => {
        if (epoch !== this.epoch) return;
        if (this.data && data.generation !== this.data.generation && this.cursor) { this.cursor = ''; this.invalidate(); return; }
        this.data = data; this.refreshAt = this.clock() + (data.indexing ? 500 : 2000);
      }).catch(error => { if (epoch === this.epoch) { this.data = {error:error.message}; this.refreshAt = this.clock()+2000; } })
        .finally(() => { if (epoch === this.epoch) this.pending = null; });
    }
    const filters = Object.entries(this.filters).filter(([key,value]) => value && !(key === 'color' && value === 'either'))
      .map(([key,value]) => key === 'collection' ? 'Selected collection' : `${key.replaceAll('_',' ')}: ${value}`).join(' · ');
    return this.data ? JSON.stringify({...this.data,notice:[filters ? `Filters: ${filters}` : 'All archived games',this.notice].filter(Boolean).join('\n')}) : '';
  }
}
function element(tag, text, parent) { const node = document.createElement(tag); if (text) node.textContent = text; parent?.append(node); return node; }
function input(parent, title, value, type = 'text') { const node = element('input','',element('label',title,parent)); node.type = type; node.value = value || ''; if(type==='number'){node.min='1';node.max='5000';} return node; }
export function installArchiveExplorer(target) {
  const explorer = new ArchiveExplorer(companionConnection);
  target.ironwoodArchiveExplorer = fen => explorer.poll(fen);
  target.ironwoodArchiveExplorerRefresh = () => explorer.invalidate();
  target.ironwoodArchiveExplorerPage = next => explorer.page(next);
  let previewing = false;
  target.ironwoodArchiveExplorerPreviewGame = async id => {
    if(target.ironwoodCompanionFeatureEnabled?.() !== true || previewing || !/^[a-f0-9]{32}$/.test(id))return;
    previewing = true;
    try {
      const client = explorer.configure(), epoch = explorer.epoch;
      if(!client)throw new Error('Reconnect to Companion first.');
      const transfer = await client.request('archive/export',{ids:[id],format:'json'});
      if(epoch!==explorer.epoch || target.ironwoodCompanionFeatureEnabled?.() !== true)return;
      await previewCompanionDocument(transfer.documents?.[0], target.document?.getElementById('chess_canvas'));
      explorer.notice = '';
    } catch(error) { explorer.notice = error.message; }
    finally { previewing = false; }
  };
  target.ironwoodOpenArchiveExplorerFilters = async () => {
  if (window.ironwoodCompanionFeatureEnabled?.() === false) return;
    if(document.querySelector('#companion-explorer-filters'))return;
    if(!document.querySelector('#companion-explorer-style')){const style=element('link','',document.head);style.id='companion-explorer-style';style.rel='stylesheet';style.href='/companion-archive.css';}
    const dialog=element('dialog','',document.body);dialog.id='companion-explorer-filters';
    element('p','IRONWOOD COMPANION',dialog).className='eyebrow';element('h2','Archive explorer filters',dialog);
    element('p','Use games stored on your Companion. Unknown ratings or dates are excluded when those filters are applied. No external database is queried.',dialog);
    const manage=element('button','Manage connection',dialog); manage.onclick=async()=>{dialog.close();await manageCompanionConnection();};
    const section=element('section','',dialog), collection=element('select','',element('label','Collection',section));collection.setAttribute('aria-label','Collection');
    element('option','All collections',collection).value='';
    const player=input(section,'Player name',explorer.filters.player), color=element('select','',element('label','Player color',section));color.setAttribute('aria-label','Player color');
    for(const [value,text]of[['either','Either color'],['white','White'],['black','Black']]){const option=element('option',text,color);option.value=value;}color.value=explorer.filters.color||'either';
    const fields={date_from:input(section,'Date from',explorer.filters.date_from,'date'),date_to:input(section,'Date to',explorer.filters.date_to,'date'),white_elo_min:input(section,'White rating minimum',explorer.filters.white_elo_min,'number'),white_elo_max:input(section,'White rating maximum',explorer.filters.white_elo_max,'number'),black_elo_min:input(section,'Black rating minimum',explorer.filters.black_elo_min,'number'),black_elo_max:input(section,'Black rating maximum',explorer.filters.black_elo_max,'number')};
    const status=element('p','',dialog);status.setAttribute('role','status');
    const actions=element('div','',dialog);actions.className='actions';const apply=element('button','Apply archive filters',actions),reset=element('button','Reset filters',actions),rebuild=element('button','Rebuild opening index',actions),close=element('button','Cancel',actions);
    apply.onclick=()=>{for(const field of Object.values(fields))if(!field.reportValidity())return;const filters={collection:collection.value,player:player.value.trim(),color:color.value};for(const [key,field]of Object.entries(fields))if(field.value)filters[key]=field.value;explorer.setFilters(filters);dialog.close();};
    reset.onclick=()=>{explorer.setFilters({});dialog.close();};close.onclick=()=>dialog.close();
    rebuild.onclick=async()=>{rebuild.disabled=true;try{const client=explorer.configure();if(!client)throw new Error('Connect to Companion archive first.');const progress=await client.request('archive/explorer/rebuild',{});explorer.cursor='';explorer.invalidate();status.textContent=`Rebuild queued: ${progress.pending} games. Archived games are preserved and remain available.`;}catch(error){status.textContent=error.message;}finally{rebuild.disabled=false;}};
    dialog.addEventListener('close',()=>dialog.remove(),{once:true});dialog.showModal();
    try{const client=explorer.configure();if(!client)throw new Error('Connect through Storage → Companion archive or Game → Companion engines first.');const [groups,progress]=await Promise.all([client.request('archive/collections'),client.request('archive/explorer/status')]);if(!dialog.isConnected)return;for(const group of groups){const option=element('option',group.name,collection);option.value=group.id;}collection.value=explorer.filters.collection||'';status.textContent=`${progress.indexed}/${progress.total} games indexed · ${progress.pending} pending · ${progress.unsupported_variants} other variants · ${progress.failed} failed. Generation ${progress.generation}.`;}catch(error){status.textContent=error.message;}
  };
  return explorer;
}
if(typeof window!=='undefined' && window.document)installArchiveExplorer(window);
