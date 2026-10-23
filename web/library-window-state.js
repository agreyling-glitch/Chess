const prefix='ironwood.saved-games.';
function writeWindowState(target,kind,value){try{target.localStorage.setItem(prefix+kind,JSON.stringify(value));}catch{}}
export function rememberPopup(target){
  const save=()=>writeWindowState(target,'popup',{width:target.innerWidth,height:target.innerHeight,left:target.screenX,top:target.screenY});
  const timer=target.setInterval(save,1000);
  target.addEventListener('resize',save);target.addEventListener('pagehide',()=>{save();target.clearInterval(timer);},{once:true});
}
