/** Local proof bridge. Calls the developer PDF utility. Does not decode DST or generate stitches. */
import {spawn} from 'node:child_process';
import {mkdtemp, readFile, rm, writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {filenameRoute} from './public/compare.mjs';

function fail(message, status = 422) {
  const error = new Error(message);
  error.status = status;
  return error;
}

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true});
    const err = [];
    let settled = false;
    const finish = (error, value) => { if (settled) return; settled = true; clearTimeout(timer); error ? reject(error) : resolve(value); };
    const timer = setTimeout(() => { child.kill('SIGKILL'); finish(fail('Proof utility exceeded 25 seconds.')); }, 25000);
    child.on('error', () => finish(fail('Proof utility could not start. Use the repository Python environment with Pillow and reportlab.', 503)));
    child.stderr.on('data', chunk => { if (Buffer.concat(err).length < 8000) err.push(chunk); });
    child.stdout.on('data', () => {});
    child.on('close', code => finish(null, {code, stderr: Buffer.concat(err).toString()}));
  });
}

function proofMessage(stderr) {
  return String(stderr || '').replace(/\s+/g, ' ').trim().replace(/^Proof refused:\s*/i, '').slice(0, 400);
}

export async function writeCompareProof({bundle, sourceBytes, sourceName, placement, pythonPath, scriptPath}) {
  const route = filenameRoute(sourceName);
  if (route.rename_automatically) throw fail('Automatic renaming is disabled.');
  if (route.route === 'unrecognised') throw fail(`${route.filename || 'That file'} is retained. It is not renamed to .dst and it is not executed.`);
  if (route.route !== 'source_raster' && route.route !== 'source_document') throw fail(route.action);
  if (!placement || placement.confirmed !== true) throw fail('Enter the physical width and confirm it. A missing DPI value is not treated as 1:1.');
  for (const [key, label, positive] of [['width_mm', 'Source width', true], ['x_mm', 'Source X', false], ['y_mm', 'Source Y', false]]) {
    const value = placement[key];
    if (typeof value !== 'number' || !Number.isFinite(value) || (positive && value <= 0) || (positive && value > 2000)) throw fail(`${label} must be a finite${positive ? ' positive' : ''} number within the proof limit.`);
  }
  if (!Number.isInteger(placement.page) || placement.page < 1 || placement.page > 100) throw fail('Choose a PDF page from 1 to 100.');
  const directory = await mkdtemp(path.join(tmpdir(), 'embroidery-proof-'));
  try {
    const safe = (route.filename || 'source').replace(/[^\w.\- ]+/g, '_').replace(/^\.+/g, '').slice(0, 80) || 'source';
    const bundlePath = path.join(directory, 'bundle.json');
    const sourcePath = path.join(directory, safe);
    const outPath = path.join(directory, 'proof.pdf');
    await writeFile(bundlePath, JSON.stringify(bundle));
    await writeFile(sourcePath, sourceBytes);
    const result = await run(pythonPath, [scriptPath, '--bundle', bundlePath, '--source', sourcePath, '--output', outPath, '--source-width-mm', String(placement.width_mm), '--source-x-mm', String(placement.x_mm), '--source-y-mm', String(placement.y_mm), '--page', String(placement.page)]);
    if (result.code !== 0) throw fail(proofMessage(result.stderr) || 'Proof utility refused the inputs.');
    const pdf = await readFile(outPath);
    const manifest = JSON.parse(await readFile(path.join(directory, 'proof.manifest.json'), 'utf8'));
    if (manifest.dst_sha256 !== bundle.provenance?.dst_sha256) throw fail('Proof DST hash does not match the stored export.');
    if (manifest.checks?.rust_executed !== false || manifest.checks?.decoder_semantics_independently_verified !== false) throw fail('Proof utility reported a check it does not perform.');
    return {pdf, manifest, route};
  } finally {
    await rm(directory, {recursive: true, force: true});
  }
}
