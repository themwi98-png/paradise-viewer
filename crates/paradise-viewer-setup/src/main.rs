//! The Paradise Viewer setup window: finds (or lets the player pick) Burnout
//! Paradise Remastered and the install folder, then builds the viewer's map
//! and installs it with a step-by-step progress list. Also removes it.
//! `--dev` runs from the project folder, like the console version.

#![windows_subsystem = "windows"]

use eframe::egui;
use paradise_viewer_setup::{self as setup, find, Layout, Options, State};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const TITLE: &str = "Paradise Viewer Setup";
const GREEN: egui::Color32 = egui::Color32::from_rgb(70, 180, 90);
const RED: egui::Color32 = egui::Color32::from_rgb(220, 80, 70);

/// What the build thread reports.
#[derive(Default)]
struct Progress {
    states: Vec<Option<State>>,
    running_since: Option<Instant>,
    finished: bool,
    error: Option<String>,
    log_tail: Vec<String>,
}

struct App {
    dev: bool,
    layout: Result<Layout, String>,
    burnout: String,
    burnout_status: Result<PathBuf, String>,
    dir: String,
    desktop: bool,
    webview2: bool,
    titles: Vec<&'static str>,
    progress: Option<Arc<Mutex<Progress>>>,
    /// Set after removing.
    removed: Option<Result<(), String>>,
}

impl App {
    fn new(dev: bool) -> Self {
        let layout = Layout::new(dev).and_then(|l| l.check().map(|_| l)).map_err(|e| format!("{e:#}"));
        let burnout = find::detect_burnout(None).map(|p| p.display().to_string()).unwrap_or_default();
        let mut app = App {
            dev,
            layout,
            burnout,
            burnout_status: Err(String::new()),
            dir: setup::default_dir().display().to_string(),
            desktop: true,
            webview2: setup::webview2_installed(),
            titles: setup::titles(),
            progress: None,
            removed: None,
        };
        app.recheck();
        app
    }

    fn recheck(&mut self) {
        let t = self.burnout.trim().trim_matches('"');
        self.burnout_status = if t.is_empty() { Err("not found yet: pick its folder".into()) } else { find::check_burnout(Path::new(t)).map_err(|e| e.to_string()) };
    }

    fn dir(&self) -> PathBuf {
        PathBuf::from(self.dir.trim().trim_matches('"'))
    }

    fn start(&mut self, ctx: &egui::Context) {
        let Ok(burnout) = self.burnout_status.clone() else { return };
        let progress = Arc::new(Mutex::new(Progress { states: vec![None; self.titles.len()], ..Default::default() }));
        self.progress = Some(progress.clone());
        let (dev, ctx) = (self.dev, ctx.clone());
        let opts = Options { burnout, dir: self.dir(), shortcuts: None, desktop: self.desktop };
        std::thread::spawn(move || {
            let report = |i: usize, state: State| {
                let mut p = progress.lock().unwrap();
                p.running_since = (state == State::Running).then(Instant::now);
                p.states[i] = Some(state);
                ctx.request_repaint();
            };
            let result = Layout::new(dev).and_then(|layout| setup::run(&layout, &opts, &report));
            let mut p = progress.lock().unwrap();
            if result.is_err() {
                p.log_tail = setup::log_tail(&opts.dir, 12);
            }
            p.finished = true;
            p.error = result.err().map(|e| format!("{e:#}"));
            ctx.request_repaint();
        });
    }

    fn choose_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(egui::RichText::new("Burnout Paradise Remastered").strong());
        ui.horizontal(|ui| {
            let edit = egui::TextEdit::singleline(&mut self.burnout).hint_text("the folder with BurnoutPR.exe").desired_width(ui.available_width() - 90.0);
            if ui.add(edit).changed() {
                self.recheck();
            }
            if ui.button("Browse…").clicked() {
                if let Some(p) = rfd::FileDialog::new().set_title("Pick Burnout Paradise Remastered").pick_folder() {
                    self.burnout = p.display().to_string();
                    self.recheck();
                }
            }
        });
        match &self.burnout_status {
            Ok(_) => ui.label(egui::RichText::new("✔ found").color(GREEN).small()),
            Err(e) => ui.label(egui::RichText::new(format!("✖ {e}")).color(RED).small()),
        };

        ui.add_space(10.0);
        ui.label(egui::RichText::new("Install to").strong());
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.dir).desired_width(ui.available_width() - 90.0));
            if ui.button("Browse…").clicked() {
                if let Some(p) = rfd::FileDialog::new().set_title("Pick the install folder").pick_folder() {
                    self.dir = p.join("Paradise Viewer").display().to_string();
                }
            }
        });
        ui.checkbox(&mut self.desktop, "Desktop shortcut (there's always one in the Start menu)");

        ui.add_space(10.0);
        if self.webview2 {
            ui.label(egui::RichText::new("✔ Microsoft Edge WebView2 (the viewer runs in it)").color(GREEN).small());
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("✖ The viewer needs Microsoft Edge WebView2 (part of Windows 11). Install the Runtime from:").color(RED).small());
                ui.hyperlink_to("Microsoft", setup::WEBVIEW2_URL);
            });
        }

        ui.add_space(14.0);
        let installed = setup::installed(&self.dir());
        ui.label(
            egui::RichText::new(if installed {
                "Already installed here: installing again updates it, redoing only what changed."
            } else {
                "This builds the viewer's map from your copy of Burnout: a few minutes and about 2 GB of free disk space. Nothing from the game is included in this download."
            })
            .small()
            .weak(),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let ready = self.burnout_status.is_ok() && !self.dir.trim().is_empty();
            if ui.add_enabled(ready, egui::Button::new(egui::RichText::new("Install").strong()).min_size(egui::vec2(140.0, 32.0))).clicked() {
                if setup::viewer_running() {
                    rfd::MessageDialog::new().set_title(TITLE).set_description("Paradise Viewer is open. Close it, then press Install again.").show();
                } else {
                    self.start(ctx);
                }
            }
            if installed && ui.add(egui::Button::new("Remove").min_size(egui::vec2(90.0, 32.0))).clicked() {
                let sure = rfd::MessageDialog::new()
                    .set_title(TITLE)
                    .set_description(format!("Remove Paradise Viewer from {} and its shortcuts?", self.dir().display()))
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .show();
                if sure == rfd::MessageDialogResult::Yes {
                    self.removed = Some(if setup::viewer_running() {
                        Err("Paradise Viewer is open: close it first.".into())
                    } else {
                        setup::remove(&self.dir(), None).map_err(|e| format!("{e:#}"))
                    });
                }
            }
        });
        match &self.removed {
            Some(Ok(())) => ui.colored_label(GREEN, "Removed."),
            Some(Err(e)) => ui.colored_label(RED, e),
            None => ui.label(""),
        };
    }

    fn progress_ui(&self, ui: &mut egui::Ui, p: &Progress) {
        for (i, title) in self.titles.iter().enumerate() {
            ui.horizontal(|ui| {
                match &p.states[i] {
                    None => ui.label(egui::RichText::new("·").weak()),
                    Some(State::Running) => ui.add(egui::Spinner::new().size(12.0)),
                    Some(State::Failed) => ui.label(egui::RichText::new("✖").color(RED)),
                    Some(_) => ui.label(egui::RichText::new("✔").color(GREEN)),
                };
                ui.label(*title);
                let note = match &p.states[i] {
                    Some(State::Running) => p.running_since.map(|t| mmss(t.elapsed().as_secs())).unwrap_or_default(),
                    Some(State::Done(s)) => mmss(*s),
                    Some(State::UpToDate) => "up to date".into(),
                    _ => String::new(),
                };
                ui.label(egui::RichText::new(note).weak());
            });
        }
    }
}

fn mmss(s: u64) -> String {
    format!("{}:{:02}", s / 60, s % 60)
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(format!("Paradise Viewer {VERSION}"));
                ui.label("Explore Burnout Paradise's whole map in 3D: the city, its collision, traffic lanes and parked cars.");
                ui.add_space(8.0);
                if let Err(e) = &self.layout {
                    ui.colored_label(RED, e);
                    return;
                }

                let Some(progress) = self.progress.clone() else {
                    self.choose_ui(ui, ctx);
                    return;
                };

                let p = progress.lock().unwrap();
                ui.label(egui::RichText::new("Installing").strong());
                ui.add_space(6.0);
                self.progress_ui(ui, &p);
                ui.add_space(12.0);
                if !p.finished {
                    ui.label(egui::RichText::new("You can leave this running.").weak());
                    ctx.request_repaint_after(std::time::Duration::from_millis(500));
                    return;
                }
                let failed = p.error.is_some();
                match &p.error {
                    None => {
                        ui.colored_label(GREEN, egui::RichText::new("Done! Open Paradise Viewer from the Start menu or below.").strong());
                    }
                    Some(e) => {
                        ui.colored_label(RED, format!("Setup stopped: {e}"));
                        ui.add_space(6.0);
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            for l in &p.log_tail {
                                ui.label(egui::RichText::new(l).monospace().small());
                            }
                        });
                        if ui.button("Open the full log").clicked() {
                            let _ = std::process::Command::new("notepad").arg(setup::log_path(&self.dir())).spawn();
                        }
                    }
                }
                drop(p);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if !failed && ui.button(egui::RichText::new("Open Paradise Viewer").strong()).clicked() {
                        let exe = self.dir().join(setup::APP_EXE);
                        let _ = std::process::Command::new(&exe).current_dir(self.dir()).spawn();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if failed && ui.button("Back").clicked() {
                        self.progress = None;
                    }
                    if ui.button("Close").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });
    }
}

fn main() -> eframe::Result {
    let dev = std::env::args().any(|a| a == "--dev");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title(TITLE).with_inner_size([620.0, 560.0]).with_min_inner_size([480.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native(TITLE, options, Box::new(move |_cc| Ok(Box::new(App::new(dev)))))
}
