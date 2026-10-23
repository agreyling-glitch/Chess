import {installCompanionFeature} from './companion-feature.js';
import {rememberPopup} from './library-window-state.js';
export function installLibraryWindow(target) {
  target.ironwoodLibraryWindowMode=true;installCompanionFeature(target);
  const opener=()=>{try{const parent=target.opener;return parent&&!parent.closed&&parent.location.origin===target.location.origin?parent:null;}catch{return null;}};
  let inherited;
  if(target.ironwoodCompanionFeatureEnabled())inherited=opener()?.ironwoodCompanionConnection?.();
  const sharedConnection=()=>{const parent=opener();return parent?.ironwoodCompanionConnection?parent.ironwoodCompanionConnection():inherited;};
  const archive=async(options={})=>{
    if(!target.ironwoodCompanionFeatureEnabled())throw Error('Companion testing is disabled.');
    const [{ArchiveLibrary},{CompanionArchiveClient},connections]=await Promise.all([import('./companion-library.js'),import('./companion-archive-client.js'),import('./companion-connection.js')]);
    target.ironwoodCompanionLibrary=archive;
    const shared=sharedConnection()||connections.companionConnection()||(options.interactive===false?null:await connections.requireCompanionConnection());
    if(!shared)throw Error('Connect to Companion to access the archive.');
    return new ArchiveLibrary(new CompanionArchiveClient(shared.endpoint,shared.credential),{...options,connection:()=>sharedConnection()||connections.companionConnection()});
  };
  target.ironwoodCompanionLibrary=archive;
  target.addEventListener('storage',event=>{
    if(event.key==='ironwood.experimental.companion')target.dispatchEvent(new target.Event('ironwood-companion-feature-changed'));
    if(event.key==='ironwood.companion.connection.v1')target.dispatchEvent(new target.Event('ironwood-companion-connection-changed'));
  });
  target.ironwoodLibraryWindowNavigate=(focus=true)=>{
    const parent=opener();
    if(parent){parent.location.reload();if(focus)parent.focus();}
    else if(focus)target.location.assign('/play/');
  };
  return {opener};
}
export async function startLibraryWindow(target=window,{loadApp=()=>import('./pkg/battle_chess.js'),loadStorage=()=>import('./game-storage.js')}={}) {
  const {opener}=installLibraryWindow(target),loading=target.document.getElementById('library-loading');
  try {
    const state=await opener()?.ironwoodLibraryPopupState?.();
    const app=await loadApp();await app.default();
    target.ironwoodExportSavedGame=app.export_saved_game;target.ironwoodPrepareArchivedPgn=app.prepare_archived_pgn;
    await loadStorage();
    if(loading)loading.hidden=true;
    await target.ironwoodOpenGameLibrary(state?{windowState:state}:null);
  }catch(error){if(loading){loading.hidden=false;loading.setAttribute('role','alert');loading.textContent=`Saved games are unavailable: ${error.message}`;}}
}
if(typeof window!=='undefined'&&window.location.pathname==='/saved-games/'){if(window.opener)rememberPopup(window);void startLibraryWindow();}
