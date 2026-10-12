/** Record the TwinLab child image for two compiles. Does not change the JSON bundle. */
import {execFile} from 'node:child_process';
import {createHash} from 'node:crypto';
import {readFile, mkdir, writeFile} from 'node:fs/promises';
import {spawn} from 'node:child_process';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const ENGINE = path.join(ROOT, 'target/debug/embroidery-twin');
const EVIDENCE = path.join(ROOT, 'evidence/lettering-l4');
const PORT = process.env.TWIN_TRACE_PORT || '4187';
const BASE = process.env.TWINLAB_URL || `http://127.0.0.1:${PORT}/`;

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function exec(cmd, args) {
  return new Promise((resolve) => {
    execFile(cmd, args, (error, stdout, stderr) => {
      resolve({code: error ? error.code ?? 1 : 0, stdout: String(stdout || ''), stderr: String(stderr || '')});
    });
  });
}

async function listenerPid(port) {
  const result = await exec('/usr/sbin/lsof', ['-nP', `-iTCP:${port}`, '-sTCP:LISTEN', '-t']);
  const pid = result.stdout.trim().split('\n').filter(Boolean)[0];
  if (!pid) throw new Error(`No listener on ${port}`);
  return Number(pid);
}

async function alive(pid) {
  const result = await exec('/bin/ps', ['-p', String(pid), '-o', 'pid=']);
  return result.code === 0 && result.stdout.trim() !== '';
}

const design = {
  schema_version: 1,
  name: 'Trace',
  revision: 1,
  objects: [{
    id: 1,
    label: 'Run',
    kind: 'running',
    points: [{x: 0, y: 0}, {x: 10, y: 0}],
    rgb: [53, 106, 210],
    spacing_mm: 2,
    angle_degrees: 0,
    max_stitch_mm: 4,
    width_mm: 1,
    compensation_mm: 0,
    underlay_mm: 0,
  }],
};

async function compile(url, token, trace) {
  const headers = {'Content-Type': 'application/json', 'X-Twin-Token': token};
  if (trace) headers['X-Worker-Trace'] = '1';
  const response = await fetch(new URL('/api/compile', url), {
    method: 'POST',
    headers,
    body: JSON.stringify({profile_id: 'generic-flat-120-v1', design}),
  });
  const body = await response.json();
  return {
    status: response.status,
    pid: response.headers.get('x-worker-pid'),
    exe: response.headers.get('x-worker-exe'),
    sha256: response.headers.get('x-worker-sha256'),
    artifact_id: body.artifact_id || null,
    engine_sha256: body.provenance?.engine_sha256 || null,
    body_has_worker: JSON.stringify(body).includes('X-Worker') || Object.hasOwn(body, 'worker'),
    error: body.error || null,
  };
}

let child = null;
if (!process.env.TWINLAB_URL) {
  child = spawn(process.execPath, [path.join(ROOT, 'apps/twinlab/server.mjs')], {
    cwd: ROOT,
    env: {...process.env, PORT},
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('TwinLab trace server did not listen.')), 10000);
    child.stdout.on('data', (chunk) => {
      if (String(chunk).includes('TwinLab:')) {
        clearTimeout(timer);
        resolve();
      }
    });
    child.on('exit', (code) => {
      clearTimeout(timer);
      reject(new Error(`TwinLab trace server exited ${code}`));
    });
  });
}

try {
  const url = new URL(BASE);
  const port = url.port;
  const nodePidBefore = await listenerPid(port);
  const session = await (await fetch(new URL('/api/session', url))).json();
  if (session.mode !== 'native-rust') throw new Error(`Expected native-rust, got ${session.mode}`);
  const plain = await compile(url, session.token, false);
  if (plain.status !== 200) throw new Error(plain.error || `Compile failed ${plain.status}`);
  if (plain.pid || plain.exe || plain.sha256) throw new Error('Worker headers were returned without a trace request.');
  const expected = sha256(await readFile(ENGINE));
  const runs = [];
  for (let index = 0; index < 2; index += 1) {
    const result = await compile(url, session.token, true);
    if (result.status !== 200) throw new Error(result.error || `Traced compile failed ${result.status}`);
    const image = sha256(await readFile(result.exe));
    const pid = Number(result.pid);
    await new Promise((resolve) => setTimeout(resolve, 50));
    runs.push({
      ...result,
      pid,
      image_sha256: image,
      still_alive: await alive(pid),
      node_pid: await listenerPid(port),
    });
  }
  const nodePidAfter = await listenerPid(port);
  const record = {
    record: 'lettering-l4-worker',
    date: '2026-10-12',
    url: BASE,
    node_pid_before: nodePidBefore,
    node_pid_after: nodePidAfter,
    node_pid_unchanged: nodePidBefore === nodePidAfter,
    expected_engine_sha256: expected,
    plain_artifact_id: plain.artifact_id,
    plain_engine_sha256: plain.engine_sha256,
    runs: runs.map((run) => ({
      pid: run.pid,
      exe: run.exe,
      header_sha256: run.sha256,
      image_sha256: run.image_sha256,
      artifact_id: run.artifact_id,
      engine_sha256: run.engine_sha256,
      still_alive: run.still_alive,
      body_has_worker: run.body_has_worker,
      node_pid: run.node_pid,
    })),
  };
  const problems = [];
  if (!record.node_pid_unchanged) problems.push('Node listener pid changed');
  if (runs[0].pid === runs[1].pid) problems.push('The two compiles used the same worker pid');
  for (const run of runs) {
    if (run.sha256 !== expected) problems.push(`Header hash ${run.sha256} is not the built engine`);
    if (run.image_sha256 !== run.sha256) problems.push(`lsof image hash does not match the header for ${run.exe}`);
    if (run.exe !== ENGINE) problems.push(`Worker exe was ${run.exe}`);
    if (run.still_alive) problems.push(`Worker ${run.pid} was still alive after the compile`);
    if (run.pid === run.node_pid) problems.push('Worker pid matched the Node listener');
    if (run.body_has_worker) problems.push('Worker identity was written into the JSON bundle');
    if (run.artifact_id !== plain.artifact_id) problems.push('Trace changed the artifact id');
    if (run.engine_sha256 !== expected) problems.push('Provenance engine hash does not match the built file');
  }
  record.passed = problems.length === 0;
  record.problems = problems;
  await mkdir(EVIDENCE, {recursive: true});
  await writeFile(path.join(EVIDENCE, 'worker-identity.json'), JSON.stringify(record, null, 2) + '\n');
  if (problems.length) {
    console.error(problems.join('\n'));
    process.exitCode = 1;
  } else {
    console.log(`worker trace passed node ${nodePidBefore} children ${runs.map((run) => run.pid).join(', ')} ${expected}`);
  }
} finally {
  if (child) child.kill('SIGTERM');
}
