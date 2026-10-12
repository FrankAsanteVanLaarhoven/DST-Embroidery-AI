/* Explicit horizontal lines for one text object. Stitches stay in the Rust engine. */
(function () {
  const POLICY = "embroideryos-line-layout/1";

  function round3(value) {
    return Math.round(Number(value) * 1000) / 1000;
  }

  function report(message) {
    const errors = document.querySelector("#errors");
    if (errors) errors.textContent = message || "";
  }

  function commit(edit, apply) {
    const drop = window.__nameDrop;
    const kept = drop && drop.keepPlacement ? drop.keepPlacement() : null;
    edit(apply);
    if (kept && drop.restorePlacement) drop.restorePlacement(kept);
  }

  function numberControl(label, value, id, valid, problem, apply, edit) {
    const wrap = document.createElement("label");
    wrap.append(document.createTextNode(label));
    const input = document.createElement("input");
    input.type = "number";
    input.step = "0.1";
    input.id = id;
    input.value = String(round3(value));
    wrap.htmlFor = id;
    let committed = round3(value);
    input.addEventListener("change", () => {
      const next = Number(input.value);
      if (!valid(next)) {
        report(problem);
        input.value = String(committed);
        return;
      }
      report("");
      if (Object.is(next, committed)) return;
      committed = next;
      commit(edit, apply(next));
    });
    wrap.append(input);
    return wrap;
  }

  window.letteringInspect = function (object, fieldsEl, edit) {
    const text = object.text;
    const wrap = document.createElement("label");
    wrap.append(document.createTextNode("Text"));
    const input = document.createElement("textarea");
    input.id = "text-content";
    input.rows = 4;
    input.value = text.content;
    input.setAttribute("aria-label", "Lettering text");
    let committed = text.content;
    input.addEventListener("change", () => {
      if (input.value === committed) return;
      committed = input.value;
      commit(edit, () => { object.text.content = input.value; });
    });
    input.addEventListener("keydown", (event) => {
      if (event.key !== "Enter") return;
      event.stopPropagation();
      if (!(event.metaKey || event.ctrlKey)) return;
      event.preventDefault();
      commit(edit, () => { object.text.content = input.value; });
      const button = document.querySelector("#generate");
      if (button) button.click();
    });
    wrap.append(input);
    fieldsEl.append(wrap);

    const mode = document.createElement("label");
    mode.className = "choice";
    const toggle = document.createElement("input");
    toggle.type = "checkbox";
    toggle.id = "line-layout";
    toggle.checked = !!text.layout;
    mode.append(toggle, document.createTextNode(" Horizontal lines"));
    toggle.addEventListener("change", () => {
      commit(edit, () => {
        if (!toggle.checked) {
          delete object.text.layout;
          return;
        }
        const cap = Number(object.text.cap_height_mm) || 8;
        object.text.layout = {
          policy: POLICY,
          alignment: "left",
          block_width_mm: 60,
          first_baseline_mm: round3(Number(object.text.origin_y_mm) + 0.92 * cap),
          baseline_spacing_mm: round3(cap * 1.6),
        };
      });
    });
    fieldsEl.append(mode);

    if (text.layout) {
      const alignment = document.createElement("label");
      alignment.append(document.createTextNode("Alignment"));
      const select = document.createElement("select");
      select.id = "line-alignment";
      alignment.htmlFor = "line-alignment";
      ["left", "centre", "right"].forEach((name) => {
        const option = document.createElement("option");
        option.value = name;
        option.textContent = name;
        select.append(option);
      });
      select.value = ["left", "centre", "right"].includes(text.layout.alignment) ? text.layout.alignment : "left";
      select.addEventListener("change", () => {
        commit(edit, () => { object.text.layout.alignment = select.value; });
      });
      alignment.append(select);
      fieldsEl.append(alignment);
      fieldsEl.append(numberControl(
        "Block width · mm",
        text.layout.block_width_mm,
        "line-block-width",
        (value) => value > 0 && value <= 10000,
        "Block width must be greater than 0 mm and at most 10000 mm. Text is not wrapped or shrunk. The design was not changed.",
        (value) => () => { object.text.layout.block_width_mm = value; },
        edit,
      ));
      fieldsEl.append(numberControl(
        "First baseline · mm",
        text.layout.first_baseline_mm,
        "line-baseline",
        (value) => Number.isFinite(value) && Math.abs(value) <= 10000,
        "The first baseline must be from -10000 mm to 10000 mm. The design was not changed.",
        (value) => () => { object.text.layout.first_baseline_mm = value; },
        edit,
      ));
      fieldsEl.append(numberControl(
        "Baseline spacing · mm",
        text.layout.baseline_spacing_mm,
        "line-spacing",
        (value) => Number.isFinite(value) && Math.abs(value) <= 1000,
        "Baseline spacing must be from -1000 mm to 1000 mm. Increase it when lines overlap. This is not a thread-collision measurement. The design was not changed.",
        (value) => () => { object.text.layout.baseline_spacing_mm = value; },
        edit,
      ));
    }

    const note = document.createElement("p");
    note.className = "hint";
    note.id = "lettering-note";
    note.textContent = "Enter inserts a line. Ctrl or Command Enter generates. One to four horizontal lines use alignment and baseline spacing. Cap height is a software size, not the full line height. Version 2 is an explicit edit. Unsupported text is rejected.";
    fieldsEl.append(note);
  };
})();
