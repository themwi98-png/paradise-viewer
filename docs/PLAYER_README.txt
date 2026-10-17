PARADISE VIEWER
===============

A 3D viewer for Burnout Paradise's whole map: the city in full detail around
you, its collision, traffic lanes and parked cars. Made as part of GTA
Paradise (https://github.com/themwi98-png/gta-paradise), but it only needs
Burnout: no GTA V.

This download contains no files from the game. The setup builds the
viewer's map on your PC from your own copy of Burnout Paradise Remastered.


WHAT YOU NEED
-------------

1. Burnout Paradise Remastered (PC).
2. Windows 10 or 11 with Microsoft Edge WebView2, which the viewer runs in.
   It's part of Windows 11 and comes with Edge on most Windows 10 PCs; the
   setup tells you if it's missing, with a link to get it.
3. About 2 GB of free disk space.


INSTALLING
----------

1. Unzip this folder anywhere and run paradise-viewer-setup.exe.
   Windows may say "Windows protected your PC" the first time: the setup
   isn't digitally signed. Click "More info", then "Run anyway".
2. It finds Burnout by itself in the usual Steam and EA app folders. If it
   isn't found, type its folder (the one with BurnoutPR.exe) or press
   Browse... to pick it.
3. Choose where to install (or keep the default) and whether you want a
   desktop shortcut. There's always one in the Start menu.
4. Press Install. It takes a few minutes (longer on a slow hard drive):
   it converts the whole city.
5. Open Paradise Viewer from the Start menu, the desktop, or the button at
   the end of the setup.


USING IT
--------

- Left-drag turns the view, right-drag moves it (hold Shift to move
  faster), the mouse wheel zooms towards the pointer.
- The whole map loads when it opens (about half a minute). Everything
  shows in low detail, except the few map units nearest the camera, which
  load in full detail as you move around. "Full detail for the 3 nearest
  units" sets how many.
- Units: type unit numbers (e.g. 395-405) and Load to show only part of
  the map. Frame and Top centre the view on what's loaded.
- Layers: Collision (stock) is Burnout's own collision. Collision (extra)
  adds the surfaces GTA Paradise made for places Burnout never let you
  reach (magenta).
- Click anything to see its map unit, object, material and position.
- Traffic: tick "Show traffic" for the traffic lanes (yellow, arrows in the
  driving direction) and parked cars.
- The editors (collision, traffic, water, spawns, sidewalks) are GTA
  Paradise's map tools; Export saves your changes as a file in your
  Downloads folder. The water, spawn and sidewalk ones work on GTA
  Paradise's GTA V conversion, so they have nothing to show without it.


WITH GTA PARADISE
-----------------

If GTA Paradise (the GTA V mod) is installed on this PC, the viewer shows
its map changes too: the collision it removed, the traffic lanes it
deleted, reversed or added, and its parked cars. Without it, you see
Burnout's map as it is.


UPDATING
--------

Download the new version, unzip it, run its paradise-viewer-setup.exe and
press Install. Only what changed is rebuilt.


REMOVING
--------

Run paradise-viewer-setup.exe and press Remove. It deletes the viewer, its
map files and its shortcuts. The viewer's own saved state (unexported
edits) is in %LOCALAPPDATA%\GTA-Paradise\viewer.


IF SOMETHING GOES WRONG
-----------------------

The setup's log is setup.log in the install folder (by default
%LOCALAPPDATA%\Programs\Paradise Viewer).
