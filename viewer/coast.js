// Map coast editor for the Paradise viewer: fixes where GTA's radar and
// pause map draw the coast. "Show map coast" lays the map's sea over the
// world in magenta (out/gta_minimap_xml/coast_view.png, written by
// `bpx gta-minimap`), so you can see where it doesn't match the real shore.
// Draw shapes that are land or sea; exported as paradise-minimap-edits-v1
// JSON, saved as tools/edits/minimap_edits.json and applied by
// `bpx gta-minimap` (the GTA Paradise setup's "Radar and map" step). Sea
// shapes win where they overlap land ones.
//
// Shapes are in GTA world coordinates (z up); the viewer is in Burnout
// coordinates (y up), converted with the build's placement. Clicks over
// open water (no ground under the cursor) land on the sea's surface.

const STORAGE_KEY = 'paradise-minimap-edits-v1';
const COLOURS = { land: 0x40e060, sea: 0x3090ff, draft: 0xffffff };
const LIFT = 0.3;

const emptyEdits = () => ({ format: 'paradise-minimap-edits-v1', land: [], sea: [] });

export function initCoast({ THREE, scene, camera, panel, pickGround }) {
  panel.innerHTML = `
    <label><input id="cbView" type="checkbox"> Show map coast (magenta = sea on the map)</label>
    <label><input id="cbShow" type="checkbox"> Show coast edits</label>
    <label><input id="cbEdit" type="checkbox"> Edit coast</label>
    <div class="row"><label style="margin:0"><input type="radio" name="cbMode" value="land" checked> Make land</label><label style="margin:0"><input type="radio" name="cbMode" value="sea"> Make sea</label><label style="margin:0"><input type="radio" name="cbMode" value="delete"> Delete shape</label></div>
    <div class="row"><button id="cbFinish">Finish shape</button><button id="cbCancel">Cancel shape</button></div>
    <div class="row"><button id="cbUndo">Undo</button><button id="cbExport">Export edits</button><button id="cbReset">Reset all</button></div>
    <div class="row"><button id="cbLoad">Load tools/edits/minimap_edits.json</button></div>
    <div id="cbStatus" style="color:var(--muted);font-size:11px;margin-top:5px;line-height:1.35"></div>`;
  const $ = (id) => panel.querySelector('#' + id);

  let placement = { offset: [0, 0, 9.9], scale: 1, heading: 0 };
  let edits = emptyEdits();
  let fromFile = false;
  let draft = [];      // GTA points of the shape being drawn
  let loaded = false;
  let overlay = null;  // the map's sea, once loaded
  const undo = [];

  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || 'null');
    if (saved && saved.format === 'paradise-minimap-edits-v1') edits = Object.assign(emptyEdits(), saved);
  } catch (_) {}

  const group = new THREE.Group();
  group.name = 'coast';
  group.visible = false;
  scene.add(group);

  function toViewer(x, y, z) {
    const p = placement;
    const s = p.scale || 1, h = (p.heading || 0) * Math.PI / 180;
    const dx = x - p.offset[0], dy = y - p.offset[1];
    const rx = (dx * Math.cos(h) + dy * Math.sin(h)) / s;
    const ry = (-dx * Math.sin(h) + dy * Math.cos(h)) / s;
    return new THREE.Vector3(rx, (z - p.offset[2]) / s + LIFT, -ry);
  }
  function toGta(v) {
    const p = placement;
    const s = p.scale || 1, h = (p.heading || 0) * Math.PI / 180;
    const x = v.x * s, y = -v.z * s;
    return [x * Math.cos(h) - y * Math.sin(h) + p.offset[0], x * Math.sin(h) + y * Math.cos(h) + p.offset[1], v.y * s + p.offset[2]];
  }
  const mode = () => panel.querySelector('input[name="cbMode"]:checked').value;

  function status(msg) {
    $('cbStatus').textContent = (msg ? msg + ' ' : '') +
      `${edits.land.length} land, ${edits.sea.length} sea shape(s) saved in this browser${fromFile ? ' (started from tools/edits/minimap_edits.json)' : ''}.` +
      ($('cbEdit').checked ? (draft.length ? ` Drawing (${draft.length} points): click to add points, Finish shape to close it.` : (mode() === 'delete' ? ' Click inside a shape to delete it (the smallest one under the cursor).' : ' Click the map to start a shape.')) : '');
  }

  function save() {
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(edits)); } catch (e) { status(`Could not save in this browser: ${e.message}`); }
  }

  function shapeMesh(points, colour) {
    const out = new THREE.Group();
    const pts = points.map((p) => toViewer(p[0], p[1], p[2]));
    const line = new THREE.LineLoop(new THREE.BufferGeometry().setFromPoints(pts), new THREE.LineBasicMaterial({ color: colour, depthTest: false }));
    line.renderOrder = 6;
    out.add(line);
    if (points.length >= 3) {
      const tris = THREE.ShapeUtils.triangulateShape(points.map((p) => new THREE.Vector2(p[0], p[1])), []);
      const g = new THREE.BufferGeometry().setFromPoints(pts);
      g.setIndex(tris.flat());
      const fill = new THREE.Mesh(g, new THREE.MeshBasicMaterial({ color: colour, transparent: true, opacity: 0.35, side: THREE.DoubleSide, depthWrite: false, depthTest: false }));
      fill.renderOrder = 5;
      out.add(fill);
    }
    return out;
  }

  function redraw() {
    while (group.children.length) {
      const c = group.children.pop();
      c.traverse((o) => { o.geometry?.dispose(); o.material?.dispose(); });
    }
    for (const s of edits.land) group.add(shapeMesh(s, COLOURS.land));
    for (const s of edits.sea) group.add(shapeMesh(s, COLOURS.sea));
    if (draft.length) {
      group.add(shapeMesh(draft, COLOURS.draft));
      for (const p of draft) {
        const dot = new THREE.Mesh(new THREE.SphereGeometry(0.8, 8, 6), new THREE.MeshBasicMaterial({ color: COLOURS.draft, depthTest: false }));
        dot.position.copy(toViewer(p[0], p[1], p[2]));
        dot.renderOrder = 7;
        group.add(dot);
      }
    }
    status();
  }

  function inside(pt, shape) {
    let c = false;
    for (let i = 0, j = shape.length - 1; i < shape.length; j = i++) {
      const [xi, yi] = shape[i], [xj, yj] = shape[j];
      if ((yi > pt[1]) !== (yj > pt[1]) && pt[0] < (xj - xi) * (pt[1] - yi) / (yj - yi) + xi) c = !c;
    }
    return c;
  }

  function area(shape) {
    let a = 0;
    for (let i = 0, j = shape.length - 1; i < shape.length; j = i++) a += (shape[j][0] + shape[i][0]) * (shape[j][1] - shape[i][1]);
    return Math.abs(a / 2);
  }

  function change(fn) {
    undo.push(JSON.stringify(edits));
    if (undo.length > 200) undo.shift();
    fn();
    save();
    redraw();
  }

  // The ground under the cursor, else the sea's surface (GTA z 0).
  const ray = new THREE.Raycaster();
  function pick(e) {
    const p = pickGround(e);
    if (p) return p;
    ray.setFromCamera(new THREE.Vector2(e.clientX / innerWidth * 2 - 1, -(e.clientY / innerHeight) * 2 + 1), camera);
    const sea = new THREE.Plane(new THREE.Vector3(0, 1, 0), placement.offset[2] / (placement.scale || 1));
    return ray.ray.intersectPlane(sea, new THREE.Vector3());
  }

  function handleClick(e) {
    if (!$('cbEdit').checked) return false;
    const p = pick(e);
    if (!p) { status('Nothing under the cursor.'); return true; }
    const g = toGta(p).map((v) => +v.toFixed(2));
    if (mode() === 'delete') {
      // The smallest shape under the cursor, so a lake drawn inside a big
      // land shape goes rather than the land.
      let best = null;
      for (const key of ['land', 'sea']) {
        edits[key].forEach((s, i) => {
          if (inside(g, s) && (!best || area(s) < best.area)) best = { key, i, area: area(s) };
        });
      }
      if (!best) { status('No shape under the cursor.'); return true; }
      change(() => edits[best.key].splice(best.i, 1));
      status(`Deleted a "${best.key}" shape.`);
      return true;
    }
    draft.push(g);
    redraw();
    return true;
  }

  async function load() {
    if (loaded) return;
    loaded = true;
    try {
      const view = await (await fetch('../../out/water/water_view.json', { cache: 'no-store' })).json();
      if (view && view.placement) placement = view.placement;
    } catch (_) {}
    if (!edits.land.length && !edits.sea.length) {
      try {
        const r = await fetch('../edits/minimap_edits.json', { cache: 'no-store' });
        if (r.ok) {
          const doc = await r.json();
          if (doc && (Array.isArray(doc.land) || Array.isArray(doc.sea))) { edits = Object.assign(emptyEdits(), doc); fromFile = true; save(); }
        }
      } catch (_) {}
    }
    redraw();
  }

  // The map's sea as a see-through layer over everything, at sea level.
  async function loadOverlay() {
    if (overlay) return true;
    await load();
    let info;
    try {
      info = await (await fetch('../../out/gta_minimap_xml/coast_view.json', { cache: 'no-store' })).json();
    } catch (_) {
      status('No map coast yet: run the GTA Paradise setup (its "Radar and map" step writes it).');
      return false;
    }
    const tex = await new THREE.TextureLoader().loadAsync('../../out/gta_minimap_xml/coast_view.png');
    tex.magFilter = THREE.NearestFilter;
    tex.colorSpace = THREE.SRGBColorSpace;
    const w = info.width * info.cell, h = info.height * info.cell;
    const mesh = new THREE.Mesh(
      new THREE.PlaneGeometry(w / (placement.scale || 1), h / (placement.scale || 1)),
      new THREE.MeshBasicMaterial({ map: tex, transparent: true, depthTest: false, depthWrite: false, side: THREE.DoubleSide }),
    );
    mesh.rotation.x = -Math.PI / 2;
    mesh.rotation.z = (placement.heading || 0) * Math.PI / 180;
    mesh.position.copy(toViewer(info.x0 + w / 2, info.y1 - h / 2, 0));
    mesh.renderOrder = 3;
    mesh.name = 'coast view';
    overlay = mesh;
    scene.add(mesh);
    return true;
  }

  $('cbView').addEventListener('change', async () => {
    if ($('cbView').checked && !(await loadOverlay())) { $('cbView').checked = false; return; }
    if (overlay) overlay.visible = $('cbView').checked;
  });
  $('cbShow').addEventListener('change', async () => {
    group.visible = $('cbShow').checked;
    if (group.visible) await load();
  });
  $('cbEdit').addEventListener('change', async () => {
    if ($('cbEdit').checked) { $('cbShow').checked = true; group.visible = true; await load(); }
    else { draft = []; redraw(); }
    status();
  });
  panel.querySelectorAll('input[name="cbMode"]').forEach((r) => r.addEventListener('change', () => status()));
  $('cbFinish').addEventListener('click', () => {
    if (mode() === 'delete') return status('Pick Make land or Make sea for the shape first.');
    if (draft.length < 3) return status('A shape needs at least 3 points.');
    const shape = draft;
    draft = [];
    change(() => edits[mode()].push(shape));
    status(`Added a "${mode()}" shape.`);
  });
  $('cbCancel').addEventListener('click', () => { draft = []; redraw(); });
  $('cbUndo').addEventListener('click', () => {
    if (draft.length) { draft.pop(); redraw(); return; }
    if (!undo.length) return status('Nothing to undo.');
    edits = JSON.parse(undo.pop());
    save();
    redraw();
  });
  $('cbReset').addEventListener('click', () => {
    if (!confirm('Remove every coast edit saved in this browser?')) return;
    change(() => { edits = emptyEdits(); });
  });
  // Replaces the browser's copy with the GTA mod's saved edits.
  $('cbLoad').addEventListener('click', async () => {
    await load();
    let doc;
    try {
      const r = await fetch('../edits/minimap_edits.json', { cache: 'no-store' });
      if (!r.ok) return status('The GTA mod has no tools/edits/minimap_edits.json yet.');
      doc = await r.json();
    } catch (e) {
      return status(`Could not load tools/edits/minimap_edits.json: ${e.message}`);
    }
    if (!doc || !(Array.isArray(doc.land) || Array.isArray(doc.sea))) return status('tools/edits/minimap_edits.json is not a coast edits file.');
    if ((edits.land.length || edits.sea.length) && !confirm('Replace the coast edits saved in this browser with tools/edits/minimap_edits.json? (Undo brings them back.)')) return;
    draft = [];
    fromFile = true;
    change(() => { edits = Object.assign(emptyEdits(), doc); });
    status('Loaded tools/edits/minimap_edits.json.');
  });
  $('cbExport').addEventListener('click', () => {
    const blob = new Blob([JSON.stringify(edits, null, 1)], { type: 'application/json' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = 'minimap_edits.json';
    a.click();
    URL.revokeObjectURL(a.href);
    status('Exported. Save it as tools/edits/minimap_edits.json, then run the GTA Paradise setup again (GTA closed).');
  });
  status();

  return { handleClick, group };
}
