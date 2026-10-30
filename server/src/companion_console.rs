//! Native Windows status dashboard. Uses system fonts and no bundled web runtime.
use super::*;
const TITLE:usize=100;
const READY:usize=101;
const CLIENTS:usize=102;
const ADDRESS:usize=103;
const ENGINE_COUNT:usize=104;
const SESSIONS:usize=105;
const DATA_PATH:usize=106;
const IMPORT_STATUS:usize=107;
const UPDATE_STATUS:usize=108;
const ACTIVITY:usize=109;
const OVERVIEW:usize=110;
const BG:u32=0x00140f0b;
const PANEL:u32=0x00211812;
const SIDE:u32=0x001a130e;
const TEXT:u32=0x00eee9e6;
const MUTED:u32=0x00b2a49a;
const ACCENT:u32=0x004ab8f0;
const BORDER:u32=0x003b2d23;
fn brush(color:u32)->HBRUSH {
    static BRUSHES:std::sync::OnceLock<std::collections::HashMap<u32,usize>>=std::sync::OnceLock::new();
    *BRUSHES.get_or_init(||[BG,PANEL,SIDE,TEXT,MUTED,ACCENT,BORDER,0x000e0a07,0x0030221a,0x0058463a].into_iter().map(|c|(c,unsafe{CreateSolidBrush(c)} as usize)).collect()).get(&color).unwrap() as HBRUSH
}
fn font(size:i32,bold:bool,mono:bool)->HFONT {
    static FONTS:std::sync::OnceLock<std::collections::HashMap<(i32,bool,bool),usize>>=std::sync::OnceLock::new();
    *FONTS.get_or_init(||{
        let mut map=std::collections::HashMap::new();
        for size in [12,13,14,15,16,24,32]{for bold in [false,true]{for mono in [false,true]{
            let handle=unsafe{CreateFontW(-size,0,0,0,if bold{600}else{400},0,0,0,DEFAULT_CHARSET as u32,0,0,CLEARTYPE_QUALITY as u32,0,wide(if mono{"Consolas"}else{"Segoe UI"}).as_ptr())};
            map.insert((size,bold,mono),handle as usize);
        }}}map
    }).get(&(size,bold,mono)).unwrap() as HFONT
}
pub unsafe fn is_console(window:HWND)->bool {unsafe{!GetDlgItem(window,TITLE as i32).is_null()}}
unsafe fn label(window:HWND,id:usize,text:&str,size:i32,bold:bool,center:bool)->HWND {unsafe{
    let child=control(window,"STATIC",text,if center{1}else{0} /* SS_CENTER / SS_LEFT */,0,0,1,1,id);
    SetWindowLongPtrW(child,GWL_STYLE,(GetWindowLongPtrW(child,GWL_STYLE) as u32&!WS_TABSTOP) as isize);
    SendMessageW(child,WM_SETFONT,font(size,bold,false) as usize,0);child
}}
pub unsafe fn create(window:HWND)->(HWND,HWND,HWND){unsafe{
    label(window,TITLE,"Overview",32,true,false);
    label(window,READY,"Ready",15,true,false);
    label(window,CLIENTS,"0",32,true,true);
    label(window,ADDRESS,"127.0.0.1:8790",16,true,true);
    label(window,ENGINE_COUNT,"0",32,true,true);
    label(window,SESSIONS,"0 active sessions",13,false,true);
    label(window,DATA_PATH,"",13,false,false);
    label(window,IMPORT_STATUS,"",13,false,false);
    label(window,UPDATE_STATUS,"",13,false,false);
    let startup=control(window,"BUTTON","Start Companion when I sign in",BS_AUTOCHECKBOX as u32,0,0,1,1,START_AT_SIGN_IN);
    SendMessageW(startup,WM_SETFONT,font(14,false,false) as usize,0);
    let backup=control(window,"EDIT","",ES_AUTOHSCROLL as u32|WS_BORDER,0,0,1,1,BACKUP_FOLDER);
    SendMessageW(backup,WM_SETFONT,font(14,false,false) as usize,0);
    let save=control(window,"BUTTON","Save folder",BS_PUSHBUTTON as u32,0,0,1,1,SAVE_BACKUP_FOLDER);
    SendMessageW(save,WM_SETFONT,font(14,false,false) as usize,0);
    refresh_startup(window);
    for (id,text) in [(OVERVIEW,"▦  Overview"),(ENGINES,"Engines"),(ARCHIVE,"Archive"),(IMPORTS,"Import"),(REPORTS,"Reports"),(SHOW,"Show token…"),(CHECK_UPDATE,"Check for updates…"),(INSTALL_UPDATE,"Update and restart"),(OPEN,"Open Ironwood"),(COPY_ADDRESS,"Copy address"),(STATUS,"Refresh"),(CLOSE_STATUS,"Close")]{
        let child=control(window,"BUTTON",text,BS_PUSHBUTTON as u32,0,0,1,1,id);
        SendMessageW(child,WM_SETFONT,font(if id==OPEN{16}else{14},id==OPEN||id==OVERVIEW,false) as usize,0);
    }
    let edit=control(window,"EDIT","",ES_READONLY as u32|ES_MULTILINE as u32|ES_AUTOVSCROLL as u32,0,0,1,1,ACTIVITY);
    SendMessageW(edit,WM_SETFONT,font(13,false,true) as usize,0);
    let rail=control(window,"STATIC","Scroll activity",0x100,0,0,14,1,111);
    ShowWindow(rail,SW_HIDE);
    SetWindowSubclass(rail,Some(status_scroll_proc),1,edit as usize);
    SetWindowSubclass(edit,Some(status_edit_proc),2,rail as usize);
    layout(window);
    (edit,rail,GetDlgItem(window,INSTALL_UPDATE as i32))
}}
unsafe fn place(window:HWND,id:usize,x:i32,y:i32,w:i32,h:i32){unsafe{let child=GetDlgItem(window,id as i32);MoveWindow(child,x,y,w.max(1),h.max(1),1);InvalidateRect(child,std::ptr::null(),1);}}
pub unsafe fn layout(window:HWND){unsafe{
    let mut r=RECT::default();GetClientRect(window,&mut r);
    let x=242;let w=(r.right-x-28).max(620);let center=x+w/2;let left_width=(w-280)/2;
    place(window,TITLE,x,25,w-200,42);place(window,READY,x,75,w-190,24);
    place(window,OPEN,x+w-170,26,170,44);
    place(window,OVERVIEW,12,20,188,44);
    for (id,y) in [(ENGINES,108),(ARCHIVE,156),(IMPORTS,204),(REPORTS,252),(SHOW,340),(CHECK_UPDATE,388),(INSTALL_UPDATE,436)]{place(window,id,12,y,188,44);}
    place(window,CLIENTS,x+12,154,left_width-72,43);
    place(window,ADDRESS,center-126,161,252,28);
    place(window,COPY_ADDRESS,center-61,200,122,32);
    place(window,ENGINE_COUNT,center+200,147,left_width-72,43);
    place(window,SESSIONS,center+200,216,left_width-72,22);
    place(window,DATA_PATH,x+160,293,w-179,43);
    place(window,IMPORT_STATUS,x+160,352,w-179,60);
    place(window,UPDATE_STATUS,x+160,423,w-179,49);
    place(window,START_AT_SIGN_IN,x+160,480,w-179,32);
    place(window,BACKUP_FOLDER,x+160,522,w-289,30);
    place(window,SAVE_BACKUP_FOLDER,x+w-115,522,105,30);
    place(window,ACTIVITY,x+15,604,w-46,r.bottom-692);
    place(window,111,x+w-26,604,12,r.bottom-692);
    place(window,STATUS,x,r.bottom-65,100,42);place(window,CLOSE_STATUS,x+w-104,r.bottom-65,104,42);
    layout_status_scroll(GetDlgItem(window,ACTIVITY as i32),GetDlgItem(window,111));
}}
unsafe fn set(window:HWND,id:usize,text:&str){unsafe{SetWindowTextW(GetDlgItem(window,id as i32),wide(text).as_ptr());}}
pub unsafe fn refresh_startup(window:HWND){unsafe{
    let checkbox=GetDlgItem(window,START_AT_SIGN_IN as i32);
    match startup::enabled(){
        Ok(enabled)=>{EnableWindow(checkbox,1);SendMessageW(checkbox,BM_SETCHECK,if enabled{BST_CHECKED}else{BST_UNCHECKED} as usize,0);}
        Err(_)=>{EnableWindow(checkbox,0);}
    }
}}
pub unsafe fn update(window:HWND,text:&str)->String {unsafe{
    refresh_startup(window);
    let field=|key:&str|text.lines().find_map(|line|line.strip_prefix(key)).unwrap_or("").trim();
    let status=text.lines().next().unwrap_or("Ready");
    set(window,READY,&format!("●  {status}  ·  Version {}",env!("CARGO_PKG_VERSION")));
    set(window,CLIENTS,field("Connected clients:"));set(window,ADDRESS,field("Address:"));
    set(window,ENGINE_COUNT,field("Registered engines:"));set(window,SESSIONS,&format!("{} active sessions",field("Engine sessions:")));
    set(window,DATA_PATH,field("Data:"));set(window,IMPORT_STATUS,field("Automatic imports:"));
    let update=text.rsplit("\r\n\r\n").next().unwrap_or("");
    let update=if update.is_empty(){"No signed update channel is configured for this development build."}else{update};
    set(window,UPDATE_STATUS,update);
    let imports=text.split_once("Automatic imports: ").map(|(_,s)|s.split("\r\n\r\n").next().unwrap_or(s)).unwrap_or("");
    format!("ok  {status} on {}\r\n\r\n--  {imports}\r\n\r\n!!  {update}",field("Address:"))
}}
unsafe fn text(dc:HDC,value:&str,r:RECT,size:i32,color:u32,center:bool){unsafe{
    SetTextColor(dc,color);SetBkMode(dc,TRANSPARENT as i32);SelectObject(dc,font(size,false,false));
    let mut r=r;DrawTextW(dc,wide(value).as_ptr(),-1,&mut r,DT_SINGLELINE|DT_VCENTER|if center{DT_CENTER}else{DT_LEFT});
}}
unsafe fn panel(dc:HDC,r:RECT,color:u32,edge:u32){unsafe{
    let pen=CreatePen(PS_SOLID,1,edge);let old_pen=SelectObject(dc,pen);let old_brush=SelectObject(dc,brush(color));
    RoundRect(dc,r.left,r.top,r.right,r.bottom,12,12);SelectObject(dc,old_brush);SelectObject(dc,old_pen);DeleteObject(pen);
}}
pub unsafe fn paint(window:HWND,dc:HDC,r:&RECT){unsafe{
    FillRect(dc,r,brush(BG));FillRect(dc,&RECT{left:0,top:0,right:212,bottom:r.bottom},brush(SIDE));
    FillRect(dc,&RECT{left:211,top:0,right:212,bottom:r.bottom},brush(BORDER));
    let x=242;let right=r.right-28;let center=(x+right)/2;let side=(right-x-280)/2;
    panel(dc,RECT{left:x,top:120,right,bottom:265},PANEL,BORDER);
    panel(dc,RECT{left:center-140,top:139,right:center+140,bottom:247},PANEL,ACCENT);
    for (l,rr) in [(x+side-48,center-140),(center+140,center+188)]{FillRect(dc,&RECT{left:l,top:192,right:rr,bottom:194},brush(0x0058463a));}
    text(dc,"Connected clients",RECT{left:x+12,top:202,right:center-200,bottom:230},13,MUTED,true);
    text(dc,"Companion",RECT{left:center-140,top:144,right:center+140,bottom:163},13,TEXT,true);
    text(dc,"Registered engines",RECT{left:center+200,top:190,right:right-12,bottom:215},13,MUTED,true);
    panel(dc,RECT{left:x,top:283,right,bottom:565},PANEL,BORDER);
    for (value,y) in [("Data folder",293),("Automatic imports",352),("Updates",423),("Settings",483),("Backup folder",524)]{text(dc,value,RECT{left:x+16,top:y,right:x+155,bottom:y+25},13,MUTED,false);}
    for y in [343,414,476]{FillRect(dc,&RECT{left:x+1,top:y,right:right-1,bottom:y+1},brush(BORDER));}
    text(dc,"FOLDERS",RECT{left:24,top:80,right:200,bottom:104},12,MUTED,false);
    text(dc,"TOOLS",RECT{left:24,top:308,right:200,bottom:336},12,MUTED,false);
    text(dc,"ACTIVITY",RECT{left:x,top:573,right,bottom:598},12,MUTED,false);
    panel(dc,RECT{left:x,top:599,right,bottom:r.bottom-80},0x000e0a07,BORDER);
    let _=window;
}}
pub unsafe fn control_color(dc:HDC,child:HWND)->LRESULT {unsafe{
    let id=GetDlgCtrlID(child) as usize;
    let bg=if id==ACTIVITY{0x000e0a07}else if matches!(id,CLIENTS|ADDRESS|ENGINE_COUNT|SESSIONS|DATA_PATH|IMPORT_STATUS|UPDATE_STATUS|START_AT_SIGN_IN){PANEL}else{BG};
    SetTextColor(dc,if id==READY{0x00b0e07f}else if id==ADDRESS{ACCENT}else{TEXT});SetBkColor(dc,bg);
    brush(bg) as LRESULT
}}
pub unsafe fn draw_button(item:&DRAWITEMSTRUCT){unsafe{
    let id=item.CtlID as usize;let sidebar=matches!(id,OVERVIEW|ENGINES|ARCHIVE|IMPORTS|REPORTS|SHOW|CHECK_UPDATE|INSTALL_UPDATE);
    let selected=item.itemState&ODS_SELECTED!=0;let disabled=item.itemState&ODS_DISABLED!=0;
    let bg=if id==OPEN{ACCENT}else if id==OVERVIEW||selected{0x0030221a}else if sidebar{SIDE}else{BG};
    FillRect(item.hDC,&item.rcItem,brush(if sidebar{SIDE}else{BG}));
    if sidebar{FillRect(item.hDC,&item.rcItem,brush(bg));}else{panel(item.hDC,item.rcItem,bg,if id==OPEN{ACCENT}else{0x0058463a});}
    if id==OVERVIEW{FillRect(item.hDC,&RECT{left:0,top:0,right:3,bottom:item.rcItem.bottom},brush(ACCENT));}
    let mut label=[0u16;128];GetWindowTextW(item.hwndItem,label.as_mut_ptr(),128);
    let mut r=item.rcItem;if sidebar{r.left+=14;}
    SelectObject(item.hDC,font(if id==OPEN{16}else{14},id==OPEN||id==OVERVIEW,false));
    SetBkMode(item.hDC,TRANSPARENT as i32);SetTextColor(item.hDC,if disabled{MUTED}else if id==OPEN{BG}else{TEXT});
    DrawTextW(item.hDC,label.as_ptr(),-1,&mut r,DT_SINGLELINE|DT_VCENTER|if sidebar{DT_LEFT}else{DT_CENTER});
    if matches!(id,ENGINES|ARCHIVE|IMPORTS|REPORTS){text(item.hDC,"↗",RECT{left:r.right-26,..r},15,MUTED,true);}
    if item.itemState&ODS_FOCUS!=0&&item.itemState&ODS_NOFOCUSRECT==0{let mut r=item.rcItem;InflateRect(&mut r,-4,-4);DrawFocusRect(item.hDC,&r);}
}}

#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn dashboard_updates_live_values_and_keeps_resized_controls_inside_window(){unsafe{
    let instance=GetModuleHandleW(std::ptr::null());let class=wide("IronwoodConsoleLayoutTest");
    let wc=WNDCLASSW{lpfnWndProc:Some(window_proc),hInstance:instance,lpszClassName:class.as_ptr(),..std::mem::zeroed()};
    assert_ne!(RegisterClassW(&wc),0);
    let window=CreateWindowExW(0,class.as_ptr(),wide("Console layout test").as_ptr(),WS_OVERLAPPEDWINDOW,0,0,1040,820,null_mut(),null_mut(),instance,std::ptr::null());
    assert!(!window.is_null());let (edit,rail,_)=create(window);
    assert!(is_console(window));
    assert_eq!(GetWindowLongPtrW(GetDlgItem(window,START_AT_SIGN_IN as i32),GWL_STYLE) as u32&0xf,BS_AUTOCHECKBOX as u32);
    let activity=update(window,"Ready\r\nVersion 0.1.3\r\nAddress: 127.0.0.1:8790\r\nConnected clients: 2\r\nEngine sessions: 1\r\nRegistered engines: 6\r\n\r\nData: C:\\Test\\Companion\r\nAutomatic imports: Importing games\r\nRead 128 · Saved 128\r\n\r\nNo update channel");
    assert!(activity.contains("Read 128"));assert!(activity.contains("No update channel"));
    let mut text_buffer=[0u16;100];GetWindowTextW(GetDlgItem(window,CLIENTS as i32),text_buffer.as_mut_ptr(),100);assert_eq!(text_buffer[0],b'2' as u16);
    for (width,height) in [(940,760),(1200,900)]{
      MoveWindow(window,0,0,width,height,0);layout(window);let mut client=RECT::default();GetClientRect(window,&mut client);
      for id in [OPEN,CLOSE_STATUS,COPY_ADDRESS,DATA_PATH,UPDATE_STATUS,START_AT_SIGN_IN,ACTIVITY]{
        let mut rect=RECT::default();GetWindowRect(GetDlgItem(window,id as i32),&mut rect);let mut point=POINT{x:rect.right,y:rect.bottom};ScreenToClient(window,&mut point);
        assert!(point.x<=client.right&&point.y<=client.bottom,"Control {id} outside {width}x{height}");
      }
    }
    SetWindowTextW(edit,wide(&activity).as_ptr());layout_status_scroll(edit,rail);
    DestroyWindow(window);UnregisterClassW(class.as_ptr(),instance);
 }}
}
