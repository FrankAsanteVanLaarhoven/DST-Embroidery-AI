/** Loopback-only demo service. No machine transfer endpoint, cloud service or JS stitch engine. */
import http from 'node:http';
import {readFile} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {spawn, execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {createHash, randomBytes, timingSafeEqual} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {writeCompareProof} from './proof.mjs';

const HERE=path.dirname(fileURLToPath(import.meta.url));
const ROOT=path.resolve(HERE,'../..');
const LIMIT=262144;
const PROOF_LIMIT=12*1024*1024;
async function readLimited(req,limit){
  if(req.headers['transfer-encoding']){const error=new Error('A bounded Content-Length is required.');error.status=411;throw error;}
  const len=Number(req.headers['content-length']);
  if(!Number.isInteger(len)||len<=0||len>limit){const error=new Error('Request must be between 1 byte and 12 MiB.');error.status=413;throw error;}
  let size=0,parts=[];
  for await(const part of req){size+=part.length;if(size>limit){const error=new Error('Input exceeds 12 MiB.');error.status=413;throw error;}parts.push(part);}
  if(size!==len){const error=new Error('Incomplete request body.');error.status=422;throw error;}
  return Buffer.concat(parts);
}
const execFileAsync=promisify(execFile);
export const sha256=data=>createHash('sha256').update(data).digest('hex');
function txtPath(output){
  let textFile=false;
  for(const line of String(output).split('\n')){
    if(line.startsWith('ftxt')){textFile=true;continue;}
    if(textFile&&line.startsWith('n'))return line.slice(1);
    if(line.startsWith('f'))textFile=false;
  }
  return '';
}
async function identifyWorker(pid){
  let detail='lsof produced no text image';
  for(let attempt=0;attempt<12;attempt++){
    try{
      const {stdout}=await execFileAsync('/usr/sbin/lsof',['-a','-p',String(pid),'-d','txt','-Fn'],{timeout:2000});
      const exe=txtPath(stdout);
      if(exe&&existsSync(exe))return {pid,exe,sha256:sha256(await readFile(exe))};
      detail=stdout.slice(0,400)||detail;
    }catch(error){detail=String(error.stderr||error.message).slice(0,400);}
    await new Promise(resolve=>setTimeout(resolve,25));
  }
  const failure=new Error('The worker image was not recorded before this compile. '+detail);
  failure.status=422;
  throw failure;
}
export function stable(value){
  if(Array.isArray(value))return '['+value.map(stable).join(',')+']';
  if(value!==null&&typeof value==='object')return '{'+Object.keys(value).sort().map(k=>JSON.stringify(k)+':'+stable(value[k])).join(',')+'}';
  return JSON.stringify(value);
}
export function runRust(input,enginePath,onWorker){
  return new Promise((resolve,reject)=>{
    const child=spawn(enginePath,[],{stdio:['pipe','pipe','pipe'],cwd:ROOT,windowsHide:true});
    let out=[],err=[],size=0,settled=false;
    const finish=(error,value)=>{if(settled)return;settled=true;clearTimeout(timer);error?reject(error):resolve(value);};
    const timer=setTimeout(()=>{child.kill('SIGKILL');finish(new Error('Rust generation exceeded the 30-second demo budget.'));},30000);
    child.on('error',()=>finish(new Error('Rust engine unavailable. Run cargo build -p embroidery-twin, then restart the service.')));
    child.stdout.on('data',chunk=>{size+=chunk.length;if(size>32*1024*1024){child.kill('SIGKILL');finish(new Error('Rust response exceeds 32 MiB.'));}else out.push(chunk);});
    child.stderr.on('data',chunk=>{if(Buffer.concat(err).length<16384)err.push(chunk);});
    child.stdin.on('error',()=>{});
    child.on('close',code=>{
      if(code!==0)return finish(new Error(Buffer.concat(err).toString().slice(0,16384)||'Rust worker failed.'));
      try{finish(null,JSON.parse(Buffer.concat(out).toString()));}catch{finish(new Error('Rust returned an invalid response.'));}
    });
    const send=()=>{if(!settled)child.stdin.end(JSON.stringify(input));};
    if(typeof onWorker==='function'){
      identifyWorker(child.pid).then(image=>{onWorker(image);send();}).catch(error=>{child.kill('SIGKILL');finish(error);});
    }else send();
  });
}
export function sealResult(result,request,engineSha,expectedMode='native-rust'){
  if(result.protocol!=='twinlab-m1/1'||result.kernel?.mode!==expectedMode)throw new Error('Expected the native Rust protocol.');
  if(stable(result.design)!==stable(request.design)||result.profile?.id!==request.profile_id)throw new Error('Rust response does not bind the submitted revision/profile.');
  if(!Array.isArray(result.dst_bytes)||result.dst_bytes.length<515||result.dst_bytes.length>512+60000*3||!result.dst_bytes.every(n=>Number.isInteger(n)&&n>=0&&n<=255))throw new Error('Invalid DST byte buffer.');
  const bytes=Buffer.from(result.dst_bytes),events=result.trace?.events;
  if(!Array.isArray(events)||events.length*3+512!==bytes.length||result.trace.record_count!==events.length)throw new Error('DST/trace cardinality mismatch.');
  // Transport checks exact raw bytes; it does NOT reinterpret or generate DST commands.
  for(let i=0;i<events.length;i++){
    const e=events[i];
    if(e.index!==i||e.byte_offset!==512+3*i||!Array.isArray(e.raw)||e.raw.length!==3||e.raw.some((n,j)=>n!==bytes[512+3*i+j]))throw new Error('DST/trace byte binding mismatch.');
    if(!Number.isFinite(e.start_ms)||!Number.isFinite(e.end_ms)||e.end_ms<=e.start_ms||e.start_ms!==(i?events[i-1].end_ms:0))throw new Error('Non-monotonic replay timeline.');
  }
  const {dst_bytes,...rest}=result;
  const provenance={design_sha256:sha256(stable(request.design)),profile_sha256:sha256(stable(result.profile)),dst_sha256:sha256(bytes),trace_sha256:sha256(stable(result.trace)),engine_sha256:engineSha};
  const id=sha256(stable({...provenance,palette:result.palette,protocol:result.protocol}));
  return {bytes,bundle:{...rest,artifact_id:id,provenance,dst_base64:bytes.toString('base64')}};
}

export function createTwinServer({enginePath=process.env.EMBROIDERY_TWIN_BIN||path.join(ROOT,'target','debug',process.platform==='win32'?'embroidery-twin.exe':'embroidery-twin'),runner=runRust,engineSha=null,exampleDesign=null}={}){
  const token=randomBytes(24).toString('hex'),artifacts=new Map(),proofs=new Map();
  let active=0,proving=0;
  const pythonPath=process.env.EMBROIDERY_PYTHON||path.join(ROOT,'.venv','bin',process.platform==='win32'?'python.exe':'python');
  const proofScript=path.join(ROOT,'tools','compat_compare','proof.py');
  const assets={'/':['index.html','text/html; charset=utf-8'],'/app.mjs':['app.mjs','text/javascript'],'/compare.mjs':['compare.mjs','text/javascript'],'/scene.mjs':['scene.mjs','text/javascript'],'/playback.mjs':['playback.mjs','text/javascript'],'/style.css':['style.css','text/css'],'/example.json':['example.json','application/json'],'/square.json':['square.json','application/json'],'/reference-square.png':['reference-square.png','image/png'],'/favicon.svg':['favicon.svg','image/svg+xml']};
  const server=http.createServer(async(req,res)=>{
    res.setHeader('X-Content-Type-Options','nosniff');
    res.setHeader('Cache-Control','no-store');
    res.setHeader('Referrer-Policy','no-referrer');
    // Pages refuse to be framed. The proof PDF is the one exception: the Compare
    // pane on this same origin reopens it in an iframe. Other origins stay blocked.
    const pagePolicy="default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data: blob:; connect-src 'self'; frame-src 'self' about:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'";
    const proofPolicy=pagePolicy.replace("frame-ancestors 'none'","frame-ancestors 'self'");
    res.setHeader('Content-Security-Policy',pagePolicy);
    const reply=(status,value)=>{res.writeHead(status,{'Content-Type':'application/json'});res.end(JSON.stringify(value));};
    const host=req.headers.host,port=server.address()?.port;
    if(![`127.0.0.1:${port}`,`localhost:${port}`].includes(host))return reply(403,{error:'Unexpected Host.'});
    if(req.headers.origin&&req.headers.origin!==`http://${host}`)return reply(403,{error:'Cross-origin request refused.'});
    if(req.headers['sec-fetch-site']==='cross-site')return reply(403,{error:'Cross-site request refused.'});
    let pathname;try{pathname=new URL(req.url,`http://${host}`).pathname;}catch{return reply(400,{error:'Invalid URL.'});}
    try{
      if(req.method==='GET'&&pathname==='/example.json'&&exampleDesign)return reply(200,exampleDesign);
      if(req.method==='GET'&&pathname==='/api/session')return reply(200,{token,engine_available:runner!==runRust||existsSync(enginePath),mode:runner===runRust?'native-rust':'transport-test-fixture',profile_id:'generic-flat-120-v1'});
      if(req.method==='GET'&&pathname.startsWith('/api/artifacts/')){
        const match=pathname.match(/^\/api\/artifacts\/([a-f0-9]{64})\.(dst|json)$/);
        const item=match&&artifacts.get(match[1]);if(!item)return reply(404,{error:'Artifact expired or not found. Recompile the current design.'});
        if(match[2]==='json')return reply(200,item.bundle);
        res.writeHead(200,{'Content-Type':'application/octet-stream','Content-Disposition':'attachment; filename="twinlab-demo.dst"','X-DST-SHA256':item.bundle.provenance.dst_sha256});return res.end(item.bytes);
      }
      if(req.method==='POST'&&pathname==='/api/compile'){
        const candidate=Buffer.from(String(req.headers['x-twin-token']||'')),expected=Buffer.from(token);
        if(candidate.length!==expected.length||!timingSafeEqual(candidate,expected))return reply(403,{error:'Missing or invalid session token.'});
        if(req.headers['content-type']?.split(';')[0]!=='application/json')return reply(415,{error:'Use application/json.'});
        if(req.headers['transfer-encoding'])return reply(411,{error:'A bounded Content-Length is required.'});
        const len=Number(req.headers['content-length']);if(!Number.isInteger(len)||len<=0||len>LIMIT)return reply(413,{error:'Request must be between 1 byte and 256 KiB.'});
        if(active>=2)return reply(429,{error:'Two compiles are already active.'});
        active++;
        try{
          let size=0,parts=[];for await(const part of req){size+=part.length;if(size>LIMIT)throw new Error('Input exceeds 256 KiB.');parts.push(part);}
          if(size!==len)throw new Error('Incomplete request body.');
          const request=JSON.parse(Buffer.concat(parts).toString());
          const identity=engineSha||sha256(await readFile(enginePath));
          const trace=process.env.EMBROIDERY_WORKER_TRACE==='1'||req.headers['x-worker-trace']==='1';
          let worker=null;
          const sealed=sealResult(await runner(request,enginePath,trace?image=>{worker=image;}:undefined),request,identity,runner===runRust?'native-rust':'transport-test-fixture');
          if(trace){
            if(!worker||!Number.isInteger(worker.pid)||typeof worker.exe!=='string'||!/^[a-f0-9]{64}$/.test(worker.sha256||''))throw new Error('Worker identity was not recorded for this compile.');
            res.setHeader('X-Worker-Pid',String(worker.pid));
            res.setHeader('X-Worker-Exe',worker.exe);
            res.setHeader('X-Worker-Sha256',worker.sha256);
          }
          artifacts.set(sealed.bundle.artifact_id,sealed);
          while(artifacts.size>8)artifacts.delete(artifacts.keys().next().value);
          return reply(200,sealed.bundle);
        }catch(error){return reply(422,{error:error.message});}finally{active--;}
      }
      if(req.method==='GET'&&pathname.startsWith('/api/proofs/')){
        const match=pathname.match(/^\/api\/proofs\/([a-f0-9]{64})\.pdf$/);
        const item=match&&proofs.get(match[1]);
        if(!item)return reply(404,{error:'Proof expired or not found. Generate it again from the current revision.'});
        res.setHeader('Content-Security-Policy',proofPolicy);
        res.writeHead(200,{'Content-Type':'application/pdf','Content-Disposition':'inline; filename="compare-proof.pdf"','X-PDF-SHA256':item.binding.pdf_sha256});
        return res.end(item.pdf);
      }
      if(req.method==='POST'&&pathname==='/api/proof'){
        const candidate=Buffer.from(String(req.headers['x-twin-token']||'')),expected=Buffer.from(token);
        if(candidate.length!==expected.length||!timingSafeEqual(candidate,expected))return reply(403,{error:'Missing or invalid session token.'});
        if(req.headers['content-type']?.split(';')[0]!=='application/json')return reply(415,{error:'Use application/json.'});
        if(proving>=1)return reply(429,{error:'A proof is already being written.'});
        proving++;
        try{
          const body=JSON.parse((await readLimited(req,PROOF_LIMIT)).toString());
          const item=artifacts.get(body?.artifact_id);
          if(!item)return reply(404,{error:'Artifact expired or not found. Recompile the current design.'});
          if(typeof body.source_base64!=='string'||!/^[A-Za-z0-9+/]+={0,2}$/.test(body.source_base64))throw Object.assign(new Error('Source must be base64.'),{status:422});
          const source=Buffer.from(body.source_base64,'base64');
          if(source.length<1||source.length>8*1024*1024)throw Object.assign(new Error('Source must be between 1 byte and 8 MiB.'),{status:413});
          if(!existsSync(pythonPath)||!existsSync(proofScript))return reply(503,{error:'Proof utility is not available in this developer environment.'});
          const placement={width_mm:body.source_width_mm,x_mm:body.source_x_mm,y_mm:body.source_y_mm,page:body.page??1,confirmed:body.scale_confirmed===true};
          const written=await writeCompareProof({bundle:item.bundle,sourceBytes:source,sourceName:String(body.source_name||''),placement,pythonPath,scriptPath:proofScript});
          if(sha256(source)!==written.manifest.source.source_sha256)throw Object.assign(new Error('Source hash changed while the proof was written.'),{status:422});
          if(sha256(written.pdf)!==written.manifest.pdf_sha256)throw Object.assign(new Error('PDF hash does not match the proof manifest.'),{status:422});
          const binding={schema:'embroideryos-compare-binding/0.2',artifact_id:item.bundle.artifact_id,design_revision:item.bundle.design.revision,design_sha256:item.bundle.provenance.design_sha256,dst_sha256:item.bundle.provenance.dst_sha256,source_sha256:written.manifest.source.source_sha256,source_name:written.route.filename,source_role:'original',width_mm:placement.width_mm,height_mm:written.manifest.source.height_mm,x_mm:placement.x_mm,y_mm:placement.y_mm,page:placement.page,pdf_sha256:written.manifest.pdf_sha256,kernel_mode_reported:item.bundle.kernel.mode,machine_release_allowed:false,approval_allowed:false,coordinate_semantics_independently_verified:false,historical_source_association_verified:false,physical_qualification:false,actual_size_panel:written.manifest.actual_size_panel===true};
          const proofId=sha256(stable(binding));
          proofs.set(proofId,{pdf:written.pdf,binding});
          while(proofs.size>8)proofs.delete(proofs.keys().next().value);
          return reply(200,{proof_id:proofId,binding,checks:written.manifest.checks});
        }catch(error){return reply(error.status||422,{error:error.message||'Proof failed.'});}
        finally{proving--;}
      }
      if(req.method!=='GET')return reply(405,{error:'Method not allowed.'});
      if(!assets[pathname]){
        if(pathname.startsWith('/api/'))return reply(404,{error:'Not found.'});
        res.writeHead(404,{'Content-Type':'text/html; charset=utf-8'});
        return res.end('<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex"><meta name="description" content="That address is not part of the local TwinLab."><title>Page not found — EmbroideryOS</title><link rel="icon" href="/favicon.svg"><link rel="stylesheet" href="/style.css"></head><body><main class="page-heading"><h1>Page not found</h1><p>That address is not part of the local TwinLab.</p><p><a href="/">Back to TwinLab</a></p></main></body></html>');
      }
      const [file,type]=assets[pathname];const contents=await readFile(path.join(HERE,'public',file));res.writeHead(200,{'Content-Type':type});res.end(contents);
    }catch(error){reply(500,{error:'Local service failed. Check the terminal and engine build.'});}
  });
  server.requestTimeout=35000;server.headersTimeout=10000;server.maxHeadersCount=32;
  return server;
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
  const port=Number(process.env.PORT||4173);if(!Number.isInteger(port)||port<1024||port>65535)throw new Error('PORT must be 1024–65535.');
  const server=createTwinServer();server.listen(port,'127.0.0.1',()=>console.log(`TwinLab: http://127.0.0.1:${port} (local demo; no machine connection)`));
}
