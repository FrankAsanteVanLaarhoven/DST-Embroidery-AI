(function () {
  var panel = document.getElementById("creative-panel");
  var form = document.getElementById("creative-form");
  if (!panel || !form) return;
  var requestField = document.getElementById("creative-request");
  var profileField = document.getElementById("creative-profile");
  var widthField = document.getElementById("creative-width");
  var heightField = document.getElementById("creative-height");
  var referenceField = document.getElementById("creative-reference");
  var transmitField = document.getElementById("creative-transmit");
  var transmitLabel = document.getElementById("creative-transmit-label");
  var policyLine = document.getElementById("creative-policy");
  var statusLine = document.getElementById("creative-status");
  var errorLine = document.getElementById("creative-error");
  var preview = document.getElementById("creative-preview");
  var image = document.getElementById("creative-image");
  var caption = document.getElementById("creative-caption");
  var generateButton = document.getElementById("creative-generate");
  var cancelButton = document.getElementById("creative-cancel");
  var selectButton = document.getElementById("creative-select");
  var downloadLink = document.getElementById("creative-download");
  var reopenButton = document.getElementById("creative-reopen");
  var shell = document.querySelector(".app-shell");
  var previousTitle = document.title;
  var policy = null;
  var job = null;
  var busy = false;
  var pollTimer = 0;

  function revisionToken() {
    var studio = window.__studioDesign;
    if (!studio || !studio.snapshot) return Promise.resolve("no-embroidery-project");
    var snap = studio.snapshot();
    if (!snap || !snap.project) return Promise.resolve("no-embroidery-project");
    var bytes = new TextEncoder().encode(JSON.stringify(snap.project) + ":" + String(snap.epoch));
    return crypto.subtle.digest("SHA-256", bytes).then(function (digest) {
      var hex = Array.from(new Uint8Array(digest), function (byte) {
        return byte.toString(16).padStart(2, "0");
      }).join("");
      return String(snap.epoch) + "-" + hex.slice(0, 32);
    });
  }

  function setError(text) {
    errorLine.textContent = text || "";
  }

  function setStatus(text) {
    statusLine.textContent = text || "";
  }

  function localTest(value) {
    return value && value.provider && value.provider.live === false && value.provider.id === "test_image";
  }

  function showCandidate(value) {
    var artifact = value.artifacts && value.artifacts[0];
    if (!artifact) {
      preview.hidden = true;
      image.removeAttribute("src");
      image.alt = "";
      selectButton.disabled = true;
      downloadLink.hidden = true;
      reopenButton.disabled = true;
      return;
    }
    var requestText = value.exact_request || "artwork";
    image.alt = "Raster artwork candidate: " + requestText.slice(0, 120);
    image.src = "/api/creative/artifacts/" + artifact.artifact_id + "?t=" + Date.now();
    preview.hidden = false;
    var kind = localTest(value)
      ? "Local test raster. Not a live provider image."
      : "Stored raster from the configured image provider.";
    caption.textContent = kind
      + " " + artifact.width + " × " + artifact.height + " px. "
      + (artifact.has_alpha ? "Alpha channel present." : "No alpha channel in the file.")
      + " SHA-256 " + artifact.sha256
      + ". Raster artwork, not editable vector geometry, layered art, or a stitch plan.";
    selectButton.disabled = value.application !== "candidate" && value.application !== "selected" && value.application !== "applied";
    downloadLink.hidden = false;
    downloadLink.href = "/api/creative/artifacts/" + artifact.artifact_id;
    downloadLink.download = "artwork-master" + (artifact.media_type === "image/jpeg" ? ".jpg" : ".png");
    reopenButton.disabled = false;
  }

  function applyJob(value) {
    job = value;
    var running = value.execution === "queued" || value.execution === "submitted";
    generateButton.disabled = busy || running;
    cancelButton.hidden = !running;
    panel.setAttribute("aria-busy", running ? "true" : "false");
    if (running) {
      setStatus(localTest(value) ? "Generating a local test raster…" : "Generating artwork…");
      setError("");
      return;
    }
    if (value.execution === "succeeded") {
      var saved = value.application === "selected" || value.application === "applied"
        ? " Artwork master saved. The open embroidery design was not changed."
        : " Candidate only. Save it as an artwork master if you want to keep this selection.";
      setStatus((localTest(value) ? "Local test raster ready." : "Artwork candidate ready.") + saved);
      setError("");
      showCandidate(value);
      return;
    }
    setStatus("");
    setError(value.error || "The artwork was not generated.");
    showCandidate(value);
  }

  function poll() {
    if (!job || !job.job_id) return;
    fetch("/api/creative/jobs/" + job.job_id, { cache: "no-store" }).then(function (response) {
      return response.json();
    }).then(function (value) {
      applyJob(value);
      if (value.execution === "queued" || value.execution === "submitted") {
        pollTimer = window.setTimeout(poll, 400);
      } else {
        busy = false;
        generateButton.disabled = false;
      }
    }).catch(function () {
      busy = false;
      generateButton.disabled = false;
      setError("The creative job could not be read. The open embroidery design was not changed. Generate again if you still need the artwork.");
    });
  }

  function loadPolicy() {
    return fetch("/api/creative/policy", { cache: "no-store" }).then(function (response) {
      return response.json();
    }).then(function (value) {
      policy = value;
      var mode = value.local_test
        ? "Local test provider. Prompts stay on this machine and the image is a test raster, not a live illustration."
        : value.live_ready
          ? "Live generation is configured with " + value.model + ". " + value.remaining_calls + " calls remain."
          : "Live generation is not ready. A key, a remaining budget, transmit permission, and the studio live switch are all required. Nothing is sent automatically.";
      policyLine.textContent = mode + " " + (value.note || "");
      var showTransmit = value.provider === "xai_images" || value.provider === "openrouter_images";
      transmitLabel.hidden = !showTransmit;
      if (!showTransmit) transmitField.checked = false;
    }).catch(function () {
      policyLine.textContent = "The creative policy could not be read. Nothing was sent.";
    });
  }

  function openPanel() {
    previousTitle = document.title;
    document.title = "Creative Director — EmbroideryOS";
    panel.hidden = false;
    if (shell) shell.inert = true;
    loadPolicy();
    requestField.focus();
  }

  function closePanel() {
    window.clearTimeout(pollTimer);
    panel.hidden = true;
    panel.removeAttribute("aria-busy");
    if (shell) shell.inert = false;
    document.title = previousTitle;
    document.getElementById("open-creative").focus();
  }

  document.getElementById("open-creative").addEventListener("click", openPanel);
  document.getElementById("creative-close").addEventListener("click", closePanel);
  panel.addEventListener("keydown", function (event) {
    if (event.key === "Escape") closePanel();
  });

  referenceField.addEventListener("change", function () {
    var file = referenceField.files && referenceField.files[0];
    var note = document.getElementById("creative-reference-note");
    if (!file) {
      note.textContent = "One PNG or JPEG reference stays on this machine. Generation does not send it. Portrait editing is not available in this step.";
      return;
    }
    note.textContent = file.name + " will be hashed locally. This generation step does not send reference images.";
  });

  form.addEventListener("submit", function (event) {
    event.preventDefault();
    if (busy) return;
    setError("");
    setStatus("Preparing the artwork brief…");
    busy = true;
    generateButton.disabled = true;
    var file = referenceField.files && referenceField.files[0];
    var referencePromise = Promise.resolve([]);
    if (file) {
      if (!/^image\/(png|jpeg)$/.test(file.type) || file.size > 500000) {
        busy = false;
        generateButton.disabled = false;
        setStatus("");
        setError("Choose a PNG or JPEG reference under 500 KB. Nothing was sent and the open design was not changed.");
        return;
      }
      referencePromise = file.arrayBuffer().then(function (buffer) {
        var bytes = new Uint8Array(buffer);
        var binary = "";
        for (var index = 0; index < bytes.length; index += 1) binary += String.fromCharCode(bytes[index]);
        return [{ data_base64: btoa(binary) }];
      });
    }
    Promise.all([revisionToken(), referencePromise]).then(function (parts) {
      var body = {
        client_request_id: "c1a-" + Date.now().toString(36) + "-" + Math.random().toString(36).slice(2, 8),
        source_revision: parts[0],
        request: requestField.value,
        art_direction: profileField.value,
        width: Number(widthField.value),
        height: Number(heightField.value),
        transmit: Boolean(transmitField.checked),
        references: parts[1],
      };
      return fetch("/api/creative/jobs", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      }).then(function (response) {
        return response.json();
      });
    }).then(function (value) {
      applyJob(value);
      if (value.execution === "queued" || value.execution === "submitted") {
        pollTimer = window.setTimeout(poll, 300);
      } else {
        busy = false;
        generateButton.disabled = false;
      }
    }).catch(function () {
      busy = false;
      generateButton.disabled = false;
      setStatus("");
      setError("The creative request could not be sent. The open embroidery design was not changed. Try again.");
    });
  });

  cancelButton.addEventListener("click", function () {
    if (!job || !job.job_id) return;
    fetch("/api/creative/jobs/" + job.job_id + "/cancel", { method: "POST" }).then(function (response) {
      return response.json();
    }).then(applyJob).catch(function () {
      setError("The cancellation could not be recorded. Do not assume the provider stopped. The open design was not changed.");
    });
  });

  selectButton.addEventListener("click", function () {
    if (!job || !job.job_id) return;
    selectButton.disabled = true;
    revisionToken().then(function (revision) {
      return fetch("/api/creative/jobs/" + job.job_id + "/select", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ current_revision: revision }),
      }).then(function (response) {
        return response.json();
      });
    }).then(function (value) {
      if (value.ok === false) {
        setError(value.error || "The artwork master was not saved. The open design was not changed.");
        selectButton.disabled = false;
        return;
      }
      applyJob(value);
      setStatus(value.note || "Artwork master saved. The open embroidery design was not changed.");
    }).catch(function () {
      selectButton.disabled = false;
      setError("The artwork master could not be saved. The open embroidery design was not changed.");
    });
  });

  reopenButton.addEventListener("click", function () {
    var artifact = job && job.artifacts && job.artifacts[0];
    if (!artifact) return;
    reopenButton.disabled = true;
    setStatus("Reopening the stored artwork…");
    fetch("/api/creative/artifacts/" + artifact.artifact_id, { cache: "no-store" }).then(function (response) {
      if (!response.ok) throw new Error("missing");
      return response.arrayBuffer();
    }).then(function (buffer) {
      return crypto.subtle.digest("SHA-256", buffer).then(function (digest) {
        var hex = Array.from(new Uint8Array(digest), function (byte) {
          return byte.toString(16).padStart(2, "0");
        }).join("");
        if (hex !== artifact.sha256) {
          setError("The reopened file does not match the stored hash. It was not replaced.");
          return;
        }
        var blob = new Blob([buffer], { type: artifact.media_type || "image/png" });
        image.src = URL.createObjectURL(blob);
        image.alt = "Reopened stored raster artwork";
        setError("");
        setStatus("Reopened the stored artwork. SHA-256 matches " + hex + ". The open embroidery design was not changed.");
      });
    }).catch(function () {
      setError("The stored artwork could not be reopened. The open embroidery design was not changed.");
    }).finally(function () {
      reopenButton.disabled = false;
    });
  });
})();
