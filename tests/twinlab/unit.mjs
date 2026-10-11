import test from 'node:test';
import assert from 'node:assert/strict';
import {once} from 'node:events';
import http from 'node:http';
import {createTwinServer,sealResult,sha256,stable} from '../../apps/twinlab/server.mjs';
import {bindingMatches,filenameRoute,mmPerPixel,placementValid} from '../../apps/twinlab/public/compare.mjs';
import {sampleTrace,prepareTrace,Playback,frameTranslation,pointUnderNeedle} from '../../apps/twinlab/public/playback.mjs';
import {multiply} from '../../apps/twinlab/public/scene.mjs';
import {dst,design,fixtureResult} from './fixture.mjs';
const request=()=>({profile_id:'generic-flat-120-v1',design:structuredClone(design)});
const sealed=(r=request(),value=fixtureResult(r),hash='1'.repeat(64))=>sealResult(value,r,hash,'transport-test-fixture');

test('golden fixture is 512-byte header plus four exact records',()=>{assert.equal(dst.length,524);assert.deepEqual([...dst.subarray(-3)],[0,0,243]);});
test('sealed export bytes remain byte-identical to baseline fixture',()=>assert.deepEqual(sealed().bytes,dst));
test('browser transport base64 reconstructs exact bytes',()=>assert.deepEqual(Buffer.from(sealed().bundle.dst_base64,'base64'),dst));
test('DST hash is computed from exact bytes',()=>assert.equal(sealed().bundle.provenance.dst_sha256,sha256(dst)));
test('unchanged source produces a deterministic artifact ID',()=>assert.equal(sealed().bundle.artifact_id,sealed().bundle.artifact_id));
test('design revisions change artifact identity even if DST bytes agree',()=>{const r=request();r.design.revision++;assert.notEqual(sealed().bundle.artifact_id,sealed(r).bundle.artifact_id);});
test('thread palette changes identity even when DST cannot encode RGB',()=>{const r=request(),v=fixtureResult(r);v.palette[0].rgb=[200,50,10];assert.notEqual(sealed().bundle.artifact_id,sealed(r,v).bundle.artifact_id);});
test('engine binary identity changes the artifact ID',()=>assert.notEqual(sealed().bundle.artifact_id,sealed(request(),fixtureResult(),'2'.repeat(64)).bundle.artifact_id));
test('revision mismatch is rejected',()=>{const r=request(),v=fixtureResult(structuredClone(r));v.design.revision++;assert.throws(()=>sealed(r,v),/revision/);});
test('profile mismatch is rejected',()=>{const r=request(),v=fixtureResult(r);v.profile.id='unknown';assert.throws(()=>sealed(r,v),/profile/);});
test('trace raw-byte tampering is rejected',()=>{const r=request(),v=fixtureResult(r);v.trace.events[0].raw[0]^=1;assert.throws(()=>sealed(r,v),/byte binding/);});
test('trace cardinality mismatch is rejected',()=>{const r=request(),v=fixtureResult(r);v.trace.events.pop();assert.throws(()=>sealed(r,v),/cardinality/);});
test('timeline discontinuity is rejected',()=>{const r=request(),v=fixtureResult(r);v.trace.events[1].start_ms++;assert.throws(()=>sealed(r,v),/timeline/);});
test('non-byte numeric values are rejected',()=>{const r=request(),v=fixtureResult(r);v.dst_bytes[0]=256;assert.throws(()=>sealed(r,v),/byte buffer/);});
test('unknown protocol is rejected',()=>{const r=request(),v=fixtureResult(r);v.protocol='fake';assert.throws(()=>sealed(r,v),/protocol/);});
test('fixture cannot impersonate the native response by default',()=>assert.throws(()=>sealResult(fixtureResult(),request(),'1'.repeat(64)),/protocol/));
test('canonical serialisation ignores object-key insertion order',()=>assert.equal(stable({b:1,a:[2,3]}),stable({a:[2,3],b:1})));

const trace=prepareTrace(fixtureResult().trace);
for(const point of [[0,0],[10,20],[-20,35],[60,-60],[-60,60]])test(`inverse frame transform holds design point ${point} under fixed needle`,()=>assert.deepEqual(pointUnderNeedle(point),[0,0,0]));
test('frame translation changes mm to metres exactly once',()=>assert.deepEqual(frameTranslation([10,20]),[-.01,0,-.02]));
test('needle is raised during XY movement',()=>{const s=sampleTrace(trace,20);assert.equal(s.needle_lift_mm,12);assert.ok(s.position_mm[0]>0&&s.position_mm[0]<10);});
test('frame stops before the needle penetrates fabric',()=>{assert.deepEqual(sampleTrace(trace,50).position_mm,sampleTrace(trace,70).position_mm);assert.equal(sampleTrace(trace,70).needle_lift_mm,0);});
test('needle penetration reveals one segment without advancing the next command',()=>{assert.equal(sampleTrace(trace,69).completed_stitches,0);assert.equal(sampleTrace(trace,70).completed_stitches,1);assert.equal(sampleTrace(trace,70).event_index,0);});
test('needle rises again before next frame movement',()=>assert.equal(sampleTrace(trace,96).needle_lift_mm,12));
test('scrubbing backward is deterministic',()=>{const a=sampleTrace(trace,175);sampleTrace(trace,290);assert.deepEqual(sampleTrace(trace,175),a);});
test('end event adds no needle penetration',()=>assert.equal(sampleTrace(trace,301).completed_stitches,3));
test('time is clamped and NaN cannot corrupt the renderer',()=>{assert.equal(sampleTrace(trace,-10).time_ms,0);assert.equal(sampleTrace(trace,99999).time_ms,301);assert.equal(sampleTrace(trace,NaN).time_ms,0);});
test('two renderers sampling one time receive identical states',()=>{for(let t=0;t<=301;t++){const a=sampleTrace(trace,t),b=sampleTrace(trace,t);assert.deepEqual(a,b);}});
test('playback speed changes wall-time advance, not the trace',()=>{const c=new Playback();const original=JSON.stringify(trace.events);c.load(trace);c.playing=true;c.speed=4;c.tick(0);c.tick(20);assert.equal(c.time,80);assert.equal(JSON.stringify(trace.events),original);});
test('paused playback does not advance',()=>{const c=new Playback();c.load(trace);c.tick(0);c.tick(100);assert.equal(c.time,0);});
test('stepping follows raw record order',()=>{const c=new Playback();c.load(trace);c.step(1);assert.equal(c.sample().event_index,1);c.step(-1);assert.equal(c.sample().event_index,0);});
test('synthetic jump display never drives needle penetration',()=>{const t={duration_ms:100,events:[{kind:'jump',index:0,from_units:[0,0],to_units:[100,0],start_ms:0,end_ms:100,color_index:0}]};for(let i=0;i<100;i++)assert.equal(sampleTrace(t,i).needle_lift_mm,12);});
test('matrix multiplication preserves identity transforms',()=>{const i=new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]);assert.deepEqual(multiply(i,i),i);});

async function withServer(fn,{runner=async r=>fixtureResult(r),...options}={}){
 const server=createTwinServer({enginePath:process.execPath,engineSha:'1'.repeat(64),runner,exampleDesign:design,...options});
 server.listen(0,'127.0.0.1');await once(server,'listening');const url=`http://127.0.0.1:${server.address().port}`;
 try{const session=await(await fetch(url+'/api/session')).json();await fn(url,session);}finally{server.closeAllConnections();await new Promise(resolve=>server.close(resolve));}
}
const post=(url,token,r=request(),extra={})=>fetch(url+'/api/compile',{method:'POST',headers:{'Content-Type':'application/json','X-Twin-Token':token,...extra},body:JSON.stringify(r)});
test('HTTP transport serves and downloads the same artifact buffer',()=>withServer(async(u,s)=>{const response=await post(u,s.token);assert.equal(response.status,200);const b=await response.json();const file=await fetch(u+`/api/artifacts/${b.artifact_id}.dst`);assert.deepEqual(Buffer.from(await file.arrayBuffer()),dst);assert.equal(file.headers.get('x-dst-sha256'),sha256(dst));}));
test('test runner status is explicitly marked as fixture',()=>withServer(async(u,s)=>assert.equal(s.mode,'transport-test-fixture')));
test('session token required',()=>withServer(async(u)=>assert.equal((await post(u,'')).status,403)));
test('cross-origin requests are refused',()=>withServer(async(u,s)=>assert.equal((await post(u,s.token,request(),{Origin:'https://example.invalid'})).status,403)));
test('unexpected Host is refused',()=>withServer(async(u)=>{const status=await new Promise((resolve,reject)=>{const r=http.get(u+'/api/session',{headers:{Host:'evil.invalid'}},res=>{res.resume();resolve(res.statusCode);});r.on('error',reject);});assert.equal(status,403);}));
test('cross-site fetch metadata is refused',()=>withServer(async(u)=>assert.equal((await fetch(u+'/api/session',{headers:{'Sec-Fetch-Site':'cross-site'}})).status,403)));
test('unsupported MIME type is refused',()=>withServer(async(u,s)=>assert.equal((await post(u,s.token,request(),{'Content-Type':'text/plain'})).status,415)));
test('oversized request is refused',()=>withServer(async(u,s)=>assert.equal((await post(u,s.token,{padding:'x'.repeat(262145)})).status,413)));
test('unknown artifact does not fall back to a newer export',()=>withServer(async(u)=>assert.equal((await fetch(u+'/api/artifacts/'+'0'.repeat(64)+'.dst')).status,404)));
test('static file traversal is not served',()=>withServer(async(u)=>assert.equal((await fetch(u+'/%2e%2e/Cargo.toml')).status,404)));
test('unimplemented methods are refused',()=>withServer(async(u)=>assert.equal((await fetch(u+'/api/session',{method:'DELETE'})).status,405)));
test('malformed JSON is rejected',()=>withServer(async(u,s)=>{const r=await fetch(u+'/api/compile',{method:'POST',headers:{'Content-Type':'application/json','X-Twin-Token':s.token},body:'{'});assert.equal(r.status,422);}));
test('worker error is a failure, never a fake trace',()=>withServer(async(u,s)=>{const r=await post(u,s.token);assert.equal(r.status,422);assert.equal((await r.json()).error,'worker test failure');},{runner:async()=>{throw new Error('worker test failure');}}));
test('artifact retention is bounded to eight immutable entries',()=>withServer(async(u,s)=>{let first;for(let i=0;i<9;i++){const r=request();r.design.revision=i;const b=await(await post(u,s.token,r)).json();if(i===0)first=b.artifact_id;}assert.equal((await fetch(u+`/api/artifacts/${first}.dst`)).status,404);}));
test('254 dpi sampling is 0.1 mm per pixel within a numerical tolerance',()=>{assert.ok(Math.abs(mmPerPixel(254)-0.1)<1e-10);assert.ok(Math.abs(mmPerPixel(300)-(25.4/300))<1e-12);assert.throws(()=>mmPerPixel(0),/positive/);});
test('a dts upload keeps its name',()=>{const route=filenameRoute('folder/Scan.DTS');assert.equal(route.filename,'Scan.DTS');assert.equal(route.route,'unrecognised');assert.equal(route.rename_automatically,false);assert.equal(route.executable,false);});
test('pdf and dst routes stay distinct',()=>{assert.equal(filenameRoute('page.pdf').machine_executable,false);assert.equal(filenameRoute('mark.dst').route,'machine_file_not_original');assert.equal(filenameRoute('art.pes').route,'adapter_required');});
test('proof binding requires the same revision, bytes and placement',()=>{
 const placement={width_mm:14,x_mm:-2,y_mm:-2,page:1,confirmed:true};
 const binding={schema:'embroideryos-compare-binding/0.2',design_revision:2,artifact_id:'a',dst_sha256:'d',source_sha256:'s',width_mm:14,x_mm:-2,y_mm:-2,page:1,machine_release_allowed:false,approval_allowed:false};
 const current={revision:2,artifactId:'a',dstSha:'d',sourceSha:'s',placement};
 assert.equal(bindingMatches(binding,current),true);
 assert.equal(bindingMatches(binding,{...current,revision:3}),false);
 assert.equal(bindingMatches(binding,{...current,placement:{...placement,width_mm:15}}),false);
 assert.equal(bindingMatches({...binding,approval_allowed:true},current),false);
 assert.equal(placementValid(placement),true);
 assert.equal(placementValid({...placement,confirmed:false}),false);
});
const png=Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==','base64');
const proofBody=(artifactId,extra={})=>({artifact_id:artifactId,source_name:'reference.png',source_base64:png.toString('base64'),source_width_mm:14,source_x_mm:-2,source_y_mm:-2,page:1,scale_confirmed:true,...extra});
test('proof pdf is bound to the stored DST and is not a release',()=>withServer(async(u,s)=>{
 const compiled=await(await post(u,s.token)).json();
 const response=await fetch(u+'/api/proof',{method:'POST',headers:{'Content-Type':'application/json','X-Twin-Token':s.token},body:JSON.stringify(proofBody(compiled.artifact_id,{dst_base64:'aa'}))});
 assert.equal(response.status,200,JSON.stringify(await response.clone().json()));
 const meta=await response.json();
 assert.equal(meta.binding.dst_sha256,compiled.provenance.dst_sha256);
 assert.equal(meta.binding.machine_release_allowed,false);
 assert.equal(meta.binding.approval_allowed,false);
 assert.equal(meta.checks.rust_executed,false);
 assert.equal(meta.checks.decoder_semantics_independently_verified,false);
 const pdf=await fetch(u+`/api/proofs/${meta.proof_id}.pdf`);
 const bytes=Buffer.from(await pdf.arrayBuffer());
 assert.equal(pdf.headers.get('content-type'),'application/pdf');
 assert.match(pdf.headers.get('content-security-policy'),/frame-ancestors 'self'/);
 assert.doesNotMatch(pdf.headers.get('content-security-policy'),/frame-ancestors 'none'/);
 const page=await fetch(u+'/');
 assert.match(page.headers.get('content-security-policy'),/frame-ancestors 'none'/);
 assert.equal(sha256(bytes),meta.binding.pdf_sha256);
 assert.equal(bytes.subarray(0,5).toString(),'%PDF-');
}));
test('proof refuses an unconfirmed scale and a dts name',()=>withServer(async(u,s)=>{
 const compiled=await(await post(u,s.token)).json();
 const unconfirmed=await fetch(u+'/api/proof',{method:'POST',headers:{'Content-Type':'application/json','X-Twin-Token':s.token},body:JSON.stringify(proofBody(compiled.artifact_id,{scale_confirmed:false}))});
 assert.equal(unconfirmed.status,422);
 const dts=await fetch(u+'/api/proof',{method:'POST',headers:{'Content-Type':'application/json','X-Twin-Token':s.token},body:JSON.stringify(proofBody(compiled.artifact_id,{source_name:'legacy.dts'}))});
 assert.equal(dts.status,422);
 assert.match((await dts.json()).error,/not renamed/);
 assert.equal((await fetch(u+'/api/proof',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(proofBody(compiled.artifact_id))})).status,403);
}));
test('real default runner does not substitute fixture when engine is absent',async()=>{
 const server=createTwinServer({enginePath:'/nonexistent/embroidery-twin'});server.listen(0,'127.0.0.1');await once(server,'listening');const u=`http://127.0.0.1:${server.address().port}`;try{const s=await(await fetch(u+'/api/session')).json();assert.equal(s.engine_available,false);assert.equal(s.mode,'native-rust');assert.equal((await post(u,s.token)).status,422);}finally{server.closeAllConnections();await new Promise(r=>server.close(r));}
});
