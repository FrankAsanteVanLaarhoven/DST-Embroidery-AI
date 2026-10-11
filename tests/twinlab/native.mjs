/** Native acceptance gate. No runner injection, fixture fallback, or auto-skip. */
import test from 'node:test';
import assert from 'node:assert/strict';
import {existsSync} from 'node:fs';
import {readFile, mkdir, writeFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {once} from 'node:events';
import {createTwinServer,runRust,sealResult,sha256} from '../../apps/twinlab/server.mjs';

const ROOT=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const ENGINE=process.env.EMBROIDERY_TWIN_BIN||path.join(ROOT,'target','debug',process.platform==='win32'?'embroidery-twin.exe':'embroidery-twin');
if(!existsSync(ENGINE))throw new Error('NATIVE GATE BLOCKED: build the real engine with cargo build -p embroidery-twin. Fixture mode is not accepted.');
const design=JSON.parse(await readFile(path.join(ROOT,'apps/twinlab/public/example.json'),'utf8'));
const request=()=>({profile_id:'generic-flat-120-v1',design:structuredClone(design)});
const engineSha=sha256(await readFile(ENGINE));
const baseline=await runRust(request(),ENGINE);
const sealed=sealResult(baseline,request(),engineSha);

await mkdir(path.join(ROOT,'evidence/native'),{recursive:true});
await writeFile(path.join(ROOT,'evidence/native/design.dst'),sealed.bytes);
await writeFile(path.join(ROOT,'evidence/native/replay.json'),JSON.stringify(sealed.bundle,null,2)+'\n');

test('real Rust returns the native protocol and positive needle placements',()=>{
  assert.equal(baseline.kernel.mode,'native-rust');
  assert.ok(baseline.trace.needle_placements>0);
});
test('example exercises running, satin and fill objects',()=>{
  assert.deepEqual(new Set(design.objects.map(o=>o.kind)),new Set(['running','satin','fill']));
  for(const object of design.objects)assert.ok(baseline.trace.events.some(e=>e.object_id===object.id));
});
test('every replay event maps to the exact exported byte offset',()=>{
  for(const e of baseline.trace.events)assert.deepEqual([...sealed.bytes.subarray(e.byte_offset,e.byte_offset+3)],e.raw);
  assert.equal(baseline.trace.events.at(-1).kind,'end');
});
test('repeated native generation is byte-deterministic',async()=>{
  const value=await runRust(request(),ENGINE);
  assert.deepEqual(value.dst_bytes,baseline.dst_bytes);
});
test('changed fill spacing changes the native export',async()=>{
  const r=request();r.design.objects.find(o=>o.kind==='fill').spacing_mm=1.4;r.design.revision++;
  const value=await runRust(r,ENGINE);
  assert.notDeepEqual(value.dst_bytes,baseline.dst_bytes);
});
test('RGB-only change updates binding even when standard DST bytes stay equal',async()=>{
  const r=request();r.design.objects[0].rgb=[13,29,71];r.design.revision++;
  const value=await runRust(r,ENGINE),next=sealResult(value,r,engineSha);
  assert.deepEqual(value.dst_bytes,baseline.dst_bytes);
  assert.notEqual(next.bundle.artifact_id,sealed.bundle.artifact_id);
});
test('native profile whitelist rejects unimplemented machine profiles',async()=>{
  const r=request();r.profile_id='unknown-oem';await assert.rejects(runRust(r,ENGINE));
});
test('native hoop gate rejects unsupported geometry',async()=>{
  const r=request();r.design.objects[0].points[0].x=61;await assert.rejects(runRust(r,ENGINE));
});
test('native generator whitelist rejects unsupported stitch types',async()=>{
  const r=request();r.design.objects[0].kind='chenille';await assert.rejects(runRust(r,ENGINE));
});
test('native input refuses raw-stitch injection',async()=>{
  const r=request();r.design.stitches=[];await assert.rejects(runRust(r,ENGINE));
});
test('actual HTTP-to-Rust compile downloads the same buffer',async()=>{
  const server=createTwinServer({enginePath:ENGINE});
  server.listen(0,'127.0.0.1');await once(server,'listening');
  const url=`http://127.0.0.1:${server.address().port}`;
  try{
    const session=await(await fetch(url+'/api/session')).json();assert.equal(session.mode,'native-rust');
    const response=await fetch(url+'/api/compile',{method:'POST',headers:{'Content-Type':'application/json','X-Twin-Token':session.token},body:JSON.stringify(request())});
    assert.equal(response.status,200);const bundle=await response.json();
    const file=await fetch(url+`/api/artifacts/${bundle.artifact_id}.dst`);
    const bytes=Buffer.from(await file.arrayBuffer());
    assert.deepEqual(bytes,sealed.bytes);assert.equal(sha256(bytes),bundle.provenance.dst_sha256);
  }finally{server.closeAllConnections();await new Promise(resolve=>server.close(resolve));}
});
