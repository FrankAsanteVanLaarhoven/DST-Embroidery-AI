// TwinLab replays a Rust execution trace. It does not plan stitches or encode DST.
// Design space is millimetres, Y down. The scene is metres, Y up, and design Y maps to scene Z.
// The orbit eye is built only in cameraFrame. lookAt derives screen right and up from that eye.

const QUALIFICATION = "Virtual demo · Generic machine · No hardware connected · Physical quality not validated";
const THREAD = ["#1d1d1f", "#005bb5", "#0d6b3a", "#8f1d14", "#5b3a8c", "#8a6a00"];
const THREAD_RGB = [
  [0.12, 0.16, 0.22],
  [0.0, 0.36, 0.71],
  [0.05, 0.42, 0.23],
  [0.62, 0.16, 0.1],
  [0.36, 0.23, 0.55],
  [0.5, 0.38, 0.05]
];
const PART_NOTES = {
  base: "Machine body. It stays still. The casing does not travel around the design.",
  needle: "One fixed needle. Machine X and Y stay at 0. Only the tip height changes. The raised position is a development assumption.",
  presser: "The presser stays on the needle axis above the fabric. It is not a measured presser, and this rig has no cap attachment.",
  carriage: "One carriage moves with the hoop. This demo does not turn extra heads into independent machines.",
  hoop: "The hoop moves by the negated design point and holds still while the needle penetrates.",
  fabric: "Stitches are drawn on the fabric. The fabric moves with the hoop and holds during penetration."
};
const FACES = [
  { shade: 1.08, idx: [2, 3, 7, 6] },
  { shade: 0.55, idx: [0, 1, 5, 4] },
  { shade: 0.82, idx: [1, 5, 7, 3] },
  { shade: 0.7, idx: [4, 0, 2, 6] },
  { shade: 0.92, idx: [4, 6, 7, 5] },
  { shade: 0.64, idx: [0, 1, 3, 2] }
];

const workspace = document.querySelector("#workspace");
const qualificationEl = document.querySelector("#qualification");
const estimateEl = document.querySelector("#estimate");
const readoutEl = document.querySelector("#readout");
const eventLabel = document.querySelector("#event-label");
const scenarioEl = document.querySelector("#scenario");
const errorEl = document.querySelector("#errors");
const modeNoteEl = document.querySelector("#mode-note");
const partNoteEl = document.querySelector("#part-note");
const factsEl = document.querySelector("#facts");
const evidenceEl = document.querySelector("#evidence");
const eventFactsEl = document.querySelector("#event-facts");
const designSvg = document.querySelector("#design");
const designDesc = document.querySelector("#design-desc");
const machineCanvas = document.querySelector("#machine");
const glStatus = document.querySelector("#gl-status");
const viewports = document.querySelector("#viewports");
const stage = document.querySelector("#stage");
const scrubEl = document.querySelector("#scrub");
const speedEl = document.querySelector("#speed");
const modeEl = document.querySelector("#mode");
const viewEl = document.querySelector("#view");
const playEl = document.querySelector("#play");
const pauseEl = document.querySelector("#pause");
const stepBackEl = document.querySelector("#step-back");
const stepForwardEl = document.querySelector("#step-forward");
const rewindEl = document.querySelector("#rewind");
const injectEl = document.querySelector("#inject");
const loadSampleEl = document.querySelector("#load-sample");
const secondViewEl = document.querySelector("#second-view");

const state = {
  project: null,
  rehearsal: null,
  eventIndex: 0,
  phase: "travel",
  t: 0,
  playing: false,
  speed: 1,
  mode: "kinematic",
  selectedPart: "needle",
  isolate: false,
  wireframe: false,
  section: false,
  poseOk: false,
  cameraOk: true,
  busy: false,
  bank: 0,
  injectionArmed: -1,
  labelKey: "",
  pointer: null,
  follower: new URLSearchParams(location.search).get("view") === "follow",
  advance: true,
  pendingPose: null,
  meshes: null,
  cam: { preset: "orbit", yaw: 0.7, pitch: 0.55, distance: 0.55, target: [0.03, 0.02, 0.04], ortho: false }
};
state.advance = !state.follower;

let gl = null;
let program = null;
let buffer = null;
let uniforms = null;
let channel = null;
const windowId = "w" + Math.random().toString(16).slice(2);

function showError(message) {
  errorEl.textContent = message || "";
}

function lerp(from, to, t) {
  return from + (to - from) * t;
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

function mm(value) {
  return (Math.round(value * 1000) / 1000).toFixed(3);
}

function motionReduced() {
  return document.documentElement.dataset.motion === "reduce" || matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function needleTip(t, up) {
  if (t <= 0.5) return up * (1 - t / 0.5);
  return up * ((t - 0.5) / 0.5);
}

function sub(a, b) {
  return [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
}

function dot(a, b) {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

function cross(a, b) {
  return [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0]
  ];
}

function norm(v) {
  const length = Math.hypot(v[0], v[1], v[2]);
  if (length < 1e-8) return [0, 0, 0];
  return [v[0] / length, v[1] / length, v[2] / length];
}

function lookAt(eye, target, up) {
  const back = norm(sub(eye, target));
  let right = cross(up, back);
  const span = Math.hypot(right[0], right[1], right[2]);
  right = span < 1e-8 ? [1, 0, 0] : [right[0] / span, right[1] / span, right[2] / span];
  const camUp = cross(back, right);
  return {
    right: right,
    up: camUp,
    back: back,
    view: [
      right[0], camUp[0], back[0], 0,
      right[1], camUp[1], back[1], 0,
      right[2], camUp[2], back[2], 0,
      -dot(right, eye), -dot(camUp, eye), -dot(back, eye), 1
    ]
  };
}

function cameraFrame() {
  const target = state.cam.target;
  const dist = state.cam.distance;
  let eye = [0, 0, 0];
  let up = [0, 1, 0];
  if (state.cam.preset === "top") {
    eye = [target[0], target[1] + dist, target[2]];
    up = [0, 0, -1];
  } else if (state.cam.preset === "front") {
    eye = [target[0], target[1] + dist * 0.18, target[2] + dist];
  } else if (state.cam.preset === "side") {
    eye = [target[0] + dist, target[1] + dist * 0.18, target[2]];
  } else {
    const cp = Math.cos(state.cam.pitch);
    const sp = Math.sin(state.cam.pitch);
    const cy = Math.cos(state.cam.yaw);
    const sy = Math.sin(state.cam.yaw);
    eye = [target[0] + dist * cp * sy, target[1] + dist * sp, target[2] + dist * cp * cy];
  }
  return lookAt(eye, target, up);
}

function mul(a, b) {
  const out = new Array(16).fill(0);
  for (let col = 0; col < 4; col += 1) {
    for (let row = 0; row < 4; row += 1) {
      out[col * 4 + row] =
        a[row] * b[col * 4] +
        a[4 + row] * b[col * 4 + 1] +
        a[8 + row] * b[col * 4 + 2] +
        a[12 + row] * b[col * 4 + 3];
    }
  }
  return out;
}

function perspective(fovy, aspect, near, far) {
  const f = 1 / Math.tan(fovy / 2);
  return [
    f / aspect, 0, 0, 0,
    0, f, 0, 0,
    0, 0, (far + near) / (near - far), -1,
    0, 0, (2 * far * near) / (near - far), 0
  ];
}

function ortho(left, right, bottom, top, near, far) {
  return [
    2 / (right - left), 0, 0, 0,
    0, 2 / (top - bottom), 0, 0,
    0, 0, 2 / (near - far), 0,
    -(right + left) / (right - left), -(top + bottom) / (top - bottom), -(far + near) / (near - far), 1
  ];
}

function translation(x, y, z) {
  return [
    1, 0, 0, 0,
    0, 1, 0, 0,
    0, 0, 1, 0,
    x, y, z, 1
  ];
}

function transformPoint(matrix, point) {
  const x = matrix[0] * point[0] + matrix[4] * point[1] + matrix[8] * point[2] + matrix[12];
  const y = matrix[1] * point[0] + matrix[5] * point[1] + matrix[9] * point[2] + matrix[13];
  const z = matrix[2] * point[0] + matrix[6] * point[1] + matrix[10] * point[2] + matrix[14];
  const w = matrix[3] * point[0] + matrix[7] * point[1] + matrix[11] * point[2] + matrix[15];
  return [x, y, z, w];
}

function checkCameraMath() {
  const frame = lookAt([0, 1, 0], [0, 0, 0], [0, 0, -1]);
  const rightOk = Math.abs(frame.right[0] - 1) < 1e-6 && Math.abs(frame.right[1]) < 1e-6 && Math.abs(frame.right[2]) < 1e-6;
  const upOk = Math.abs(frame.up[0]) < 1e-6 && Math.abs(frame.up[1]) < 1e-6 && Math.abs(frame.up[2] + 1) < 1e-6;
  const mvp = mul(perspective(Math.PI / 4, 1, 0.01, 10), frame.view);
  const clip = transformPoint(mvp, [0, 0, 0]);
  const center = Math.abs(clip[3]) > 1e-6 && Math.abs(clip[0] / clip[3]) < 1e-4 && Math.abs(clip[1] / clip[3]) < 1e-4;
  return rightOk && upOk && center;
}

function poseAt(item, phase, t, up) {
  const clamped = clamp(t, 0, 1);
  if (phase === "penetrate") {
    return {
      designX: item.design_x_mm,
      designY: item.design_y_mm,
      hoopX: item.hoop_x_mm,
      hoopY: item.hoop_y_mm,
      needle: needleTip(clamped, up)
    };
  }
  return {
    designX: lerp(item.from_x_mm, item.design_x_mm, clamped),
    designY: lerp(item.from_y_mm, item.design_y_mm, clamped),
    hoopX: lerp(item.hoop_from_x_mm, item.hoop_x_mm, clamped),
    hoopY: lerp(item.hoop_from_y_mm, item.hoop_y_mm, clamped),
    needle: up
  };
}

function currentEvent() {
  if (!state.rehearsal) return null;
  return state.rehearsal.trace.events[state.eventIndex] || null;
}

function currentPose() {
  const item = currentEvent();
  if (!item || !state.poseOk) return null;
  return poseAt(item, state.phase, state.t, state.rehearsal.needle_up_mm);
}

function markerPoint() {
  const item = currentEvent();
  if (!item) return null;
  if (!state.poseOk) {
    return {
      designX: item.design_x_mm,
      designY: item.design_y_mm,
      hoopX: item.hoop_x_mm,
      hoopY: item.hoop_y_mm,
      needle: state.rehearsal.needle_up_mm
    };
  }
  return currentPose();
}

function posesMatch(rehearsal) {
  const samples = rehearsal.pose_samples || [];
  if (typeof rehearsal.needle_up_mm !== "number" || !samples.length) return false;
  for (const sample of samples) {
    const item = rehearsal.trace.events[sample.event_index];
    if (!item) return false;
    const local = poseAt(item, sample.phase, sample.t, rehearsal.needle_up_mm);
    if (Math.abs(local.designX - sample.design_x_mm) > 1e-6) return false;
    if (Math.abs(local.designY - sample.design_y_mm) > 1e-6) return false;
    if (Math.abs(local.hoopX - sample.hoop_x_mm) > 1e-6) return false;
    if (Math.abs(local.hoopY - sample.hoop_y_mm) > 1e-6) return false;
    if (Math.abs(local.needle - sample.needle_tip_above_fabric_mm) > 1e-6) return false;
  }
  return true;
}

function durations(item) {
  if (item.kind === "sew") return { travel: 0.04, penetrate: 0.06 };
  if (item.kind === "jump") {
    const dist = Math.hypot(item.design_x_mm - item.from_x_mm, item.design_y_mm - item.from_y_mm);
    return { travel: Math.max(0.04, dist / 200), penetrate: 0 };
  }
  if (item.kind === "color_change") return { travel: 2, penetrate: 0 };
  if (item.kind === "trim") return { travel: 0.4, penetrate: 0 };
  return { travel: 0, penetrate: 0 };
}

function kindLabel(kind) {
  if (kind === "sew") return "sew";
  if (kind === "jump") return "jump";
  if (kind === "color_change") return "colour change";
  if (kind === "trim") return "trim";
  if (kind === "end") return "end";
  return kind;
}

function showScenario(injection) {
  scenarioEl.textContent = injection.message + " Origin: " + injection.origin + ". Observed: " + (injection.observed ? "yes" : "no") + ".";
}

function clearScenario() {
  scenarioEl.textContent = "";
}

function noteInjection(pause) {
  const injection = state.rehearsal && state.rehearsal.injection;
  const item = currentEvent();
  if (!injection || !item || item.index !== injection.at_event) {
    if (!pause) clearScenario();
    return false;
  }
  showScenario(injection);
  if (pause && state.injectionArmed !== item.index) {
    state.playing = false;
    state.bank = 0;
    return true;
  }
  return false;
}

function stepToNext() {
  const events = state.rehearsal.trace.events;
  if (state.eventIndex + 1 >= events.length) {
    state.playing = false;
    state.phase = "travel";
    state.t = 1;
    state.bank = 0;
    return false;
  }
  state.eventIndex += 1;
  state.phase = "travel";
  state.t = 0;
  return true;
}

function finishPhase(item) {
  if (state.phase === "travel" && item.penetrates) {
    state.phase = "penetrate";
    state.t = 0;
    return true;
  }
  return stepToNext();
}

function advance(demoSeconds) {
  state.bank += demoSeconds;
  let guard = 0;
  while (state.playing && state.bank > 0 && guard < 10) {
    guard += 1;
    const item = currentEvent();
    if (!item) {
      state.playing = false;
      break;
    }
    if (noteInjection(true)) break;
    const spanSet = durations(item);
    if (motionReduced()) {
      const span = item.penetrates ? spanSet.travel + spanSet.penetrate : spanSet.travel;
      if (item.penetrates) {
        state.phase = "penetrate";
        state.t = 0.5;
      } else {
        state.phase = "travel";
        state.t = 1;
      }
      if (span <= 0) {
        if (!stepToNext()) break;
        continue;
      }
      if (state.bank < span) break;
      state.bank -= span;
      if (!stepToNext()) break;
      continue;
    }
    const span = state.phase === "penetrate" ? spanSet.penetrate : spanSet.travel;
    if (span <= 0) {
      if (!finishPhase(item)) break;
      continue;
    }
    const remain = (1 - state.t) * span;
    if (state.bank < remain) {
      state.t += state.bank / span;
      state.bank = 0;
    } else {
      state.bank -= remain;
      state.t = 1;
      if (!finishPhase(item)) break;
    }
  }
}

function seek(index) {
  const events = state.rehearsal.trace.events;
  state.eventIndex = clamp(index | 0, 0, events.length - 1);
  state.phase = "travel";
  state.t = 1;
  state.playing = false;
  state.bank = 0;
  state.injectionArmed = -1;
  state.labelKey = "";
  noteInjection(false);
}

function stepBy(dir) {
  if (dir < 0 && state.eventIndex > 0) state.eventIndex -= 1;
  if (dir > 0 && state.eventIndex + 1 < state.rehearsal.trace.events.length) state.eventIndex += 1;
  state.phase = "travel";
  state.t = 1;
  state.playing = false;
  state.bank = 0;
  state.injectionArmed = -1;
  state.labelKey = "";
  noteInjection(false);
}

function rewind() {
  state.eventIndex = 0;
  state.phase = "travel";
  state.t = 0;
  state.playing = false;
  state.bank = 0;
  state.injectionArmed = -1;
  state.labelKey = "";
  clearScenario();
}

function onPlay() {
  if (state.mode !== "kinematic" || !state.poseOk || !state.rehearsal) return;
  const events = state.rehearsal.trace.events;
  if (state.eventIndex >= events.length - 1 && state.phase === "travel" && state.t >= 1) {
    state.eventIndex = 0;
    state.phase = "travel";
    state.t = 0;
    state.injectionArmed = -1;
  }
  const item = currentEvent();
  const injection = state.rehearsal.injection;
  if (injection && item && item.index === injection.at_event) state.injectionArmed = item.index;
  state.playing = true;
  state.bank = 0;
}

function refresh() {
  if (!state.rehearsal) return;
  updateReadout();
  draw2d();
  draw3d();
}

function runControl(action, extra) {
  if (!state.rehearsal) return;
  if (action === "play") onPlay();
  else if (action === "pause") {
    state.playing = false;
    state.bank = 0;
  } else if (action === "seek") seek(extra.index);
  else if (action === "step") stepBy(extra.dir);
  else if (action === "rewind") rewind();
  else if (action === "speed") state.speed = extra.speed;
  renderControls();
  refresh();
}

function publishPose() {
  if (!channel || !state.advance || !state.rehearsal) return;
  channel.postMessage({
    type: "pose",
    source: windowId,
    eventIndex: state.eventIndex,
    phase: state.phase,
    t: state.t,
    playing: state.playing,
    speed: state.speed
  });
}

function sendOrDo(action, extra) {
  const payload = Object.assign({ action: action }, extra || {});
  if (state.advance) {
    runControl(action, payload);
    publishPose();
  } else if (channel) {
    channel.postMessage(Object.assign({ type: "control", source: windowId }, payload));
  } else {
    showError("This follow view cannot reach the main TwinLab window. Open the second view from TwinLab again.");
  }
}

function applyRemote(msg) {
  if (!state.rehearsal) {
    state.pendingPose = msg;
    return;
  }
  const events = state.rehearsal.trace.events;
  state.eventIndex = clamp(msg.eventIndex | 0, 0, events.length - 1);
  state.phase = msg.phase === "penetrate" ? "penetrate" : "travel";
  state.t = clamp(Number(msg.t) || 0, 0, 1);
  state.playing = Boolean(msg.playing);
  if (typeof msg.speed === "number" && msg.speed > 0) {
    state.speed = msg.speed;
    speedEl.value = String(msg.speed);
  }
  const injection = state.rehearsal.injection;
  if (injection && state.eventIndex === injection.at_event) showScenario(injection);
  state.labelKey = "";
  renderControls();
  refresh();
}

function applyInjection(injection) {
  if (!state.rehearsal || !injection) return;
  if (injection.observed || injection.origin !== "injected_scenario") {
    showError("The thread-break marker is not labelled as an injected scenario. Playback is stopped.");
    state.poseOk = false;
    state.playing = false;
    renderControls();
    return;
  }
  state.rehearsal.injection = injection;
  seek(injection.at_event);
  renderControls();
  refresh();
}

function renderControls() {
  const ready = Boolean(state.rehearsal) && !state.busy && state.poseOk;
  const geometric = state.mode === "geometric";
  playEl.disabled = !ready || geometric;
  pauseEl.disabled = !ready;
  stepBackEl.disabled = !ready;
  stepForwardEl.disabled = !ready;
  rewindEl.disabled = !ready;
  injectEl.disabled = !ready || state.busy;
  scrubEl.disabled = !ready;
  loadSampleEl.disabled = state.busy;
  if (state.rehearsal) {
    scrubEl.max = String(Math.max(0, state.rehearsal.trace.events.length - 1));
    scrubEl.value = String(state.eventIndex);
  }
}

function setEstimate(seconds) {
  estimateEl.textContent = "Estimated execution is " + seconds.toFixed(3) + " seconds at the declared demo rates. Playback speed does not change this estimate.";
  estimateEl.dataset.seconds = String(seconds);
}

function fillDl(list, rows) {
  list.replaceChildren();
  for (const [term, detail] of rows) {
    const dt = document.createElement("dt");
    dt.textContent = term;
    const dd = document.createElement("dd");
    dd.textContent = detail;
    if (term === "DST SHA-256" || term === "Revision") dd.className = "hash";
    list.append(dt, dd);
  }
}

function fillFacts() {
  const rehearsal = state.rehearsal;
  const trace = rehearsal.trace;
  fillDl(factsEl, [
    ["Profile", rehearsal.profile.manufacturer + " " + rehearsal.profile.model],
    ["Evidence", rehearsal.profile.evidence],
    ["Server mode", rehearsal.mode],
    ["Display mode", state.mode === "geometric" ? "geometric preview" : "kinematic demo"],
    ["Hoop", rehearsal.hoop_width_mm + " × " + rehearsal.hoop_height_mm + " mm"],
    ["Needle-down", String(trace.needle_down_count)],
    ["Jumps", String(trace.jump_count)],
    ["Colour changes", String(trace.color_change_count)],
    ["Trims", String(trace.trim_count)],
    ["Encoded records", String(trace.encoded_record_count)],
    ["Revision", rehearsal.revision],
    ["DST SHA-256", trace.dst_sha256],
    ["Design frame", rehearsal.transform.design],
    ["Scene frame", rehearsal.transform.scene],
    ["Mechanism", rehearsal.transform.mechanism],
    ["Timing", rehearsal.profile.timing_note]
  ]);
  evidenceEl.replaceChildren();
  const caps = rehearsal.profile.capabilities || {};
  for (const name of Object.keys(caps)) {
    const li = document.createElement("li");
    li.textContent = name.replaceAll("_", " ") + ": " + caps[name];
    evidenceEl.append(li);
  }
  for (const mode of rehearsal.profile.unavailable_modes || []) {
    const li = document.createElement("li");
    li.textContent = mode.id.replaceAll("_", " ") + " unavailable. " + mode.reason;
    evidenceEl.append(li);
  }
  modeNoteEl.textContent = state.mode === "geometric"
    ? "Geometric preview shows placement only. Timed playback is off. This is not a timing or quality claim."
    : "Kinematic demo uses declared linear moves and the development timing assumptions. It is not agreement with a physical machine.";
}

function fillEventFacts(item) {
  const rows = [
    ["Kind", kindLabel(item.kind)],
    ["Design", mm(item.design_x_mm) + " mm, " + mm(item.design_y_mm) + " mm"],
    ["Units", item.units_x + ", " + item.units_y],
    ["Colour", String(item.color_index)],
    ["Object", item.object_id == null ? "none" : String(item.object_id)],
    ["Plan stitch", item.plan_stitch_index == null ? "none" : String(item.plan_stitch_index)],
    ["File records", item.file_record_count + " from record " + item.file_record_index]
  ];
  for (const warning of item.warnings || []) {
    rows.push(["Warning", warning.message + " Origin: " + warning.origin + ". Observed: " + (warning.observed ? "yes" : "no") + "."]);
  }
  fillDl(eventFactsEl, rows);
}

function updateReadout() {
  const item = currentEvent();
  const pose = markerPoint();
  if (!item || !pose) {
    readoutEl.textContent = "No trace loaded.";
    return;
  }
  let text = "Design " + mm(pose.designX) + " mm, " + mm(pose.designY) + " mm. Hoop " + mm(pose.hoopX) + " mm, " + mm(pose.hoopY) + " mm. Needle tip " + mm(pose.needle) + " mm.";
  if (state.pointer) text += " Pointer " + mm(state.pointer.x) + " mm, " + mm(state.pointer.y) + " mm.";
  readoutEl.textContent = text;
  readoutEl.dataset.designX = String(pose.designX);
  readoutEl.dataset.designY = String(pose.designY);
  readoutEl.dataset.event = String(item.index);
  const key = item.index + ":" + state.phase;
  if (key !== state.labelKey) {
    state.labelKey = key;
    eventLabel.textContent = "Event " + item.index + ", " + kindLabel(item.kind) + ". Object " + (item.object_id == null ? "none" : item.object_id) + ".";
    scrubEl.value = String(item.index);
    scrubEl.setAttribute("aria-valuetext", eventLabel.textContent);
    fillEventFacts(item);
    designDesc.textContent = "Decoded " + kindLabel(item.kind) + " at " + mm(item.design_x_mm) + " mm, " + mm(item.design_y_mm) + " mm. Y increases downward.";
  }
}

function ns(name) {
  return document.createElementNS("http://www.w3.org/2000/svg", name);
}

function threadCss(index) {
  if (index === 0 && document.documentElement.dataset.appearance === "dark") return "#f5f5f7";
  return THREAD[index % THREAD.length];
}

function segments(pose) {
  const events = state.rehearsal.trace.events;
  const list = [];
  for (let i = 0; i < events.length; i += 1) {
    const item = events[i];
    if (item.kind !== "sew" && item.kind !== "jump") continue;
    let show = i < state.eventIndex;
    let x2 = item.design_x_mm;
    let y2 = item.design_y_mm;
    if (i === state.eventIndex) {
      if (item.kind === "sew") show = state.phase === "penetrate" && state.t >= 0.5;
      else if (pose && state.phase === "travel" && state.t > 0) {
        show = true;
        x2 = pose.designX;
        y2 = pose.designY;
      }
    }
    if (!show) continue;
    list.push({ index: i, kind: item.kind, color: item.color_index, x1: item.from_x_mm, y1: item.from_y_mm, x2: x2, y2: y2 });
  }
  return list;
}

function draw2d() {
  const rehearsal = state.rehearsal;
  const keep = new Set(["design-title", "design-desc"]);
  for (const child of Array.from(designSvg.children)) {
    if (!keep.has(child.id)) child.remove();
  }
  const width = rehearsal.hoop_width_mm;
  const height = rehearsal.hoop_height_mm;
  const pad = 8;
  designSvg.setAttribute("viewBox", (-pad) + " " + (-pad) + " " + (width + pad * 2) + " " + (height + pad * 2));
  const hoop = ns("rect");
  hoop.setAttribute("class", "hoop");
  hoop.setAttribute("x", "0");
  hoop.setAttribute("y", "0");
  hoop.setAttribute("width", String(width));
  hoop.setAttribute("height", String(height));
  hoop.setAttribute("stroke-width", state.selectedPart === "hoop" ? "1.1" : "0.4");
  if (state.selectedPart === "fabric") {
    const cloth = ns("rect");
    cloth.setAttribute("x", "0");
    cloth.setAttribute("y", "0");
    cloth.setAttribute("width", String(width));
    cloth.setAttribute("height", String(height));
    cloth.setAttribute("fill", "rgba(0, 91, 181, 0.08)");
    cloth.setAttribute("stroke", "none");
    designSvg.append(cloth);
  }
  designSvg.append(hoop);
  const pose = markerPoint();
  for (const segment of segments(pose)) {
    const line = ns("line");
    line.setAttribute("class", segment.kind === "sew" ? "stitch" : "jump");
    line.setAttribute("x1", String(segment.x1));
    line.setAttribute("y1", String(segment.y1));
    line.setAttribute("x2", String(segment.x2));
    line.setAttribute("y2", String(segment.y2));
    line.setAttribute("stroke", threadCss(segment.color));
    if (segment.index === state.eventIndex) line.setAttribute("stroke-width", "1.1");
    designSvg.append(line);
  }
  if (!pose) return;
  const mark = ns("circle");
  mark.setAttribute("class", "mark");
  mark.setAttribute("cx", String(pose.designX));
  mark.setAttribute("cy", String(pose.designY));
  mark.setAttribute("r", state.selectedPart === "needle" ? "1.8" : "1.1");
  designSvg.append(mark);
}

function clientToDesign(clientX, clientY) {
  const rect = designSvg.getBoundingClientRect();
  const box = designSvg.viewBox.baseVal;
  return {
    x: ((clientX - rect.left) / rect.width) * box.width + box.x,
    y: ((clientY - rect.top) / rect.height) * box.height + box.y
  };
}

function distanceToSegment(px, py, x1, y1, x2, y2) {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const length = dx * dx + dy * dy;
  if (length === 0) return Math.hypot(px - x1, py - y1);
  const t = clamp(((px - x1) * dx + (py - y1) * dy) / length, 0, 1);
  return Math.hypot(px - (x1 + t * dx), py - (y1 + t * dy));
}

function nearestEvent(x, y) {
  let best = -1;
  let bestDistance = 3;
  for (const item of state.rehearsal.trace.events) {
    if (item.kind !== "sew" && item.kind !== "jump") continue;
    const distance = distanceToSegment(x, y, item.from_x_mm, item.from_y_mm, item.design_x_mm, item.design_y_mm);
    if (distance <= bestDistance) {
      bestDistance = distance;
      best = item.index;
    }
  }
  return best;
}

function corner(min, max, bits) {
  return [
    (bits & 1) ? max[0] : min[0],
    (bits & 2) ? max[1] : min[1],
    (bits & 4) ? max[2] : min[2]
  ];
}

function boxMesh(min, max, rgb) {
  const solid = [];
  for (const face of FACES) {
    const verts = face.idx.map((bits) => corner(min, max, bits));
    for (const index of [0, 1, 2, 0, 2, 3]) {
      const vert = verts[index];
      solid.push(vert[0], vert[1], vert[2], rgb[0] * face.shade, rgb[1] * face.shade, rgb[2] * face.shade);
    }
  }
  const wire = [];
  const edges = [[0, 1], [1, 3], [3, 2], [2, 0], [4, 5], [5, 7], [7, 6], [6, 4], [0, 4], [1, 5], [3, 7], [2, 6]];
  for (const [a, b] of edges) {
    const va = corner(min, max, a);
    const vb = corner(min, max, b);
    wire.push(va[0], va[1], va[2], rgb[0], rgb[1], rgb[2], vb[0], vb[1], vb[2], rgb[0], rgb[1], rgb[2]);
  }
  return { solid: new Float32Array(solid), wire: new Float32Array(wire) };
}

function buildMeshes() {
  const width = state.rehearsal.hoop_width_mm / 1000;
  const height = state.rehearsal.hoop_height_mm / 1000;
  const rail = 0.004;
  const blue = [0.0, 0.36, 0.71];
  const metal = [0.55, 0.58, 0.62];
  state.meshes = {
    base: [
      boxMesh([-0.16, -0.028, -0.08], [0.24, -0.008, 0.22], [0.35, 0.37, 0.4]),
      boxMesh([-0.028, -0.008, -0.045], [0.028, 0.09, -0.012], [0.28, 0.3, 0.33])
    ],
    needle: [boxMesh([-0.0012, 0, -0.0012], [0.0012, 0.055, 0.0012], [0.78, 0.8, 0.84])],
    presser: [
      boxMesh([-0.01, 0.0022, -0.01], [-0.0022, 0.0048, 0.01], metal),
      boxMesh([0.0022, 0.0022, -0.01], [0.01, 0.0048, 0.01], metal)
    ],
    carriage: [boxMesh([-0.014, -0.008, -0.014], [width + 0.014, -0.0025, height + 0.014], [0.45, 0.48, 0.52])],
    hoop: [
      boxMesh([-rail, 0.001, -rail], [width + rail, 0.007, rail], blue),
      boxMesh([-rail, 0.001, height - rail], [width + rail, 0.007, height + rail], blue),
      boxMesh([-rail, 0.001, -rail], [rail, 0.007, height + rail], blue),
      boxMesh([width - rail, 0.001, -rail], [width + rail, 0.007, height + rail], blue)
    ],
    fabric: [boxMesh([0, -0.0014, 0], [width, 0.0005, height], [0.94, 0.92, 0.86])]
  };
}

function compileShader(type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    gl.deleteShader(shader);
    throw new Error("shader");
  }
  return shader;
}

function initGl() {
  if (!state.cameraOk) {
    glStatus.textContent = "3D camera math failed its check, so the scene is not drawn. The 2D view still shows the decoded trace.";
    return;
  }
  gl = machineCanvas.getContext("webgl", { antialias: true, alpha: false, preserveDrawingBuffer: true });
  if (!gl) {
    glStatus.textContent = "3D did not start because this browser has no WebGL context. The 2D view still shows the decoded trace.";
    return;
  }
  try {
    const vertex = compileShader(gl.VERTEX_SHADER, [
      "attribute vec3 aPos;",
      "attribute vec3 aColor;",
      "uniform mat4 uMvp;",
      "uniform mat4 uModel;",
      "uniform float uHighlight;",
      "varying vec3 vColor;",
      "varying vec3 vWorld;",
      "void main() {",
      "  vec4 world = uModel * vec4(aPos, 1.0);",
      "  vWorld = world.xyz;",
      "  vColor = mix(aColor, vec3(0.15, 0.45, 0.95), uHighlight * 0.55);",
      "  gl_Position = uMvp * vec4(aPos, 1.0);",
      "}"
    ].join("\n"));
    const fragment = compileShader(gl.FRAGMENT_SHADER, [
      "precision mediump float;",
      "varying vec3 vColor;",
      "varying vec3 vWorld;",
      "uniform float uSection;",
      "void main() {",
      "  if (uSection > 0.5 && vWorld.x > 0.0) discard;",
      "  gl_FragColor = vec4(vColor, 1.0);",
      "}"
    ].join("\n"));
    program = gl.createProgram();
    gl.attachShader(program, vertex);
    gl.attachShader(program, fragment);
    gl.bindAttribLocation(program, 0, "aPos");
    gl.bindAttribLocation(program, 1, "aColor");
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error("link");
    uniforms = {
      mvp: gl.getUniformLocation(program, "uMvp"),
      model: gl.getUniformLocation(program, "uModel"),
      highlight: gl.getUniformLocation(program, "uHighlight"),
      section: gl.getUniformLocation(program, "uSection")
    };
    buffer = gl.createBuffer();
    gl.enable(gl.DEPTH_TEST);
    gl.disable(gl.CULL_FACE);
    glStatus.textContent = "Interactive scene. Drag to orbit, shift-drag to pan, wheel to zoom. Declared linear moves between exported points. The casing stays still.";
  } catch (error) {
    gl = null;
    glStatus.textContent = "3D did not start because the scene shader failed. The 2D view still shows the decoded trace.";
  }
}

function resizeCanvas() {
  const rect = machineCanvas.getBoundingClientRect();
  const ratio = Math.min(window.devicePixelRatio || 1, 2);
  const width = Math.max(1, Math.round(rect.width * ratio));
  const height = Math.max(1, Math.round(rect.height * ratio));
  if (machineCanvas.width !== width || machineCanvas.height !== height) {
    machineCanvas.width = width;
    machineCanvas.height = height;
  }
}

function drawInterleaved(data, mode, model, viewProjection, highlight) {
  if (!data.length) return;
  const mvp = mul(viewProjection, model);
  gl.useProgram(program);
  gl.uniformMatrix4fv(uniforms.mvp, false, new Float32Array(mvp));
  gl.uniformMatrix4fv(uniforms.model, false, new Float32Array(model));
  gl.uniform1f(uniforms.highlight, highlight ? 1 : 0);
  gl.uniform1f(uniforms.section, state.section ? 1 : 0);
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 24, 0);
  gl.enableVertexAttribArray(1);
  gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 24, 12);
  gl.drawArrays(mode, 0, data.length / 6);
}

function drawPart(name, meshes, model, viewProjection) {
  if (state.isolate && state.selectedPart !== name) return;
  const highlight = state.selectedPart === name;
  for (const mesh of meshes) {
    drawInterleaved(state.wireframe ? mesh.wire : mesh.solid, state.wireframe ? gl.LINES : gl.TRIANGLES, model, viewProjection, highlight);
  }
}

function stitchData(pose) {
  const data = [];
  for (const segment of segments(pose)) {
    const rgb = THREAD_RGB[segment.color % THREAD_RGB.length];
    const y = 0.0008;
    data.push(
      segment.x1 / 1000, y, segment.y1 / 1000, rgb[0], rgb[1], rgb[2],
      segment.x2 / 1000, y, segment.y2 / 1000, rgb[0], rgb[1], rgb[2]
    );
  }
  return new Float32Array(data);
}

function draw3d() {
  if (!gl || !state.meshes) return;
  resizeCanvas();
  if (machineCanvas.clientWidth < 2 || machineCanvas.clientHeight < 2) return;
  gl.viewport(0, 0, machineCanvas.width, machineCanvas.height);
  const dark = document.documentElement.dataset.appearance === "dark";
  gl.clearColor(dark ? 0.04 : 0.98, dark ? 0.04 : 0.98, dark ? 0.05 : 0.99, 1);
  gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
  const aspect = machineCanvas.width / Math.max(1, machineCanvas.height);
  const frame = cameraFrame();
  let projection;
  if (state.cam.ortho) {
    const half = state.cam.distance * 0.35;
    projection = ortho(-half * aspect, half * aspect, -half, half, 0.01, 10);
  } else {
    projection = perspective(Math.PI / 4, aspect, 0.01, 10);
  }
  const viewProjection = mul(projection, frame.view);
  const pose = markerPoint();
  const hoop = pose ? translation(pose.hoopX / 1000, 0, pose.hoopY / 1000) : translation(0, 0, 0);
  const tip = pose ? pose.needle / 1000 : (state.rehearsal.needle_up_mm / 1000);
  const still = translation(0, 0, 0);
  drawPart("base", state.meshes.base, still, viewProjection);
  drawPart("needle", state.meshes.needle, translation(0, tip, 0), viewProjection);
  drawPart("presser", state.meshes.presser, still, viewProjection);
  drawPart("carriage", state.meshes.carriage, hoop, viewProjection);
  drawPart("hoop", state.meshes.hoop, hoop, viewProjection);
  drawPart("fabric", state.meshes.fabric, hoop, viewProjection);
  if (!state.isolate || state.selectedPart === "fabric") {
    drawInterleaved(stitchData(pose), gl.LINES, hoop, viewProjection, state.selectedPart === "fabric");
  }
}

function reasonFor(modeId) {
  const modes = (state.rehearsal && state.rehearsal.profile.unavailable_modes) || [];
  const found = modes.find((item) => item.id === modeId);
  return found ? found.reason : "That mode is unavailable for this demo.";
}

function adopt(rehearsal) {
  state.rehearsal = rehearsal;
  state.playing = false;
  state.bank = 0;
  state.injectionArmed = -1;
  state.labelKey = "";
  qualificationEl.textContent = rehearsal.qualification || QUALIFICATION;
  const qualified = rehearsal.qualification === QUALIFICATION
    && rehearsal.machine_release_allowed === false
    && rehearsal.hardware_connected === false
    && rehearsal.physical_quality_validated === false;
  state.poseOk = qualified && posesMatch(rehearsal);
  document.documentElement.dataset.poseCheck = state.poseOk ? "pass" : "fail";
  document.documentElement.dataset.qualification = qualified ? "match" : "mismatch";
  if (!qualified) {
    showError("This rehearsal claims hardware, a machine release, or a different qualification. Playback is stopped.");
  } else if (!state.poseOk) {
    showError("The page pose does not match the Rust rig. Playback is stopped. Reload TwinLab.");
  } else {
    showError("");
  }
  setEstimate(rehearsal.trace.estimated_execution_seconds);
  buildMeshes();
  const calibrated = modeEl.querySelector('option[value="calibrated"]');
  const connected = modeEl.querySelector('option[value="connected"]');
  calibrated.textContent = "Calibrated process model — " + reasonFor("calibrated_process_model");
  connected.textContent = "Connected read-only twin — " + reasonFor("connected_readonly_twin");
  calibrated.disabled = true;
  connected.disabled = true;
  fillFacts();
  if (rehearsal.injection) {
    if (rehearsal.injection.observed || rehearsal.injection.origin !== "injected_scenario") {
      showError("The thread-break marker is not labelled as an injected scenario. Playback is stopped.");
      state.poseOk = false;
      document.documentElement.dataset.poseCheck = "fail";
    } else {
      seek(rehearsal.injection.at_event);
    }
  } else {
    rewind();
  }
  if (state.pendingPose) {
    const pending = state.pendingPose;
    state.pendingPose = null;
    applyRemote(pending);
  }
  renderControls();
  updateReadout();
}

async function rehearse(project, scenario) {
  state.project = project;
  state.busy = true;
  state.playing = false;
  workspace.setAttribute("aria-busy", "true");
  eventLabel.textContent = "Rehearsing the exported file…";
  renderControls();
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 30000);
  try {
    const payload = scenario ? { project: project, scenario: scenario } : { project: project };
    const response = await fetch("/api/rehearse", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
      signal: controller.signal
    });
    const data = await response.json();
    if (!response.ok || !data.ok) {
      showError((data && data.error) || "The rehearsal did not complete. Generate the design again, then open TwinLab.");
      return;
    }
    adopt(data);
  } catch (error) {
    if (error && error.name === "AbortError") {
      showError("Rehearsal took longer than 30 seconds and was stopped. Load the sample and rehearse again.");
    } else {
      showError("TwinLab could not reach the studio. Start it with cargo run -p embroidery-studio and reload this page.");
    }
  } finally {
    clearTimeout(timer);
    state.busy = false;
    workspace.setAttribute("aria-busy", "false");
    renderControls();
  }
}

async function loadSample() {
  state.busy = true;
  renderControls();
  try {
    const response = await fetch("/api/sample");
    const data = await response.json();
    if (!response.ok) {
      showError((data && data.error) || "The sample project did not load. Run cargo run -p embroidery-studio and try again.");
      state.busy = false;
      renderControls();
      return;
    }
    try {
      sessionStorage.setItem("embroideryos.rehearsal", JSON.stringify(data));
    } catch (error) {
      showError("The browser could not store this project for TwinLab. The sample can still play in this window.");
    }
    await rehearse(data, null);
  } catch (error) {
    showError("TwinLab could not reach the studio. Start it with cargo run -p embroidery-studio and reload this page.");
    state.busy = false;
    renderControls();
  }
}

async function injectBreak() {
  if (!state.project || !state.poseOk) return;
  const atEvent = state.eventIndex;
  await rehearse(state.project, { kind: "thread_break", at_event: atEvent });
  if (channel && state.rehearsal && state.rehearsal.injection) {
    channel.postMessage({ type: "injection", source: windowId, injection: state.rehearsal.injection });
  }
}

function selectPart(name) {
  state.selectedPart = name;
  for (const button of document.querySelectorAll("#parts button")) {
    button.setAttribute("aria-pressed", button.dataset.part === name ? "true" : "false");
  }
  partNoteEl.textContent = PART_NOTES[name] || "";
}

function togglePressed(button, key) {
  state[key] = !state[key];
  button.setAttribute("aria-pressed", state[key] ? "true" : "false");
}

function tick(now) {
  if (!state.lastNow) state.lastNow = now;
  const dt = Math.min(0.05, (now - state.lastNow) / 1000);
  state.lastNow = now;
  if (state.playing && state.advance && state.poseOk && state.mode === "kinematic" && state.rehearsal) {
    advance(dt * state.speed);
    publishPose();
  }
  if (state.rehearsal) {
    draw2d();
    draw3d();
    updateReadout();
  }
  requestAnimationFrame(tick);
}

function bootChannel() {
  if (typeof BroadcastChannel !== "function") {
    if (state.follower) showError("This browser has no BroadcastChannel, so the second view cannot share the clock. Use the main TwinLab window.");
    return;
  }
  channel = new BroadcastChannel("embroideryos.twinlab");
  channel.onmessage = (message) => {
    const data = message.data;
    if (!data || data.source === windowId) return;
    if (data.type === "hello" && state.advance) publishPose();
    else if (data.type === "pose" && !state.advance) applyRemote(data);
    else if (data.type === "control" && state.advance) {
      runControl(data.action, data);
      publishPose();
    } else if (data.type === "injection") applyInjection(data.injection);
  };
}

playEl.addEventListener("click", () => sendOrDo("play"));
pauseEl.addEventListener("click", () => sendOrDo("pause"));
stepBackEl.addEventListener("click", () => sendOrDo("step", { dir: -1 }));
stepForwardEl.addEventListener("click", () => sendOrDo("step", { dir: 1 }));
rewindEl.addEventListener("click", () => sendOrDo("rewind"));
injectEl.addEventListener("click", () => { injectBreak(); });
loadSampleEl.addEventListener("click", () => { loadSample(); });
secondViewEl.addEventListener("click", () => {
  const popup = window.open("/twinlab?view=follow", "embroideryos-twinlab-follow");
  if (!popup) showError("The browser blocked the second TwinLab window. Allow pop-ups for this local page and open the second view again.");
});
scrubEl.addEventListener("input", () => sendOrDo("seek", { index: Number(scrubEl.value) }));
speedEl.addEventListener("change", () => {
  const value = Number(speedEl.value);
  if (!Number.isFinite(value) || value <= 0) return;
  sendOrDo("speed", { speed: value });
});
modeEl.addEventListener("change", () => {
  const value = modeEl.value;
  if (value === "calibrated" || value === "connected") {
    const reason = reasonFor(value === "calibrated" ? "calibrated_process_model" : "connected_readonly_twin");
    modeEl.value = state.mode;
    showError(reason);
    return;
  }
  state.mode = value;
  if (value === "geometric") {
    state.playing = false;
    state.bank = 0;
    publishPose();
  }
  fillFacts();
  renderControls();
});
viewEl.addEventListener("change", () => {
  viewports.dataset.view = viewEl.value;
});
document.querySelector("#cam-top").addEventListener("click", () => { state.cam.preset = "top"; });
document.querySelector("#cam-front").addEventListener("click", () => { state.cam.preset = "front"; });
document.querySelector("#cam-side").addEventListener("click", () => { state.cam.preset = "side"; });
document.querySelector("#cam-ortho").addEventListener("click", (domEvent) => {
  state.cam.ortho = !state.cam.ortho;
  domEvent.currentTarget.setAttribute("aria-pressed", state.cam.ortho ? "true" : "false");
});
document.querySelector("#wireframe").addEventListener("click", (domEvent) => togglePressed(domEvent.currentTarget, "wireframe"));
document.querySelector("#isolate").addEventListener("click", (domEvent) => togglePressed(domEvent.currentTarget, "isolate"));
document.querySelector("#section").addEventListener("click", (domEvent) => togglePressed(domEvent.currentTarget, "section"));
for (const button of document.querySelectorAll("#parts button")) {
  button.addEventListener("click", () => selectPart(button.dataset.part));
}
stage.addEventListener("keydown", (domEvent) => {
  if (domEvent.target !== stage) return;
  if (domEvent.key === "ArrowRight") {
    domEvent.preventDefault();
    sendOrDo("step", { dir: 1 });
  } else if (domEvent.key === "ArrowLeft") {
    domEvent.preventDefault();
    sendOrDo("step", { dir: -1 });
  } else if (domEvent.key === "Home") {
    domEvent.preventDefault();
    sendOrDo("rewind");
  } else if (domEvent.key === " ") {
    domEvent.preventDefault();
    sendOrDo(state.playing ? "pause" : "play");
  }
});
designSvg.addEventListener("pointermove", (domEvent) => {
  if (!state.rehearsal) return;
  state.pointer = clientToDesign(domEvent.clientX, domEvent.clientY);
});
designSvg.addEventListener("pointerleave", () => { state.pointer = null; });
designSvg.addEventListener("click", (domEvent) => {
  if (!state.rehearsal || !state.poseOk) return;
  const point = clientToDesign(domEvent.clientX, domEvent.clientY);
  const hit = nearestEvent(point.x, point.y);
  if (hit >= 0) sendOrDo("seek", { index: hit });
});
machineCanvas.addEventListener("pointerdown", (domEvent) => {
  machineCanvas.setPointerCapture(domEvent.pointerId);
  state.drag = { x: domEvent.clientX, y: domEvent.clientY, pan: domEvent.shiftKey || domEvent.button === 1 };
});
machineCanvas.addEventListener("pointermove", (domEvent) => {
  if (!state.drag) return;
  const dx = domEvent.clientX - state.drag.x;
  const dy = domEvent.clientY - state.drag.y;
  state.drag.x = domEvent.clientX;
  state.drag.y = domEvent.clientY;
  if (state.drag.pan || domEvent.shiftKey) {
    const frame = cameraFrame();
    const scale = state.cam.distance * 0.0022;
    state.cam.target[0] += (-frame.right[0] * dx + frame.up[0] * dy) * scale;
    state.cam.target[1] += (-frame.right[1] * dx + frame.up[1] * dy) * scale;
    state.cam.target[2] += (-frame.right[2] * dx + frame.up[2] * dy) * scale;
  } else {
    state.cam.preset = "orbit";
    state.cam.yaw -= dx * 0.005;
    state.cam.pitch = clamp(state.cam.pitch + dy * 0.005, 0.08, 1.4);
  }
});
machineCanvas.addEventListener("pointerup", () => { state.drag = null; });
machineCanvas.addEventListener("pointercancel", () => { state.drag = null; });
machineCanvas.addEventListener("wheel", (domEvent) => {
  domEvent.preventDefault();
  state.cam.distance = clamp(state.cam.distance * Math.exp(domEvent.deltaY * 0.001), 0.08, 2.5);
}, { passive: false });

if (state.follower) {
  secondViewEl.disabled = true;
  secondViewEl.textContent = "Following the main window";
}

state.cameraOk = checkCameraMath();
document.documentElement.dataset.cameraCheck = state.cameraOk ? "pass" : "fail";
if (!state.cameraOk) showError("The 3D camera basis failed its check. The 2D view can still load a trace.");
initGl();
bootChannel();
selectPart("needle");
requestAnimationFrame(tick);

(async function start() {
  let project = null;
  try {
    const saved = sessionStorage.getItem("embroideryos.rehearsal");
    if (saved) project = JSON.parse(saved);
  } catch (error) {
    project = null;
  }
  if (project && project.format === "embroidery-project") await rehearse(project, null);
  else await loadSample();
  if (state.follower && channel) channel.postMessage({ type: "hello", source: windowId });
})();
