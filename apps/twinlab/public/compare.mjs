/** Compare binding and display. No stitch generation, no DST decoder, no compatibility certificate. */

const RASTER = new Set(['.png', '.jpg', '.jpeg', '.webp', '.tif', '.tiff', '.bmp']);

export function filenameRoute(name) {
  const base = String(name ?? '').split(/[/\\]/).pop() || '';
  const match = base.match(/(\.[^.]+)$/);
  const ext = match ? match[1].toLowerCase() : '';
  const common = {filename: base, extension: ext, rename_automatically: false, executable: false, machine_executable: false};
  if (ext === '.dts') return {...common, route: 'unrecognised', action: 'Name retained. Content is not identified, so the file is not renamed to .dst and is not executed.'};
  if (ext === '.pdf') return {...common, route: 'source_document', action: 'Uploaded PDF is source artwork. A generated proof is a separate document.'};
  if (RASTER.has(ext)) return {...common, route: 'source_raster', action: 'Enter the physical width. Sampling pitch is not a measurement of scanner accuracy.'};
  if (ext === '.dst') return {...common, route: 'machine_file_not_original', action: 'A DST file is a machine export. Use the machine-file view for the compiled bytes, and attach the artwork separately.'};
  return {...common, route: 'adapter_required', action: 'This extension has no qualified adapter. The original name is unchanged.'};
}

export function mmPerPixel(dpi) {
  if (typeof dpi !== 'number' || !Number.isFinite(dpi) || dpi <= 0) throw new Error('dpi must be positive and finite');
  return 25.4 / dpi;
}

export function placementValid(placement) {
  if (!placement) return false;
  const finite = value => typeof value === 'number' && Number.isFinite(value);
  return finite(placement.width_mm) && placement.width_mm > 0 && placement.width_mm <= 2000
    && finite(placement.x_mm) && finite(placement.y_mm)
    && Number.isInteger(placement.page) && placement.page >= 1 && placement.page <= 100
    && placement.confirmed === true;
}

export function bindingMatches(binding, current) {
  if (!binding || !current?.placement) return false;
  return binding.schema === 'embroideryos-compare-binding/0.2'
    && binding.design_revision === current.revision
    && binding.artifact_id === current.artifactId
    && binding.dst_sha256 === current.dstSha
    && binding.source_sha256 === current.sourceSha
    && binding.width_mm === current.placement.width_mm
    && binding.x_mm === current.placement.x_mm
    && binding.y_mm === current.placement.y_mm
    && binding.page === current.placement.page
    && binding.machine_release_allowed === false
    && binding.approval_allowed === false;
}

function add(xs, ys, x, y) { xs.push(x); ys.push(y); }

function boundsOf(model) {
  const xs = [], ys = [];
  if (model.showDesign && model.design) for (const object of model.design.objects) {
    for (const point of object.points) add(xs, ys, point.x, point.y);
    for (const hole of object.holes || []) for (const point of hole) add(xs, ys, point.x, point.y);
  }
  if (model.showTrace && model.bundle) for (const event of model.bundle.trace.events) {
    add(xs, ys, event.from_units[0] / 10, event.from_units[1] / 10);
    add(xs, ys, event.to_units[0] / 10, event.to_units[1] / 10);
  }
  const image = model.image;
  if (model.showSource && image && model.placement && image.complete && image.naturalWidth) {
    const height = model.placement.width_mm * image.naturalHeight / image.naturalWidth;
    add(xs, ys, model.placement.x_mm, model.placement.y_mm);
    add(xs, ys, model.placement.x_mm + model.placement.width_mm, model.placement.y_mm + height);
  }
  if (!xs.length) { add(xs, ys, -60, -60); add(xs, ys, 60, 60); }
  let loX = Math.min(...xs), hiX = Math.max(...xs), loY = Math.min(...ys), hiY = Math.max(...ys);
  if (hiX - loX < 1) { loX -= 0.5; hiX += 0.5; }
  if (hiY - loY < 1) { loY -= 0.5; hiY += 0.5; }
  const pad = Math.max(1, Math.max(hiX - loX, hiY - loY) * 0.08);
  return {loX: loX - pad, hiX: hiX + pad, loY: loY - pad, hiY: hiY + pad};
}

export function drawCompare(canvas, model) {
  const context = canvas.getContext('2d');
  const width = canvas.clientWidth, height = canvas.clientHeight;
  if (width < 2 || height < 2) return null;
  const dpr = Math.min(devicePixelRatio || 1, 2);
  const pixelWidth = Math.max(1, Math.round(width * dpr)), pixelHeight = Math.max(1, Math.round(height * dpr));
  if (canvas.width !== pixelWidth || canvas.height !== pixelHeight) { canvas.width = pixelWidth; canvas.height = pixelHeight; }
  context.setTransform(dpr, 0, 0, dpr, 0, 0);
  context.clearRect(0, 0, width, height);
  context.fillStyle = model.dark ? '#161c25' : '#f0f3f8';
  context.fillRect(0, 0, width, height);
  const bounds = boundsOf(model);
  const fit = Math.max(0.05, Math.min((width - 28) / (bounds.hiX - bounds.loX), (height - 28) / (bounds.hiY - bounds.loY)));
  const scale = model.camera?.zoom || fit;
  const at = (x, y) => [
    width / 2 + (x - (bounds.loX + bounds.hiX) / 2 + (model.camera?.panX || 0)) * scale,
    height / 2 + (y - (bounds.loY + bounds.hiY) / 2 + (model.camera?.panY || 0)) * scale
  ];
  context.strokeStyle = model.dark ? '#303b4a' : '#dfe5ef';
  context.lineWidth = 1;
  context.strokeRect(...(() => { const a = at(-60, -60), b = at(60, 60); return [a[0], a[1], b[0] - a[0], b[1] - a[1]]; })());
  if (model.showSource && model.image?.complete && model.image.naturalWidth && model.placement) {
    const place = model.placement;
    const imageHeight = place.width_mm * model.image.naturalHeight / model.image.naturalWidth;
    const origin = at(place.x_mm, place.y_mm);
    const opposite = at(place.x_mm + place.width_mm, place.y_mm + imageHeight);
    context.save();
    context.globalAlpha = model.opacity;
    context.drawImage(model.image, origin[0], origin[1], opposite[0] - origin[0], opposite[1] - origin[1]);
    context.restore();
  }
  if (model.showDesign && model.design) {
    for (const object of model.design.objects) {
      context.beginPath();
      for (const ring of [object.points, ...(object.holes || [])]) {
        ring.forEach((point, index) => {
          const mapped = at(point.x, point.y);
          index ? context.lineTo(...mapped) : context.moveTo(...mapped);
        });
        context.closePath();
      }
      const colour = '#' + object.rgb.map(value => value.toString(16).padStart(2, '0')).join('');
      if (object.kind === 'fill') {
        context.fillStyle = colour + '22';
        context.fill('evenodd');
      }
      context.strokeStyle = colour;
      context.lineWidth = 1.4;
      context.stroke();
    }
  }
  if (model.showTrace && model.bundle) {
    context.save();
    context.beginPath();
    context.rect(0, 0, width * model.wipe, height);
    context.clip();
    let sewn = 0;
    for (const event of model.bundle.trace.events) {
      if (event.kind !== 'stitch' && event.kind !== 'jump') continue;
      if (model.colour !== 'all' && event.color_index !== model.colour) continue;
      const stitch = event.kind === 'stitch';
      if (stitch) sewn += 1;
      const colour = model.bundle.palette[event.color_index]?.rgb || [30, 40, 50];
      const start = at(event.from_units[0] / 10, event.from_units[1] / 10);
      const end = at(event.to_units[0] / 10, event.to_units[1] / 10);
      context.strokeStyle = `rgb(${colour.join(',')})`;
      context.globalAlpha = model.followClock && stitch && sewn > model.completedStitches ? 0.12 : (stitch ? 0.95 : 0.45);
      context.lineWidth = stitch ? 1.2 : 0.7;
      context.setLineDash(stitch ? [] : [3, 3]);
      context.beginPath();
      context.moveTo(...start);
      context.lineTo(...end);
      context.stroke();
    }
    context.restore();
  }
  context.setLineDash([]);
  context.globalAlpha = 1;
  context.font = '11px -apple-system, sans-serif';
  context.fillStyle = model.dark ? '#c5d0e0' : '#3c4a5e';
  context.fillText('Screen scale is not physical size. The 100 mm bar is on the PDF proof.', 12, height - 12);
  canvas.dataset.pxPerMm = String(scale);
  return scale;
}
