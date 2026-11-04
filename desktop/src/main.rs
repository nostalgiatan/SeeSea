//! SeeSea desktop: a window, a tray icon and a global shortcut around `seesea desk`.
//!
//! The shell starts the `seesea` binary as `seesea desk` on a free loopback port, waits until it answers,
//! and shows the workbench page it serves. Closing the window hides it to the tray, so the services started
//! from the workbench keep running; Quit stops the process and lets it close its listeners first. The page
//! is a remote page as far as Tauri is concerned: no capability is granted to it, so it cannot call into the
//! shell, and links that leave the workbench open in the default browser.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const WINDOW: &str = "main";

/// The running `seesea desk` and where it listens.
#[derive(Default)]
struct Backend {
    child: Mutex<Option<Child>>,
    base: Mutex<Option<String>>,
}

struct Words {
    show: &'static str,
    browser: &'static str,
    quit: &'static str,
    no_binary: &'static str,
    exited: &'static str,
    timeout: &'static str,
    spawn: &'static str,
    title: &'static str,
}

fn words() -> Words {
    let lang = ["LC_ALL", "LC_MESSAGES", "LANG"].iter().find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty())).unwrap_or_default();
    if lang.to_ascii_lowercase().starts_with("zh") {
        Words {
            show: "显示 SeeSea",
            browser: "在浏览器中打开",
            quit: "退出",
            no_binary: "找不到 seesea 程序。把它放在本程序旁边、放进 PATH，或用环境变量 SEESEA_BIN 指定。",
            exited: "seesea 启动后退出了：",
            timeout: "seesea 在 90 秒内没有就绪。",
            spawn: "无法启动 seesea：",
            title: "SeeSea 没能启动",
        }
    } else {
        Words {
            show: "Show SeeSea",
            browser: "Open in browser",
            quit: "Quit",
            no_binary: "Cannot find the seesea program. Put it next to this one, on the PATH, or name it in SEESEA_BIN.",
            exited: "seesea exited right after starting: ",
            timeout: "seesea was not ready within 90 seconds.",
            spawn: "Cannot start seesea: ",
            title: "SeeSea could not start",
        }
    }
}

/// `SEESEA_BIN`, then next to this program (where an installer puts it), then the PATH.
fn find_seesea() -> Option<PathBuf> {
    let name = if cfg!(windows) { "seesea.exe" } else { "seesea" };
    if let Some(p) = std::env::var_os("SEESEA_BIN").map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(p);
    }
    if let Some(p) = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join(name))).filter(|p| p.is_file()) {
        return Some(p);
    }
    std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|d| d.join(name)).find(|p| p.is_file()))
}

fn spawn_desk(bin: &Path, port: u16) -> std::io::Result<Child> {
    let mut c = Command::new(bin);
    c.args(["desk", "--no-open", "--addr", &format!("127.0.0.1:{port}")]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::inherit());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // no console window
    }
    #[cfg(target_os = "linux")]
    {
        // If this program dies without a chance to clean up, `seesea` is told to stop too, and closes its listeners.
        use std::os::unix::process::CommandExt;
        // SAFETY: only an async-signal-safe call runs between fork and exec.
        unsafe {
            c.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
    }
    c.spawn()
}

fn answers(port: u16) -> bool {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(300)) else { return false };
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    if s.write_all(b"GET /health HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").is_err() {
        return false;
    }
    let mut head = String::new();
    let _ = s.take(64).read_to_string(&mut head);
    head.contains(" 200")
}

/// Start `seesea desk`; the error is what the window shows. This runs on the main thread: on Linux the
/// child is told to stop when the thread that started it ends, so a short-lived thread must not start it.
fn launch_backend(app: &AppHandle) -> Result<u16, String> {
    let w = words();
    let bin = find_seesea().ok_or(w.no_binary)?;
    let port = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).and_then(|l| l.local_addr()).map_err(|e| format!("{}{e}", w.spawn))?.port();
    let child = spawn_desk(&bin, port).map_err(|e| format!("{}{}: {e}", w.spawn, bin.display()))?;
    let backend = app.state::<Backend>();
    *backend.child.lock().unwrap() = Some(child);
    Ok(port)
}

/// Wait until the `seesea desk` started on `port` answers.
fn wait_backend(app: &AppHandle, port: u16) -> Result<u16, String> {
    let w = words();
    let backend = app.state::<Backend>();
    let started = Instant::now();
    loop {
        if answers(port) {
            *backend.base.lock().unwrap() = Some(format!("http://127.0.0.1:{port}/"));
            return Ok(port);
        }
        if let Some(status) = backend.child.lock().unwrap().as_mut().and_then(|c| c.try_wait().ok().flatten()) {
            return Err(format!("{}{status}", w.exited));
        }
        if started.elapsed() > Duration::from_secs(90) {
            return Err(w.timeout.into());
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

/// Ask `seesea` to stop (it closes the services it started), then make sure.
fn stop_backend(app: &AppHandle) {
    let Some(mut child) = app.state::<Backend>().child.lock().unwrap().take() else { return };
    #[cfg(unix)]
    {
        let _ = Command::new("kill").args(["-TERM", &child.id().to_string()]).status();
        let until = Instant::now() + Duration::from_secs(6);
        while Instant::now() < until {
            if child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn on_signal(_: libc::c_int) {
    STOP.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// SIGTERM and SIGINT end the program the way Quit does, so `seesea` is stopped too.
#[cfg(unix)]
fn quit_on_signals(app: AppHandle) {
    // SAFETY: the handler only stores into an atomic.
    unsafe {
        libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
    }
    std::thread::spawn(move || {
        while !STOP.load(std::sync::atomic::Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(200));
        }
        app.exit(0);
    });
}

fn open_external(url: &str) {
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(url);
        c
    } else if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    let _ = cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

/// Links that open a new tab in a browser navigate this window instead, where `on_navigation` sends them out.
const KEEP_LINKS_OUT: &str = "document.addEventListener('click', function (e) { var a = e.target.closest && e.target.closest('a[target=_blank]'); if (a && a.href) { e.preventDefault(); window.location.assign(a.href); } }, true);";

fn build_window(app: &AppHandle, port: Option<u16>, error: Option<(String, String)>) {
    if app.get_webview_window(WINDOW).is_some() {
        return;
    }
    let (url, script) = match (port, error) {
        (Some(p), _) => (WebviewUrl::External(format!("http://127.0.0.1:{p}/").parse().expect("a local URL")), KEEP_LINKS_OUT.to_string()),
        (None, e) => {
            let (title, message) = e.unwrap_or_default();
            (WebviewUrl::App("index.html".into()), format!("window.__err = {};", serde_json::json!({"title": title, "message": message})))
        }
    };
    let built = WebviewWindowBuilder::new(app, WINDOW, url)
        .title("SeeSea")
        // The workbench draws its own title bar; the error page has none, so it keeps the system's.
        .decorations(port.is_none())
        .inner_size(1240.0, 820.0)
        .min_inner_size(760.0, 560.0)
        .initialization_script(script)
        .on_navigation(move |url| {
            let ours = match port {
                Some(p) => url.scheme() == "http" && url.host_str() == Some("127.0.0.1") && url.port() == Some(p),
                None => true,
            };
            if !ours && matches!(url.scheme(), "http" | "https") {
                open_external(url.as_str());
            }
            ours
        })
        .build();
    if let Err(e) = built {
        eprintln!("cannot open the window: {e}");
    }
}

fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(WINDOW) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// The shortcut: bring the window up with the search box focused, or put it away again.
fn summon(app: &AppHandle) {
    let Some(w) = app.get_webview_window(WINDOW) else { return };
    if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    show(app);
    let _ = w.eval("if (document.querySelector('#q')) { if (location.hash !== '#/search') location.hash = '#/search'; document.querySelector('#q').focus(); }");
}

// The workbench page draws its own title bar and asks for these; see capabilities/workbench.json.
#[tauri::command]
fn win_minimize(window: tauri::WebviewWindow) {
    let _ = window.minimize();
}

#[tauri::command]
fn win_toggle_maximize(window: tauri::WebviewWindow) {
    let _ = if window.is_maximized().unwrap_or(false) { window.unmaximize() } else { window.maximize() };
}

/// The close button hides to the tray, like closing the window always did.
#[tauri::command]
fn win_hide(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

#[tauri::command]
fn win_drag(window: tauri::WebviewWindow) {
    let _ = window.start_dragging();
}

/// `edge` is one of n, s, e, w, ne, nw, se, sw.
#[tauri::command]
fn win_resize(window: tauri::WebviewWindow, edge: String) {
    use tauri::ResizeDirection::*;
    let dir = match edge.as_str() {
        "n" => North,
        "s" => South,
        "e" => East,
        "w" => West,
        "ne" => NorthEast,
        "nw" => NorthWest,
        "se" => SouthEast,
        "sw" => SouthWest,
        _ => return,
    };
    let _ = window.start_resize_dragging(dir);
}

fn main() {
    let shortcut = Shortcut::new(Some(if cfg!(target_os = "macos") { Modifiers::SUPER | Modifiers::SHIFT } else { Modifiers::CONTROL | Modifiers::SHIFT }), Code::Space);
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show(app)))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        summon(app);
                    }
                })
                .build(),
        )
        .manage(Backend::default())
        .invoke_handler(tauri::generate_handler![win_minimize, win_toggle_maximize, win_hide, win_drag, win_resize])
        .on_window_event(|window, event| {
            // Closing hides to the tray; Quit in the tray menu ends the program.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            let w = words();
            let menu = Menu::with_items(
                app,
                &[
                    &MenuItem::with_id(app, "show", w.show, true, None::<&str>)?,
                    &MenuItem::with_id(app, "browser", w.browser, true, None::<&str>)?,
                    &MenuItem::with_id(app, "quit", w.quit, true, None::<&str>)?,
                ],
            )?;
            let mut tray = TrayIconBuilder::new().tooltip("SeeSea").menu(&menu).show_menu_on_left_click(false);
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "show" => show(app),
                "browser" => {
                    if let Some(base) = app.state::<Backend>().base.lock().unwrap().clone() {
                        open_external(&base);
                    }
                }
                "quit" => app.exit(0),
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                    summon(tray.app_handle());
                }
            })
            .build(app)?;
            if let Err(e) = app.global_shortcut().register(shortcut) {
                eprintln!("the global shortcut is not available here: {e}");
            }
            #[cfg(unix)]
            quit_on_signals(app.handle().clone());
            let handle = app.handle().clone();
            let launched = launch_backend(&handle);
            std::thread::spawn(move || {
                let started = launched.and_then(|port| wait_backend(&handle, port));
                let h = handle.clone();
                let _ = handle.run_on_main_thread(move || match started {
                    Ok(port) => build_window(&h, Some(port), None),
                    Err(message) => build_window(&h, None, Some((words().title.into(), message))),
                });
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building SeeSea");
    app.run(|app, event| match event {
        // The last window is only hidden, but keep the program alive if something closes it anyway.
        RunEvent::ExitRequested { api, code, .. } if code.is_none() => api.prevent_exit(),
        RunEvent::Exit => stop_backend(app),
        _ => {}
    });
}
