# Paradise Viewer

A 3D viewer and map editor for Burnout Paradise's whole map, as a Windows
desktop app. It shows the city in full detail around the camera and low detail
everywhere else, along with Burnout's collision, traffic lanes and parked cars.

It's the map tool of [GTA Paradise](https://github.com/themwi98-png/gta-paradise),
the mod that puts Paradise City into GTA V. It only needs Burnout Paradise
Remastered, though. If GTA Paradise is installed on the same PC, the viewer
shows the mod's map edits and its editors work on them. Without the mod, it
shows stock Burnout.

No game files are included. The setup converts the map from the player's own
copy of Burnout.

## Using it

Download the release zip, unzip it and run `paradise-viewer-setup.exe`. The
player instructions are in [docs/PLAYER_README.txt](docs/PLAYER_README.txt)
(shipped as README.txt).

## What's here

| Path | What |
|---|---|
| `viewer/` | The viewer page: three.js (r160, in `lib/`), the map loading and streaming, and the editors (collision, traffic, water, spawns, sidewalks) |
| `crates/paradise-viewer` | The app: the page in a window (wry + tao on Microsoft Edge WebView2), with its files answered inside the app. The page is built into the exe. |
| `crates/paradise-viewer-setup` | The setup: a window (`paradise-viewer-setup`) and a console version for testing (`paradise-viewer-setup-cli`) |
| `docs/PLAYER_README.txt` | The player readme in the release |
| `tools/package.sh` | Builds the release folder and zip in `dist/` |

## Building

You need Rust, and the [gta-paradise](https://github.com/themwi98-png/gta-paradise)
project checked out next to this one (`../gta-paradise`). It provides the
Burnout converter (`bpx`) and the added collision
(`tools/collision/paradise_extra.pcx`).

```bash
cargo build --release
```

- `target/release/paradise-viewer.exe --root ../gta-paradise` shows the GTA
  project's own map data (its `out\`) and edits.
- `target/release/paradise-viewer-setup.exe --dev` runs the setup from here.
  It uses `../gta-paradise/target/release/bpx.exe`, so build that first with
  `cargo build --release -p bpx` in gta-paradise.
- `bash tools/package.sh` builds the release into `dist/Paradise-Viewer-<version>.zip`.
  The version comes from `crates/paradise-viewer-setup/Cargo.toml`.

## How it fits together

The page loads its map data from `../../out/...` and edits from `../edits/...`,
relative to where it sits in the GTA project. The app serves the page at
`/tools/viewer/` on its own scheme (`http://paradise.localhost`) and answers:

- `/tools/viewer/...` from the page built into the exe;
- `/out/...` from the map folder. That's the install folder or the GTA project
  (`--root`). Files only the GTA build makes (the water view, F9 marks) come
  from the mod's folder;
- `/tools/edits/...` from the GTA mod's folder. That's the GTA project, or
  the working folder the GTA setup recorded as `work` in
  `%LOCALAPPDATA%\GTA-Paradise\setup.json`. With no mod there are no edits.

The page's own saved state (localStorage) is kept in
`%LOCALAPPDATA%\GTA-Paradise\viewer`.

The setup converts the map into the install folder
(`%LOCALAPPDATA%\Programs\Paradise Viewer` by default). It runs `bpx export`,
`overview`, `collision` (stock, and with the added pieces) and `traffic-json`,
then copies the app and makes the shortcuts. Steps whose inputs haven't changed
are skipped on the next run, and **Remove** deletes only what the setup made.

To test the app from outside, start it with
`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333` and
connect a Chrome DevTools Protocol client to port 9333.

## License

MIT
