//! Paradise Viewer setup: builds the map viewer's files from the player's own
//! copy of Burnout Paradise Remastered and installs the viewer app. Burnout
//! is all it needs (no GTA V); nothing from the game ships with it.
//!
//! The install folder is laid out like the project folder, which is what the
//! viewer app looks for next to itself:
//!   paradise-viewer.exe
//!   out\unitN, out\textures        full-detail models (bpx export)
//!   out\overview                   low-detail models (bpx overview)
//!   out\collision                  Burnout's collision (bpx collision)
//!   out\collision_extra            plus the project's added pieces
//!   out\traffic\traffic.json       lanes and parked cars (bpx traffic-json)
//!   out\stamps, setup.log          the setup's own
//! Steps whose inputs haven't changed since the last run are skipped.
//!
//! No map edits are installed: the viewer shows the GTA mod's own when the
//! mod is on the PC, and stock Burnout otherwise.
//!
//! Release layout, next to the setup exe:
//!   tools\bpx.exe                  the converter
//!   app\paradise-viewer.exe        the viewer
//!   data\collision\*.pcx           the project's added collision
//! Two front ends share this: `paradise-viewer-setup` (the window) and
//! `paradise-viewer-setup-cli` (the same in a console, for testing).

use anyhow::{bail, Context, Result};
use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};

pub mod find;

/// A command that doesn't open a console window of its own.
fn command(exe: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut c = std::process::Command::new(exe);
    c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    c
}

/// Bump to make every player's next run rebuild everything.
pub const SETUP_VERSION: &str = "1";
/// The viewer app's file name in the install folder.
pub const APP_EXE: &str = "paradise-viewer.exe";
/// Start menu and desktop shortcuts.
pub const SHORTCUT: &str = "Paradise Viewer.lnk";

/// Where the setup's own files are.
pub struct Layout {
    pub bpx: PathBuf,
    pub app: PathBuf,
    /// collision\.
    pub data: PathBuf,
}

impl Layout {
    /// `dev`: from the project folder (the app built here, the converter
    /// and added collision from the gta-paradise project next to it);
    /// otherwise from the release folder.
    pub fn new(dev: bool) -> Result<Layout> {
        let exe_dir = std::env::current_exe()?.parent().context("no exe folder")?.to_path_buf();
        if dev {
            let root = exe_dir
                .ancestors()
                .find(|d| d.join("Cargo.toml").is_file() && d.join("viewer").is_dir())
                .context("--dev: project folder not found")?;
            let gta = root.parent().context("no folder above the project")?.join("gta-paradise");
            return Ok(Layout { bpx: gta.join("target/release/bpx.exe"), app: root.join("target/release").join(APP_EXE), data: gta.join("tools") });
        }
        Ok(Layout { bpx: exe_dir.join("tools/bpx.exe"), app: exe_dir.join("app").join(APP_EXE), data: exe_dir.join("data") })
    }

    /// Is the setup folder complete?
    pub fn check(&self) -> Result<()> {
        for (what, p) in [("converter", &self.bpx), ("viewer", &self.app), ("added collision", &self.extra_collision())] {
            if !p.is_file() {
                bail!("the {what} is missing ({}); is the setup folder complete?", p.display());
            }
        }
        Ok(())
    }

    fn extra_collision(&self) -> PathBuf {
        self.data.join("collision/paradise_extra.pcx")
    }
}

/// What the player picked.
pub struct Options {
    pub burnout: PathBuf,
    /// The install folder.
    pub dir: PathBuf,
    /// Where the shortcuts go: None for the Start menu (and the desktop if
    /// `desktop`); Some(folder) puts both there instead (testing).
    pub shortcuts: Option<PathBuf>,
    pub desktop: bool,
}

/// %LOCALAPPDATA%\Programs\Paradise Viewer.
pub fn default_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default().join("Programs").join("Paradise Viewer")
}

/// Is the viewer installed in `dir`?
pub fn installed(dir: &Path) -> bool {
    dir.join(APP_EXE).is_file()
}

/// Is the viewer open (its exe can't be replaced then)?
pub fn viewer_running() -> bool {
    command("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {APP_EXE}")])
        .output()
        .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains(APP_EXE))
}

/// Is Microsoft Edge WebView2 (which the viewer runs in) installed?
pub fn webview2_installed() -> bool {
    const KEY: &str = r"Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    [format!(r"HKLM\SOFTWARE\WOW6432Node\{KEY}"), format!(r"HKLM\SOFTWARE\{KEY}"), format!(r"HKCU\Software\{KEY}")].iter().any(|k| {
        command("reg").args(["query", k, "/v", "pv"]).output().is_ok_and(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).contains(" 0.0.0.0"))
    })
}

pub const WEBVIEW2_URL: &str = "https://developer.microsoft.com/microsoft-edge/webview2/";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Running,
    /// Done, in this many seconds.
    Done(u64),
    UpToDate,
    Failed,
}

/// Progress: (step index, state).
pub type Report<'a> = &'a dyn Fn(usize, State);

struct Step {
    name: &'static str,
    title: &'static str,
    /// Files and folders it makes (cleared first; all there = up to date).
    outputs: &'static [&'static str],
    run: fn(&Ctx) -> Result<()>,
}

const STEPS: &[Step] = &[
    Step { name: "models", title: "City models and textures (the long one)", outputs: &["out/textures"], run: models },
    Step { name: "overview", title: "Low-detail overview", outputs: &["out/overview"], run: overview },
    Step { name: "collision", title: "Collision", outputs: &["out/collision"], run: collision },
    Step { name: "extra", title: "Collision with the added pieces", outputs: &["out/collision_extra"], run: extra_collision },
    Step { name: "traffic", title: "Traffic lanes and parked cars", outputs: &["out/traffic/traffic.json"], run: traffic },
    Step { name: "app", title: "Installing the viewer", outputs: &[], run: app },
];

pub fn titles() -> Vec<&'static str> {
    STEPS.iter().map(|s| s.title).collect()
}

struct Ctx<'a> {
    layout: &'a Layout,
    opts: &'a Options,
    log: RefCell<std::fs::File>,
}

impl Ctx<'_> {
    fn path(&self, rel: &str) -> PathBuf {
        self.opts.dir.join(rel)
    }

    fn burnout(&self) -> String {
        self.opts.burnout.to_string_lossy().into_owned()
    }

    fn note(&self, line: &str) {
        let _ = writeln!(self.log.borrow_mut(), "{line}");
    }

    /// Runs bpx in the install folder, its output going to the log.
    fn bpx(&self, args: &[&str]) -> Result<()> {
        self.note(&format!("> bpx {}", args.join(" ")));
        let log = self.log.borrow_mut().try_clone()?;
        let status = command(&self.layout.bpx)
            .args(args)
            .current_dir(&self.opts.dir)
            .env("BPX_STOCK_COL", self.opts.burnout.join("WORLDCOL.BIN"))
            .stdout(log.try_clone()?)
            .stderr(log)
            .status()
            .with_context(|| format!("starting {}", self.layout.bpx.display()))?;
        if !status.success() {
            bail!("bpx {} failed", args.first().unwrap_or(&""));
        }
        Ok(())
    }

    /// Deletes install-folder paths (files or folders), if there.
    fn clear(&self, rels: &[&str]) -> Result<()> {
        for rel in rels {
            let p = self.path(rel);
            if p.is_dir() {
                std::fs::remove_dir_all(&p).with_context(|| format!("clearing {}", p.display()))?;
            } else if p.is_file() {
                std::fs::remove_file(&p)?;
            }
        }
        Ok(())
    }
}

pub fn log_path(dir: &Path) -> PathBuf {
    dir.join("setup.log")
}

pub fn log_tail(dir: &Path, n: usize) -> Vec<String> {
    let text = std::fs::read_to_string(log_path(dir)).unwrap_or_default();
    let mut tail: Vec<String> = text.lines().rev().take(n).map(str::to_string).collect();
    tail.reverse();
    tail
}

/// FNV-1a, 64-bit (stable from build to build, unlike std's hasher).
struct Fnv(u64);

impl Fnv {
    fn add(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ b as u64).wrapping_mul(0x0100_0000_01b3);
        }
        self.0 = (self.0 ^ 0xff).wrapping_mul(0x0100_0000_01b3);
    }
}

/// What a step's output depends on: the setup and converter versions, the
/// Burnout folder and the setup's own data.
fn fingerprint(c: &Ctx, step: &Step) -> Result<String> {
    let mut h = Fnv(0xcbf2_9ce4_8422_2325);
    h.add(SETUP_VERSION.as_bytes());
    h.add(step.name.as_bytes());
    h.add(&std::fs::read(&c.layout.bpx)?);
    h.add(c.burnout().as_bytes());
    if step.name == "extra" {
        h.add(&std::fs::read(c.layout.extra_collision())?);
    }
    Ok(format!("{:016x}", h.0))
}

fn stamp_path(c: &Ctx, step: &Step) -> PathBuf {
    c.path("out/stamps").join(step.name)
}

/// Builds the viewer's files and installs it.
pub fn run(layout: &Layout, opts: &Options, report: Report) -> Result<()> {
    layout.check()?;
    find::check_burnout(&opts.burnout)?;
    std::fs::create_dir_all(opts.dir.join("out/stamps")).with_context(|| format!("creating {}", opts.dir.display()))?;
    let log = std::fs::File::create(log_path(&opts.dir))?;
    let c = Ctx { layout, opts, log: RefCell::new(log) };
    c.note(&format!("Paradise Viewer setup {} · Burnout: {} · install: {}", env!("CARGO_PKG_VERSION"), c.burnout(), opts.dir.display()));
    for (i, step) in STEPS.iter().enumerate() {
        // The app step (copying and shortcuts) is quick and always runs.
        let print = if step.outputs.is_empty() { None } else { Some(fingerprint(&c, step)?) };
        let done = print.as_ref().is_some_and(|p| {
            std::fs::read_to_string(stamp_path(&c, step)).ok().as_deref() == Some(p.as_str()) && step.outputs.iter().all(|o| c.path(o).exists())
        });
        if done {
            report(i, State::UpToDate);
            continue;
        }
        report(i, State::Running);
        c.note(&format!("== {}", step.title));
        let _ = std::fs::remove_file(stamp_path(&c, step));
        let start = std::time::Instant::now();
        if let Err(e) = (step.run)(&c) {
            c.note(&format!("failed: {e:#}"));
            report(i, State::Failed);
            return Err(e.context(step.title));
        }
        if let Some(p) = print {
            std::fs::write(stamp_path(&c, step), p)?;
        }
        report(i, State::Done(start.elapsed().as_secs()));
    }
    c.note("done");
    Ok(())
}

// --- Steps ------------------------------------------------------------------

/// Copies of the project's edits the first test build put in tools\edits;
/// removed now (the viewer reads the GTA mod's own).
const EDITS: [&str; 5] = ["collision_edits.json", "sidewalk_edits.json", "spawn_blocks.json", "traffic_edits.json", "water_edits.json"];

/// out\unitN (full-detail models).
fn is_unit_dir(name: &str) -> bool {
    name.strip_prefix("unit").is_some_and(|n| n.parse::<u32>().is_ok())
}

fn models(c: &Ctx) -> Result<()> {
    // out\unitN folders and the textures they share.
    if let Ok(dir) = std::fs::read_dir(c.path("out")) {
        for e in dir.flatten() {
            if is_unit_dir(&e.file_name().to_string_lossy()) {
                std::fs::remove_dir_all(e.path())?;
            }
        }
    }
    c.clear(&["out/textures"])?;
    c.bpx(&["export", &c.burnout(), "out", "all"])
}

fn overview(c: &Ctx) -> Result<()> {
    c.clear(&["out/overview"])?;
    c.bpx(&["overview", &c.burnout(), "out/overview", "all"])
}

fn collision(c: &Ctx) -> Result<()> {
    c.clear(&["out/collision"])?;
    c.bpx(&["collision", &c.burnout(), "out/collision", "all"])
}

fn extra_collision(c: &Ctx) -> Result<()> {
    c.clear(&["out/collision_extra"])?;
    let pcx = c.layout.extra_collision().to_string_lossy().into_owned();
    let stock = c.opts.burnout.join("WORLDCOL.BIN").to_string_lossy().into_owned();
    c.bpx(&["collision", &pcx, "out/collision_extra", "all", "--base", &stock])
}

fn traffic(c: &Ctx) -> Result<()> {
    c.clear(&["out/traffic"])?;
    c.bpx(&["traffic-json", &c.burnout(), "out/traffic/traffic.json"])
}

fn app(c: &Ctx) -> Result<()> {
    let exe = c.path(APP_EXE);
    std::fs::copy(&c.layout.app, &exe).with_context(|| format!("copying the viewer to {} (is it open? close it first)", exe.display()))?;
    for e in EDITS {
        let _ = std::fs::remove_file(c.path("tools/edits").join(e));
    }
    for rel in ["tools/edits", "tools"] {
        let _ = std::fs::remove_dir(c.path(rel));
    }
    for lnk in shortcut_paths(c.opts) {
        c.note(&format!("shortcut {}", lnk.display()));
        shortcut(&lnk, &exe)?;
    }
    Ok(())
}

// --- Shortcuts and removing -------------------------------------------------

/// A Windows special folder (Programs, Desktop), as PowerShell reports it.
fn special_folder(name: &str) -> Option<PathBuf> {
    let out = command("powershell").args(["-NoProfile", "-Command", &format!("[Environment]::GetFolderPath('{name}')")]).output().ok()?;
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!p.is_empty()).then(|| PathBuf::from(p))
}

fn shortcut_paths(opts: &Options) -> Vec<PathBuf> {
    match &opts.shortcuts {
        Some(dir) => vec![dir.join(SHORTCUT)],
        None => {
            let desktop = if opts.desktop { special_folder("Desktop") } else { None };
            special_folder("Programs").into_iter().chain(desktop).map(|d| d.join(SHORTCUT)).collect()
        }
    }
}

fn shortcut(lnk: &Path, target: &Path) -> Result<()> {
    let script = "$s = (New-Object -ComObject WScript.Shell).CreateShortcut($env:PV_LNK); $s.TargetPath = $env:PV_TARGET; \
                  $s.WorkingDirectory = $env:PV_DIR; $s.Description = 'Burnout Paradise map viewer'; $s.Save()";
    let status = command("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .env("PV_LNK", lnk)
        .env("PV_TARGET", target)
        .env("PV_DIR", target.parent().unwrap_or(Path::new(".")))
        .status()?;
    if !status.success() || !lnk.is_file() {
        bail!("couldn't make the shortcut {}", lnk.display());
    }
    Ok(())
}

/// Removes the viewer from `dir`: only what the setup put there (the folder
/// itself too if that leaves it empty), and its shortcuts.
pub fn remove(dir: &Path, shortcuts: Option<&Path>) -> Result<()> {
    if !installed(dir) {
        bail!("the viewer isn't installed in {}", dir.display());
    }
    let mut rels: Vec<String> = [APP_EXE, "setup.log", "out/textures", "out/overview", "out/collision", "out/collision_extra", "out/traffic", "out/stamps"]
        .map(String::from)
        .into();
    rels.extend(EDITS.iter().map(|e| format!("tools/edits/{e}")));
    if let Ok(out) = std::fs::read_dir(dir.join("out")) {
        rels.extend(out.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| is_unit_dir(n)).map(|n| format!("out/{n}")));
    }
    for rel in rels {
        let p = dir.join(rel);
        if p.is_dir() {
            std::fs::remove_dir_all(&p).with_context(|| format!("removing {}", p.display()))?;
        } else if p.is_file() {
            std::fs::remove_file(&p).with_context(|| format!("removing {} (is the viewer open?)", p.display()))?;
        }
    }
    // The folders too, if that left them empty.
    for rel in ["out", "tools/edits", "tools", ""] {
        let _ = std::fs::remove_dir(dir.join(rel));
    }
    let opts = Options { burnout: PathBuf::new(), dir: dir.to_path_buf(), shortcuts: shortcuts.map(Path::to_path_buf), desktop: true };
    for lnk in shortcut_paths(&opts) {
        let _ = std::fs::remove_file(lnk);
    }
    Ok(())
}
