const state = {
  project: null,
  selected: null,
  undo: [],
  redo: [],
  result: null,
  basis: null,
  notice: "",
  busy: false,
};

const objectsEl = document.querySelector("#objects");
const fieldsEl = document.querySelector("#fields");
const canvas = document.querySelector("#canvas");
const errorsEl = document.querySelector("#errors");
const notesEl = document.querySelector("#notes");
const checksEl = document.querySelector("#checks");
const riskEl = document.querySelector("#risk");
const readinessEl = document.querySelector("#readiness");
const generateButton = document.querySelector("#generate");
const reviewButton = document.querySelector("#review");
const exportButton = document.querySelector("#export");
const verifyButton = document.querySelector("#verify");
const rehearseButton = document.querySelector("#rehearse");

document.querySelector("#inspector").addEventListener("submit", (event) => {
  event.preventDefault();
  generate();
});
document.querySelector("#undo").addEventListener("click", undo);
document.querySelector("#redo").addEventListener("click", redo);
reviewButton.addEventListener("click", recordReview);
exportButton.addEventListener("click", downloadDst);
verifyButton.addEventListener("click", verifyExport);
rehearseButton.addEventListener("click", openTwinLab);
document.querySelector("#appearance").addEventListener("change", saveAppearance);
document.querySelector("#motion").addEventListener("change", savePreferences);
document.querySelector("#opaque").addEventListener("change", savePreferences);
document.addEventListener("keydown", onKeydown);

function remember(snapshot) {
  state.undo.push(snapshot);
  if (state.undo.length > 50) state.undo.shift();
  state.redo = [];
}

function snapshot() {
  return JSON.stringify(state.project);
}

function restore(text) {
  state.project = JSON.parse(text);
  if (!state.project.objects.some((object) => object.object_id === state.selected)) {
    state.selected = state.project.objects[0] ? state.project.objects[0].object_id : null;
  }
  state.notice = "";
  showError("");
  render();
}

function undo() {
  if (!state.undo.length || state.busy) return;
  state.redo.push(snapshot());
  restore(state.undo.pop());
}

function redo() {
  if (!state.redo.length || state.busy) return;
  state.undo.push(snapshot());
  restore(state.redo.pop());
}

function edit(mutator) {
  const before = snapshot();
  mutator();
  if (snapshot() !== before) {
    state.notice = "";
    showError("");
    remember(before);
    render();
  }
}

function selectedObject() {
  return state.project.objects.find((object) => object.object_id === state.selected) || null;
}

function dirty() {
  return !state.basis || canonical(state.project) !== state.basis;
}

function canonical(project) {
  const copy = JSON.parse(JSON.stringify(project));
  delete copy.review;
  return stable(copy);
}

function stable(value) {
  if (Array.isArray(value)) return "[" + value.map(stable).join(",") + "]";
  if (value && typeof value === "object") {
    return "{" + Object.keys(value).sort().map((key) => JSON.stringify(key) + ":" + stable(value[key])).join(",") + "}";
  }
  return JSON.stringify(value);
}

function render() {
  const project = state.project;
  const object = selectedObject();
  objectsEl.replaceChildren();
  project.objects.forEach((item) => {
    const li = document.createElement("li");
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = item.label + " · " + item.kind;
    button.setAttribute("aria-pressed", item.object_id === state.selected ? "true" : "false");
    button.addEventListener("click", () => {
      state.selected = item.object_id;
      render();
    });
    li.append(button);
    objectsEl.append(li);
  });
  renderFields(object);
  renderCanvas();
  renderResult();
  const changed = dirty();
  const ready = state.result && state.result.readiness && state.result.readiness.engineering_export_allowed && !changed;
  reviewButton.disabled = state.busy || !ready || (state.result && state.result.readiness.reviewed_export_allowed);
  exportButton.disabled = state.busy || !ready || !state.result.dst_hex;
  verifyButton.disabled = state.busy || !state.result || !state.result.dst_hex || changed;
  rehearseButton.disabled = state.busy || !ready;
  document.querySelector("#undo").disabled = state.busy || state.undo.length === 0;
  document.querySelector("#redo").disabled = state.busy || state.redo.length === 0;
  generateButton.disabled = state.busy;
}

function renderFields(object) {
  fieldsEl.replaceChildren();
  if (!object) {
    const empty = document.createElement("p");
    empty.className = "hint";
    empty.textContent = "This project has no objects.";
    fieldsEl.append(empty);
    return;
  }
  const known = object.kind === "running" || object.kind === "satin" || object.kind === "fill";
  if (!known) {
    const warning = document.createElement("p");
    warning.className = "hint";
    warning.textContent = object.kind + " is planned and unavailable. Choose running, satin, or fill.";
    fieldsEl.append(warning);
  }
  fieldsEl.append(field("Label", "text", object.label, (value) => { object.label = value; }));
  const kind = document.createElement("label");
  kind.textContent = "Stitch type";
  const select = document.createElement("select");
  ["running", "satin", "fill"].forEach((name) => {
    const option = document.createElement("option");
    option.value = name;
    option.textContent = name;
    option.selected = object.kind === name;
    select.append(option);
  });
  if (!known) {
    const option = document.createElement("option");
    option.value = object.kind;
    option.textContent = object.kind + " (unavailable)";
    option.selected = true;
    select.append(option);
  }
  select.addEventListener("change", () => edit(() => { object.kind = select.value; }));
  kind.append(select);
  fieldsEl.append(kind);
  if (object.kind === "fill") {
    fieldsEl.append(numberField("Angle (degrees)", object.angle_degrees, (value) => { object.angle_degrees = value; }));
    fieldsEl.append(numberField("Row spacing (mm)", object.density_mm, (value) => { object.density_mm = value; }));
    fieldsEl.append(numberField("Maximum stitch (mm)", object.max_stitch_mm, (value) => { object.max_stitch_mm = value; }));
    fieldsEl.append(numberField("Underlay spacing (mm)", object.underlay_spacing_mm, (value) => { object.underlay_spacing_mm = value; }));
    fieldsEl.append(check("Perpendicular underlay", object.underlay.includes("perpendicular_run"), (checked) => {
      object.underlay = checked ? ["perpendicular_run"] : [];
    }));
  }
  if (object.kind === "satin") {
    fieldsEl.append(numberField("Station spacing (mm)", object.density_mm, (value) => { object.density_mm = value; }));
    fieldsEl.append(numberField("Column width (mm)", object.column_width_mm, (value) => { object.column_width_mm = value; }));
    fieldsEl.append(numberField("Pull compensation (mm)", object.pull_compensation_mm, (value) => { object.pull_compensation_mm = value; }));
    fieldsEl.append(numberField("Underlay stitch (mm)", object.max_stitch_mm, (value) => { object.max_stitch_mm = value; }));
    const paired = object.underlay.includes("centre_run") && object.underlay.includes("edge_run");
    fieldsEl.append(check("Centre and edge underlay", paired, (checked) => {
      object.underlay = checked ? ["centre_run", "edge_run"] : [];
    }));
    const pairNote = document.createElement("p");
    pairNote.className = "hint";
    pairNote.textContent = "The satin generator sews centre run and edge run together. Zigzag underlay is planned and unavailable.";
    fieldsEl.append(pairNote);
  }
  if (object.kind === "running") {
    fieldsEl.append(numberField("Maximum stitch (mm)", object.max_stitch_mm, (value) => { object.max_stitch_mm = value; }));
  }
  fieldsEl.append(pointsField(object));
  const move = document.createElement("div");
  move.className = "row";
  [["Left", -1, 0], ["Right", 1, 0], ["Up", 0, -1], ["Down", 0, 1]].forEach(([label, dx, dy]) => {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = label;
    button.addEventListener("click", () => nudge(dx, dy));
    move.append(button);
  });
  fieldsEl.append(move);
}

function field(label, type, value, apply) {
  const wrap = document.createElement("label");
  wrap.append(document.createTextNode(label));
  const input = document.createElement("input");
  input.type = type;
  input.value = value;
  input.addEventListener("change", () => edit(() => apply(input.value)));
  wrap.append(input);
  return wrap;
}

function numberField(label, value, apply) {
  const wrap = document.createElement("label");
  wrap.append(document.createTextNode(label));
  const input = document.createElement("input");
  input.type = "number";
  input.step = "0.1";
  input.value = String(value);
  input.addEventListener("change", () => {
    const next = Number(input.value);
    if (!Number.isFinite(next)) {
      showError("Enter a finite number for " + label + ", then generate again.");
      input.value = String(value);
      return;
    }
    edit(() => apply(next));
  });
  wrap.append(input);
  return wrap;
}

function check(label, checked, apply) {
  const wrap = document.createElement("label");
  wrap.className = "choice";
  const input = document.createElement("input");
  input.type = "checkbox";
  input.checked = checked;
  input.addEventListener("change", () => edit(() => apply(input.checked)));
  wrap.append(input, document.createTextNode(label));
  return wrap;
}

function pointsField(object) {
  const wrap = document.createElement("label");
  wrap.append(document.createTextNode("Geometry (x, y millimetres)"));
  const area = document.createElement("textarea");
  area.rows = 5;
  area.value = object.points.map((point) => point.x + ", " + point.y).join("\n");
  area.addEventListener("change", () => {
    const points = parsePoints(area.value);
    if (!points) {
      showError("Enter one x, y pair per line, in millimetres, then generate again.");
      area.value = object.points.map((point) => point.x + ", " + point.y).join("\n");
      return;
    }
    edit(() => { object.points = points; });
  });
  wrap.append(area);
  return wrap;
}

function parsePoints(text) {
  const points = [];
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const parts = trimmed.split(/[,\s]+/);
    if (parts.length !== 2) return null;
    const x = Number(parts[0]);
    const y = Number(parts[1]);
    if (!Number.isFinite(x) || !Number.isFinite(y)) return null;
    points.push({ x, y });
  }
  return points.length ? points : null;
}

function nudge(dx, dy) {
  const object = selectedObject();
  if (!object) return;
  edit(() => {
    object.points.forEach((point) => {
      point.x = Math.round((point.x + dx) * 1000) / 1000;
      point.y = Math.round((point.y + dy) * 1000) / 1000;
    });
  });
}

function renderCanvas() {
  const project = state.project;
  canvas.setAttribute("viewBox", "0 0 " + project.hoop_width_mm + " " + project.hoop_height_mm);
  canvas.replaceChildren(canvas.querySelector("title"), canvas.querySelector("desc"));
  const hoop = document.createElementNS("http://www.w3.org/2000/svg", "rect");
  hoop.setAttribute("class", "hoop");
  hoop.setAttribute("x", "0");
  hoop.setAttribute("y", "0");
  hoop.setAttribute("width", String(project.hoop_width_mm));
  hoop.setAttribute("height", String(project.hoop_height_mm));
  canvas.append(hoop);
  const plan = state.result && state.result.plan;
  const desc = canvas.querySelector("desc");
  if (!plan) {
    desc.textContent = "No stitch plan yet. Generate stitches to draw the Rust engine output.";
    return;
  }
  plan.stitches.forEach((stitch) => {
    if (stitch.kind === "color_change") {
      const mark = document.createElementNS("http://www.w3.org/2000/svg", "circle");
      mark.setAttribute("class", "mark");
      mark.setAttribute("cx", String(stitch.to.x));
      mark.setAttribute("cy", String(stitch.to.y));
      mark.setAttribute("r", "0.8");
      canvas.append(mark);
      return;
    }
    const line = document.createElementNS("http://www.w3.org/2000/svg", "line");
    line.setAttribute("class", stitch.kind === "jump" ? "jump" : "stitch");
    line.setAttribute("x1", String(stitch.from.x));
    line.setAttribute("y1", String(stitch.from.y));
    line.setAttribute("x2", String(stitch.to.x));
    line.setAttribute("y2", String(stitch.to.y));
    canvas.append(line);
  });
  desc.textContent = "Needle-down stitches are solid lines. Jumps are dashed. The paths are the stitch plan returned by the Rust engine.";
}

function renderResult() {
  const changed = dirty();
  const result = state.result;
  if (!result) {
    readinessEl.textContent = "Draft";
    document.querySelector("#stitch-count").textContent = "None yet";
    document.querySelector("#revision").textContent = "Not generated";
    document.querySelector("#preview-note").textContent = "Generate stitches to fill this preview from the Rust engine.";
    notesEl.replaceChildren();
    checksEl.replaceChildren();
    riskEl.replaceChildren();
    return;
  }
  readinessEl.textContent = changed ? "Draft" : result.readiness.label;
  const sewing = (result.plan.stitches || []).filter((stitch) => stitch.kind !== "jump" && stitch.kind !== "trim" && stitch.kind !== "color_change").length;
  document.querySelector("#stitch-count").textContent = String(sewing);
  document.querySelector("#revision").textContent = changed ? "Edits are not generated yet" : result.revision.slice(0, 12);
  document.querySelector("#preview-note").textContent = changed
    ? "These edits are not in the stitch plan yet. Generate stitches to update the preview."
    : (state.notice || "Preview matches the generated revision. Machine release remains unavailable.");
  notesEl.replaceChildren();
  (result.readiness.notes || []).forEach((note) => {
    const p = document.createElement("p");
    p.className = "hint";
    p.textContent = note;
    notesEl.append(p);
  });
  checksEl.replaceChildren();
  const validation = result.validation || { errors: [], warnings: [] };
  if (validation.errors.length === 0 && validation.warnings.length === 0) {
    const item = document.createElement("li");
    item.textContent = "No geometry errors on the generated plan.";
    checksEl.append(item);
  }
  validation.errors.forEach((message) => addItem(checksEl, message));
  validation.warnings.forEach((message) => addItem(checksEl, message));
  riskEl.replaceChildren();
  const flags = (result.risk && result.risk.flags) || [];
  if (flags.length === 0) {
    const item = document.createElement("li");
    item.textContent = "No length-density flags.";
    riskEl.append(item);
  }
  flags.forEach((flag) => addItem(riskEl, flag.message));
}

function addItem(list, message) {
  const item = document.createElement("li");
  item.textContent = message;
  list.append(item);
}

function showError(message) {
  errorsEl.textContent = message || "";
}

async function post(url, payload) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 30000);
  try {
    const response = await fetch(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(payload),
      signal: controller.signal,
    });
    const body = await response.json();
    if (!response.ok || body.ok === false) {
      throw new Error(body.error || "The studio could not complete that request. Check the project and try again.");
    }
    return body;
  } catch (error) {
    if (error.name === "AbortError") {
      throw new Error("Generation did not finish within 30 seconds. Check that the studio server is running and try again.");
    }
    throw error;
  } finally {
    clearTimeout(timer);
  }
}

async function generate() {
  if (state.busy || !state.project) return;
  state.notice = "";
  state.busy = true;
  generateButton.textContent = "Generating stitches…";
  generateButton.setAttribute("aria-busy", "true");
  showError("");
  render();
  try {
    const result = await post("/api/generate", state.project);
    state.result = result;
    state.basis = canonical(state.project);
    render();
  } catch (error) {
    showError(error.message);
  } finally {
    state.busy = false;
    generateButton.textContent = "Generate stitches";
    generateButton.removeAttribute("aria-busy");
    render();
  }
}

async function recordReview() {
  const reviewer = document.querySelector("#reviewer").value.trim();
  if (!reviewer) {
    showError("Enter a reviewer name, then record the geometry review.");
    return;
  }
  state.busy = true;
  render();
  try {
    const stamped = await post("/api/review", { project: state.project, reviewer });
    state.project = stamped.project;
    showError("");
    state.busy = false;
    await generate();
  } catch (error) {
    showError(error.message);
    state.busy = false;
    render();
  }
}

function openTwinLab() {
  if (!state.project || dirty() || !state.result || !state.result.readiness.engineering_export_allowed) {
    return;
  }
  try {
    sessionStorage.setItem("embroideryos.rehearsal", JSON.stringify(state.project));
  } catch (error) {
    showError("The browser could not store this project for TwinLab. Open TwinLab and load the sample project.");
    return;
  }
  window.location.href = "/twinlab";
}

function downloadDst() {
  if (!state.result || !state.result.dst_hex || dirty()) return;
  const hex = state.result.dst_hex;
  const bytes = new Uint8Array(hex.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16);
  }
  const blob = new Blob([bytes], { type: "application/octet-stream" });
  const link = document.createElement("a");
  const name = (state.project.name || "design").replace(/[^A-Za-z0-9._-]+/g, "_");
  link.href = URL.createObjectURL(blob);
  link.download = name + ".dst";
  link.click();
  URL.revokeObjectURL(link.href);
}

async function verifyExport() {
  if (!state.result || !state.result.dst_hex) return;
  state.busy = true;
  render();
  try {
    const report = await post("/api/verify", { dst_hex: state.result.dst_hex });
    const last = report.last ? " Last point " + report.last.x + ", " + report.last.y + " mm." : "";
    showError("");
    state.notice = "Read back " + report.stitch_count + " commands." + last + " " + report.note;
  } catch (error) {
    showError(error.message);
  } finally {
    state.busy = false;
    render();
  }
}

function onKeydown(event) {
  const typing = event.target.closest("input, textarea, select");
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z" && !typing) {
    event.preventDefault();
    if (event.shiftKey) redo();
    else undo();
    return;
  }
  if (typing || state.busy) return;
  const step = event.shiftKey ? 5 : 1;
  if (event.key === "ArrowLeft") { event.preventDefault(); nudge(-step, 0); }
  if (event.key === "ArrowRight") { event.preventDefault(); nudge(step, 0); }
  if (event.key === "ArrowUp") { event.preventDefault(); nudge(0, -step); }
  if (event.key === "ArrowDown") { event.preventDefault(); nudge(0, step); }
}

function saveAppearance() {
  const value = document.querySelector("#appearance").value;
  try { localStorage.setItem("embroideryos.appearance", value); } catch (error) { /* preference storage is optional */ }
  applyAppearance();
}

function savePreferences() {
  const motion = document.querySelector("#motion").checked ? "reduce" : "full";
  const opaque = document.querySelector("#opaque").checked ? "true" : "false";
  document.documentElement.dataset.motion = motion;
  document.documentElement.dataset.opaque = opaque;
  try {
    localStorage.setItem("embroideryos.motion", motion);
    localStorage.setItem("embroideryos.opaque", opaque);
  } catch (error) { /* preference storage is optional */ }
}

function applyAppearance() {
  const choice = document.querySelector("#appearance").value;
  const dark = choice === "dark" || (choice === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.appearance = dark ? "dark" : "light";
}

function loadPreferences() {
  let appearance = "system";
  let motion = document.documentElement.dataset.motion || "full";
  let opaque = document.documentElement.dataset.opaque || "true";
  try {
    appearance = localStorage.getItem("embroideryos.appearance") || appearance;
    motion = localStorage.getItem("embroideryos.motion") || motion;
    opaque = localStorage.getItem("embroideryos.opaque") || opaque;
  } catch (error) { /* keep the defaults set in the page head */ }
  document.querySelector("#appearance").value = appearance;
  document.querySelector("#motion").checked = motion === "reduce";
  document.querySelector("#opaque").checked = opaque !== "false";
  applyAppearance();
  savePreferences();
}

async function start() {
  loadPreferences();
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applyAppearance);
  try {
    const response = await fetch("/api/sample");
    if (!response.ok) throw new Error("sample");
    state.project = await response.json();
    state.selected = state.project.objects[0].object_id;
    showError("");
    render();
  } catch (error) {
    showError("The studio server is not running. From the repository root run cargo run -p embroidery-studio, then open http://127.0.0.1:4173. This page does not generate stitches.");
  }
}

start();
