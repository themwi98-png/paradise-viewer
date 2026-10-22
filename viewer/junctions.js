// Junctions panel for the Paradise viewer: where GTA's traffic is controlled
// at junctions in the GTA mod (its plugin, junctions.rs). Shown from
// out/traffic/junctions.json (`bpx gta-junctions`, GTA world coordinates):
//
// - red: Burnout's own traffic lights. Their stop lines are drawn across the
//   lanes, coloured by which set of lights holds them ("Play the lights" runs
//   Burnout's light cycle on them); cars stop there at red.
// - green: junctions without lights, found where lanes cross; cars give way
//   there to cars crossing their path.
// - blue: junctions you marked (cars give way there too).
//
// Edit: click a junction to select it, change its size or switch it off;
// "Add junction" and a click on the map marks a new one. Exported as
// paradise-junction-edits-v1 JSON, saved as tools/edits/junction_edits.json
// and applied by `bpx gta-junctions` (the GTA setup's Roads step).

const STORAGE_KEY = 'paradise-junction-edits-v1';
const LIFT = 0.5;
const RING_SEGMENTS = 48;
const DEFAULT_RADIUS = 15;
const COLOURS = { lights: 0xff5040, found: 0x40e070, marked: 0x40b0ff, off: 0x808080, selected: 0xffffff };
// One colour per set of lights in a junction.
const SET_COLOURS = [0xffd23f, 0x00e5ff, 0xff40ff, 0x9dff3f, 0xff9f2e, 0x9e8cff, 0xffffff, 0xff7a9e];
const SIGNAL = { green: 0x30ff60, amber: 0xffa000, red: 0xff2a20 };
const AMBER = 3; // s a light shows amber after green (as the plugin)

const emptyEdits = () => ({ format: 'paradise-junction-edits-v1', off: [], radius: {}, added: [] });

export function initJunctions({ THREE, scene, panel, pickGround }) {
  panel.innerHTML = `
    <label><input id="jShow" type="checkbox"> Show junctions</label>
    <label><input id="jPlay" type="checkbox"> Play the lights (Burnout's cycle)</label>
    <label><input id="jEdit" type="checkbox"> Edit junctions</label>
    <div class="row"><label style="margin:0"><input type="radio" name="jMode" value="select" checked> Select</label><label style="margin:0"><input type="radio" name="jMode" value="add"> Add junction</label></div>
    <div class="row"><input id="jRadius" type="number" min="3" max="100" step="1" value="${DEFAULT_RADIUS}" style="width:60px"> m <button id="jSetRadius">Set size</button></div>
    <div class="row"><button id="jToggle">Switch off / on</button><button id="jUndo">Undo</button></div>
    <div class="row"><button id="jExport">Export edits</button><button id="jReset">Reset all</button></div>
    <div id="jInfo" style="font-size:11px;margin-top:5px;line-height:1.35;white-space:pre-line"></div>
    <div id="jStatus" style="color:var(--muted);font-size:11px;margin-top:5px;line-height:1.35"></div>`;
  const $ = (id) => panel.querySelector('#' + id);

  let placement = { offset: [0, 0, 9.9], scale: 1, heading: 0 };
  let builtIn = [];        // from junctions.json
  let edits = emptyEdits();
  let fromFile = false;
  let loaded = false;
  let selected = null;     // junction id
  const undo = [];
  let stopBars = [];       // { mesh, junction, set } for playing the lights

  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || 'null');
    if (saved && saved.format === 'paradise-junction-edits-v1') edits = Object.assign(emptyEdits(), saved);
  } catch (_) {}

  const group = new THREE.Group();
  group.name = 'junctions';
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
  const mode = () => panel.querySelector('input[name="jMode"]:checked').value;

  // Every junction with the edits applied: { id, kind, pos, radius, off, states, stops, lights }.
  function junctions() {
    const list = builtIn.map((j) => ({ ...j, radius: edits.radius[j.id] ?? j.radius, off: edits.off.includes(j.id) }));
    edits.added.forEach((a, i) => list.push({ id: `M${i}`, kind: 'marked', pos: [a[0], a[1], a[2]], radius: a[3], off: false, states: [], stops: [], lights: [] }));
    return list;
  }

  function status(msg) {
    const lights = builtIn.filter((j) => j.kind === 'lights').length;
    const found = builtIn.length - lights;
    $('jStatus').textContent = (msg ? msg + ' ' : '') +
      (loaded ? `${lights} with Burnout's traffic lights, ${found} found where lanes cross, ${edits.added.length} marked by you, ${edits.off.length} switched off, ${Object.keys(edits.radius).length} resized` : '') +
      (fromFile ? ' (started from tools/edits/junction_edits.json)' : '') +
      ($('jEdit').checked ? (mode() === 'add' ? ' · Click the map to mark a junction (size from the box).' : ' · Click a junction to select it.') : '');
  }

  function save() {
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(edits)); } catch (e) { status(`Could not save in this browser: ${e.message}`); }
  }

  function change(fn) {
    undo.push(JSON.stringify(edits));
    if (undo.length > 200) undo.shift();
    fn();
    save();
    redraw();
  }

  // Each light set's signal at time t (s), as the plugin runs it.
  function signals(j, t) {
    const out = new Array(8).fill('green');
    if (!j.states.length) return out;
    const cycle = j.states.reduce((a, s) => a + s[0], 0) || 1;
    let x = ((t % cycle) + cycle) % cycle;
    let i = 0;
    while (i + 1 < j.states.length && x >= j.states[i][0]) { x -= j.states[i][0]; i++; }
    const mask = j.states[i][1], prev = j.states[(i + j.states.length - 1) % j.states.length][1];
    for (let k = 0; k < 8; k++) {
      const bit = 1 << k;
      out[k] = !(mask & bit) ? 'green' : !(prev & bit) && x < AMBER ? 'amber' : 'red';
    }
    return out;
  }

  function ring(j, colour) {
    const pts = [];
    for (let k = 0; k < RING_SEGMENTS; k++) {
      const a = (k / RING_SEGMENTS) * Math.PI * 2;
      pts.push(toViewer(j.pos[0] + Math.cos(a) * j.radius, j.pos[1] + Math.sin(a) * j.radius, j.pos[2]));
    }
    const line = new THREE.LineLoop(new THREE.BufferGeometry().setFromPoints(pts), new THREE.LineBasicMaterial({ color: colour, depthTest: false }));
    line.renderOrder = 6;
    return line;
  }

  function redraw() {
    while (group.children.length) {
      const c = group.children.pop();
      c.traverse((o) => { o.geometry?.dispose(); o.material?.dispose(); });
    }
    stopBars = [];
    for (const j of junctions()) {
      const colour = j.off ? COLOURS.off : COLOURS[j.kind];
      group.add(ring(j, colour));
      if (j.id === selected) {
        const outer = ring({ ...j, radius: j.radius + 1.5 }, COLOURS.selected);
        group.add(outer);
      }
      if (j.off) continue;
      for (const s of j.stops) {
        // A bar across the lane at the stop line.
        const rx = s.dir[1], ry = -s.dir[0];
        const a = toViewer(s.pos[0] - rx * s.w, s.pos[1] - ry * s.w, s.pos[2]);
        const b = toViewer(s.pos[0] + rx * s.w, s.pos[1] + ry * s.w, s.pos[2]);
        const tip = toViewer(s.pos[0] + s.dir[0] * 1.5, s.pos[1] + s.dir[1] * 1.5, s.pos[2]);
        const geo = new THREE.BufferGeometry().setFromPoints([a, b, b, tip, tip, a]);
        const bar = new THREE.LineSegments(geo, new THREE.LineBasicMaterial({ color: SET_COLOURS[s.c % SET_COLOURS.length], depthTest: false }));
        bar.renderOrder = 7;
        group.add(bar);
        stopBars.push({ mesh: bar, junction: j, set: s.c });
      }
      for (const l of j.lights) {
        const dot = new THREE.Mesh(new THREE.OctahedronGeometry(0.6), new THREE.MeshBasicMaterial({ color: SET_COLOURS[l.c % SET_COLOURS.length], depthTest: false }));
        dot.position.copy(toViewer(l.pos[0], l.pos[1], l.pos[2]));
        dot.renderOrder = 7;
        group.add(dot);
      }
    }
    showInfo();
    status();
  }

  function describeStates(j) {
    if (!j.states.length) return '';
    const sets = new Set(j.stops.map((s) => s.c));
    return j.states.map(([secs, mask]) => {
      const go = [...sets].filter((k) => !(mask & (1 << k))).map((k) => k + 1);
      return `${secs} s: ${go.length ? 'set ' + go.join(', ') + ' green' : 'all red'}`;
    }).join(' · ');
  }

  function showInfo() {
    const j = junctions().find((x) => x.id === selected);
    if (!j) { $('jInfo').textContent = ''; return; }
    const what = { lights: "Burnout's traffic lights", found: 'no lights (found where lanes cross): cars give way', marked: 'marked by you: cars give way' }[j.kind];
    const sets = new Set(j.stops.map((s) => s.c)).size;
    $('jInfo').textContent = `${j.id}: ${what}${j.off ? ' — SWITCHED OFF' : ''}\n` +
      `size ${j.radius.toFixed(1)} m at ${j.pos.map((v) => v.toFixed(1)).join(', ')}` +
      (j.kind === 'lights' ? `\n${j.stops.length} stop lines in ${sets} light sets (colours); cycle: ${describeStates(j)}` : '');
    $('jRadius').value = Math.round(j.radius);
  }

  function pick(g) {
    let best = null, bestScore = Infinity;
    for (const j of junctions()) {
      const d = Math.hypot(g[0] - j.pos[0], g[1] - j.pos[1]);
      if (d > j.radius + 2 || Math.abs(g[2] - j.pos[2]) > 25) continue;
      const score = d / j.radius;
      if (score < bestScore) { bestScore = score; best = j; }
    }
    return best;
  }

  function handleClick(e) {
    if (!$('jEdit').checked) return false;
    const p = pickGround(e);
    if (!p) { status('Nothing under the cursor.'); return true; }
    const g = toGta(p);
    if (mode() === 'add') {
      const r = Math.max(3, Math.min(100, +$('jRadius').value || DEFAULT_RADIUS));
      change(() => edits.added.push([+g[0].toFixed(2), +g[1].toFixed(2), +g[2].toFixed(2), r]));
      selected = `M${edits.added.length - 1}`;
      redraw();
      status('Marked a junction.');
      return true;
    }
    const j = pick(g);
    selected = j ? j.id : null;
    redraw();
    return true;
  }

  async function load() {
    if (loaded) return;
    try {
      const doc = await (await fetch('../../out/traffic/junctions.json', { cache: 'no-store' })).json();
      if (doc.placement) placement = doc.placement;
      builtIn = doc.junctions || [];
    } catch (_) {
      status('No out/traffic/junctions.json: it comes with the GTA mod (its setup makes it).');
      return;
    }
    loaded = true;
    if (!edits.off.length && !Object.keys(edits.radius).length && !edits.added.length) {
      try {
        const r = await fetch('../edits/junction_edits.json', { cache: 'no-store' });
        if (r.ok) {
          const doc = await r.json();
          if (doc && doc.format === 'paradise-junction-edits-v1') { edits = Object.assign(emptyEdits(), doc); fromFile = true; save(); }
        }
      } catch (_) {}
    }
    redraw();
  }

  // Playing the lights: recolour the stop lines a few times a second.
  setInterval(() => {
    if (!group.visible || !$('jPlay').checked) return;
    const t = performance.now() / 1000;
    for (const b of stopBars) b.mesh.material.color.setHex(SIGNAL[signals(b.junction, t)[b.set]]);
  }, 200);

  $('jShow').addEventListener('change', async () => {
    group.visible = $('jShow').checked;
    if (group.visible) await load();
  });
  $('jPlay').addEventListener('change', () => { if (!$('jPlay').checked) redraw(); });
  $('jEdit').addEventListener('change', async () => {
    if ($('jEdit').checked) { $('jShow').checked = true; group.visible = true; await load(); }
    status();
  });
  panel.querySelectorAll('input[name="jMode"]').forEach((r) => r.addEventListener('change', () => status()));
  $('jSetRadius').addEventListener('click', () => {
    const j = junctions().find((x) => x.id === selected);
    if (!j) return status('Select a junction first.');
    const r = Math.max(3, Math.min(100, +$('jRadius').value || DEFAULT_RADIUS));
    change(() => {
      if (j.kind === 'marked') edits.added[+j.id.slice(1)][3] = r;
      else edits.radius[j.id] = r;
    });
    status(`${j.id} is now ${r} m.`);
  });
  $('jToggle').addEventListener('click', () => {
    const j = junctions().find((x) => x.id === selected);
    if (!j) return status('Select a junction first.');
    if (j.kind === 'marked') {
      change(() => edits.added.splice(+j.id.slice(1), 1));
      selected = null;
      redraw();
      return status('Removed your junction.');
    }
    change(() => {
      if (j.off) edits.off = edits.off.filter((id) => id !== j.id);
      else edits.off.push(j.id);
    });
    status(`${j.id} switched ${j.off ? 'on' : 'off'}.`);
  });
  $('jUndo').addEventListener('click', () => {
    if (!undo.length) return status('Nothing to undo.');
    edits = JSON.parse(undo.pop());
    save();
    redraw();
  });
  $('jReset').addEventListener('click', () => {
    if (!confirm('Remove every junction edit saved in this viewer?')) return;
    selected = null;
    change(() => { edits = emptyEdits(); });
  });
  $('jExport').addEventListener('click', () => {
    const blob = new Blob([JSON.stringify(edits, null, 1)], { type: 'application/json' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = 'junction_edits.json';
    a.click();
    URL.revokeObjectURL(a.href);
    status('Exported to Downloads as junction_edits.json.');
  });
  status();

  return { handleClick, group };
}
