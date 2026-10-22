//! The Paradise map viewer / editor as a desktop app: the viewer page
//! (viewer/, built into the exe) in a window of its own, with no web
//! server or browser. The page's requests are answered inside the app: the
//! viewer from the exe, the map data from the project's out\ folder, and,
//! when the GTA mod is on this PC, its saved map edits and the GTA-side map
//! files (water view, F9 marks) from the mod's folder (see `find_mod`).
//!
//! The project folder is the one with out\overview in it: `--root <folder>`,
//! else the first found above the exe or the current folder, else the one
//! picked last time; failing those the app asks for it.
//!
//! The page runs in Microsoft Edge WebView2 (part of Windows 11). Its stored
//! state (edits not yet exported, ...) is kept in
//! %LOCALAPPDATA%\GTA-Paradise\viewer, separate from any browser's.
//!
//! %LOCALAPPDATA%\GTA-Paradise\viewer.log (new each start) says which
//! folders it uses and lists the files a page asked for that weren't found.

#![windows_subsystem = "windows"]

use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use wry::http::{Request, Response};
use wry::{RequestAsyncResponder, WebContext, WebViewBuilder};

include!(concat!(env!("OUT_DIR"), "/viewer_files.rs"));

const TITLE: &str = "Paradise Viewer";
/// The page, on the app's own scheme (WebView2 shows `paradise://` as
/// http://paradise.localhost). It's served at /tools/viewer/, where it sits
/// in the GTA Paradise project, since its links to the map data (../../out/)
/// and edits (../edits/) are relative to that.
const START: &str = "http://paradise.localhost/tools/viewer/index.html";
/// Requests are answered on this many threads, so loading the map never holds
/// up the window.
const WORKERS: usize = 8;

fn main() {
    if let Err(e) = run() {
        rfd::MessageDialog::new().set_level(rfd::MessageLevel::Error).set_title(TITLE).set_description(e).show();
    }
}

fn run() -> Result<(), String> {
    let Some(root) = find_root()? else { return Ok(()) };
    let gta_mod = find_mod(&root);
    log(&format!("map data from {}; GTA mod {}", root.display(), gta_mod.as_ref().map_or("not found (its map edits and files can't be shown)".to_string(), |m| format!("in {}", m.display()))));
    let dirs = Arc::new(Dirs { root, gta_mod });

    let (tx, rx) = mpsc::channel::<(Request<Vec<u8>>, RequestAsyncResponder)>();
    let rx = Arc::new(Mutex::new(rx));
    for _ in 0..WORKERS {
        let (rx, dirs) = (rx.clone(), dirs.clone());
        std::thread::spawn(move || loop {
            let job = rx.lock().unwrap().recv();
            let Ok((request, responder)) = job else { break };
            responder.respond(serve(&dirs, &request));
        });
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(TITLE)
        .with_inner_size(LogicalSize::new(1600.0, 950.0))
        .with_maximized(true)
        .build(&event_loop)
        .map_err(|e| e.to_string())?;
    let mut context = WebContext::new(local_dir().map(|d| d.join("viewer")));
    let webview = WebViewBuilder::new_with_web_context(&mut context)
        .with_asynchronous_custom_protocol("paradise".into(), move |_, request, responder| {
            let _ = tx.send((request, responder));
        })
        .with_url(START)
        .with_background_color((0x1b, 0x1d, 0x21, 0xff))
        .build(&window)
        .map_err(|e| {
            format!(
                "Couldn't open the viewer: {e}\n\nIt needs Microsoft Edge WebView2, which comes with Windows 11. \
                 On Windows 10, install the \"WebView2 Runtime\" from Microsoft."
            )
        })?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        let _ = (&webview, &context);
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *control_flow = ControlFlow::Exit;
        }
    })
}

/// %LOCALAPPDATA%\GTA-Paradise, where the setup keeps its files too.
/// Adds a line to viewer.log (the first call starts it afresh; at most 200
/// "not found" lines).
fn log(line: &str) {
    use std::io::Write as _;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    static STARTED: AtomicBool = AtomicBool::new(false);
    static MISSING: AtomicUsize = AtomicUsize::new(0);
    let Some(path) = local_dir().map(|d| d.join("viewer.log")) else { return };
    if line.starts_with("not found") && MISSING.fetch_add(1, Ordering::Relaxed) >= 200 {
        return;
    }
    let fresh = !STARTED.swap(true, Ordering::SeqCst);
    let file = if fresh { std::fs::File::create(&path) } else { std::fs::OpenOptions::new().append(true).open(&path) };
    if let Ok(mut f) = file {
        let _ = writeln!(f, "{line}");
    }
}

fn local_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|l| PathBuf::from(l).join("GTA-Paradise"))
}

/// Where the project folder picked last time is remembered.
fn remembered_file() -> Option<PathBuf> {
    local_dir().map(|d| d.join("viewer_root.txt"))
}

/// Where the page's files come from.
struct Dirs {
    /// The folder with out\ (the map data).
    root: PathBuf,
    /// The GTA mod's folder; None shows Burnout as it is.
    gta_mod: Option<PathBuf>,
}

/// The folder the GTA mod was built in (with its map edits in tools\edits),
/// when the mod is on this PC: the GTA Paradise project when the viewer
/// shows its map (`--root`, or the viewer run from inside it), else the one
/// the GTA setup recorded (`work` in its setup.json), else its
/// default working folder. Without the mod, the viewer shows stock Burnout
/// and its editors start empty.
fn find_mod(root: &Path) -> Option<PathBuf> {
    let project = root.join("tools/edits").is_dir().then(|| root.to_path_buf());
    let recorded = local_dir()
        .and_then(|d| std::fs::read(d.join("setup.json")).ok())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|c| c["work"].as_str().map(PathBuf::from));
    let default = local_dir().map(|d| d.join("work"));
    project.into_iter().chain(recorded).chain(default).find(|w| w.join("tools").join("edits").is_dir())
}

fn is_project(p: &Path) -> bool {
    p.join("out").join("overview").is_dir()
}

/// The project folder (see the top), or None if the player cancelled.
fn find_root() -> Result<Option<PathBuf>, String> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--root") {
        let p = PathBuf::from(args.get(i + 1).ok_or("--root needs a folder")?);
        return if is_project(&p) { Ok(Some(p)) } else { Err(format!("{} has no out\\overview folder.", p.display())) };
    }
    let above = |p: Option<PathBuf>| p.map(|p| p.ancestors().map(Path::to_path_buf).collect::<Vec<_>>()).unwrap_or_default();
    let remembered = remembered_file().and_then(|f| std::fs::read_to_string(f).ok()).map(|s| PathBuf::from(s.trim()));
    let found = above(std::env::current_exe().ok()).into_iter().chain(above(std::env::current_dir().ok())).chain(remembered).find(|p| is_project(p));
    if found.is_some() {
        return Ok(found);
    }
    loop {
        let Some(p) = rfd::FileDialog::new().set_title("Choose the GTA Paradise folder (the one with out and tools in it)").pick_folder() else {
            return Ok(None);
        };
        if is_project(&p) {
            if let Some(f) = remembered_file() {
                let _ = std::fs::create_dir_all(f.parent().unwrap());
                let _ = std::fs::write(f, p.to_string_lossy().as_bytes());
            }
            return Ok(Some(p));
        }
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title(TITLE)
            .set_description("That folder has no out\\overview folder in it. Pick the gta-paradise folder whose map the viewer should show.")
            .show();
    }
}

/// Answers one request: /tools/viewer/... from the exe; /out/... from the
/// project folder, or from the GTA mod's for what only the mod has (the
/// water view, F9 marks); /tools/edits/... from the GTA mod's (none without
/// it); anything else not found.
fn serve(dirs: &Dirs, request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = percent_decode(request.uri().path());
    let file = |dir: &Path, rel: &str| inside(dir, rel).and_then(|p| std::fs::read(p).ok()).map(Cow::Owned);
    let from_mod = |rel: &str| dirs.gta_mod.as_deref().and_then(|m| file(m, rel));
    let body: Option<Cow<'static, [u8]>> = if let Some(rest) = path.strip_prefix("/tools/viewer/") {
        VIEWER.iter().find(|(name, _)| *name == rest).map(|(_, bytes)| Cow::Borrowed(*bytes))
    } else if path.starts_with("/out/") {
        file(&dirs.root, &path).or_else(|| from_mod(&path))
    } else if path.starts_with("/tools/edits/") {
        from_mod(&path)
    } else {
        None
    };
    if body.is_none() {
        log(&format!("not found: {path}"));
    }
    match body {
        Some(body) => Response::builder().header("Content-Type", content_type(&path)).body(body),
        None => Response::builder().status(404).body(Cow::Borrowed(&b""[..])),
    }
    .unwrap()
}

/// `path` (from the URL) as a file under `root`, refusing anything that
/// would lead outside it.
fn inside(root: &Path, path: &str) -> Option<PathBuf> {
    let rel = Path::new(path.trim_start_matches('/'));
    rel.components().all(|c| matches!(c, Component::Normal(_))).then(|| root.join(rel))
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" | "gltf" => "application/json",
        "csv" => "text/csv; charset=utf-8",
        "png" => "image/png",
        "css" => "text/css",
        _ => "application/octet-stream",
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        match (b[i], b.get(i + 1).and_then(|&c| hex(c)), b.get(i + 2).and_then(|&c| hex(c))) {
            (b'%', Some(h), Some(l)) => {
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            (c, ..) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
