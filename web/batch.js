/* Bounded name drop. Stitches stay in the Rust engine. */
(function () {
  const HOST = document.querySelector("#name-drop");
  const PROFILE = "native-hoop-v1";
  const MAX_ROWS = 100;
  let serial = 0;
  let localSeq = 1;
  let built = false;
  let running = false;
  let stale = false;
  let certifies = false;
  let note = "Add a row or import a UTF-8 CSV with the header item_id,text. Duplicate text is kept when the item ids differ.";
  let snapshotId = null;
  let preparedEpoch = null;
  let templateObjectId = null;
  let storedTemplate = null;
  let placement = null;
  let placementReady = false;
  let selectedLocal = null;
  let rows = [];

  function studio() {
    return window.__studioDesign.snapshot();
  }

  function round1(value) {
    return Math.round(value * 10) / 10;
  }

  function explicitLineCount(content) {
    const text = String(content || "");
    let count = 1;
    for (let index = 0; index < text.length; index += 1) {
      if (text[index] === "\n") count += 1;
      else if (text[index] === "\r" && text[index + 1] === "\n") {
        count += 1;
        index += 1;
      }
    }
    return Math.min(4, Math.max(1, count));
  }

  function suggestedPlacement(project, object) {
    const text = object.text;
    const cap = Number(text.cap_height_mm);
    let top = text.origin_y_mm - cap * 0.75;
    let bottom = text.origin_y_mm + cap * 1.75;
    const layout = text.layout;
    if (layout && Number.isFinite(Number(layout.first_baseline_mm)) && Number.isFinite(Number(layout.baseline_spacing_mm))) {
      const first = Number(layout.first_baseline_mm);
      const last = first + (explicitLineCount(text.content) - 1) * Number(layout.baseline_spacing_mm);
      const scale = cap / 100;
      top = Math.min(first, last) - 120 * scale;
      bottom = Math.max(first, last) + 50 * scale;
    }
    const minX = Math.max(0, round1(text.origin_x_mm - 4));
    const minY = Math.max(0, round1(top));
    let maxX = round1(project.hoop_width_mm - 1);
    let maxY = Math.min(project.hoop_height_mm, bottom);
    maxY = round1(maxY);
    if (maxX < minX + 0.2) maxX = round1(minX + 0.2);
    if (maxY < minY + 0.2) maxY = round1(minY + 0.2);
    return { min_x_mm: minX, min_y_mm: minY, max_x_mm: maxX, max_y_mm: maxY };
  }

  function itemIdOk(value) {
    return /^[A-Za-z0-9][A-Za-z0-9_-]{0,31}$/.test(value);
  }

  function blankRow(itemId, text) {
    localSeq += 1;
    return {
      local_id: localSeq,
      item_id: itemId,
      text: text,
      status: "queued",
      error: "",
      project: null,
      project_revision: null,
      dst_sha256: null,
      dst_hex: null,
      replay: null,
      bundle: null,
      binding: null,
      proof_id: null,
      pdf_base64: null,
      glyph_codes: [],
      font_id: null,
      font_version: null,
      font_sha256: null,
      verifiedText: null,
      artifactCurrent: false,
      proofCurrent: false,
      dstVerifiedReuse: false,
    };
  }

  function selectedRow() {
    return rows.find((row) => row.local_id === selectedLocal) || null;
  }

  function counts() {
    const tally = { queued: 0, running: 0, completed: 0, failed: 0, cancelled: 0 };
    rows.forEach((row) => {
      if (Object.prototype.hasOwnProperty.call(tally, row.status)) tally[row.status] += 1;
    });
    return tally;
  }

  function templateProject() {
    const view = studio();
    if (view.inspection && storedTemplate) return storedTemplate;
    return view.project;
  }

  function textObject(project) {
    if (!project) return null;
    return project.objects.find((object) => object.object_id === (templateObjectId || studio().selected) && object.kind === "text")
      || project.objects.find((object) => object.kind === "text")
      || null;
  }

  function markStale(message) {
    if (stale && note === message) {
      paint();
      return;
    }
    stale = true;
    serial += 1;
    running = false;
    certifies = false;
    snapshotId = null;
    preparedEpoch = null;
    storedTemplate = null;
    templateObjectId = null;
    placementReady = false;
    rows.forEach((row) => {
      row.proofCurrent = false;
      row.artifactCurrent = false;
      if (row.status === "running") {
        row.status = "queued";
        row.error = "This result belonged to an earlier snapshot. It was not applied.";
      }
    });
    note = message;
    paint();
  }

  function dropLate(token, epoch) {
    if (studio().epoch !== epoch) {
      markStale("The open design changed. This name drop is not the current binding. Late results were not applied.");
      return true;
    }
    return token !== serial;
  }

  function onDesign() {
    const view = studio();
    if (preparedEpoch !== null && view.epoch !== preparedEpoch) {
      markStale("The open design changed. This name drop is not the current binding. Late results were not applied.");
    }
  }

  function onItemEdit(itemId) {
    const row = rows.find((entry) => entry.item_id === itemId);
    if (!row) return;
    const view = studio();
    const object = view.project && view.project.objects.find((entry) => entry.object_id === view.selected);
    if (object && object.text) row.text = object.text.content;
    row.proofCurrent = false;
    row.artifactCurrent = false;
    certifies = false;
    row.status = "queued";
    row.error = "This row changed. Generate it again. The previous proof is not current.";
    note = "One row changed. The completion manifest is not current. Other rows stay listed.";
    paint();
  }

  function touchTable(message) {
    serial += 1;
    running = false;
    certifies = false;
    snapshotId = null;
    rows.forEach((row) => {
      row.proofCurrent = false;
    });
    note = message;
    paint();
  }

  function invalidateRow(row) {
    row.proofCurrent = false;
    row.artifactCurrent = false;
    certifies = false;
    row.status = "queued";
    row.error = "This row changed. Generate it again. The previous proof is not current.";
    touchTable("The table changed. Generate it again. Earlier proofs are not current.");
  }

  function replaceRows(entries) {
    rows = entries.map((entry) => blankRow(entry.item_id, entry.text));
    selectedLocal = rows.length ? rows[0].local_id : null;
    const body = HOST.querySelector("tbody");
    if (body) delete body.dataset.key;
    touchTable("The table was replaced. Generate it to bind these rows. Nothing has been certified.");
  }

  function parseCsvRow(line) {
    const comma = line.indexOf(",");
    if (comma < 0) throw new Error("The CSV row needs item_id and text. Nothing was generated.");
    const itemId = line.slice(0, comma);
    if (itemId.includes('"')) throw new Error("The item id must not be quoted. Nothing was generated.");
    let rest = line.slice(comma + 1);
    if (rest.startsWith('"')) {
      let text = "";
      for (let index = 1; index < rest.length; index += 1) {
        const character = rest[index];
        if (character === '"') {
          if (rest[index + 1] === '"') {
            text += '"';
            index += 1;
          } else if (index === rest.length - 1) {
            return { item_id: itemId, text: text };
          } else {
            throw new Error("The quoted text has trailing characters. Nothing was generated.");
          }
        } else {
          text += character;
        }
      }
      throw new Error("The quoted text is missing its closing quote. Nothing was generated.");
    }
    if (rest.includes('"')) throw new Error("Quote the text when it contains a quotation mark. Nothing was generated.");
    return { item_id: itemId, text: rest };
  }

  function csvRecords(text) {
    const records = [];
    let current = "";
    let inQuotes = false;
    let sawComma = false;
    for (let index = 0; index < text.length; index += 1) {
      const character = text[index];
      if (inQuotes) {
        if (character === '"') {
          current += '"';
          if (text[index + 1] === '"') {
            current += '"';
            index += 1;
          } else {
            inQuotes = false;
          }
          continue;
        }
        if (character === "\r") {
          if (text[index + 1] !== "\n") throw new Error("The CSV must use LF or CRLF line breaks. Nothing was generated.");
          current += "\r\n";
          index += 1;
          continue;
        }
        current += character;
        continue;
      }
      if (character === '"') {
        if (!sawComma) throw new Error("The item id must not be quoted. Nothing was generated.");
        inQuotes = true;
        current += '"';
        continue;
      }
      if (character === "\n" || character === "\r") {
        if (character === "\r") {
          if (text[index + 1] !== "\n") throw new Error("The CSV must use LF or CRLF line breaks. Nothing was generated.");
          index += 1;
        }
        records.push(current);
        current = "";
        sawComma = false;
        continue;
      }
      if (character === ",") sawComma = true;
      current += character;
    }
    if (inQuotes) throw new Error("The quoted text is missing its closing quote. Nothing was generated.");
    if (current !== "") records.push(current);
    return records;
  }

  function parseCsv(text) {
    if (text.charCodeAt(0) === 0xfeff) throw new Error("The CSV must be UTF-8 without a byte order mark. Nothing was generated.");
    if (text.includes("\0")) throw new Error("The CSV contains a null character. Nothing was generated.");
    const records = csvRecords(text);
    if (records[0] !== "item_id,text") throw new Error("The CSV header must be item_id,text. Nothing was generated.");
    const parsed = [];
    for (let index = 1; index < records.length; index += 1) {
      if (records[index] === "") throw new Error("CSV row " + (index + 1) + " is empty. Remove the blank line and submit the table again. Nothing was generated.");
      parsed.push(parseCsvRow(records[index]));
    }
    if (!parsed.length) throw new Error("The name table is empty. Add a row and submit it again. Nothing was generated.");
    if (parsed.length > MAX_ROWS) throw new Error("This name drop accepts " + MAX_ROWS + " rows. The table has " + parsed.length + ". Remove rows and submit it again. Nothing was generated.");
    const seen = new Set();
    parsed.forEach((row) => {
      if (!itemIdOk(row.item_id)) throw new Error(row.item_id + " is not a usable item id. Use letters, digits, hyphens, and underscores. Nothing was generated.");
      if (seen.has(row.item_id)) throw new Error(row.item_id + " is repeated. Item ids must be unique. Duplicate text is allowed under different ids. Nothing was generated.");
      seen.add(row.item_id);
    });
    return parsed;
  }

  function readPlacement() {
    const value = (key) => Number(HOST.querySelector("#name-drop-" + key).value);
    return {
      min_x_mm: value("min-x"),
      min_y_mm: value("min-y"),
      max_x_mm: value("max-x"),
      max_y_mm: value("max-y"),
    };
  }

  function placementOk(area) {
    return ["min_x_mm", "min_y_mm", "max_x_mm", "max_y_mm"].every((key) => Number.isFinite(area[key]))
      && area.max_x_mm - area.min_x_mm >= 0.2
      && area.max_y_mm - area.min_y_mm >= 0.2;
  }

  function requestBody() {
    const project = templateProject();
    const object = textObject(project);
    return {
      profile_id: PROFILE,
      object_id: object ? object.object_id : studio().selected,
      hoop_width_mm: project.hoop_width_mm,
      hoop_height_mm: project.hoop_height_mm,
      placement: readPlacement(),
      template: project,
      items: rows.map((row) => ({ item_id: row.item_id, text: row.text })),
    };
  }

  async function postJson(url, payload) {
    const response = await fetch(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(payload),
    });
    let body = {};
    try { body = await response.json(); } catch (error) { body = { error: "The studio returned a response that is not JSON." }; }
    return { status: response.status, body: body };
  }

  function currentEpoch() {
    return studio().epoch === preparedEpoch && !stale;
  }

  function applyItem(row, sentText, result) {
    if (!currentEpoch()) return false;
    if (result.status !== 200) {
      row.status = "failed";
      row.error = (result.body && result.body.error) || "This row was not generated. The template was not changed.";
      row.artifactCurrent = false;
      row.proofCurrent = false;
      row.project = null;
      return true;
    }
    const item = (result.body.items || []).find((entry) => entry.item_id === row.item_id);
    if (!item || item.text !== sentText) {
      row.status = "failed";
      row.error = "The response did not keep this item id and entered text. It was not applied.";
      row.text = sentText;
      row.artifactCurrent = false;
      row.proofCurrent = false;
      return true;
    }
    row.status = item.status;
    row.error = item.error || "";
    row.glyph_codes = item.glyph_codes || [];
    row.font_id = item.font_id || null;
    row.font_version = item.font_version || null;
    row.font_sha256 = item.font_sha256 || null;
    row.proofCurrent = false;
    row.dstVerifiedReuse = item.dst_verified_reuse === true;
    if (item.status === "completed" && item.project && item.dst_sha256 && item.dst_hex && item.machine_release_allowed === false) {
      row.project = item.project;
      row.project_revision = item.project_revision;
      row.dst_sha256 = item.dst_sha256;
      row.dst_hex = item.dst_hex;
      row.replay = item.replay;
      row.verifiedText = item.text;
      row.artifactCurrent = true;
      const content = item.project.objects && item.project.objects.find((object) => object.object_id === templateObjectId);
      if (!content || !content.text || content.text.content !== sentText) {
        row.status = "failed";
        row.error = "The derived project did not keep the entered text. It was not certified.";
        row.artifactCurrent = false;
        row.project = null;
      }
    } else {
      row.artifactCurrent = false;
      row.project = null;
      row.dst_hex = null;
    }
    return true;
  }

  function syncTableFields() {
    const tbody = HOST.querySelector("tbody");
    if (!tbody) return;
    rows.forEach((row, index) => {
      const tr = tbody.children[index];
      if (!tr) return;
      const idInput = tr.querySelector("[data-id]");
      const textInput = tr.querySelector("[data-text]");
      if (idInput) row.item_id = idInput.value;
      if (textInput) row.text = textInput.value;
    });
  }

  async function generateBatch() {
    if (running) return;
    syncTableFields();
    const view = studio();
    if (view.inspection && !storedTemplate) {
      note = "Show the template before generating. The open design is one derived name, and the template binding is gone.";
      paint();
      return;
    }
    const area = readPlacement();
    if (!placementOk(area)) {
      note = "The placement area needs finite edges and at least 0.2 mm in each direction. Nothing was generated.";
      paint();
      return;
    }
    if (!rows.length || rows.length > MAX_ROWS) {
      note = "This name drop accepts 1 to " + MAX_ROWS + " rows. Nothing was generated.";
      paint();
      return;
    }
    const seen = new Set();
    for (const row of rows) {
      if (!itemIdOk(row.item_id)) {
        note = row.item_id + " is not a usable item id. Use letters, digits, hyphens, and underscores. Nothing was generated.";
        paint();
        return;
      }
      if (seen.has(row.item_id)) {
        note = row.item_id + " is repeated. Item ids must be unique. Duplicate text is allowed under different ids. Nothing was generated.";
        paint();
        return;
      }
      seen.add(row.item_id);
    }
    const project = view.inspection ? storedTemplate : JSON.parse(JSON.stringify(view.project));
    const object = textObject(project);
    if (!object || !object.text) {
      note = "Select one text object. Nothing was generated.";
      paint();
      return;
    }
    serial += 1;
    const token = serial;
    const epoch = view.epoch;
    running = true;
    stale = false;
    certifies = false;
    storedTemplate = project;
    templateObjectId = object.object_id;
    preparedEpoch = epoch;
    placement = area;
    placementReady = true;
    rows.forEach((row) => {
      row.status = "queued";
      row.error = "";
      row.proofCurrent = false;
      row.artifactCurrent = false;
    });
    note = "Preparing the name drop. The template is kept.";
    paint();
    const body = requestBody();
    body.template = project;
    body.object_id = object.object_id;
    let prepared;
    try {
      prepared = await postJson("/api/batch/prepare", body);
    } catch (error) {
      if (token !== serial) return;
      running = false;
      note = "The studio did not answer. Check that it is running and try again. Nothing was generated.";
      paint();
      return;
    }
    if (dropLate(token, epoch)) return;
    if (prepared.status !== 200 || !prepared.body.snapshot_id) {
      running = false;
      note = (prepared.body && prepared.body.error) || "The table was not accepted. Nothing was generated.";
      paint();
      return;
    }
    snapshotId = prepared.body.snapshot_id;
    for (const row of rows) {
      if (dropLate(token, epoch)) return;
      row.status = "running";
      note = "Generating " + row.item_id + ". Cancellation takes effect between rows.";
      paint();
      const payload = requestBody();
      payload.template = project;
      payload.object_id = object.object_id;
      payload.item_id = row.item_id;
      payload.expected_snapshot_id = snapshotId;
      payload.include_dst = true;
      if (row.dst_sha256 && row.verifiedText === row.text) payload.presented_dst_sha256 = row.dst_sha256;
      const sent = row.text;
      let result;
      try {
        result = await postJson("/api/batch/item", payload);
      } catch (error) {
        if (token !== serial) return;
        row.status = "failed";
        row.error = "The studio did not answer for this row. Generate it again. The template was not changed.";
        continue;
      }
      if (dropLate(token, epoch)) return;
      applyItem(row, sent, result);
      paint();
    }
    if (dropLate(token, epoch)) return;
    note = "Checking the batch. A failed row blocks the completion manifest.";
    paint();
    let finished;
    try {
      const payload = requestBody();
      payload.template = project;
      payload.object_id = object.object_id;
      payload.expected_snapshot_id = snapshotId;
      finished = await postJson("/api/batch/finish", payload);
    } catch (error) {
      finished = { status: 0, body: { error: "The completion check did not answer. The batch was not certified." } };
    }
    if (dropLate(token, epoch)) return;
    running = false;
    if (finished.status !== 200) {
      certifies = false;
      note = (finished.body && finished.body.error) || "The batch was not certified. Successful rows can still be inspected.";
      paint();
      return;
    }
    certifies = finished.body.certifies_batch === true && finished.body.machine_release_allowed === false;
    const serverItems = finished.body.items || [];
    if (serverItems.length !== rows.length) {
      certifies = false;
      note = "The completion check did not return every row. The batch was not certified.";
      paint();
      return;
    }
    serverItems.forEach((item, index) => {
      if (item.item_id !== rows[index].item_id || item.text !== rows[index].text) {
        certifies = false;
        rows[index].status = "failed";
        rows[index].error = "The completion check did not keep this row's id and text. It was not certified.";
        rows[index].artifactCurrent = false;
      } else if (item.status !== "completed") {
        rows[index].status = item.status;
        rows[index].artifactCurrent = false;
        rows[index].error = item.error || rows[index].error;
      }
    });
    if (rows.some((row) => row.status !== "completed")) certifies = false;
    const tally = counts();
    note = certifies
      ? "Every row has an engineering result. Write a proof for each row before the completion bundle. Machine release remains unavailable."
      : "The batch is incomplete. " + tally.completed + " completed, " + tally.failed + " failed, " + tally.cancelled + " cancelled. Successful rows can still be inspected.";
    paint();
  }

  async function cancelBatch() {
    serial += 1;
    const token = serial;
    running = false;
    certifies = false;
    rows.forEach((row) => {
      if (row.status === "queued" || row.status === "running") {
        row.status = "cancelled";
        row.error = "This row was cancelled before generation. It is not part of a completion manifest.";
        row.artifactCurrent = false;
        row.proofCurrent = false;
      }
    });
    note = "Cancellation was requested. Completed rows can still be inspected. No completion manifest was issued.";
    paint();
    if (!snapshotId || !storedTemplate) return;
    try {
      const payload = requestBody();
      payload.template = storedTemplate;
      payload.expected_snapshot_id = snapshotId;
      const cancelled = await postJson("/api/batch/cancel", payload);
      if (token !== serial) return;
      if (cancelled.body && cancelled.body.certifies_batch === true) {
        certifies = false;
        note = "The server reported a completion after cancellation. That result was not accepted.";
      }
    } catch (error) {
      if (token === serial) note = "The cancellation notice did not reach the studio. This page still has no completion manifest.";
    }
    paint();
  }

  function bundleReady() {
    if (!certifies || stale || !snapshotId || studio().epoch !== preparedEpoch) return false;
    return rows.length > 0 && rows.every((row) => row.status === "completed"
      && row.artifactCurrent
      && row.proofCurrent
      && row.binding
      && row.binding.snapshot_id === snapshotId
      && row.binding.item_id === row.item_id
      && row.binding.dst_sha256 === row.dst_sha256
      && row.binding.project_revision === row.project_revision
      && row.binding.machine_release_allowed === false);
  }

  async function sha256Hex(bytes) {
    const digest = await crypto.subtle.digest("SHA-256", bytes);
    return Array.from(new Uint8Array(digest)).map((byte) => byte.toString(16).padStart(2, "0")).join("");
  }

  function hexToBytes(hex) {
    const bytes = new Uint8Array(hex.length / 2);
    for (let index = 0; index < bytes.length; index += 1) bytes[index] = Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16);
    return bytes;
  }

  function base64ToBytes(text) {
    const binary = atob(text);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
    return bytes;
  }

  function downloadBlob(filename, bytes, type) {
    const link = document.createElement("a");
    link.href = URL.createObjectURL(new Blob([bytes], { type: type }));
    link.download = filename;
    link.click();
    URL.revokeObjectURL(link.href);
  }

  async function downloadDst(row) {
    if (!itemIdOk(row.item_id) || !row.artifactCurrent || !row.dst_hex) return;
    const bytes = hexToBytes(row.dst_hex);
    const hash = await sha256Hex(bytes);
    if (hash !== row.dst_sha256) {
      note = "The DST bytes do not match the recorded hash. The download was refused.";
      paint();
      return;
    }
    downloadBlob(row.item_id + ".dst", bytes, "application/octet-stream");
  }

  async function downloadPdf(row) {
    if (!row.proofCurrent || !row.pdf_base64 || !row.binding) return;
    const bytes = base64ToBytes(row.pdf_base64);
    const hash = await sha256Hex(bytes);
    if (hash !== row.binding.pdf_sha256) {
      note = "The PDF bytes do not match the proof binding. The download was refused.";
      row.proofCurrent = false;
      certifies = false;
      paint();
      return;
    }
    downloadBlob(row.item_id + ".proof.pdf", bytes, "application/pdf");
  }

  async function prove(row) {
    if (!row.artifactCurrent || !snapshotId || running) return;
    const token = serial;
    const epoch = preparedEpoch;
    const sent = row.text;
    note = "Writing the proof for " + row.item_id + ". It uses the constructed 14 mm reference, not the artwork.";
    paint();
    const payload = requestBody();
    payload.template = storedTemplate;
    payload.item_id = row.item_id;
    payload.expected_snapshot_id = snapshotId;
    payload.scale_confirmed = true;
    payload.source_width_mm = 14;
    payload.source_x_mm = 0;
    payload.source_y_mm = 0;
    const proved = await postJson("/api/batch/proof", payload);
    if (dropLate(token, epoch)) return;
    if (token !== serial || snapshotId !== payload.expected_snapshot_id) return;
    const binding = proved.body && proved.body.binding;
    if (proved.status !== 200 || !binding || binding.item_id !== row.item_id || binding.snapshot_id !== snapshotId || binding.dst_sha256 !== row.dst_sha256 || binding.project_revision !== row.project_revision || binding.machine_release_allowed !== false || proved.body.proof_current !== false) {
      row.proofCurrent = false;
      note = (proved.body && proved.body.error) || "The proof was not bound to this row. It was not marked current.";
      paint();
      return;
    }
    if (row.text !== sent) {
      note = "The row text changed while the proof was written. That proof is not current.";
      paint();
      return;
    }
    row.binding = binding;
    row.proof_id = proved.body.proof_id;
    row.pdf_base64 = proved.body.pdf_base64;
    row.bundle = proved.body.bundle;
    row.proofCurrent = true;
    note = "Proof " + row.proof_id.slice(0, 12) + " is current for " + row.item_id + " only. Machine release remains unavailable.";
    paint();
  }

  function downloadBundle() {
    if (!bundleReady()) return;
    const manifest = {
      schema: "embroideryos-name-drop-batch/0.1",
      certifies_batch: true,
      machine_release_allowed: false,
      physical_qualification: false,
      snapshot_id: snapshotId,
      profile_id: PROFILE,
      object_id: templateObjectId,
      placement: placement,
      counts: counts(),
      items: rows.map((row) => ({
        item_id: row.item_id,
        text: row.text,
        project: row.project,
        project_revision: row.project_revision,
        dst_sha256: row.dst_sha256,
        dst_hex: row.dst_hex,
        replay: row.replay,
        proof_id: row.proof_id,
        binding: row.binding,
        pdf_base64: row.pdf_base64,
        glyph_codes: row.glyph_codes,
        font_id: row.font_id,
        font_version: row.font_version,
        font_sha256: row.font_sha256,
      })),
    };
    downloadBlob("name-drop-batch.json", new TextEncoder().encode(JSON.stringify(manifest, null, 2)), "application/json");
  }

  function loadExample() {
    if (running) return;
    replaceRows([
      { item_id: "ITEM-001", text: "ALPINE 08" },
      { item_id: "ITEM-002", text: "ALPINE 09" },
      { item_id: "ITEM-003", text: "O'NEIL-08" },
      { item_id: "ITEM-004", text: "ALPINE 08" },
    ]);
    note = "Example loaded. ITEM-001 and ITEM-004 both say ALPINE 08. Generate to keep both.";
    paint();
  }

  function ensureBuilt() {
    if (built) return;
    built = true;
    HOST.innerHTML = [
      "<h2>Name drop</h2>",
      "<p class=\"hint\">One text object and profile native-hoop-v1. Block 01 version 1 stays pinned until version 2 is chosen. Names that sew outside the placement area are rejected. Lettering size is not reduced. Machine release stays unavailable.</p>",
      "<p id=\"name-drop-font\" class=\"hint\"></p>",
      "<label>Font version <select id=\"name-drop-font-version\"></select></label>",
      "<div class=\"placement\">",
      "<label>Min X · mm<input id=\"name-drop-min-x\" type=\"number\" step=\"0.1\"></label>",
      "<label>Min Y · mm<input id=\"name-drop-min-y\" type=\"number\" step=\"0.1\"></label>",
      "<label>Max X · mm<input id=\"name-drop-max-x\" type=\"number\" step=\"0.1\"></label>",
      "<label>Max Y · mm<input id=\"name-drop-max-y\" type=\"number\" step=\"0.1\"></label>",
      "</div>",
      "<p id=\"name-drop-counts\" class=\"counts\"></p>",
      "<p id=\"name-drop-message\" role=\"status\" aria-live=\"polite\"></p>",
      "<div class=\"row\">",
      "<button type=\"button\" id=\"name-drop-add\">Add row</button>",
      "<button type=\"button\" id=\"name-drop-example\">Load example</button>",
      "<label class=\"file-button\" for=\"name-drop-csv\">Import CSV<input id=\"name-drop-csv\" type=\"file\" accept=\".csv,text/csv,text/plain\"></label>",
      "<button type=\"button\" id=\"name-drop-generate\" class=\"primary\">Generate names</button>",
      "<button type=\"button\" id=\"name-drop-cancel\">Cancel</button>",
      "<button type=\"button\" id=\"name-drop-template\">Show template</button>",
      "</div>",
      "<div class=\"table-wrap\"><table class=\"name-table\"><caption class=\"hint\">Item id is the file name. The text is sewn as entered.</caption><thead><tr><th scope=\"col\">Show</th><th scope=\"col\">Item id</th><th scope=\"col\">Text</th><th scope=\"col\">Status</th><th scope=\"col\">Remove</th></tr></thead><tbody></tbody></table></div>",
      "<p id=\"name-drop-proof\" class=\"hint\"></p>",
      "<pre id=\"name-drop-glyphs\" class=\"hint\"></pre>",
      "<div class=\"row\">",
      "<button type=\"button\" id=\"name-drop-project\">Download project</button>",
      "<button type=\"button\" id=\"name-drop-dst\">Download DST</button>",
      "<button type=\"button\" id=\"name-drop-replay\">Download replay</button>",
      "<button type=\"button\" id=\"name-drop-write\">Write PDF proof</button>",
      "<button type=\"button\" id=\"name-drop-pdf\">Download PDF</button>",
      "<button type=\"button\" id=\"name-drop-bundle\">Download complete bundle</button>",
      "</div>",
    ].join("");
    HOST.querySelector("#name-drop-add").addEventListener("click", () => {
      if (running || rows.length >= MAX_ROWS) return;
      const used = new Set(rows.map((entry) => entry.item_id));
      let number = rows.length + 1;
      let itemId = "ITEM-" + String(number).padStart(3, "0");
      while (used.has(itemId) && number < 1000) {
        number += 1;
        itemId = "ITEM-" + String(number).padStart(3, "0");
      }
      const row = blankRow(itemId, "");
      rows.push(row);
      selectedLocal = row.local_id;
      const body = HOST.querySelector("tbody");
      if (body) delete body.dataset.key;
      invalidateRow(row);
    });
    HOST.querySelector("#name-drop-example").addEventListener("click", loadExample);
    HOST.querySelector("#name-drop-font-version").addEventListener("change", () => {
      const select = HOST.querySelector("#name-drop-font-version");
      if (running || studio().inspection) {
        note = "Return to the template before changing the font. A derived row keeps the font it was generated with.";
        paint();
        return;
      }
      const kept = placement ? {
        min_x_mm: placement.min_x_mm,
        min_y_mm: placement.min_y_mm,
        max_x_mm: placement.max_x_mm,
        max_y_mm: placement.max_y_mm,
      } : null;
      const pin = (studio().fonts || []).find((font) => font.version === select.value);
      if (!pin || !window.__studioDesign.applyFontPin(pin)) {
        note = "That font version was not applied. The open design was not changed.";
        paint();
        return;
      }
      if (kept) {
        placement = kept;
        placementReady = true;
      }
      select.blur();
      note = "Font version " + pin.version + " is an explicit edit. Previous name-drop results are not current. The entered names were not changed.";
      paint();
    });
    HOST.querySelector("#name-drop-csv").addEventListener("change", async (event) => {
      const file = event.target.files && event.target.files[0];
      event.target.value = "";
      if (!file || running) return;
      try {
        replaceRows(parseCsv(await file.text()));
        note = "CSV imported. " + rows.length + " rows kept, including duplicate text under different ids. Generate to bind them.";
      } catch (error) {
        note = error.message;
      }
      paint();
    });
    HOST.querySelector("#name-drop-generate").addEventListener("click", () => { generateBatch().catch((error) => { note = error.message; running = false; paint(); }); });
    HOST.querySelector("#name-drop-cancel").addEventListener("click", () => { cancelBatch().catch((error) => { note = error.message; paint(); }); });
    HOST.querySelector("#name-drop-template").addEventListener("click", () => {
      if (!storedTemplate || !templateObjectId) {
        note = "Generate once before returning to the template.";
        paint();
        return;
      }
      window.__studioDesign.showTemplate(JSON.parse(JSON.stringify(storedTemplate)), templateObjectId);
    });
    ["min-x", "min-y", "max-x", "max-y"].forEach((key) => {
      HOST.querySelector("#name-drop-" + key).addEventListener("change", () => {
        placement = readPlacement();
        placementReady = true;
        rows.forEach((row) => {
          row.artifactCurrent = false;
          row.proofCurrent = false;
          if (row.status === "completed" || row.status === "failed") row.status = "queued";
        });
        touchTable("The placement area changed. Names that do not fit are rejected. Lettering size was not changed.");
      });
    });
    HOST.querySelector("tbody").addEventListener("keydown", (event) => {
      if (event.key !== "Enter") return;
      const text = event.target.closest("[data-text]");
      if (!text) return;
      event.stopPropagation();
      if (!(event.metaKey || event.ctrlKey)) return;
      event.preventDefault();
      const tr = text.closest("tr");
      const row = rows.find((entry) => String(entry.local_id) === tr.dataset.local);
      if (!row || running) return;
      row.text = text.value;
      HOST.querySelector("#name-drop-generate").click();
    });
    HOST.querySelector("tbody").addEventListener("change", (event) => {
      const tr = event.target.closest("tr");
      if (!tr) return;
      const row = rows.find((entry) => String(entry.local_id) === tr.dataset.local);
      if (!row || running) return;
      if (event.target.matches("[data-id]")) {
        row.item_id = event.target.value;
        invalidateRow(row);
      } else if (event.target.matches("[data-text]")) {
        row.text = event.target.value;
        const view = studio();
        if (view.inspection === row.item_id) window.__studioDesign.applyInspectionText(row.text);
        invalidateRow(row);
      } else if (event.target.matches("[data-select]")) {
        selectRow(row);
      }
    });
    HOST.querySelector("tbody").addEventListener("click", (event) => {
      const radio = event.target.closest("[data-select]");
      if (radio) {
        const picked = radio.closest("tr");
        const chosen = picked && rows.find((entry) => String(entry.local_id) === picked.dataset.local);
        if (chosen && !running) selectRow(chosen);
        return;
      }
      const button = event.target.closest("[data-remove]");
      if (!button || running) return;
      const local = Number(button.dataset.remove);
      rows = rows.filter((row) => row.local_id !== local);
      if (selectedLocal === local) selectedLocal = rows.length ? rows[0].local_id : null;
      const body = HOST.querySelector("tbody");
      if (body) delete body.dataset.key;
      touchTable("A row was removed. Generate the remaining table. Removed names are not silently skipped in a completion bundle.");
    });
    HOST.querySelector("#name-drop-project").addEventListener("click", () => {
      const row = selectedRow();
      if (!row || !row.artifactCurrent || !row.project || !itemIdOk(row.item_id)) return;
      downloadBlob(row.item_id + ".embroidery-project.json", new TextEncoder().encode(JSON.stringify(row.project, null, 2)), "application/json");
    });
    HOST.querySelector("#name-drop-dst").addEventListener("click", () => {
      const row = selectedRow();
      if (row) downloadDst(row);
    });
    HOST.querySelector("#name-drop-replay").addEventListener("click", () => {
      const row = selectedRow();
      if (!row || !row.proofCurrent || !row.bundle || !itemIdOk(row.item_id)) return;
      downloadBlob(row.item_id + ".replay.json", new TextEncoder().encode(JSON.stringify(row.bundle, null, 2)), "application/json");
    });
    HOST.querySelector("#name-drop-write").addEventListener("click", () => {
      const row = selectedRow();
      if (row) prove(row).catch((error) => { note = error.message; paint(); });
    });
    HOST.querySelector("#name-drop-pdf").addEventListener("click", () => {
      const row = selectedRow();
      if (row) downloadPdf(row);
    });
    HOST.querySelector("#name-drop-bundle").addEventListener("click", downloadBundle);
  }

  function selectRow(row) {
    if (selectedLocal === row.local_id && studio().inspection === row.item_id) return;
    selectedLocal = row.local_id;
    if (row.proofCurrent && row.proof_id) note = "Proof " + row.proof_id.slice(0, 12) + " is current for " + row.item_id + " only. Machine release remains unavailable.";
    else if (row.artifactCurrent) note = row.item_id + " is showing its engineering result. Its proof is not current.";
    else note = row.item_id + " is selected. " + (row.error || "It has no current export.");
    paint();
    if (row.artifactCurrent && row.project && templateObjectId) {
      window.__studioDesign.openDerived(JSON.parse(JSON.stringify(row.project)), templateObjectId, row.item_id);
    }
  }

  function paint() {
    if (!built) return;
    const view = studio();
    const project = templateProject();
    const object = textObject(project);
    if (!placementReady && project && object && object.text) {
      placement = suggestedPlacement(project, object);
      placementReady = true;
    }
    if (placement) {
      [["min-x", placement.min_x_mm], ["min-y", placement.min_y_mm], ["max-x", placement.max_x_mm], ["max-y", placement.max_y_mm]].forEach((pair) => {
        const input = HOST.querySelector("#name-drop-" + pair[0]);
        if (document.activeElement !== input) input.value = String(pair[1]);
      });
    }
    const font = object && object.text;
    HOST.querySelector("#name-drop-font").textContent = font
      ? "Font " + font.font_id + " version " + font.font_version + ". Content " + font.font_sha256 + ". Profile " + PROFILE + ". Hoop " + project.hoop_width_mm + " × " + project.hoop_height_mm + " mm."
      : "Select a text object. Version 1 stays in place until version 2 is chosen.";
    const select = HOST.querySelector("#name-drop-font-version");
    const fonts = studio().fonts || [];
    const fontKey = fonts.map((entry) => entry.version + entry.sha256).join("|");
    if (select.dataset.key !== fontKey) {
      select.dataset.key = fontKey;
      select.replaceChildren();
      fonts.forEach((entry) => {
        const option = document.createElement("option");
        option.value = entry.version;
        option.textContent = "Block 01 version " + entry.version;
        select.append(option);
      });
    }
    if (document.activeElement !== select && font) select.value = font.font_version;
    select.disabled = running || !font || !!view.inspection;
    const tally = counts();
    HOST.querySelector("#name-drop-counts").textContent = "Queued " + tally.queued + " · Running " + tally.running + " · Completed " + tally.completed + " · Failed " + tally.failed + " · Cancelled " + tally.cancelled;
    HOST.querySelector("#name-drop-message").textContent = note;
    const tbody = HOST.querySelector("tbody");
    const key = rows.map((row) => row.local_id).join(",");
    if (tbody.dataset.key !== key) {
      tbody.dataset.key = key;
      tbody.replaceChildren();
      rows.forEach((row) => {
        const tr = document.createElement("tr");
        tr.dataset.local = String(row.local_id);
        tr.innerHTML = "<td><input type=\"radio\" name=\"name-drop-row\" data-select></td><td><input data-id type=\"text\" maxlength=\"32\" autocomplete=\"off\" spellcheck=\"false\"></td><td><textarea data-text rows=\"2\" autocomplete=\"off\" spellcheck=\"false\"></textarea></td><td class=\"status\" data-status></td><td><button type=\"button\" data-remove>Remove</button></td>";
        tr.querySelector("[data-remove]").dataset.remove = String(row.local_id);
        tbody.append(tr);
      });
    }
    rows.forEach((row, index) => {
      const tr = tbody.children[index];
      const idInput = tr.querySelector("[data-id]");
      const textInput = tr.querySelector("[data-text]");
      const radio = tr.querySelector("[data-select]");
      if (document.activeElement !== idInput) idInput.value = row.item_id;
      if (document.activeElement !== textInput) textInput.value = row.text;
      idInput.setAttribute("aria-label", "Item id " + (index + 1));
      textInput.setAttribute("aria-label", "Text for " + row.item_id);
      radio.setAttribute("aria-label", "Show " + row.item_id);
      radio.checked = row.local_id === selectedLocal;
      const detail = row.error ? row.status + " — " + row.error : row.status;
      tr.querySelector("[data-status]").textContent = detail;
    });
    const row = selectedRow();
    const proof = HOST.querySelector("#name-drop-proof");
    if (!row) proof.textContent = "Select a row to inspect its project, DST, replay, and proof.";
    else if (row.proofCurrent) proof.textContent = "Current proof for " + row.item_id + " only. Proof " + row.proof_id + ". DST " + row.dst_sha256 + ". Machine release remains unavailable.";
    else if (row.artifactCurrent) proof.textContent = row.item_id + " has an engineering DST " + row.dst_sha256 + ". Its proof is not current.";
    else proof.textContent = row.item_id + " has no current export. " + (row.error || "Generate this row.");
    HOST.querySelector("#name-drop-glyphs").textContent = row && row.glyph_codes.length
      ? row.glyph_codes.join("\n")
      : "";
    const ready = !!(row && row.artifactCurrent && snapshotId && !stale && view.epoch === preparedEpoch);
    HOST.querySelector("#name-drop-project").disabled = !ready;
    HOST.querySelector("#name-drop-dst").disabled = !ready;
    HOST.querySelector("#name-drop-write").disabled = !ready || running;
    HOST.querySelector("#name-drop-replay").disabled = !(row && row.proofCurrent);
    HOST.querySelector("#name-drop-pdf").disabled = !(row && row.proofCurrent);
    HOST.querySelector("#name-drop-bundle").disabled = !bundleReady();
    HOST.querySelector("#name-drop-generate").disabled = running;
    HOST.querySelector("#name-drop-generate").setAttribute("aria-busy", running ? "true" : "false");
    HOST.querySelector("#name-drop-cancel").disabled = !running;
    HOST.querySelector("#name-drop-add").disabled = running || rows.length >= MAX_ROWS;
    HOST.querySelector("#name-drop-example").disabled = running;
  }

  function sync() {
    onDesign();
    if (HOST.hidden) return;
    ensureBuilt();
    paint();
  }

  function currentHandoff() {
    if (stale || !snapshotId) return null;
    const view = studio();
    if (view.epoch !== preparedEpoch) return null;
    const row = selectedRow();
    if (!row || !row.proofCurrent || !row.bundle || !row.binding) return null;
    const bundle = row.bundle;
    const binding = row.binding;
    const batch = bundle.batch;
    const events = bundle.trace && bundle.trace.events;
    if (bundle.protocol !== "embroidery-project-replay/1") return null;
    if (!bundle.kernel || bundle.kernel.mode !== "native-rust") return null;
    if (!batch || batch.item_id !== row.item_id || batch.snapshot_id !== snapshotId) return null;
    if (batch.project_revision !== row.project_revision || typeof batch.text !== "string") return null;
    if (!bundle.provenance || bundle.provenance.dst_sha256 !== row.dst_sha256) return null;
    if (!Array.isArray(events) || !events.length || bundle.trace.record_count !== events.length) return null;
    if (!events[events.length - 1] || events[events.length - 1].kind !== "end") return null;
    if (binding.schema !== "embroideryos-name-drop-binding/0.1") return null;
    if (binding.snapshot_id !== snapshotId || binding.item_id !== row.item_id) return null;
    if (binding.dst_sha256 !== row.dst_sha256 || binding.project_revision !== row.project_revision) return null;
    if (binding.machine_release_allowed !== false || binding.physical_qualification !== false) return null;
    if (!/^[a-f0-9]{64}$/.test(binding.pdf_sha256 || "") || !/^[a-f0-9]{64}$/.test(row.dst_sha256 || "")) return null;
    return {
      replay: bundle,
      binding: binding,
      item_id: row.item_id,
      project_revision: row.project_revision,
      dst_sha256: row.dst_sha256,
      pdf_sha256: binding.pdf_sha256,
    };
  }

  window.__nameDrop = {
    sync: sync,
    onDesign: onDesign,
    onItemEdit: onItemEdit,
    keepPlacement: function () {
      return placement ? {
        min_x_mm: placement.min_x_mm,
        min_y_mm: placement.min_y_mm,
        max_x_mm: placement.max_x_mm,
        max_y_mm: placement.max_y_mm,
      } : null;
    },
    restorePlacement: function (kept) {
      if (!kept) return;
      placement = {
        min_x_mm: kept.min_x_mm,
        min_y_mm: kept.min_y_mm,
        max_x_mm: kept.max_x_mm,
        max_y_mm: kept.max_y_mm,
      };
      placementReady = true;
      paint();
    },
    snapshot: function () {
      return {
        stale: stale,
        certifies: certifies,
        snapshotId: snapshotId,
        running: running,
        rows: rows.map((row) => ({
          item_id: row.item_id,
          text: row.text,
          status: row.status,
          error: row.error,
          artifactCurrent: row.artifactCurrent,
          proofCurrent: row.proofCurrent,
          dst_sha256: row.dst_sha256,
          proof_id: row.proof_id,
          project_revision: row.project_revision,
          font_sha256: row.font_sha256,
          font_version: row.font_version,
          glyph_codes: row.glyph_codes,
        })),
        selected: selectedRow() ? selectedRow().item_id : null,
        proofText: HOST.querySelector("#name-drop-proof") ? HOST.querySelector("#name-drop-proof").textContent : "",
        counts: counts(),
      };
    },
    currentHandoff: currentHandoff,
  };
  sync();
})();
