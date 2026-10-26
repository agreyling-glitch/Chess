//! Optional Windows desktop shell. No HTTP endpoint exposes the owner token.
use ironwood_server::{Config, Control, config::Error};
use std::{
    cell::RefCell,
    ffi::c_void,
    net::SocketAddr,
    path::{Path, PathBuf},
    ptr::null_mut,
    thread,
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::{Dwm::DwmSetWindowAttribute, Gdi::*},
    System::{DataExchange::*, LibraryLoader::GetModuleHandleW, Memory::*},
    UI::{Controls::*, Input::KeyboardAndMouse::{EnableWindow,GetCapture,SetCapture,ReleaseCapture,SetFocus}, Shell::*, WindowsAndMessaging::*},
};

#[path = "companion_console.rs"]
mod console;
const COPY_ADDRESS: usize = 27;
const CALLBACK: u32 = WM_APP + 1;
const STATUS: usize = 10;
const OPEN: usize = 11;
const ENGINES: usize = 12;
const ARCHIVE: usize = 13;
const DATA: usize = 14;
const COPY: usize = 15;
const SHOW: usize = 16;
const QUIT: usize = 17;
const REVEAL: usize = 20;
const CLOSE_TOKEN: usize = 21;
const CLOSE_STATUS: usize = 22;
const CHECK_UPDATE: usize = 23;
const INSTALL_UPDATE: usize = 24;
const IMPORTS: usize = 25;
const REPORTS: usize = 26;
const MASK: &str = "••••••••••••••••••••••••••••••••";

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
pub fn notice(message: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(message).as_ptr(),
            wide("Ironwood Companion").as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
fn friendly_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
    }
}
fn ironwood_url(origins: &[String]) -> String {
    let origin = origins
        .iter()
        .find(|s| s.starts_with("http://127.0.0.1:"))
        .or_else(|| origins.iter().find(|s| s.starts_with("http://localhost:")))
        .or_else(|| origins.iter().find(|s| s.starts_with("https://")))
        .map(String::as_str)
        .unwrap_or("https://ironwoodchess.com");
    format!("{}/play/", origin.trim_end_matches('/'))
}

pub struct TrayHandle {
    window: usize,
    thread: Option<thread::JoinHandle<()>>,
}
impl Drop for TrayHandle {
    fn drop(&mut self) {
        unsafe {
            PostMessageW(self.window as HWND, WM_CLOSE, 0, 0);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
struct State {
    root: HWND,
    token_window: HWND,
    token_edit: HWND,
    reveal_button: HWND,
    status_window: HWND,
    status_edit: HWND,
    status_scroll: HWND,
    last_status_text: String,
    revealed: bool,
    icon: HICON,
    last_status: String,
    taskbar_created: u32,
    config: Config,
    directory: PathBuf,
    configuration_path: PathBuf,
    address: SocketAddr,
    control: Control,
    update_message: String,
    update_button: HWND,
    last_update_state: String,
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

pub fn start(
    config: Config,
    directory: PathBuf,
    configuration_path: PathBuf,
    address: SocketAddr,
    control: Control,
    show_status: bool,
) -> Result<TrayHandle, Error> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let worker = thread::Builder::new().name("companion-tray".into()).spawn(move || {
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let class = wide("IronwoodCompanionTray");
            let wc = WNDCLASSW { lpfnWndProc: Some(window_proc), hInstance: instance,
                lpszClassName: class.as_ptr(), hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                hbrBackground: background(), ..std::mem::zeroed() };
            if RegisterClassW(&wc) == 0 {
                let _ = tx.send(Err("Could not register the Windows tray window")); return;
            }
            let root = CreateWindowExW(0, class.as_ptr(), wide("Ironwood Companion").as_ptr(), 0,
                0, 0, 0, 0, null_mut(), null_mut(), instance, std::ptr::null());
            if root.is_null() { let _ = tx.send(Err("Could not create the Windows tray window")); return; }
            STATE.with(|slot| *slot.borrow_mut() = Some(State { root,token_window:null_mut(),token_edit:null_mut(),reveal_button:null_mut(),
                status_window:null_mut(),status_edit:null_mut(),status_scroll:null_mut(),last_status_text:String::new(),revealed:false,icon:null_mut(),last_status:String::new(),taskbar_created:RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
                config,directory,configuration_path,address,control,
                update_message:String::new(),update_button:null_mut(),last_update_state:String::new() }));
            let added = STATE.with(|slot| slot.borrow_mut().as_mut().unwrap().update_icon(true));
            if !added {
                DestroyWindow(root); STATE.with(|slot| *slot.borrow_mut()=None);
                let _ = tx.send(Err("The Windows notification area is unavailable. Use ironwood-server --no-tray for headless operation.")); return;
            }
            SetTimer(root, 1, 1000, None);
            if show_status { PostMessageW(root, WM_COMMAND, STATUS, 0); }
            let _ = tx.send(Ok(root as usize));
            let mut message: MSG = std::mem::zeroed();
            while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
                let dialogs=STATE.with(|slot|slot.borrow().as_ref().map(|s|[s.token_window,s.status_window]).unwrap_or([null_mut();2]));
                if dialogs.into_iter().any(|dialog|!dialog.is_null()&&IsDialogMessageW(dialog,&message)!=0) { continue; }
                TranslateMessage(&message); DispatchMessageW(&message);
            }
            STATE.with(|slot| *slot.borrow_mut()=None);
            UnregisterClassW(class.as_ptr(), instance);
        }
    })?;
    match rx.recv() {
        Ok(Ok(window)) => Ok(TrayHandle {
            window,
            thread: Some(worker),
        }),
        Ok(Err(message)) => {
            let _ = worker.join();
            Err(message.into())
        }
        Err(_) => {
            let _ = worker.join();
            Err("Windows tray initialization failed".into())
        }
    }
}

impl State {
    fn status(&self) -> String {
        if self.control.stopping() {
            "Stopping".into()
        } else if self.control.active_engines() > 0 {
            "Engine active".into()
        } else if let Some(label)=self.control.imports.activity_label() {
            label.into()
        } else {
            "Ready".into()
        }
    }
    fn notification(&self) -> NOTIFYICONDATAW {
        let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = self.root;
        data.uID = 1;
        data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        data.uCallbackMessage = CALLBACK;
        data.hIcon = self.icon;
        let tip = wide(&format!("Ironwood Companion · {}", self.status()));
        let length = (tip.len() - 1).min(data.szTip.len() - 1);
        data.szTip[..length].copy_from_slice(&tip[..length]);
        data
    }
    fn update_icon(&mut self, add: bool) -> bool {
        let status = self.status();
        if !add && self.last_status == status {
            return true;
        }
        let old = self.icon;
        self.icon = icon();
        let success = unsafe {
            Shell_NotifyIconW(if add { NIM_ADD } else { NIM_MODIFY }, &self.notification()) != 0
        };
        if !old.is_null() {
            unsafe {
                DestroyIcon(old);
            }
        }
        self.last_status = status;
        success
    }
    fn open(&mut self, destination: &str) {
        let result = unsafe {
            ShellExecuteW(
                self.root,
                wide("open").as_ptr(),
                wide(destination).as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if result as isize <= 32 {
            self.show_status("Windows could not open the selected location.");
        }
    }
    fn menu(&mut self) {
        unsafe {
            let menu = CreatePopupMenu();
            if menu.is_null() {
                return;
            }
            for (id, label) in [
                (STATUS, format!("Status: {}", self.status())),
                (OPEN, "Open Ironwood".into()),
                (ENGINES, "Open engines folder".into()),
                (ARCHIVE, "Open archive folder".into()),
                (IMPORTS, "Open import folder".into()),
                (REPORTS, "Open import reports".into()),
                (DATA, "Open Companion data folder".into()),
                (COPY, "Copy connection token".into()),
                (SHOW, "Show connection token…".into()),
                (CHECK_UPDATE, "Check for updates…".into()),
                (INSTALL_UPDATE, "Update and restart".into()),
                (QUIT, "Quit Companion".into()),
            ] {
                if matches!(id, ENGINES | COPY | QUIT) {
                    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                }
                let disabled = (id == INSTALL_UPDATE && !self.can_update())
                    || (id == CHECK_UPDATE
                        && (!self.control.updates.status(&self.control).can_check));
                AppendMenuW(
                    menu,
                    MF_OWNERDRAW | if disabled { MF_GRAYED } else { 0 },
                    id,
                    wide(&label).as_ptr(),
                );
            }
            let info = MENUINFO {
                cbSize: std::mem::size_of::<MENUINFO>() as u32,
                fMask: MIM_BACKGROUND,
                hbrBack: background(),
                ..std::mem::zeroed()
            };
            SetMenuInfo(menu, &info);
            let mut point: POINT = std::mem::zeroed();
            GetCursorPos(&mut point);
            SetForegroundWindow(self.root);
            let command = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                self.root,
                std::ptr::null(),
            );
            DestroyMenu(menu);
            PostMessageW(self.root, WM_NULL, 0, 0);
            self.action(command as usize);
        }
    }
    fn action(&mut self, command: usize) {
        match command {
            CHECK_UPDATE => {
                if let Err(message) = self.control.updates.check() {
                    self.update_message = message.into();
                }
                self.action(STATUS);
            }
            INSTALL_UPDATE => {
                if let Err(message) = self.control.updates.install(&self.control) {
                    self.update_message = message.into();
                }
                self.action(STATUS);
            }
            COPY_ADDRESS => { if !copy_token(self.status_window,&format!("http://{}",self.address)) {self.show_status("Could not copy address. Try again when the clipboard is available.");} },
            STATUS => self.refresh_status(true),
            OPEN => self.open(&ironwood_url(&self.config.allowed_origins)),
            ENGINES => self.open(&friendly_path(
                &std::env::var_os("IRONWOOD_ENGINE_DIRECTORY")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| self.directory.join("engines")),
            )),
            ARCHIVE => self.open(&friendly_path(
                &self
                    .config
                    .archive
                    .as_ref()
                    .and_then(|p| p.parent())
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| self.directory.join("archive")),
            )),
            DATA => self.open(&friendly_path(&self.directory)),
            IMPORTS => self.open(&friendly_path(&self.directory.join("Import"))),
            REPORTS => self.open(&friendly_path(&self.directory.join("Reports"))),
            COPY => {
                if !copy_token(self.root, self.config.owner_credential()) {
                    self.show_status(
                        "Could not copy the token. Try again when the clipboard is available.",
                    );
                }
            }
            SHOW => self.show_token(),
            REVEAL => {
                self.revealed = !self.revealed;
                unsafe {
                    SendMessageW(
                        self.token_edit,
                        EM_SETPASSWORDCHAR,
                        if self.revealed { 0 } else { 0x25cf },
                        0,
                    );
                    SetWindowTextW(
                        self.token_edit,
                        wide(if self.revealed {
                            self.config.owner_credential()
                        } else {
                            MASK
                        })
                        .as_ptr(),
                    );
                    SetWindowTextW(
                        self.reveal_button,
                        wide(if self.revealed { "Hide" } else { "Reveal" }).as_ptr(),
                    );
                    InvalidateRect(self.token_edit, std::ptr::null(), 1);
                }
            }
            CLOSE_TOKEN => unsafe {
                PostMessageW(self.token_window, WM_CLOSE, 0, 0);
            },
            CLOSE_STATUS => unsafe {
                PostMessageW(self.status_window, WM_CLOSE, 0, 0);
            },
            QUIT => self.control.shutdown(),
            _ => {}
        }
    }
    fn can_update(&self) -> bool {
        self.control.updates.status(&self.control).can_install
    }
    fn refresh_status(&mut self,activate:bool) {
        if !activate && (self.status_window.is_null() || unsafe {IsWindowVisible(self.status_window)==0 || IsIconic(self.status_window)!=0}) {return;}
                let count = std::fs::read(&self.configuration_path)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                    .and_then(|v| v["engines"].as_array().map(Vec::len))
                    .unwrap_or(self.config.engines.len());
                let prior = std::fs::read_to_string(self.directory.join("last-update.txt"))
                    .unwrap_or_default();
                self.show_status_text(&format!("{}\r\nVersion {}\r\nAddress: {}\r\nConnected clients: {}\r\nEngine sessions: {}\r\nRegistered engines: {}\r\n\r\nData: {}\r\nAutomatic imports: {}\r\n\r\n{}",
                    self.status(),env!("CARGO_PKG_VERSION"),self.address,self.control.active_connections(),self.control.active_engines(),count,friendly_path(&self.directory),self.control.imports.status_details(),if self.update_message.is_empty() { &prior } else { &self.update_message }),activate);
    }
    fn poll_update(&mut self) {
        let update = self.control.updates.status(&self.control);
        let key = format!("{}:{}", update.state, update.message);
        if key != self.last_update_state {
            self.last_update_state = key;
            self.update_message = update.message;
            if !self.status_window.is_null() {
                self.refresh_status(false);
            }
        }
        unsafe {
            if !self.update_button.is_null() {
                EnableWindow(self.update_button, update.can_install as i32);
            }
        }
    }
    fn show_status(&mut self,text:&str){self.show_status_text(text,true);}
    fn show_status_text(&mut self, text: &str,activate:bool) {
        unsafe {
            if self.status_window.is_null() {
                self.status_window = CreateWindowExW(
                    WS_EX_DLGMODALFRAME | WS_EX_APPWINDOW,
                    wide("IronwoodCompanionTray").as_ptr(),
                    wide("Ironwood Companion status").as_ptr(),
                    WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    1040,
                    820,
                    null_mut(),
                    null_mut(),
                    GetModuleHandleW(std::ptr::null()),
                    std::ptr::null(),
                );
                if self.status_window.is_null() {
                    return;
                }
                self.last_status_text.clear();
                let (edit,rail,update)=console::create(self.status_window);
                self.status_edit=edit;self.status_scroll=rail;self.update_button=update;
                EnableWindow(self.update_button,self.can_update() as i32);
            }
            if self.last_status_text!=text {
                let mut start=0u32;let mut end=0u32;
                SendMessageW(self.status_edit,EM_GETSEL,&mut start as *mut u32 as usize,&mut end as *mut u32 as isize);
                let first=SendMessageW(self.status_edit,EM_GETFIRSTVISIBLELINE,0,0);
                let activity=console::update(self.status_window,text);
                SetWindowTextW(self.status_edit,wide(&activity).as_ptr());
                layout_status_scroll(self.status_edit,self.status_scroll);
                SendMessageW(self.status_edit,EM_SETSEL,start as usize,end as isize);
                scroll_status_to(self.status_edit,self.status_scroll,first as i32);
                InvalidateRect(self.status_scroll,std::ptr::null(),0);
                self.last_status_text=text.into();
            }
            if activate {
                ShowWindow(self.status_window, SW_SHOW);
                ShowWindow(self.status_window, SW_RESTORE);
                SetForegroundWindow(self.status_window);
            }
        }
    }
    fn show_token(&mut self) {
        unsafe {
            if !self.token_window.is_null() {
                ShowWindow(self.token_window, SW_RESTORE);
                SetForegroundWindow(self.token_window);
                return;
            }
            self.revealed = false;
            self.token_window = CreateWindowExW(
                WS_EX_DLGMODALFRAME | WS_EX_APPWINDOW,
                wide("IronwoodCompanionTray").as_ptr(),
                wide("Companion connection token").as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                650,
                280,
                null_mut(),
                null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            if self.token_window.is_null() {
                self.show_status("Could not open the token window.");
                return;
            }
            control(
                self.token_window,
                "STATIC",
                "Use this token to connect Ironwood to Companion.",
                0,
                20,
                18,
                590,
                24,
                0,
            );
            control(
                self.token_window,
                "STATIC",
                "Keep it private. Clipboard tools may retain copied tokens.",
                0,
                20,
                46,
                590,
                24,
                0,
            );
            control(
                self.token_window,
                "STATIC",
                "Connection token",
                0,
                20,
                74,
                590,
                22,
                0,
            );
            self.token_edit = control(
                self.token_window,
                "EDIT",
                MASK,
                WS_BORDER | ES_READONLY as u32 | ES_PASSWORD as u32 | ES_AUTOHSCROLL as u32,
                20,
                100,
                590,
                30,
                0,
            );
            SendMessageW(self.token_edit, EM_SETPASSWORDCHAR, 0x25cf, 0);
            self.reveal_button = control(
                self.token_window,
                "BUTTON",
                "Reveal",
                BS_PUSHBUTTON as u32,
                20,
                138,
                110,
                34,
                REVEAL,
            );
            control(
                self.token_window,
                "BUTTON",
                "Copy token",
                BS_PUSHBUTTON as u32,
                144,
                138,
                120,
                34,
                COPY,
            );
            control(
                self.token_window,
                "BUTTON",
                "Close",
                BS_PUSHBUTTON as u32,
                500,
                138,
                110,
                34,
                CLOSE_TOKEN,
            );
            ShowWindow(self.token_window, SW_SHOW);
            ShowWindow(self.token_window, SW_RESTORE);
            SetForegroundWindow(self.token_window);
        }
    }
}
#[derive(Debug,PartialEq)]
struct ScrollThumb { top:i32,height:i32,max_first:i32,travel:i32 }
fn scroll_thumb(lines:i32,visible:i32,first:i32,height:i32)->Option<ScrollThumb>{
    let visible=visible.max(1);
    if lines<=visible||height<=0{return None;}
    let thumb=((i64::from(height)*i64::from(visible)/i64::from(lines)) as i32).clamp(24.min(height),height);
    let max_first=lines-visible;let travel=height-thumb;
    let top=(i64::from(first.clamp(0,max_first))*i64::from(travel)/i64::from(max_first)) as i32;
    Some(ScrollThumb{top,height:thumb,max_first,travel})
}
unsafe fn status_lines(edit:HWND)->(i32,i32,i32){unsafe{
    let mut rect:RECT=std::mem::zeroed();SendMessageW(edit,EM_GETRECT,0,&mut rect as *mut RECT as isize);
    let dc=GetDC(edit);let font=SendMessageW(edit,WM_GETFONT,0,0) as HGDIOBJ;
    let old=if font.is_null(){null_mut()}else{SelectObject(dc,font)};
    let mut metrics:TEXTMETRICW=std::mem::zeroed();GetTextMetricsW(dc,&mut metrics);
    if !old.is_null(){SelectObject(dc,old);}ReleaseDC(edit,dc);
    (SendMessageW(edit,EM_GETLINECOUNT,0,0) as i32,(rect.bottom-rect.top)/metrics.tmHeight.max(1),SendMessageW(edit,EM_GETFIRSTVISIBLELINE,0,0) as i32)
}}
unsafe fn layout_status_scroll(edit:HWND,rail:HWND){unsafe{
    if edit.is_null()||rail.is_null(){return;}
    // Measure at full width first, so a formerly visible bar cannot itself keep
    // text wrapped enough to prevent it disappearing again.
    let flags=SWP_NOMOVE|SWP_NOZORDER|SWP_NOACTIVATE;
    let mut edit_rect=RECT::default();let mut rail_rect=RECT::default();
    GetWindowRect(edit,&mut edit_rect);GetWindowRect(rail,&mut rail_rect);
    let width=(rail_rect.right-edit_rect.left).max(1);let height=(rail_rect.bottom-rail_rect.top).max(1);
    SetWindowPos(edit,null_mut(),0,0,width,height,flags);
    let (lines,visible,_)=status_lines(edit);
    if lines>visible.max(1){SetWindowPos(edit,null_mut(),0,0,(rail_rect.left-edit_rect.left-6).max(1),height,flags);ShowWindow(rail,SW_SHOWNOACTIVATE);}else{ShowWindow(rail,SW_HIDE);}
    InvalidateRect(rail,std::ptr::null(),0);
}}
unsafe fn scroll_status_to(edit:HWND,rail:HWND,target:i32){unsafe{
    let (lines,visible,first)=status_lines(edit);let target=target.clamp(0,(lines-visible.max(1)).max(0));
    SendMessageW(edit,EM_LINESCROLL,0,(target-first) as isize);InvalidateRect(rail,std::ptr::null(),0);
}}
unsafe fn paint_status_scroll(dc:HDC,rect:&RECT,thumb:Option<&ScrollThumb>){unsafe{
    FillRect(dc,rect,inset());
    if let Some(t)=thumb{
        FrameRect(dc,rect,gold());
        let body=RECT{left:2,top:t.top,right:rect.right-2,bottom:t.top+t.height};FillRect(dc,&body,gold());
        let highlight=CreateSolidBrush(0x00b7dff7);let shadow=CreateSolidBrush(0x002b547a);
        FillRect(dc,&RECT{left:2,top:t.top,right:rect.right-2,bottom:t.top+2},highlight);
        FillRect(dc,&RECT{left:2,top:t.top+t.height-2,right:rect.right-2,bottom:t.top+t.height},shadow);
        DeleteObject(highlight);DeleteObject(shadow);
    }
}}
unsafe extern "system" fn status_edit_proc(window:HWND,message:u32,wparam:WPARAM,lparam:LPARAM,id:usize,data:usize)->LRESULT{unsafe{
    if message==WM_MOUSEWHEEL{
        let delta=((wparam>>16) as u16 as i16) as i32+GetWindowLongPtrW(window,GWLP_USERDATA) as i32;
        let ticks=delta/120;SetWindowLongPtrW(window,GWLP_USERDATA,(delta%120) as isize);
        let (_,visible,first)=status_lines(window);let mut amount=3u32;
        SystemParametersInfoW(SPI_GETWHEELSCROLLLINES,0,&mut amount as *mut u32 as *mut c_void,0);
        let step=if amount==u32::MAX{visible.max(1)}else{amount.min(i32::MAX as u32) as i32};
        scroll_status_to(window,data as HWND,first.saturating_sub(ticks.saturating_mul(step)));return 0;
    }
    let result=DefSubclassProc(window,message,wparam,lparam);
    if matches!(message,WM_MOUSEWHEEL|WM_KEYDOWN|WM_CHAR|WM_VSCROLL|EM_LINESCROLL|WM_SIZE){InvalidateRect(data as HWND,std::ptr::null(),0);}
    if message==WM_NCDESTROY{RemoveWindowSubclass(window,Some(status_edit_proc),id);}
    result
}}
unsafe extern "system" fn status_scroll_proc(window:HWND,message:u32,wparam:WPARAM,lparam:LPARAM,id:usize,data:usize)->LRESULT{unsafe{
    let edit=data as HWND;
    if message==WM_NCDESTROY{RemoveWindowSubclass(window,Some(status_scroll_proc),id);return DefSubclassProc(window,message,wparam,lparam);}
    let mut rect:RECT=std::mem::zeroed();GetClientRect(window,&mut rect);
    let (lines,visible,first)=status_lines(edit);let thumb=scroll_thumb(lines,visible,first,rect.bottom);
    match message {
        WM_ERASEBKGND=>1,
        WM_PAINT=>{
            let mut paint:PAINTSTRUCT=std::mem::zeroed();let dc=BeginPaint(window,&mut paint);
            paint_status_scroll(dc,&rect,thumb.as_ref());
            EndPaint(window,&paint);0
        },
        WM_LBUTTONDOWN=>{
            if let Some(t)=thumb{
                let y=((lparam>>16) as u16 as i16) as i32;
                SetFocus(edit);
                if y>=t.top&&y<t.top+t.height{SetWindowLongPtrW(window,GWLP_USERDATA,(y-t.top+1) as isize);SetCapture(window);}
                else{scroll_status_to(edit,window,first+if y<t.top{-visible.max(1)}else{visible.max(1)});}
            }0
        },
        WM_MOUSEMOVE=>{
            let offset=GetWindowLongPtrW(window,GWLP_USERDATA);
            if offset>0&&GetCapture()==window{if let Some(t)=thumb{
                let y=((lparam>>16) as u16 as i16) as i32;
                let top=(y-(offset as i32-1)).clamp(0,t.travel);
                let target=(i64::from(top)*i64::from(t.max_first)/i64::from(t.travel.max(1))) as i32;
                scroll_status_to(edit,window,target);
            }}0
        },
        WM_LBUTTONUP=>{SetWindowLongPtrW(window,GWLP_USERDATA,0);if GetCapture()==window{ReleaseCapture();}0},
        WM_CAPTURECHANGED=>{SetWindowLongPtrW(window,GWLP_USERDATA,0);0},
        WM_MOUSEWHEEL=>SendMessageW(edit,message,wparam,lparam),
        WM_KEYDOWN=>{SetFocus(edit);SendMessageW(edit,message,wparam,lparam)},
        _=>DefSubclassProc(window,message,wparam,lparam),
    }
}}
// Controls contain only a placeholder until the user explicitly chooses Reveal.
#[allow(clippy::too_many_arguments)]
fn control(
    parent: HWND,
    class: &str,
    text: &str,
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: usize,
) -> HWND {
    unsafe {
        let child = CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
                | if class == "BUTTON" {
                    (style & !0xf) | BS_OWNERDRAW as u32
                } else {
                    style
                },
            x,
            y,
            w,
            h,
            parent,
            id as *mut c_void,
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        );
        SendMessageW(
            child,
            WM_SETFONT,
            GetStockObject(DEFAULT_GUI_FONT) as usize,
            1,
        );
        child
    }
}
fn copy_token(owner: HWND, token: &str) -> bool {
    unsafe {
        let text = wide(token);
        let memory = GlobalAlloc(GMEM_MOVEABLE, text.len() * 2);
        if memory.is_null() {
            return false;
        }
        let target = GlobalLock(memory);
        if target.is_null() {
            GlobalFree(memory);
            return false;
        }
        std::ptr::copy_nonoverlapping(text.as_ptr(), target as *mut u16, text.len());
        GlobalUnlock(memory);
        if OpenClipboard(owner) == 0 {
            GlobalFree(memory);
            return false;
        }
        let success = EmptyClipboard() != 0 && !SetClipboardData(13, memory).is_null(); // CF_UNICODETEXT
        CloseClipboard();
        if !success {
            GlobalFree(memory);
        }
        success
    }
}
// The approved Ironwood brand icon, also used by the browser favicon.
fn icon() -> HICON {
    let bytes = include_bytes!("../assets/ironwood.ico");
    let entry = 6 + 16; // The shared favicon contains 16, 32 and 48 pixel entries.
    let length = u32::from_le_bytes(bytes[entry + 8..entry + 12].try_into().unwrap()) as usize;
    let offset = u32::from_le_bytes(bytes[entry + 12..entry + 16].try_into().unwrap()) as usize;
    unsafe {
        CreateIconFromResourceEx(
            bytes[offset..offset + length].as_ptr(),
            length as u32,
            1,
            0x30000,
            32,
            32,
            0,
        )
    }
}
const DARK: u32 = 0x001b1917; // COLORREF uses BGR.
const INSET: u32 = 0x0014100d;
const GOLD: u32 = 0x0073b9df;
fn background() -> HBRUSH {
    static BRUSH: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *BRUSH.get_or_init(|| unsafe { CreateSolidBrush(DARK) as usize }) as HBRUSH
}
fn inset() -> HBRUSH {
    static BRUSH: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *BRUSH.get_or_init(|| unsafe { CreateSolidBrush(INSET) as usize }) as HBRUSH
}
fn gold() -> HBRUSH {
    static BRUSH: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *BRUSH.get_or_init(|| unsafe { CreateSolidBrush(GOLD) as usize }) as HBRUSH
}
fn draw_item(item: &DRAWITEMSTRUCT) {
    unsafe {
        // Keyboard focus remains after activation; only a pressed button or
        // selected menu item should retain the gold fill.
        let selected = item.itemState & ODS_SELECTED != 0;
        FillRect(
            item.hDC,
            &item.rcItem,
            if selected { gold() } else { inset() },
        );
        let mut rect = item.rcItem;
        if item.CtlType == ODT_BUTTON {
            FrameRect(item.hDC, &rect, gold());
        }
        let mut label = [0u16; 256];
        if item.CtlType == ODT_MENU {
            let text = match item.itemID as usize {
                STATUS => "Companion status…",
                OPEN => "Open Ironwood",
                ENGINES => "Open engines folder",
                ARCHIVE => "Open archive folder",
                IMPORTS => "Open import folder",
                REPORTS => "Open import reports",
                DATA => "Open Companion data folder",
                COPY => "Copy connection token",
                SHOW => "Show connection token…",
                QUIT => "Quit Companion",
                CHECK_UPDATE => "Check for updates…",
                INSTALL_UPDATE => "Update and restart",
                _ => "",
            };
            let text = wide(text);
            label[..text.len()].copy_from_slice(&text);
            rect.left += 16;
        } else {
            GetWindowTextW(item.hwndItem, label.as_mut_ptr(), label.len() as i32);
        }
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(
            item.hDC,
            if item.itemState & ODS_DISABLED != 0 {
                0x00808080
            } else if selected {
                INSET
            } else {
                GOLD
            },
        );
        SelectObject(item.hDC, GetStockObject(DEFAULT_GUI_FONT));
        DrawTextW(
            item.hDC,
            label.as_ptr(),
            -1,
            &mut rect,
            DT_SINGLELINE
                | DT_VCENTER
                | if item.CtlType == ODT_BUTTON {
                    DT_CENTER
                } else {
                    DT_LEFT
                },
        );
        if item.itemState & ODS_FOCUS != 0 && item.itemState & ODS_NOFOCUSRECT == 0 {
            InflateRect(&mut rect, -4, -4);
            DrawFocusRect(item.hDC, &rect);
        }
    }
}
unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_SIZE => unsafe {if console::is_console(window){console::layout(window);RedrawWindow(window,std::ptr::null(),null_mut(),RDW_INVALIDATE|RDW_ERASE|RDW_ALLCHILDREN);return 0;} DefWindowProcW(window,message,wparam,lparam)},
        WM_GETMINMAXINFO => unsafe {if console::is_console(window){let info=&mut *(lparam as *mut MINMAXINFO);info.ptMinTrackSize=POINT{x:940,y:760};return 0;} DefWindowProcW(window,message,wparam,lparam)},
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT => unsafe {
            if console::is_console(window){return console::control_color(wparam as HDC,lparam as HWND); }
            SetTextColor(wparam as HDC, GOLD);
            SetBkColor(wparam as HDC, INSET);
            inset() as LRESULT
        },
        WM_MEASUREITEM => unsafe {
            let item = &mut *(lparam as *mut MEASUREITEMSTRUCT);
            item.itemWidth = 280;
            item.itemHeight = 34;
            1
        },
        WM_DRAWITEM => {
            let item=unsafe{&*(lparam as *const DRAWITEMSTRUCT)};
            if unsafe{console::is_console(window)} {unsafe{console::draw_button(item)};}else{draw_item(item);}
            1
        }
        WM_PAINT => unsafe {
            let mut paint: PAINTSTRUCT = std::mem::zeroed();
            let dc = BeginPaint(window, &mut paint);
            let mut rect: RECT = std::mem::zeroed();
            GetClientRect(window, &mut rect);
            if console::is_console(window){console::paint(window,dc,&rect);}else{
            FillRect(dc, &rect, background());
            FrameRect(dc, &rect, gold());}
            EndPaint(window, &paint);
            0
        },
        WM_CREATE => unsafe {
            static IMAGE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
            let image = *IMAGE.get_or_init(|| icon() as usize) as HICON;
            // Unsupported attributes on older Windows versions are harmless.
            for (attribute, value) in [(20, 1u32), (34, GOLD), (35, DARK), (36, GOLD)] {
                DwmSetWindowAttribute(window, attribute, &value as *const u32 as *const c_void, 4);
            }
            SendMessageW(window, WM_SETICON, ICON_SMALL as usize, image as isize);
            SendMessageW(window, WM_SETICON, ICON_BIG as usize, image as isize);
            0
        },
        WM_QUERYENDSESSION => 1,
        WM_ENDSESSION => {
            if wparam != 0 {
                STATE.with(|slot| {
                    if let Ok(state) = slot.try_borrow()
                        && let Some(state) = state.as_ref()
                    {
                        state.control.shutdown();
                    }
                });
            }
            0
        }
        WM_COMMAND => {
            if wparam & 0xffff == 2 {
                unsafe {
                    PostMessageW(window, WM_CLOSE, 0, 0);
                }
                return 0;
            }
            STATE.with(|slot| {
                if let Ok(mut state) = slot.try_borrow_mut()
                    && let Some(state) = state.as_mut()
                {
                    state.action(wparam & 0xffff);
                }
            });
            0
        }
        CALLBACK => {
            STATE.with(|slot| {
                if let Ok(mut state) = slot.try_borrow_mut()
                    && let Some(state) = state.as_mut()
                {
                    if matches!(lparam as u32, WM_RBUTTONUP | WM_CONTEXTMENU) {
                        state.menu();
                    } else if lparam as u32 == WM_LBUTTONUP {
                        state.action(STATUS);
                    }
                }
            });
            0
        }
        WM_TIMER => {
            STATE.with(|slot| {
                if let Ok(mut state) = slot.try_borrow_mut()
                    && let Some(state) = state.as_mut()
                {
                    state.update_icon(false);
                    state.poll_update();
                    state.refresh_status(false);
                }
            });
            0
        }
        WM_CLOSE => {
            if STATE.with(|slot| slot.try_borrow_mut().is_err()) {
                unsafe {
                    EndMenu();
                    PostMessageW(window, WM_CLOSE, 0, 0);
                }
                return 0;
            }
            let children = STATE.with(|slot| {
                if let Some(state) = slot.borrow_mut().as_mut() {
                    if window == state.token_window {
                        unsafe {
                            SetWindowTextW(state.token_edit, wide("").as_ptr());
                        }
                        state.token_window = null_mut();
                        state.token_edit = null_mut();
                        state.revealed = false;
                    } else if window == state.status_window {
                        state.status_window = null_mut();
                        state.status_edit = null_mut();
                        state.status_scroll = null_mut();
                        state.update_button = null_mut();
                    } else {
                        unsafe {
                            Shell_NotifyIconW(NIM_DELETE, &state.notification());
                            KillTimer(window, 1);
                        }
                        if !state.icon.is_null() {
                            unsafe {
                                DestroyIcon(state.icon);
                            }
                            state.icon = null_mut();
                        }
                        return [state.token_window, state.status_window];
                    }
                }
                [null_mut(); 2]
            });
            unsafe {
                for child in children {
                    if !child.is_null() {
                        DestroyWindow(child);
                    }
                }
                DestroyWindow(window);
            }
            0
        }
        WM_DESTROY => {
            STATE.with(|slot| {
                if slot
                    .borrow()
                    .as_ref()
                    .is_some_and(|state| window == state.root)
                {
                    unsafe {
                        PostQuitMessage(0);
                    }
                }
            });
            0
        }
        _ => {
            let recreated = message >= 0xc000
                && STATE.with(|slot| {
                    slot.try_borrow().ok().is_some_and(|s| {
                        s.as_ref()
                            .is_some_and(|state| message == state.taskbar_created)
                    })
                });
            if recreated {
                STATE.with(|slot| {
                    if let Some(state) = slot.borrow_mut().as_mut() {
                        state.update_icon(true);
                    }
                });
                return 0;
            }
            unsafe { DefWindowProcW(window, message, wparam, lparam) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scroll_thumb_only_exists_for_overflow_and_reaches_both_ends(){
        assert!(scroll_thumb(20,20,0,345).is_none());assert!(scroll_thumb(1,20,0,345).is_none());
        let top=scroll_thumb(100,20,0,345).unwrap();let end=scroll_thumb(100,20,80,345).unwrap();
        assert_eq!(top.top,0);assert_eq!(end.top+end.height,345);
        assert_eq!(top.max_first,80);assert!(top.height>=24);
        assert_eq!(scroll_thumb(i32::MAX,20,i32::MAX,345).unwrap().height,24);
    }
    #[test]
    fn gold_scroll_pixels_match_the_status_theme(){unsafe{
        let screen=GetDC(null_mut());let dc=CreateCompatibleDC(screen);let bitmap=CreateCompatibleBitmap(screen,14,345);let previous=SelectObject(dc,bitmap);
        let thumb=scroll_thumb(100,20,20,345).unwrap();
        paint_status_scroll(dc,&RECT{left:0,top:0,right:14,bottom:345},Some(&thumb));
        let actual=GetPixel(dc,7,thumb.top+thumb.height/2);
        SelectObject(dc,previous);DeleteObject(bitmap);DeleteDC(dc);ReleaseDC(null_mut(),screen);
        assert_eq!(actual,GOLD);
    }}
    #[test]
    fn status_scroll_hides_when_text_fits_and_scrolls_overflow(){unsafe{
        let instance=GetModuleHandleW(std::ptr::null());let class=wide("IronwoodStatusScrollTest");
        let definition=WNDCLASSW{lpfnWndProc:Some(DefWindowProcW),hInstance:instance,lpszClassName:class.as_ptr(),..std::mem::zeroed()};
        assert_ne!(RegisterClassW(&definition),0);
        let parent=CreateWindowExW(0,class.as_ptr(),wide("Status scroll test").as_ptr(),WS_OVERLAPPED,0,0,650,560,null_mut(),null_mut(),instance,std::ptr::null());
        assert!(!parent.is_null());
        let edit=control(parent,"EDIT","",ES_READONLY as u32|ES_MULTILINE as u32|ES_AUTOVSCROLL as u32,20,18,590,345,0);
        let rail=control(parent,"STATIC","Scroll status",0x100,596,18,14,345,0);
        SetWindowSubclass(edit,Some(status_edit_proc),2,rail as usize);
        SetWindowTextW(edit,wide("Everything fits").as_ptr());layout_status_scroll(edit,rail);
        assert_eq!(GetWindowLongPtrW(rail,GWL_STYLE)&WS_VISIBLE as isize,0);
        SetWindowTextW(edit,wide(&"A status line\r\n".repeat(100)).as_ptr());layout_status_scroll(edit,rail);
        assert_ne!(GetWindowLongPtrW(rail,GWL_STYLE)&WS_VISIBLE as isize,0);
        scroll_status_to(edit,rail,i32::MAX);let (_,_,bottom)=status_lines(edit);assert!(bottom>0);
        SendMessageW(edit,WM_MOUSEWHEEL,(120u32<<16) as usize,0);assert!(status_lines(edit).2<bottom);
        SetWindowTextW(edit,wide("Short again").as_ptr());layout_status_scroll(edit,rail);
        assert_eq!(GetWindowLongPtrW(rail,GWL_STYLE)&WS_VISIBLE as isize,0);
        let mut rect:RECT=std::mem::zeroed();GetClientRect(edit,&mut rect);assert_eq!(rect.right,590);
        DestroyWindow(parent);UnregisterClassW(class.as_ptr(),instance);
    }}
    #[test]
    fn desktop_destinations_have_no_credentials() {
        assert_eq!(
            friendly_path(Path::new(r"\\?\C:\Engines\lc0")),
            r"C:\Engines\lc0"
        );
        assert_eq!(
            friendly_path(Path::new(r"\\?\UNC\nas\engines")),
            r"\\nas\engines"
        );
        assert_eq!(
            ironwood_url(&["http://127.0.0.1:8788".into()]),
            "http://127.0.0.1:8788/play/"
        );
        assert_eq!(ironwood_url(&[]), "https://ironwoodchess.com/play/");
    }
}
