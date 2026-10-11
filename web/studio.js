const PALETTE = [
  { name: "Forest", ink: "#1e4d3c" },
  { name: "Far ridge", ink: "#6d8b78" },
  { name: "Snow", ink: "#8d8778" },
  { name: "Shadow", ink: "#3f5c4c" },
  { name: "Morning sun", ink: "#c4923a" },
  { name: "Lower ridge", ink: "#244033" },
  { name: "Gold", ink: "#b08a3e" },
];
const NOMINAL_SPM = 650;
const VIEWS = ["overview", "library", "studio", "validation", "approvals", "production", "materials", "insights", "settings"];

const state = {
  project: null,
  selected: null,
  undo: [],
  redo: [],
  result: null,
  basis: null,
  notice: "",
  busy: false,
  view: "studio",
  mode: "artwork",
  zoom: 1,
  library: [],
  cursor: null,
  playing: false,
  proposal: null,
  designProposal: null,
  listening: false,
  recognition: null,
  attachedImage: null,
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
const verifyButton = document.querySelector("#verify");
const rehearseButton = document.querySelector("#rehearse");

document.querySelector("#inspector").addEventListener("submit", (event) => {
  event.preventDefault();
  generate();
});
generateButton.addEventListener("click", generate);
document.querySelector("#undo").addEventListener("click", undo);
document.querySelector("#redo").addEventListener("click", redo);
reviewButton.addEventListener("click", () => {
  const reviewer = document.querySelector("#reviewer").value.trim();
  if (!reviewer) {
    showView("approvals");
    showError("Enter a reviewer name, then record the geometry review.");
    return;
  }
  recordReview();
});
document.querySelectorAll("[data-export]").forEach((button) => {
  button.addEventListener("click", downloadDst);
});
verifyButton.addEventListener("click", verifyExport);
rehearseButton.addEventListener("click", openTwinLab);
document.querySelector("#appearance").addEventListener("change", saveAppearance);
document.querySelector("#motion").addEventListener("change", savePreferences);
document.querySelector("#opaque").addEventListener("change", savePreferences);
document.querySelector("#add-object").addEventListener("click", addObject);
document.querySelector("#open-sample").addEventListener("click", openSample);
document.querySelector("#explore-proposal").addEventListener("click", exploreProposal);
document.querySelector("#apply-proposal").addEventListener("click", applyProposal);
document.querySelector("#record-review").addEventListener("click", recordReview);
document.querySelector("#zoom-out").addEventListener("click", () => setZoom(state.zoom - 0.25));
document.querySelector("#zoom-in").addEventListener("click", () => setZoom(state.zoom + 0.25));
document.querySelector("#rail-toggle").addEventListener("click", toggleRail);
document.querySelector("#rail-collapse").addEventListener("click", toggleRail);
document.querySelector("#prompt-form").addEventListener("submit", (event) => {
  event.preventDefault();
  askDesign();
});
document.querySelector("#design-image").addEventListener("change", (event) => {
  const file = event.target.files && event.target.files[0];
  if (file) attachFile(file);
});
document.querySelector("#clear-image").addEventListener("click", clearAttachedImage);
document.querySelector("#voice-input").addEventListener("click", voiceInput);
document.querySelector("#apply-design").addEventListener("click", () => applyDesign(false));
document.querySelector("#apply-generate").addEventListener("click", () => applyDesign(true));
document.querySelector("#discard-design").addEventListener("click", discardDesign);
document.querySelector("#design-prompt").addEventListener("paste", onPromptPaste);
document.querySelector("#design-prompt").addEventListener("keydown", (event) => {
  if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
    event.preventDefault();
    askDesign();
  }
});
document.querySelector("#play-sequence").addEventListener("click", togglePlay);
document.querySelector("#sequence").addEventListener("input", (event) => {
  state.playing = false;
  state.cursor = Number(event.target.value);
  render();
});
document.querySelectorAll("[data-view]").forEach((button) => {
  button.addEventListener("click", () => showView(button.dataset.view));
});
["artwork", "stitches", "travel", "density"].forEach((mode) => {
  document.querySelector("#mode-" + mode).addEventListener("click", () => {
    state.mode = mode;
    render();
  });
});
document.addEventListener("keydown", onKeydown);
window.addEventListener("hashchange", () => showView(location.hash.slice(1)));

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
  state.selected = state.project.objects[0] ? state.project.objects[0].object_id : null;
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
    state.playing = false;
    showError("");
    remember(before);
    render();
  }
}

function selectedObject() {
  if (!state.project) return null;
  return state.project.objects.find((object) => object.object_id === state.selected) || null;
}

function dirty() {
  return !state.basis || canonical(state.project) !== state.basis;
}

function unsavedEdits() {
  return state.undo.length > 0 || (state.basis && canonical(state.project) !== state.basis);
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

function showView(name) {
  if (!VIEWS.includes(name)) name = "studio";
  state.view = name;
  document.querySelectorAll("[data-page]").forEach((section) => {
    section.hidden = section.dataset.page !== name;
  });
  document.querySelectorAll("[data-view]").forEach((button) => {
    button.setAttribute("aria-current", button.dataset.view === name ? "page" : "false");
  });
  if (location.hash !== "#" + name) history.replaceState(null, "", "#" + name);
}

function setZoom(value) {
  state.zoom = Math.min(3, Math.max(0.5, Math.round(value * 4) / 4));
  render();
}

function kindLabel(kind) {
  if (kind === "fill") return "Scanline fill";
  if (kind === "running") return "Running stitch";
  if (kind === "satin") return "Satin";
  return kind + " (unavailable)";
}

function ink(index) {
  return PALETTE[index % PALETTE.length].ink;
}

function render() {
  const project = state.project;
  if (!project) return;
  const object = selectedObject();
  document.querySelector("#project-title").textContent = project.name || "Untitled";
  document.querySelector("#inspector-name").textContent = object ? object.label : "No object selected";
  document.title = (project.name || "Design") + " — EmbroideryOS";
  objectsEl.replaceChildren();
  project.objects.forEach((item) => {
    const li = document.createElement("li");
    const button = document.createElement("button");
    button.type = "button";
    const chip = document.createElement("span");
    chip.className = "swatch";
    chip.style.background = ink(item.color_index);
    const copy = document.createElement("span");
    copy.className = "object-copy";
    const name = document.createElement("span");
    name.textContent = item.label;
    const kind = document.createElement("small");
    kind.textContent = kindLabel(item.kind);
    copy.append(name, kind);
    button.append(chip, copy);
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
  renderChrome(project);
  const changed = dirty();
  const ready = state.result && state.result.readiness && state.result.readiness.engineering_export_allowed && !changed;
  reviewButton.disabled = state.busy || !ready || (state.result && state.result.readiness.reviewed_export_allowed);
  document.querySelectorAll("[data-export]").forEach((button) => {
    button.disabled = state.busy || !ready || !state.result.dst_hex;
  });
  verifyButton.disabled = state.busy || !state.result || !state.result.dst_hex || changed;
  rehearseButton.disabled = state.busy || !ready;
  document.querySelector("#undo").disabled = state.busy || state.undo.length === 0;
  document.querySelector("#redo").disabled = state.busy || state.redo.length === 0;
  generateButton.disabled = state.busy;
  document.querySelector("#apply-proposal").disabled = !state.proposal || !state.proposal.apply;
  document.querySelector("#record-review").disabled = reviewButton.disabled;
  document.querySelector("#ask-design").disabled = state.busy;
  document.querySelector("#voice-input").disabled = state.busy;
  document.querySelector("#apply-design").disabled = state.busy || !state.designProposal;
  document.querySelector("#apply-generate").disabled = state.busy || !state.designProposal;
}

function renderChrome(project) {
  const revision = state.result && !dirty() ? state.result.revision.slice(0, 12) : "not generated";
  const kind = project.name === "Alpine Club" ? "Outdoor badge" : project.objects.length + " objects";
  document.querySelector("#project-sub").textContent = kind + " · Revision " + revision + " · Local draft";
  document.querySelector("#hoop-note").textContent = project.hoop_width_mm + " × " + project.hoop_height_mm + " mm · no physical fabric simulation";
  document.querySelector("#library-count").textContent = String(state.library.length);
  const sewing = sewingStitches();
  const jumps = jumpCount();
  document.querySelector("#overview-copy").textContent = project.name + " is a local draft. The stitch plan comes from the Rust engine. Physical validation has not been performed.";
  fillFacts(document.querySelector("#overview-facts"), [
    ["Hoop", project.hoop_width_mm + " × " + project.hoop_height_mm + " mm"],
    ["Objects", String(project.objects.length)],
    ["Needle-down", sewing == null ? "Not generated" : String(sewing)],
    ["Readiness", readinessEl.textContent],
  ]);
  const fitsTwin = project.hoop_width_mm <= 120 && project.hoop_height_mm <= 120;
  document.querySelector("#profile-note").textContent = fitsTwin
    ? "This hoop fits the studio rehearsal. The separate Compare workspace uses generic-flat-120-v1 and still does not connect a machine."
    : "This " + project.hoop_width_mm + " × " + project.hoop_height_mm + " mm hoop does not fit the Compare profile generic-flat-120-v1. Studio rehearsal uses the project hoop. Neither path releases a machine.";
  fillFacts(document.querySelector("#production-facts"), [
    ["Engineering export", dirty() || !state.result ? "Not current" : "Ready to download"],
    ["Machine release", "Unavailable"],
    ["Compare profile", fitsTwin ? "Within 120 mm" : "Outside 120 mm"],
  ]);
  document.querySelector("#approval-state").textContent = state.result && state.result.readiness.review_matches_revision
    ? "The geometry review matches this revision."
    : "No geometry review matches this revision.";
  renderLibrary();
  renderMaterials();
  renderSequence(sewing, jumps);
  const notes = {
    artwork: "Geometry preview. Display colours show the regions. Faint lines are the Rust stitch plan.",
    stitches: "Needle-down stitches are solid lines from the Rust plan. Jumps are dashed.",
    travel: "Only jump commands from the Rust plan are drawn.",
    density: "Colour shows segment length in the Rust plan. It is not a fabric simulation.",
  };
  document.querySelector("#mode-note").textContent = notes[state.mode];
  document.querySelectorAll("[data-mode], .tabs button").forEach((button) => {
    if (button.id && button.id.startsWith("mode-")) {
      button.setAttribute("aria-selected", button.id === "mode-" + state.mode ? "true" : "false");
    }
  });
}

function fillFacts(list, rows) {
  list.replaceChildren();
  rows.forEach(([name, value]) => {
    const term = document.createElement("dt");
    term.textContent = name;
    const detail = document.createElement("dd");
    detail.textContent = value;
    list.append(term, detail);
  });
}

function renderLibrary() {
  const list = document.querySelector("#library");
  list.replaceChildren();
  state.library.forEach((item) => {
    const button = document.createElement("button");
    button.type = "button";
    const name = document.createElement("span");
    name.textContent = item.name;
    const detail = document.createElement("small");
    detail.textContent = item.objects + " objects · " + item.hoop_width_mm + " × " + item.hoop_height_mm + " mm";
    button.append(name, detail);
    button.addEventListener("click", () => openLibrary(item.file));
    list.append(button);
  });
}

function renderMaterials() {
  const list = document.querySelector("#materials");
  list.replaceChildren();
  const seen = new Set();
  state.project.objects.forEach((object) => {
    if (seen.has(object.color_index)) return;
    seen.add(object.color_index);
    const item = document.createElement("li");
    const chip = document.createElement("span");
    chip.className = "swatch";
    chip.style.background = ink(object.color_index);
    const text = document.createElement("span");
    const colour = PALETTE[object.color_index % PALETTE.length];
    text.textContent = "Colour " + (object.color_index + 1) + " · " + colour.name;
    item.append(chip, text);
    list.append(item);
  });
}

function sewingStitches() {
  if (!state.result || !state.result.plan) return null;
  return state.result.plan.stitches.filter((stitch) => stitch.kind !== "jump" && stitch.kind !== "trim" && stitch.kind !== "color_change").length;
}

function jumpCount() {
  if (!state.result || !state.result.plan) return null;
  return state.result.plan.stitches.filter((stitch) => stitch.kind === "jump").length;
}

function renderSequence(sewing, jumps) {
  const bar = document.querySelector("#sequence-bar");
  const range = document.querySelector("#sequence");
  const total = state.result && state.result.plan ? state.result.plan.stitches.length : 0;
  bar.replaceChildren();
  if (!total) {
    range.disabled = true;
    range.max = "1";
    document.querySelector("#sequence-caption").textContent = "Not generated";
    document.querySelector("#sequence-state").textContent = "Not generated";
    return;
  }
  const counts = new Map();
  state.result.plan.stitches.forEach((stitch) => {
    if (stitch.kind === "jump" || stitch.kind === "trim" || stitch.kind === "color_change") return;
    counts.set(stitch.color_index, (counts.get(stitch.color_index) || 0) + 1);
  });
  counts.forEach((count, index) => {
    const piece = document.createElement("span");
    piece.style.width = (count / Math.max(sewing || 1, 1) * 100) + "%";
    piece.style.background = ink(index);
    bar.append(piece);
  });
  range.disabled = false;
  range.max = String(total);
  if (state.cursor == null || state.cursor > total) state.cursor = total;
  range.value = String(state.cursor);
  const colours = new Set(state.project.objects.map((object) => object.color_index)).size;
  document.querySelector("#sequence-caption").textContent = (sewing || 0) + " stitches · " + colours + " display colours";
  document.querySelector("#sequence-state").textContent = state.cursor >= total ? "Complete" : "Record " + state.cursor + " / " + total;
  const play = document.querySelector("#play-sequence");
  play.textContent = state.playing ? "❚❚" : "▶";
  play.setAttribute("aria-label", state.playing ? "Pause stitch sequence" : "Play stitch sequence");
  void jumps;
}

function renderFields(object) {
  fieldsEl.replaceChildren();
  const geometry = document.querySelector("#geometry");
  geometry.replaceChildren();
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
  fieldsEl.append(field("Object name", "text", object.label, (value) => { object.label = value; }));
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
  select.addEventListener("change", () => edit(() => {
    object.kind = select.value;
    if (select.value !== "fill") delete object.holes;
  }));
  kind.append(select);
  fieldsEl.append(kind);
  if (object.kind === "fill") {
    const pair = document.createElement("div");
    pair.className = "pair";
    pair.append(
      colourField(object),
      numberField("Row spacing · mm", object.density_mm, (value) => { object.density_mm = value; }),
    );
    const pairTwo = document.createElement("div");
    pairTwo.className = "pair";
    pairTwo.append(
      numberField("Fill angle · °", object.angle_degrees, (value) => { object.angle_degrees = value; }),
      numberField("Max stitch · mm", object.max_stitch_mm, (value) => { object.max_stitch_mm = value; }),
    );
    fieldsEl.append(pair, pairTwo);
    fieldsEl.append(numberField("Underlay spacing · mm", object.underlay_spacing_mm, (value) => { object.underlay_spacing_mm = value; }));
    fieldsEl.append(check("Perpendicular underlay", object.underlay.includes("perpendicular_run"), (checked) => {
      object.underlay = checked ? ["perpendicular_run"] : [];
    }));
  }
  if (object.kind === "satin") {
    fieldsEl.append(numberField("Station spacing · mm", object.density_mm, (value) => { object.density_mm = value; }));
    fieldsEl.append(numberField("Column width · mm", object.column_width_mm, (value) => { object.column_width_mm = value; }));
    fieldsEl.append(numberField("Pull compensation · mm", object.pull_compensation_mm, (value) => { object.pull_compensation_mm = value; }));
    fieldsEl.append(numberField("Underlay stitch · mm", object.max_stitch_mm, (value) => { object.max_stitch_mm = value; }));
    const paired = object.underlay.includes("centre_run") && object.underlay.includes("edge_run");
    fieldsEl.append(check("Centre and edge underlay", paired, (checked) => {
      object.underlay = checked ? ["centre_run", "edge_run"] : [];
    }));
    const pairNote = document.createElement("p");
    pairNote.className = "hint";
    pairNote.textContent = "The satin generator sews centre run and edge run together. Zigzag underlay is planned and unavailable.";
    fieldsEl.append(pairNote);
  }
  if (object.kind !== "fill") fieldsEl.append(colourField(object));
  if (object.kind === "running") {
    fieldsEl.append(numberField("Maximum stitch · mm", object.max_stitch_mm, (value) => { object.max_stitch_mm = value; }));
  }
  const origin = bounds(object);
  const movePair = document.createElement("div");
  movePair.className = "pair";
  movePair.append(
    numberField("Move X · mm", origin.x, (value) => translate(object, value - bounds(object).x, 0)),
    numberField("Move Y · mm", origin.y, (value) => translate(object, 0, value - bounds(object).y)),
  );
  fieldsEl.append(movePair);
  const order = document.createElement("div");
  order.className = "row";
  [["Earlier", -1], ["Later", 1]].forEach(([label, direction]) => {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = label;
    button.addEventListener("click", () => reorder(direction));
    order.append(button);
  });
  const remove = document.createElement("button");
  remove.type = "button";
  remove.textContent = "Delete";
  remove.addEventListener("click", removeObject);
  order.append(remove);
  fieldsEl.append(order);
  const limit = document.createElement("p");
  limit.className = "hint";
  limit.textContent = "Perpendicular underlay and satin pull compensation use the Rust generators. Stitch-level sculpting and a fabric simulation are not implemented. Unsupported controls are not shown as active.";
  fieldsEl.append(limit);
  const geometryTitle = document.createElement("h2");
  geometryTitle.textContent = "Geometry";
  geometry.append(geometryTitle, pointsField(object));
  if (object.kind === "fill") geometry.append(holesField(object));
  const nudgeRow = document.createElement("div");
  nudgeRow.className = "row";
  [["Left", -1, 0], ["Right", 1, 0], ["Up", 0, -1], ["Down", 0, 1]].forEach(([label, dx, dy]) => {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = label;
    button.addEventListener("click", () => nudge(dx, dy));
    nudgeRow.append(button);
  });
  geometry.append(nudgeRow);
}

function colourField(object) {
  const wrap = document.createElement("label");
  wrap.append(document.createTextNode("Thread colour"));
  const bar = document.createElement("span");
  bar.className = "thread-bar";
  bar.style.background = ink(object.color_index);
  bar.setAttribute("aria-hidden", "true");
  const select = document.createElement("select");
  PALETTE.forEach((colour, index) => {
    const option = document.createElement("option");
    option.value = String(index);
    option.textContent = colour.name;
    option.selected = object.color_index === index;
    select.append(option);
  });
  if (object.color_index >= PALETTE.length) {
    const option = document.createElement("option");
    option.value = String(object.color_index);
    option.textContent = "Colour " + (object.color_index + 1);
    option.selected = true;
    select.append(option);
  }
  select.addEventListener("change", () => edit(() => { object.color_index = Number(select.value); }));
  wrap.append(bar, select);
  return wrap;
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
  input.value = String(Math.round(value * 1000) / 1000);
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
  area.rows = 4;
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

function holesField(object) {
  const wrap = document.createElement("label");
  wrap.append(document.createTextNode("Holes (x, y millimetres, blank line between rings)"));
  const area = document.createElement("textarea");
  area.id = "object-holes";
  area.rows = 4;
  area.value = (object.holes || []).map((ring) => ring.map((point) => point.x + ", " + point.y).join("\n")).join("\n\n");
  area.addEventListener("change", () => {
    const holes = parseHoles(area.value);
    if (!holes) {
      showError("Enter hole rings as x, y pairs. Leave a blank line between rings, then generate again.");
      area.value = (object.holes || []).map((ring) => ring.map((point) => point.x + ", " + point.y).join("\n")).join("\n\n");
      return;
    }
    edit(() => {
      if (holes.length) object.holes = holes;
      else delete object.holes;
    });
  });
  wrap.append(area);
  return wrap;
}

function parseHoles(text) {
  const rings = [];
  let current = [];
  const flush = () => {
    if (!current.length) return;
    rings.push(current);
    current = [];
  };
  for (const line of text.split("\n")) {
    if (!line.trim()) {
      flush();
      continue;
    }
    const parts = line.trim().split(/[,\s]+/);
    if (parts.length !== 2) return null;
    const x = Number(parts[0]);
    const y = Number(parts[1]);
    if (!Number.isFinite(x) || !Number.isFinite(y)) return null;
    current.push({ x, y });
  }
  flush();
  return rings;
}

function bounds(object) {
  return {
    x: Math.min(...object.points.map((point) => point.x)),
    y: Math.min(...object.points.map((point) => point.y)),
  };
}

function translate(object, dx, dy) {
  const move = (point) => {
    point.x = Math.round((point.x + dx) * 1000) / 1000;
    point.y = Math.round((point.y + dy) * 1000) / 1000;
  };
  object.points.forEach(move);
  (object.holes || []).forEach((ring) => ring.forEach(move));
}

function nudge(dx, dy) {
  const object = selectedObject();
  if (!object) return;
  edit(() => translate(object, dx, dy));
}

function reorder(direction) {
  const index = state.project.objects.findIndex((object) => object.object_id === state.selected);
  const next = index + direction;
  if (index < 0 || next < 0 || next >= state.project.objects.length) return;
  edit(() => {
    const [item] = state.project.objects.splice(index, 1);
    state.project.objects.splice(next, 0, item);
  });
}

function addObject() {
  const next = state.project.objects.reduce((max, object) => Math.max(max, object.object_id), 0) + 1;
  edit(() => {
    state.project.objects.push({
      object_id: next,
      label: "New region",
      kind: "fill",
      points: [{ x: 20, y: 20 }, { x: 40, y: 20 }, { x: 40, y: 36 }, { x: 20, y: 36 }],
      angle_degrees: 0,
      density_mm: 1.2,
      column_width_mm: 2,
      max_stitch_mm: 3.5,
      underlay_spacing_mm: 0,
      underlay: [],
      pull_compensation_mm: 0,
      color_index: 0,
    });
    state.selected = next;
  });
}

function removeObject() {
  if (state.project.objects.length < 2) {
    showError("Keep at least one object. Add another region before removing this one.");
    return;
  }
  edit(() => {
    state.project.objects = state.project.objects.filter((object) => object.object_id !== state.selected);
    state.selected = state.project.objects[0].object_id;
  });
}

function renderCanvas() {
  const project = state.project;
  const zoom = state.zoom || 1;
  const width = project.hoop_width_mm / zoom;
  const height = project.hoop_height_mm / zoom;
  const originX = (project.hoop_width_mm - width) / 2;
  const originY = (project.hoop_height_mm - height) / 2;
  canvas.setAttribute("viewBox", originX + " " + originY + " " + width + " " + height);
  document.querySelector("#zoom-label").textContent = Math.round(zoom * 100) + "%";
  canvas.replaceChildren(canvas.querySelector("title"), canvas.querySelector("desc"));
  const hoop = document.createElementNS("http://www.w3.org/2000/svg", "rect");
  hoop.setAttribute("class", "hoop");
  hoop.setAttribute("x", "0");
  hoop.setAttribute("y", "0");
  hoop.setAttribute("width", String(project.hoop_width_mm));
  hoop.setAttribute("height", String(project.hoop_height_mm));
  hoop.setAttribute("rx", "4");
  canvas.append(hoop);
  const plan = state.result && state.result.plan;
  for (const object of project.objects) {
    appendRing(object.points, object, false);
    for (const hole of object.holes || []) appendRing(hole, object, true);
    if (plan && state.mode === "artwork") drawObjectPlan(plan, object);
  }
  if (state.mode === "artwork" || !plan) {
    finishCanvas(plan
      ? "Geometry preview of the editable regions. Faint lines are the Rust stitch plan. Choose Stitches to inspect every command."
      : "Outlines show the editable region, including holes. Generate stitches to draw the Rust engine output.");
    return;
  }
  drawPlan(plan, state.mode);
  finishCanvas("Needle-down stitches are solid lines. Jumps are dashed, including travel across a hole. The paths are the stitch plan returned by the Rust engine.");
}

function finishCanvas(text) {
  appendSelection(selectedObject());
  canvas.querySelector("desc").textContent = text;
}

function drawObjectPlan(plan, object) {
  const limit = state.cursor == null ? plan.stitches.length : state.cursor;
  plan.stitches.slice(0, limit).forEach((stitch) => {
    if (stitch.color_index !== object.color_index) return;
    if (stitch.kind === "jump" || stitch.kind === "color_change" || stitch.kind === "trim") return;
    drawSegment(stitch, ink(stitch.color_index), false, true);
  });
}

function drawPlan(plan, mode) {
  const limit = state.cursor == null ? plan.stitches.length : state.cursor;
  plan.stitches.slice(0, limit).forEach((stitch) => {
    if (stitch.kind === "color_change" || stitch.kind === "trim") return;
    if (mode === "artwork") {
      if (stitch.kind === "jump") return;
      drawSegment(stitch, ink(stitch.color_index), false, true);
      return;
    }
    if (mode === "travel" && stitch.kind !== "jump") return;
    if (mode === "stitches" && stitch.kind === "jump") {
      drawSegment(stitch, "#8d8778", true, false);
      return;
    }
    if (stitch.kind === "jump") {
      drawSegment(stitch, "#8d8778", true, false);
      return;
    }
    const colour = mode === "density" ? densityColour(stitch) : ink(stitch.color_index);
    drawSegment(stitch, colour, false, false);
  });
}

function appendRing(points, object, hole) {
  if (!points || points.length < 2) return;
  const polygon = document.createElementNS("http://www.w3.org/2000/svg", "polygon");
  const selected = object && object.object_id === state.selected && !hole;
  polygon.setAttribute("class", selected ? "region selected" : "region");
  polygon.setAttribute("points", points.map((point) => point.x + "," + point.y).join(" "));
  if (state.mode === "artwork") {
    if (hole) {
      polygon.style.fill = "var(--paper)";
      polygon.style.stroke = "#1f6feb";
      polygon.style.strokeWidth = "0.35";
    } else if (object.kind === "running") {
      polygon.style.fill = "none";
      polygon.style.stroke = ink(object.color_index);
      polygon.style.strokeWidth = "1.35";
    } else {
      polygon.style.fill = ink(object.color_index);
      polygon.style.fillOpacity = "0.94";
      polygon.style.stroke = "none";
    }
  }
  canvas.append(polygon);
}

function appendSelection(object) {
  if (!object || !object.points || !object.points.length) return;
  const xs = object.points.map((point) => point.x);
  const ys = object.points.map((point) => point.y);
  const pad = 4;
  const rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
  rect.setAttribute("class", "selection");
  rect.setAttribute("x", String(Math.min(...xs) - pad));
  rect.setAttribute("y", String(Math.min(...ys) - pad));
  rect.setAttribute("width", String(Math.max(...xs) - Math.min(...xs) + pad * 2));
  rect.setAttribute("height", String(Math.max(...ys) - Math.min(...ys) + pad * 2));
  canvas.append(rect);
}

function drawSegment(stitch, colour, jump, faint) {
  const line = document.createElementNS("http://www.w3.org/2000/svg", "line");
  line.setAttribute("class", faint ? "stitch faint" : (jump ? "jump" : "stitch"));
  line.setAttribute("x1", String(stitch.from.x));
  line.setAttribute("y1", String(stitch.from.y));
  line.setAttribute("x2", String(stitch.to.x));
  line.setAttribute("y2", String(stitch.to.y));
  line.style.stroke = colour;
  canvas.append(line);
}

function densityColour(stitch) {
  const length = Math.hypot(stitch.to.x - stitch.from.x, stitch.to.y - stitch.from.y);
  if (length >= 3) return "#c4923a";
  if (length >= 1.5) return "#6d8b78";
  return "#1e4d3c";
}

function renderResult() {
  const changed = dirty();
  const result = state.result;
  if (!result) {
    readinessEl.textContent = "Draft";
    document.querySelector("#stitch-count").textContent = "None yet";
    document.querySelector("#travel-count").textContent = "None yet";
    document.querySelector("#nominal-time").textContent = "Not generated";
    document.querySelector("#revision").textContent = "Not generated";
    document.querySelector("#preview-note").textContent = "Generate stitches to fill this preview from the Rust engine.";
    notesEl.replaceChildren();
    checksEl.replaceChildren();
    riskEl.replaceChildren();
    return;
  }
  readinessEl.textContent = changed ? "Draft · edits are not generated yet" : result.readiness.label;
  const sewing = sewingStitches();
  const jumps = jumpCount();
  document.querySelector("#stitch-count").textContent = String(sewing);
  document.querySelector("#travel-count").textContent = String(jumps);
  const minutes = sewing / NOMINAL_SPM;
  document.querySelector("#nominal-time").textContent = minutes.toFixed(2) + " min";
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

function setPromptStatus(message) {
  document.querySelector("#prompt-status").textContent = message || "";
}

function onPromptPaste(event) {
  const items = event.clipboardData && event.clipboardData.items;
  if (!items) return;
  for (const item of items) {
    if (item.type === "image/png" || item.type === "image/jpeg") {
      const file = item.getAsFile();
      if (file) {
        event.preventDefault();
        attachFile(file);
      }
      return;
    }
  }
}

function attachFile(file) {
  prepareImage(file).then((url) => {
    state.attachedImage = url;
    const preview = document.querySelector("#prompt-preview");
    const thumb = document.querySelector("#prompt-thumb");
    thumb.src = url;
    preview.hidden = false;
    setPromptStatus("Image attached. It is sent only when you choose Propose design. Audio is never sent.");
  }).catch((error) => {
    clearAttachedImage();
    setPromptStatus(error.message);
    showError(error.message);
  });
}

function clearAttachedImage() {
  state.attachedImage = null;
  const input = document.querySelector("#design-image");
  input.value = "";
  const thumb = document.querySelector("#prompt-thumb");
  thumb.removeAttribute("src");
  document.querySelector("#prompt-preview").hidden = true;
}

function prepareImage(file) {
  const allowed = file.type === "image/png" || file.type === "image/jpeg";
  if (!allowed) {
    return Promise.reject(new Error("Attach a JPEG or PNG image. Other file types are not sent. The design was not changed."));
  }
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error("That image could not be read. Attach a JPEG or PNG and try again."));
    reader.onload = () => {
      const image = new Image();
      image.onload = () => {
        const edge = Math.max(image.width, image.height);
        const scale = Math.min(1, 768 / edge);
        const board = document.createElement("canvas");
        board.width = Math.max(1, Math.round(image.width * scale));
        board.height = Math.max(1, Math.round(image.height * scale));
        const context = board.getContext("2d");
        if (!context) {
          reject(new Error("This browser could not prepare the image. Attach a smaller JPEG or PNG and try again."));
          return;
        }
        context.drawImage(image, 0, 0, board.width, board.height);
        let quality = 0.72;
        let url = board.toDataURL("image/jpeg", quality);
        while (url.length > 700000 && quality > 0.4) {
          quality -= 0.1;
          url = board.toDataURL("image/jpeg", quality);
        }
        if (!url.startsWith("data:image/jpeg;base64,") || url.length > 700000) {
          reject(new Error("That image is still larger than the studio can send. Use a smaller JPEG or PNG and attach it again."));
          return;
        }
        resolve(url);
      };
      image.onerror = () => reject(new Error("That image could not be read. Attach a JPEG or PNG and try again."));
      image.src = String(reader.result || "");
    };
    reader.readAsDataURL(file);
  });
}

function voiceInput() {
  const Speech = window.SpeechRecognition || window.webkitSpeechRecognition;
  if (!Speech) {
    setPromptStatus("Voice input is not available in this browser. Type or paste the prompt instead. Audio is not sent.");
    return;
  }
  if (state.listening && state.recognition) {
    state.recognition.stop();
    return;
  }
  const recognition = new Speech();
  recognition.lang = "en-US";
  recognition.interimResults = false;
  recognition.onresult = (event) => {
    const said = event.results && event.results[0] && event.results[0][0] ? event.results[0][0].transcript : "";
    if (!said) return;
    const box = document.querySelector("#design-prompt");
    box.value = box.value.trim() ? box.value.trim() + " " + said : said;
    setPromptStatus("Voice text is in the prompt. Audio stayed in this browser and was not sent.");
  };
  recognition.onerror = () => {
    setPromptStatus("Voice input stopped. Type or paste the prompt instead. Audio is not sent.");
  };
  recognition.onend = () => {
    state.listening = false;
    state.recognition = null;
    const button = document.querySelector("#voice-input");
    button.setAttribute("aria-pressed", "false");
    button.textContent = "Voice";
  };
  state.recognition = recognition;
  state.listening = true;
  const button = document.querySelector("#voice-input");
  button.setAttribute("aria-pressed", "true");
  button.textContent = "Stop voice";
  setPromptStatus("Listening. Speech becomes prompt text in this browser. Audio is not sent to the model.");
  try {
    recognition.start();
  } catch (error) {
    state.listening = false;
    button.setAttribute("aria-pressed", "false");
    button.textContent = "Voice";
    setPromptStatus("Voice input is not available in this browser. Type or paste the prompt instead. Audio is not sent.");
  }
}

async function askDesign() {
  if (state.busy) return;
  const prompt = document.querySelector("#design-prompt").value.trim();
  if (!prompt && !state.attachedImage) {
    const message = "Enter a prompt or attach an image, then ask again. The design was not changed.";
    setPromptStatus(message);
    showError(message);
    return;
  }
  state.busy = true;
  state.designProposal = null;
  document.querySelector("#design-proposal").hidden = true;
  const ask = document.querySelector("#ask-design");
  ask.textContent = "Proposing…";
  ask.setAttribute("aria-busy", "true");
  setPromptStatus("Asking for a geometry proposal. Stitches are not generated yet.");
  showError("");
  render();
  try {
    const body = await post("/api/propose", {
      prompt,
      image_data_url: state.attachedImage || "",
    }, 100000);
    state.designProposal = body;
    showDesignProposal(body);
    setPromptStatus("Proposal ready. The current design is unchanged until you apply it.");
  } catch (error) {
    state.designProposal = null;
    setPromptStatus(error.message);
    showError(error.message);
  } finally {
    state.busy = false;
    ask.textContent = "Propose design";
    ask.removeAttribute("aria-busy");
    render();
  }
}

function showDesignProposal(body) {
  const panel = document.querySelector("#design-proposal");
  panel.hidden = false;
  document.querySelector("#design-proposal-text").textContent = body.summary || "Geometry proposal ready.";
  const list = document.querySelector("#design-proposal-list");
  list.replaceChildren();
  (body.objects || []).forEach((object) => {
    const item = document.createElement("li");
    item.textContent = object.label + " · " + kindLabel(object.kind) + " · " + object.points + " points";
    list.append(item);
  });
}

function applyDesign(andGenerate) {
  if (!state.designProposal || !state.designProposal.project) {
    showError("Ask for a proposal before applying one.");
    return;
  }
  const project = JSON.parse(JSON.stringify(state.designProposal.project));
  edit(() => {
    state.project = project;
    state.selected = project.objects[0] ? project.objects[0].object_id : null;
  });
  state.designProposal = null;
  document.querySelector("#design-proposal").hidden = true;
  document.querySelector("#design-proposal-list").replaceChildren();
  document.querySelector("#design-proposal-text").textContent = "";
  setPromptStatus("The geometry is now an edit. Generate stitches to update the Rust plan. This proposal did not approve itself.");
  if (andGenerate) generate();
}

function discardDesign() {
  state.designProposal = null;
  document.querySelector("#design-proposal").hidden = true;
  document.querySelector("#design-proposal-list").replaceChildren();
  document.querySelector("#design-proposal-text").textContent = "";
  setPromptStatus("Proposal discarded. The current design was not changed.");
  showError("");
  render();
}

function exploreProposal() {
  const object = selectedObject();
  state.proposal = null;
  if (!object) {
    document.querySelector("#proposal").textContent = "Select an object before asking for a proposal.";
    render();
    return;
  }
  if (object.kind === "fill" && object.density_mm > 1.6) {
    state.proposal = {
      text: "Row spacing is " + object.density_mm + " mm. Applying 1.2 mm uses the same Rust fill with tighter rows. Generate again after applying it.",
      apply: () => { object.density_mm = 1.2; },
    };
  } else if (object.kind === "fill" && !(object.underlay || []).includes("perpendicular_run")) {
    state.proposal = {
      text: "Perpendicular underlay is off. Applying it sets a 2.4 mm underlay spacing and asks the Rust fill to sew that underlay before the rows.",
      apply: () => {
        object.underlay = ["perpendicular_run"];
        object.underlay_spacing_mm = 2.4;
      },
    };
  } else {
    state.proposal = {
      text: "No spacing or underlay change is proposed for this object. Generate stitches to refresh the Rust plan.",
      apply: null,
    };
  }
  document.querySelector("#proposal").textContent = state.proposal.text;
  render();
}

function applyProposal() {
  if (!state.proposal || !state.proposal.apply) {
    showError("Explore a proposal before applying one.");
    return;
  }
  const apply = state.proposal.apply;
  edit(() => apply());
  state.proposal = null;
  document.querySelector("#proposal").textContent = "The proposal is now an edit. Generate stitches to update the Rust plan.";
}

async function post(url, payload, timeoutMs) {
  const limit = timeoutMs || 30000;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), limit);
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
      if (limit > 30000) {
        throw new Error("The proposal did not finish within 100 seconds. Ask again. The design was not changed.");
      }
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
  state.playing = false;
  generateButton.textContent = "Generating stitches…";
  generateButton.setAttribute("aria-busy", "true");
  showError("");
  render();
  try {
    const result = await post("/api/generate", state.project);
    state.result = result;
    state.basis = canonical(state.project);
    state.cursor = result.plan.stitches.length;
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
    showView("approvals");
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
  if (!state.project || dirty() || !state.result || !state.result.readiness.engineering_export_allowed) return;
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

function togglePlay() {
  const total = state.result && state.result.plan ? state.result.plan.stitches.length : 0;
  if (!total) return;
  if (state.playing) {
    state.playing = false;
    render();
    return;
  }
  if (document.documentElement.dataset.motion === "reduce") {
    state.cursor = total;
    render();
    return;
  }
  state.playing = true;
  if (state.cursor == null || state.cursor >= total) state.cursor = 0;
  const step = () => {
    if (!state.playing) return;
    state.cursor = Math.min(total, state.cursor + Math.max(30, Math.floor(total / 70)));
    render();
    if (state.cursor >= total) {
      state.playing = false;
      render();
      return;
    }
    requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
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

function applyRail(collapsed, moveFocus) {
  document.documentElement.dataset.rail = collapsed ? "collapsed" : "open";
  const expanded = collapsed ? "false" : "true";
  const label = collapsed ? "Show sidebar" : "Hide sidebar";
  const toggle = document.querySelector("#rail-toggle");
  toggle.setAttribute("aria-expanded", expanded);
  toggle.setAttribute("aria-label", label);
  toggle.textContent = collapsed ? "⟩" : "⟨";
  const collapse = document.querySelector("#rail-collapse");
  collapse.setAttribute("aria-expanded", expanded);
  collapse.textContent = "Hide sidebar";
  try { localStorage.setItem("embroideryos.rail", collapsed ? "collapsed" : "open"); } catch (error) { /* preference storage is optional */ }
  if (moveFocus && collapsed) toggle.focus();
}

function toggleRail() {
  applyRail(document.documentElement.dataset.rail !== "collapsed", true);
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

function adopt(project) {
  state.project = project;
  state.selected = project.objects[0] ? project.objects[0].object_id : null;
  state.undo = [];
  state.redo = [];
  state.result = null;
  state.basis = null;
  state.notice = "";
  state.cursor = null;
  state.playing = false;
  state.proposal = null;
  showError("");
  showView("studio");
  render();
  generate();
}

async function openLibrary(file) {
  if (unsavedEdits() && !window.confirm("Open another design and discard ungenerated edits?")) return;
  const response = await fetch("/api/projects/" + encodeURIComponent(file));
  const body = await response.json();
  if (!response.ok) {
    showError(body.error || "That library project did not open. Choose a listed design and try again.");
    return;
  }
  adopt(body);
}

async function openSample() {
  if (unsavedEdits() && !window.confirm("Open the club mark sample and discard ungenerated edits?")) return;
  const response = await fetch("/api/sample");
  if (!response.ok) {
    showError("The club mark sample did not load. From the repository root run cargo run -p embroidery-studio, then try again.");
    return;
  }
  adopt(await response.json());
}

async function start() {
  loadPreferences();
  applyRail(document.documentElement.dataset.rail === "collapsed", false);
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applyAppearance);
  showView(location.hash.slice(1) || "studio");
  try {
    const library = await fetch("/api/library");
    if (library.ok) state.library = await library.json();
    const alpine = state.library.find((item) => item.file === "13_alpine_club.json");
    if (alpine) {
      const response = await fetch("/api/projects/" + alpine.file);
      if (response.ok) {
        adopt(await response.json());
        return;
      }
    }
    const response = await fetch("/api/sample");
    if (!response.ok) throw new Error("sample");
    adopt(await response.json());
  } catch (error) {
    showError("The studio server is not running. From the repository root run cargo run -p embroidery-studio, then open http://127.0.0.1:4173. This page does not generate stitches.");
  }
}

start();
