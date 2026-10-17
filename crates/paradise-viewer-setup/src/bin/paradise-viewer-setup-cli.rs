//! The Paradise Viewer setup in a console (for testing; players use the
//! window, paradise-viewer-setup).

use anyhow::Context;
use paradise_viewer_setup::{self as setup, find, Layout, Options, State};
use std::path::PathBuf;

const HELP: &str = "usage: paradise-viewer-setup-cli [options]
  --burnout <folder>     Burnout Paradise Remastered (found if not given)
  --to <folder>          install folder (default %LOCALAPPDATA%\\Programs\\Paradise Viewer)
  --no-desktop           no desktop shortcut
  --shortcuts <folder>   put the shortcuts there instead of the Start menu / desktop
  --remove               remove the viewer from the install folder
  --dev                  run from the project folder (tools built locally)";

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let dir = value("--to").unwrap_or_else(setup::default_dir);
    let shortcuts = value("--shortcuts");
    if args.iter().any(|a| a == "--remove") {
        setup::remove(&dir, shortcuts.as_deref())?;
        println!("removed from {}", dir.display());
        return Ok(());
    }
    let layout = Layout::new(args.iter().any(|a| a == "--dev"))?;
    let burnout = match value("--burnout") {
        Some(p) => find::check_burnout(&p)?,
        None => find::detect_burnout(None).context("Burnout Paradise Remastered wasn't found: give its folder with --burnout")?,
    };
    let opts = Options { burnout, dir, shortcuts, desktop: !args.iter().any(|a| a == "--no-desktop") };
    let titles = setup::titles();
    let report = |i: usize, s: State| match s {
        State::Running => println!("{} ...", titles[i]),
        State::Done(secs) => println!("{}: done ({}:{:02})", titles[i], secs / 60, secs % 60),
        State::UpToDate => println!("{}: up to date", titles[i]),
        State::Failed => println!("{}: FAILED (see {})", titles[i], setup::log_path(&opts.dir).display()),
    };
    setup::run(&layout, &opts, &report)?;
    println!("installed in {}", opts.dir.display());
    if !setup::webview2_installed() {
        println!("note: Microsoft Edge WebView2 wasn't found; the viewer needs it: {}", setup::WEBVIEW2_URL);
    }
    Ok(())
}
