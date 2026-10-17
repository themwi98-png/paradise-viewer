# Paradise Viewer v0.1.0

The first release: Burnout Paradise's whole map in 3D, as a Windows app. All
you need is Burnout Paradise Remastered. The setup builds the viewer's map from
your own copy of the game, and no game files are included in the download.

---

## What it does

- **The whole map at once.** Every part of Paradise City and Big Surf Island
  shows in low detail, and the few map units nearest the camera load in full
  detail, with textures, as you move around. "Full detail for the
  _N_ nearest units" sets how many (0–12, default 3).
- **Collision layers.** Burnout's own collision, coloured by surface type, and
  a second layer with the surfaces GTA Paradise added for places Burnout never
  let you reach (in magenta).
- **Traffic.** Burnout's traffic lanes, with arrows showing the driving
  direction, and its parked cars.
- **Click anything** to see its map unit, object, material and position.
- **A desktop app.** No browser and no web server: it opens in its own window.

## With GTA Paradise

The viewer is also the map editor for
[GTA Paradise](https://github.com/themwi98-png/gta-paradise), the mod that
puts Paradise City into GTA V. If the mod is installed on the same PC, the
viewer shows its map changes: the collision it removed, the traffic lanes it
deleted, reversed or added, its parked cars and its water fixes. Its editors
work on them, and **Export** saves your changes as a file. Without the mod,
you see Burnout's map as it is.

## Installing

1. Download **Paradise-Viewer-0.1.0.zip** below, unzip it anywhere and run
   `paradise-viewer-setup.exe`. Windows may say "Windows protected your PC"
   the first time, because the setup isn't digitally signed. Click "More info",
   then "Run anyway".
2. It finds Burnout in the usual Steam and EA app folders. If yours is
   somewhere else, press Browse… and pick the folder with `BurnoutPR.exe`.
3. Press Install. It takes a few minutes (longer on a slow hard drive) and
   uses about 2 GB.
4. Open **Paradise Viewer** from the Start menu or the desktop.

**Controls:** left-drag turns the view, right-drag moves it (hold Shift to move
faster), and the mouse wheel zooms towards the pointer.

To remove it, run the setup again and press **Remove**.

## You need

- Burnout Paradise Remastered for PC (Steam or EA app). The 2009 PC version,
  *The Ultimate Box*, isn't supported.
- Windows 10 or 11 with Microsoft Edge WebView2. It's part of Windows 11; on
  Windows 10 the setup tells you if it's missing.
- About 2 GB of free disk space, and a graphics card that handles 3D in a
  browser (WebGL) well.

## Known limits

- The water, pedestrian-spawn and sidewalk editors work on GTA Paradise's
  GTA V conversion, so they're empty without the mod.
- Paradise Viewer doesn't appear in Windows' "Apps & features"; remove it
  from the setup.
