// Race editor for the Paradise viewer: Burnout-style races, each a start
// (and the way the cars face there), optional checkpoints in order, and a
// finish. There's no route: racers find their own way from point to point,
// as in Burnout. The dashed lines between the points only show the order.
// Exported as paradise-races-v1 JSON and saved as tools/edits/races.json.
//
// Start green (S), checkpoints orange (numbered), finish red (F); F9 marks
// (spawns panel) are yellow.
//
// Points are GTA world coordinates (z up); the start heading is in GTA
// degrees (0 north, counter-clockwise). The viewer is in Burnout
// coordinates (y up), converted with the build's placement.

const STORAGE_KEY = 'paradise-races-v1';
const COLOURS = { start: 0x2ecc40, checkpoint: 0xff8c00, finish: 0xff4136, line: 0xffffff };
/// A checkpoint click this close (screen pixels) to an existing checkpoint's
/// marker removes it.
const REMOVE_WITHIN = 14;
/// Length (m) of the arrow showing the way the cars face at the start.
const ARROW = 25;

const emptyEdits = () => ({ format: 'paradise-races-v1', races: [] });
const newRace = (name) => ({ name, start: null, heading: 0, checkpoints: [], finish: null });

export function initRaces({ THREE, scene, camera, panel, pickGround }) {
  panel.innerHTML = `
    <label><input id="rShow" type="checkbox"> Show races</label>
    <label><input id="rEdit" type="checkbox"> Edit races</label>
    <div class="row"><select id="rRace" style="flex:1;min-width:0;background:#222;color:var(--fg);border:1px solid #444;border-radius:4px"></select><button id="rNew">New race</button></div>
    <div class="row"><input id="rName" type="text" placeholder="Race name" style="flex:1"><button id="rDelete">Delete race</button></div>
    <div class="row"><label style="margin:0"><input type="radio" name="rTool" value="start" checked> Start</label><label style="margin:0"><input type="radio" name="rTool" value="checkpoint"> Checkpoint</label><label style="margin:0"><input type="radio" name="rTool" value="finish"> Finish</label></div>
    <div class="row"><button id="rUndo">Undo</button><button id="rExport">Export races</button></div>
    <div class="row"><button id="rLoad">Load tools/edits/races.json</button></div>
    <div id="rStatus" style="color:var(--muted);font-size:11px;margin-top:5px;line-height:1.35"></div>`;
  const $ = (id) => panel.querySelector('#' + id);
  const tool = () => panel.querySelector('input[name=rTool]:checked').value;

  let placement = { offset: [0, 0, 9.9], scale: 1, heading: 0 };
  let edits = emptyEdits();
  let fromFile = false;
  let current = 0;      // index of the race being edited
  let aiming = false;   // the start was just placed: the next click sets its heading
  let loading = null;   // load(), once started
  const undo = [];

  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || 'null');
    if (saved && saved.format === 'paradise-races-v1') edits = Object.assign(emptyEdits(), saved);
  } catch (_) {}

  const group = new THREE.Group();
  group.name = 'races';
  group.visible = false;
  scene.add(group);

  // GTA world <-> viewer (Burnout) coordinates.
  function toViewer([x, y, z]) {
    const p = placement;
    const s = p.scale || 1, h = (p.heading || 0) * Math.PI / 180;
    const dx = x - p.offset[0], dy = y - p.offset[1];
    const rx = (dx * Math.cos(h) + dy * Math.sin(h)) / s;
    const ry = (-dx * Math.sin(h) + dy * Math.cos(h)) / s;
    return new THREE.Vector3(rx, (z - p.offset[2]) / s, -ry);
  }
  function toGta(v) {
    const p = placement;
    const s = p.scale || 1, h = (p.heading || 0) * Math.PI / 180;
    const x = v.x * s, y = -v.z * s;
    return [x * Math.cos(h) - y * Math.sin(h) + p.offset[0], x * Math.sin(h) + y * Math.cos(h) + p.offset[1], v.y * s + p.offset[2]];
  }
  const round = (p) => p.map((c) => +c.toFixed(1));
  // GTA heading of the way from a to b.
  const headingTo = (a, b) => ((Math.atan2(-(b[0] - a[0]), b[1] - a[1]) * 180 / Math.PI) + 360) % 360;
  const dist = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);

  const race = () => edits.races[current];

  function describe(r) {
    const missing = [!r.start && 'start', !r.finish && 'finish'].filter(Boolean);
    const cps = `${r.checkpoints.length} checkpoint${r.checkpoints.length === 1 ? '' : 's'}`;
    return missing.length ? `${cps}, no ${missing.join(' or ')} yet` : cps;
  }

  function status(msg) {
    const n = edits.races.length;
    const r = race();
    let hint = '';
    if ($('rEdit').checked) {
      if (!r) hint = ' Click New race to start one.';
      else if (aiming) hint = ' Click where the cars should face at the start.';
      else hint = { start: ' Click the map to place the start, then click again where the cars face.',
                    checkpoint: ' Click to add a checkpoint (it goes where it fits best in the order); click one to remove it.',
                    finish: ' Click the map to place the finish.' }[tool()];
    }
    $('rStatus').textContent = (msg ? msg + ' ' : '') +
      `${n} race${n === 1 ? '' : 's'} saved in this browser${fromFile ? ' (started from tools/edits/races.json)' : ''}.` +
      (r ? ` "${r.name}": ${describe(r)}.` : '') + hint;
  }

  function save() {
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(edits)); } catch (e) { status(`Could not save in this browser: ${e.message}`); }
  }
  function remember() {
    undo.push(JSON.stringify({ edits, current }));
    if (undo.length > 200) undo.shift();
  }

  function fillList() {
    const list = $('rRace');
    list.innerHTML = '';
    edits.races.forEach((r, i) => {
      const o = document.createElement('option');
      o.value = i;
      o.textContent = r.name || `Race ${i + 1}`;
      list.appendChild(o);
    });
    current = Math.min(current, Math.max(edits.races.length - 1, 0));
    list.value = current;
    $('rName').value = race()?.name || '';
  }

  // Markers keep the same size on screen at any distance and draw over
  // everything, so they can be found from far away.
  const textures = [];
  function marker(colour, text, at, size, opacity) {
    const c = document.createElement('canvas');
    c.width = c.height = 64;
    const g = c.getContext('2d');
    g.beginPath(); g.arc(32, 32, 26, 0, Math.PI * 2);
    g.fillStyle = '#' + new THREE.Color(colour).getHexString(); g.fill();
    g.lineWidth = 6; g.strokeStyle = '#000'; g.stroke();
    g.font = `bold ${text.length > 2 ? 22 : 30}px sans-serif`;
    g.textAlign = 'center'; g.textBaseline = 'middle';
    g.fillStyle = '#000'; g.fillText(text, 32, 34);
    const map = new THREE.CanvasTexture(c);
    textures.push(map);
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map, depthTest: false, sizeAttenuation: false, transparent: true, opacity }));
    sprite.scale.set(size, size, 1);
    sprite.position.copy(at);
    sprite.renderOrder = 7;
    sprite.frustumCulled = false;
    return sprite;
  }

  function line(points, opacity) {
    const geometry = new THREE.BufferGeometry().setFromPoints(points);
    const l = new THREE.Line(geometry, new THREE.LineDashedMaterial({ color: COLOURS.line, dashSize: 6, gapSize: 4, depthTest: false, transparent: true, opacity }));
    l.computeLineDistances();
    l.renderOrder = 6;
    l.frustumCulled = false;
    return l;
  }

  function arrow(r, opacity) {
    const from = toViewer(r.start);
    const h = r.heading * Math.PI / 180;
    const to = toViewer([r.start[0] - Math.sin(h) * ARROW, r.start[1] + Math.cos(h) * ARROW, r.start[2]]);
    const dir = to.clone().sub(from);
    const a = new THREE.ArrowHelper(dir.clone().normalize(), from, dir.length(), COLOURS.start, ARROW * 0.35 / (placement.scale || 1), ARROW * 0.25 / (placement.scale || 1));
    for (const m of [a.line.material, a.cone.material]) { m.depthTest = false; m.transparent = true; m.opacity = opacity; }
    a.line.renderOrder = a.cone.renderOrder = 6;
    a.traverse((o) => { o.frustumCulled = false; });
    return a;
  }

  // The race in its order: start, checkpoints, finish (those placed).
  const chain = (r) => [r.start, ...r.checkpoints, r.finish].filter(Boolean);

  function redraw() {
    while (group.children.length) {
      const c = group.children.pop();
      c.traverse((o) => { o.geometry?.dispose(); o.material?.dispose(); });
    }
    while (textures.length) textures.pop().dispose();
    edits.races.forEach((r, i) => {
      // The race being edited in full; the others faded.
      const opacity = i === current ? 1 : 0.35;
      const size = i === current ? 0.03 : 0.02;
      const points = chain(r).map(toViewer);
      if (points.length > 1) group.add(line(points, opacity));
      if (r.start) {
        group.add(arrow(r, opacity));
        group.add(marker(COLOURS.start, 'S', toViewer(r.start), size * 1.2, opacity));
      }
      r.checkpoints.forEach((p, k) => group.add(marker(COLOURS.checkpoint, String(k + 1), toViewer(p), size, opacity)));
      if (r.finish) group.add(marker(COLOURS.finish, 'F', toViewer(r.finish), size * 1.2, opacity));
    });
    status();
  }

  // A new checkpoint goes where it adds the least distance to the race
  // (between the two points it sits best between), so checkpoints can be
  // added in any order.
  function insertCheckpoint(r, p) {
    const before = r.start ? [r.start] : [];
    const after = r.finish ? [r.finish] : [];
    const all = [...before, ...r.checkpoints, ...after];
    if (all.length < 2) {
      r.checkpoints.push(p);
      return r.checkpoints.length;
    }
    let best = 0, bestCost = Infinity;
    for (let k = 0; k < all.length - 1; k++) {
      const cost = dist(all[k], p) + dist(p, all[k + 1]) - dist(all[k], all[k + 1]);
      if (cost < bestCost) { bestCost = cost; best = k; }
    }
    // Also before the first point or after the last one, where there's no
    // start or finish to hold that end.
    if (!r.start && dist(p, all[0]) < bestCost) { best = -1; bestCost = dist(p, all[0]); }
    if (!r.finish && dist(all[all.length - 1], p) < bestCost) { best = all.length - 1; }
    // Position in the whole list -> position among the checkpoints.
    const at = Math.max(0, Math.min(r.checkpoints.length, best + 1 - before.length));
    r.checkpoints.splice(at, 0, p);
    return at + 1;
  }

  function handleClick(e) {
    if (!$('rEdit').checked) return false;
    const r = race();
    if (!r) { status('Click New race first.'); return true; }
    const hit = pickGround(e);
    if (!hit) { status('Nothing under the cursor.'); return true; }
    const p = round(toGta(hit));
    const where = `GTA (${p[0].toFixed(0)}, ${p[1].toFixed(0)}, ${p[2].toFixed(0)})`;
    remember();
    let msg;
    if (aiming) {
      r.heading = +headingTo(r.start, p).toFixed(1);
      aiming = false;
      msg = `Start faces ${r.heading.toFixed(0)}°.`;
    } else if (tool() === 'start') {
      r.start = p;
      // Facing the next point until the heading is clicked.
      const next = r.checkpoints[0] || r.finish;
      if (next) r.heading = +headingTo(p, next).toFixed(1);
      aiming = true;
      msg = `Start at ${where}.`;
    } else if (tool() === 'finish') {
      r.finish = p;
      msg = `Finish at ${where}.`;
    } else {
      const near = r.checkpoints.findIndex((c) => {
        const v = toViewer(c).project(camera);
        return v.z < 1 && Math.hypot((v.x + 1) / 2 * innerWidth - e.clientX, (1 - v.y) / 2 * innerHeight - e.clientY) <= REMOVE_WITHIN;
      });
      if (near >= 0) {
        r.checkpoints.splice(near, 1);
        msg = `Removed checkpoint ${near + 1}.`;
      } else {
        msg = `Added checkpoint ${insertCheckpoint(r, p)} at ${where}.`;
      }
    }
    save();
    redraw();
    status(msg);
    return true;
  }

  // The placement and, with no races in this browser, tools/edits/races.json
  // (every call waits for the same one load).
  function load() {
    loading ??= (async () => {
      try {
        const view = await (await fetch('../../out/water/water_view.json', { cache: 'no-store' })).json();
        if (view && view.placement) placement = view.placement;
      } catch (_) {}
      if (!edits.races.length) await loadFile(false);
      fillList();
      redraw();
    })();
    return loading;
  }

  // tools/edits/races.json in place of this browser's races.
  async function loadFile(ask) {
    try {
      const r = await fetch('../edits/races.json', { cache: 'no-store' });
      if (!r.ok) { if (ask) status('No tools/edits/races.json found.'); return; }
      const doc = await r.json();
      if (!doc || doc.format !== 'paradise-races-v1' || !Array.isArray(doc.races)) { if (ask) status('tools/edits/races.json is not a races file.'); return; }
      if (ask && edits.races.length && !confirm(`Replace the ${edits.races.length} race(s) in this browser with the ${doc.races.length} in tools/edits/races.json?`)) return;
      if (ask) remember();
      edits = Object.assign(emptyEdits(), doc);
      fromFile = true;
      current = 0;
      aiming = false;
      save();
      if (ask) { fillList(); redraw(); status(`Loaded ${doc.races.length} race(s).`); }
    } catch (_) { if (ask) status('Could not read tools/edits/races.json.'); }
  }

  async function show() {
    $('rShow').checked = true;
    group.visible = true;
    await load();
  }

  $('rShow').addEventListener('change', async () => {
    group.visible = $('rShow').checked;
    if (group.visible) await load();
  });
  $('rEdit').addEventListener('change', async () => {
    aiming = false;
    if ($('rEdit').checked) await show();
    status();
  });
  panel.querySelectorAll('input[name=rTool]').forEach((b) => b.addEventListener('change', () => { aiming = false; status(); }));
  $('rRace').addEventListener('change', () => {
    current = +$('rRace').value;
    aiming = false;
    $('rName').value = race()?.name || '';
    redraw();
  });
  $('rNew').addEventListener('click', async () => {
    await show();
    remember();
    edits.races.push(newRace(`Race ${edits.races.length + 1}`));
    current = edits.races.length - 1;
    aiming = false;
    panel.querySelector('input[name=rTool][value=start]').checked = true;
    if (!$('rEdit').checked) $('rEdit').checked = true;
    save();
    fillList();
    redraw();
    $('rName').select();
  });
  $('rName').addEventListener('change', () => {
    const r = race();
    if (!r) return;
    remember();
    r.name = $('rName').value.trim() || `Race ${current + 1}`;
    save();
    fillList();
    status();
  });
  $('rDelete').addEventListener('click', () => {
    const r = race();
    if (!r || !confirm(`Delete "${r.name}"?`)) return;
    remember();
    edits.races.splice(current, 1);
    aiming = false;
    save();
    fillList();
    redraw();
  });
  $('rUndo').addEventListener('click', () => {
    if (!undo.length) return status('Nothing to undo.');
    ({ edits, current } = JSON.parse(undo.pop()));
    aiming = false;
    save();
    fillList();
    redraw();
  });
  $('rLoad').addEventListener('click', () => loadFile(true));
  $('rExport').addEventListener('click', () => {
    const unfinished = edits.races.filter((r) => !r.start || !r.finish).map((r) => r.name);
    const blob = new Blob([JSON.stringify(edits, null, 1)], { type: 'application/json' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = 'races.json';
    a.click();
    URL.revokeObjectURL(a.href);
    status('Exported. Save it as tools/edits/races.json.' + (unfinished.length ? ` Still missing a start or finish (left out in game): ${unfinished.join(', ')}.` : ''));
  });
  fillList();
  status();
  // Loaded from the start (small files), so New race answers at once even
  // while the map is still loading.
  load();

  return { handleClick, group };
}
