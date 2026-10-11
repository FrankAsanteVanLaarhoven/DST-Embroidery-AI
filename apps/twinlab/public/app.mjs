import {Playback,clamp} from './playback.mjs';
import {MachineScene} from './scene.mjs';
import {bindingMatches,drawCompare,filenameRoute,mmPerPixel,placementValid} from './compare.mjs';
const $=id=>document.getElementById(id);
const clock=new Playback();
let design=null,selected=1,vertex=0,bundle=null,session=null,serial=0,dirty=true,redraw=true,compileTimer=null,history=[],future=[],dark=false,scene=null,cameraIndex=0;
let original=null,originalImage=null,proof=null,proofBusy=false,compareView='original',compareCamera={panX:0,panY:0,zoom:null},overlay={opacity:.55,wipe:1,independent:false,colour:'all',flicker:false},colourKey='';
const copy=value=>JSON.parse(JSON.stringify(value));
const rgbToHex=rgb=>'#'+rgb.map(v=>v.toString(16).padStart(2,'0')).join('');
const hexToRgb=hex=>[1,3,5].map(i=>parseInt(hex.slice(i,i+2),16));
const timeString=t=>`${String(Math.floor(t/60000)).padStart(2,'0')}:${(t/1000%60).toFixed(1).padStart(4,'0')}`;
const message=(text,error=false)=>{$('message').textContent=text;$('message').classList.toggle('error',error);};
const current=()=>design.objects.find(o=>o.id===selected)||design.objects[0];
function saveFile(name,bytes,type){const u=URL.createObjectURL(new Blob([bytes],{type}));const a=document.createElement('a');a.href=u;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(u),1000);}
function stage(id,text,passed=false){const li=$('stage-'+id);li.querySelector('span').textContent=text;li.classList.toggle('passed',passed);}
function updateControls(){
 const ready=!!bundle&&!dirty;
 for(const id of ['play','start','back','next','scrub','download','evidence'])$(id).disabled=!ready;
 $('undo').disabled=!history.length;$('redo').disabled=!future.length;
 $('remove').disabled=design.objects.length<2;
 $('revision').textContent=`Revision ${design.revision}`;
 $('play').textContent=clock.playing?'Ⅱ':'▶';
 $('mode').textContent=session?.mode==='transport-test-fixture'?'Recorded codec fixture · NOT live Rust':ready?'Native Rust · byte identity verified':dirty?'Draft · recompilation required':'Native Rust';
 renderCompare();
}
function invalidate(){
 clearTimeout(compileTimer);serial++;$('compile').disabled=false;dirty=true;bundle=null;proof=null;clock.playing=false;clock.trace=null;clock.time=0;scene?.setBundle(null);
 clearProofFrame();
 for(const s of ['rust','export','decode','sync'])stage(s,'Pending');
 for(const id of ['stitches-stat','records-stat','size-stat','colours-stat'])$(id).textContent='—';
 $('hash').textContent='Current design has not been compiled';$('integrity').textContent='Previous export/replay invalidated';$('colour-stops').replaceChildren();
 redraw=true;updateControls();
 if(session?.engine_available&&session.mode==='native-rust')compileTimer=setTimeout(compileDesign,450);
}
function edit(action){history.push(copy(design));if(history.length>50)history.shift();future=[];action();design.revision++;invalidate();renderInspector();renderObjects();persist();}
function persist(){try{localStorage.setItem('twinlab-m1-design',JSON.stringify(design));}catch{message('Browser storage unavailable. Use Save design to retain changes.');}}
function renderObjects(){
 $('name').value=design.name;$('object-count').textContent=design.objects.length;
 const fragment=document.createDocumentFragment();
 for(const o of design.objects){const b=document.createElement('button');b.className='object'+(o.id===selected?' selected':'');b.setAttribute('aria-pressed',String(o.id===selected));b.dataset.objectId=o.id;
  const swatch=document.createElement('span');swatch.className='swatch';swatch.style.background=rgbToHex(o.rgb);
  const label=document.createElement('span');label.className='object-title';label.textContent=o.label;
  const kind=document.createElement('span');kind.className='object-kind';kind.textContent=`${o.kind} · ${o.points.length} points`;label.append(kind);b.append(swatch,label);
  b.onclick=()=>{selected=o.id;vertex=0;renderObjects();renderInspector();redraw=true;};fragment.append(b);
 }
 $('objects').replaceChildren(fragment);updateControls();
}
function field(label,type,value,onChange,{min,max,step,id}={}){
 const wrap=document.createElement('label');wrap.className='field';wrap.textContent=label;const input=document.createElement('input');input.type=type;input.value=value;
 if(min!==undefined)input.min=min;if(max!==undefined)input.max=max;if(step!==undefined)input.step=step;if(id)input.id=id;
 // Replacing the inspector may blur this element and dispatch change again.
 // Commit the value before invoking the edit so duplicate events cannot re-enter DOM replacement.
 const read=()=>type==='number'?Number(input.value):input.value;let committed=read();
 input.addEventListener('change',()=>{const next=read();if(Object.is(next,committed))return;committed=next;onChange(next);});wrap.append(input);return wrap;
}
function renderInspector(){
 const o=current();vertex=clamp(vertex,0,o.points.length-1);const grid=document.createElement('div');grid.className='inspector-grid';
 const label=field('Object label','text',o.label,v=>edit(()=>current().label=v));label.classList.add('full');grid.append(label);
 grid.append(field('Thread colour','color',rgbToHex(o.rgb),v=>edit(()=>current().rgb=hexToRgb(v)),{id:'object-color'}));
 grid.append(field('Max stitch · mm','number',o.max_stitch_mm,v=>edit(()=>current().max_stitch_mm=v),{min:.5,max:10,step:.1,id:'max-stitch'}));
 if(o.kind!=='running')grid.append(field('Spacing · mm','number',o.spacing_mm,v=>edit(()=>current().spacing_mm=v),{min:.2,max:4,step:.1,id:'spacing'}));
 if(o.kind==='fill')grid.append(field('Fill angle · °','number',o.angle_degrees,v=>edit(()=>current().angle_degrees=v),{min:-360,max:360,step:5,id:'angle'}));
 if(o.kind==='satin')grid.append(field('Column width · mm','number',o.width_mm,v=>edit(()=>current().width_mm=v),{min:.5,max:10,step:.1,id:'width'}));
 if(o.kind!=='running')grid.append(field('Underlay · mm; 0 off','number',o.underlay_mm,v=>edit(()=>current().underlay_mm=v),{min:0,max:10,step:.2,id:'underlay'}));
 const pointLabel=document.createElement('label');pointLabel.className='field full';pointLabel.textContent='Control point';const pointSelect=document.createElement('select');pointSelect.id='vertex';
 o.points.forEach((_,i)=>{const option=new Option(`Point ${i+1}`,String(i));option.selected=i===vertex;pointSelect.append(option);});pointSelect.onchange=()=>{vertex=Number(pointSelect.value);renderInspector();redraw=true;};pointLabel.append(pointSelect);grid.append(pointLabel);
 grid.append(field('X · mm','number',o.points[vertex].x,v=>edit(()=>current().points[vertex].x=v),{min:-60,max:60,step:1,id:'point-x'}));
 grid.append(field('Y · mm','number',o.points[vertex].y,v=>edit(()=>current().points[vertex].y=v),{min:-60,max:60,step:1,id:'point-y'}));
 $('inspector').replaceChildren(grid);
}
async function digest(bytes){const hash=await crypto.subtle.digest('SHA-256',bytes);return [...new Uint8Array(hash)].map(b=>b.toString(16).padStart(2,'0')).join('');}
async function compileDesign(){
 clearTimeout(compileTimer);if(!design||!session)return;
 const ticket=++serial,snapshot=copy(design);$('compile').disabled=true;message(session.mode==='native-rust'?'Generating in Rust, compiling exact DST bytes and decoding the export…':'Loading the repository’s recorded square fixture. This does not exercise Rust generation.');
 try{
  const response=await fetch('/api/compile',{method:'POST',headers:{'Content-Type':'application/json','X-Twin-Token':session.token},body:JSON.stringify({profile_id:'generic-flat-120-v1',design:snapshot})});
  const value=await response.json();if(!response.ok)throw new Error(value.error||'Compilation failed.');
  const bytes=Uint8Array.from(atob(value.dst_base64),c=>c.charCodeAt(0));
  if(await digest(bytes)!==value.provenance.dst_sha256)throw new Error('Downloaded DST bytes failed their SHA-256 check. Replay refused.');
  if(ticket!==serial)return;
  if(value.design.revision!==design.revision)throw new Error('Stale design revision returned. Replay refused.');
  bundle=value;dirty=false;clock.load(value.trace);scene?.setBundle(value);
  stage('design',`r${design.revision}`,true);stage('rust',session.mode==='native-rust'?'Generated':'Not exercised',session.mode==='native-rust');
  stage('export','SHA-256 checked',true);stage('decode',session.mode==='native-rust'?'Raw records':'Recorded fixture',session.mode==='native-rust');stage('sync','One clock',true);
  $('stitches-stat').textContent=value.trace.needle_placements.toLocaleString();$('records-stat').textContent=value.trace.record_count.toLocaleString();$('size-stat').textContent=bytes.length.toLocaleString();$('colours-stat').textContent=value.palette.length;
  $('hash').textContent=value.provenance.dst_sha256;$('integrity').textContent='Verified in this browser · not a signed release';
  $('scrub').max=value.trace.duration_ms;$('scrub').value=0;$('colour-stops').replaceChildren(...value.palette.map(p=>{const el=document.createElement('span');el.className='stop';el.style.background=rgbToHex(p.rgb);return el;}));
  $('engine-version').textContent=`${value.kernel.generation} → ${value.kernel.export} · ${value.kernel.version}`;
  message(session.mode==='native-rust'?`Compiled ${value.trace.record_count.toLocaleString()} exact DST records. Playback uses decoded bytes, not design geometry.`:'Fixture replay only. Editing, compilation and Rust execution are not validated by this view.');
  redraw=true;updateControls();
 }catch(error){if(ticket===serial){message(error.message,true);dirty=true;bundle=null;clock.playing=false;clock.trace=null;scene?.setBundle(null);for(const id of ['rust','export','decode','sync'])stage(id,'Blocked');redraw=true;updateControls();}}
 finally{if(ticket===serial)$('compile').disabled=false;}
}
function canvasTransform(){const c=$('design-canvas'),w=c.clientWidth,h=c.clientHeight,scale=Math.max(.1,Math.min(w-44,h-90)/140);return {w,h,scale,cx:w/2,cy:h/2+7};}
function draw2D(state){
 const canvas=$('design-canvas'),ctx=canvas.getContext('2d'),{w,h,scale,cx,cy}=canvasTransform(),dpr=Math.min(devicePixelRatio||1,2);
 if(canvas.width!==Math.round(w*dpr)||canvas.height!==Math.round(h*dpr)){canvas.width=Math.max(1,Math.round(w*dpr));canvas.height=Math.max(1,Math.round(h*dpr));}
 ctx.setTransform(dpr,0,0,dpr,0,0);ctx.fillStyle=dark?'#161c25':'#f0f3f8';ctx.fillRect(0,0,w,h);
 const at=p=>[cx+p[0]*scale,cy+p[1]*scale];
 ctx.fillStyle=dark?'#232b35':'#fffcf6';ctx.fillRect(cx-60*scale,cy-60*scale,120*scale,120*scale);
 for(let n=-60;n<=60;n+=10){ctx.strokeStyle=n===0?(dark?'#526178':'#bcc9dc'):(dark?'#303b4a':'#dfe5ef');ctx.lineWidth=n===0?1:.6;ctx.beginPath();ctx.moveTo(cx+n*scale,cy-60*scale);ctx.lineTo(cx+n*scale,cy+60*scale);ctx.moveTo(cx-60*scale,cy+n*scale);ctx.lineTo(cx+60*scale,cy+n*scale);ctx.stroke();}
 ctx.strokeStyle=dark?'#8295ae':'#99abc2';ctx.lineWidth=1;ctx.setLineDash([4,4]);ctx.strokeRect(cx-60*scale,cy-60*scale,120*scale,120*scale);ctx.setLineDash([]);
 ctx.font='8px -apple-system, sans-serif';ctx.fillStyle=dark?'#a9b5c8':'#708199';for(const n of [-60,-30,0,30,60]){ctx.fillText(String(n),cx+n*scale-4,cy-60*scale-6);ctx.fillText(String(n),cx-60*scale-21,cy+n*scale+3);}
 for(const o of design.objects){ctx.beginPath();o.points.forEach((p,i)=>{const q=at([p.x,p.y]);i?ctx.lineTo(...q):ctx.moveTo(...q);});if(o.kind==='fill'){ctx.closePath();ctx.fillStyle=rgbToHex(o.rgb)+(bundle?'0c':'25');ctx.fill();}ctx.strokeStyle=rgbToHex(o.rgb);ctx.globalAlpha=bundle?.5:.9;ctx.lineWidth=o.id===selected?1.5:1;ctx.stroke();ctx.globalAlpha=1;}
 if(bundle){
  let sewn=0;for(const e of bundle.trace.events){if(e.kind!=='stitch')continue;const complete=sewn<state.completed_stitches;sewn++;ctx.strokeStyle=rgbToHex(bundle.palette[e.color_index].rgb);ctx.globalAlpha=complete?.94:.13;ctx.lineWidth=complete?1.1:.6;ctx.beginPath();ctx.moveTo(...at(e.from_units.map(n=>n/10)));ctx.lineTo(...at(e.to_units.map(n=>n/10)));ctx.stroke();}ctx.globalAlpha=1;
  const p=at(state.position_mm);ctx.strokeStyle=dark?'#f6f9ff':'#1a2a41';ctx.lineWidth=1.4;ctx.beginPath();ctx.arc(...p,5,0,Math.PI*2);ctx.moveTo(p[0]-9,p[1]);ctx.lineTo(p[0]+9,p[1]);ctx.moveTo(p[0],p[1]-9);ctx.lineTo(p[0],p[1]+9);ctx.stroke();
 }
 const o=current();for(let i=0;i<o.points.length;i++){const p=at([o.points[i].x,o.points[i].y]);ctx.beginPath();ctx.arc(...p,i===vertex?4:3,0,Math.PI*2);ctx.fillStyle=i===vertex?'#2464da':(dark?'#20252e':'#fff');ctx.fill();ctx.strokeStyle='#2464da';ctx.lineWidth=1.1;ctx.stroke();}
 canvas.dataset.eventIndex=String(state.event_index);canvas.dataset.completedStitches=String(state.completed_stitches);
}
function paint(state){
 draw2D(state);scene?.render(state,dark);$('event-label').textContent=bundle?state.kind.replace('_',' ').toUpperCase():'NOT COMPILED';$('time-label').textContent=`${timeString(state.time_ms)} / ${timeString(bundle?.trace.duration_ms||0)}`;
 $('record-value').textContent=bundle?`${state.event_index+1} / ${bundle.trace.record_count}`:'—';$('x-value').textContent=state.position_mm[0].toFixed(1);$('y-value').textContent=state.position_mm[1].toFixed(1);$('needle-value').textContent=bundle?.palette[state.color_index]?.needle||1;
 $('bytes-value').textContent=state.raw?state.raw.map(n=>n.toString(16).padStart(2,'0')).join(' ').toUpperCase():'—';$('sync-value').textContent=bundle?(scene?.available?'2D ↔ 3D synchronised':'2D only · WebGL unavailable'):'No replay artifact';$('scrub').value=state.time_ms;$('play').textContent=clock.playing?'Ⅱ':'▶';
 drawOverlay(state);
 if($('twin-event'))$('twin-event').textContent=bundle&&!dirty?`Shared clock · record ${state.event_index+1} / ${bundle.trace.record_count} · ${state.kind}`:'Waiting for a current trace.';
 redraw=false;
}
function readPlacement(){return {width_mm:Number($('source-width').value),x_mm:Number($('source-x').value),y_mm:Number($('source-y').value),page:Number($('source-page').value),confirmed:$('scale-confirm').checked};}
function compareFacts(){return {revision:design?.revision,artifactId:bundle?.artifact_id||null,dstSha:bundle?.provenance?.dst_sha256||null,sourceSha:original?.sha256||null,placement:readPlacement()};}
function scaleNote(){
 const dpi=Number($('source-dpi').value);
 if(!originalImage?.naturalWidth||!Number.isFinite(dpi)||dpi<=0)return 'Nominal sampling is 25.4 ÷ dpi millimetres per pixel. 254 dpi is 0.1 mm per pixel. Enter the physical width. DPI is not assumed from the file.';
 let nominal;
 try{nominal=originalImage.naturalWidth*mmPerPixel(dpi);}catch{return 'Enter a positive dpi. Sampling pitch is not scanner accuracy.';}
 const entered=Number($('source-width').value),gap=Math.abs(nominal-entered);
 if(Number.isFinite(entered)&&gap>0.5)return `Nominal width at ${dpi} dpi is ${nominal.toFixed(2)} mm. The entered width differs by ${gap.toFixed(2)} mm. Confirm the physical width before a proof.`;
 return `Nominal width at ${dpi} dpi is ${nominal.toFixed(2)} mm. This is sampling pitch, not scanner accuracy.`;
}
function syncColourOptions(){
 const key=bundle?bundle.palette.map(entry=>entry.rgb.join(',')).join('|'):'';
 if(key===colourKey)return;
 colourKey=key;const select=$('overlay-colour'),current=select.value;
 select.replaceChildren(new Option('All colours','all'));
 if(bundle)bundle.palette.forEach((entry,index)=>select.append(new Option(`Needle ${entry.needle}`,String(index))));
 select.value=[...select.options].some(option=>option.value===current)?current:'all';
 overlay.colour=select.value==='all'?'all':Number(select.value);
}
function showCompare(view){
 compareView=view;
 document.querySelectorAll('[data-compare]').forEach(button=>{const on=button.dataset.compare===view;button.classList.toggle('chosen',on);button.setAttribute('aria-selected',String(on));});
 document.querySelectorAll('[data-compare-pane]').forEach(pane=>{pane.hidden=pane.dataset.comparePane!==view;});
 $('compare-canvas').hidden=view==='pdf';
 redraw=true;
}
function renderCompare(){
 if(!$('compare-status')||!design)return;
 syncColourOptions();
 const placement=readPlacement(),facts=compareFacts(),current=!!(proof&&!dirty&&bindingMatches(proof.binding,facts));
 $('original-name').textContent=original?original.name:'None';
 $('original-hash').textContent=original?original.sha256:'—';
 $('original-route').textContent=original?original.route.action:'PNG, JPEG, WebP, TIFF, BMP, or a source PDF. A .dts upload keeps its name.';
 $('scale-note').textContent=scaleNote();
 stage('original',original?'Retained':'None',!!original);
 stage('proof',current?'Bound':'None',current);
 const list=$('editable-list');
 list.replaceChildren(...design.objects.map(object=>{const item=document.createElement('li');item.textContent=`${object.label} · ${object.kind} · ${object.points.length} points · colour ${object.rgb.join(',')}`;return item;}));
 if(!bundle||dirty){
  $('machine-summary').textContent='Compile this revision to decode its DST. A previous export is not the current machine file.';
  $('machine-window').textContent='No current machine file.';
  $('twin-summary').textContent='The machine twin follows the current export. Recompile before treating the 3D view as this revision.';
 }else{
  $('machine-summary').textContent=`DST · ${bundle.trace.record_count.toLocaleString()} records · ${bundle.trace.needle_placements.toLocaleString()} needle placements · SHA-256 ${bundle.provenance.dst_sha256}. Object names and the full palette are sidecar data. Standard DST does not carry them.`;
  const events=bundle.trace.events,shown=events.length>7?[...events.slice(0,6),events.at(-1)]:events;
  $('machine-window').textContent=shown.map(event=>`#${event.index} ${event.kind} raw ${event.raw.join(',')} to ${event.to_units.join(',')} @ ${event.byte_offset}`).join('\n')+(events.length>7?`\n… ${events.length-7} records omitted from this window. Replay uses every record.`:'');
  $('twin-summary').textContent=`Profile ${bundle.profile.id}. Artifact ${bundle.artifact_id}. Hardware is not connected.`;
 }
 const sourceReady=!!original&&(original.route.route==='source_raster'||original.route.route==='source_document');
 $('proof-generate').disabled=proofBusy||dirty||!bundle||!sourceReady||!placementValid(placement);
 $('proof-download').disabled=!current;
 $('proof-reopen').disabled=!current;
 if(current){
  $('compare-status').textContent=`Bound to revision ${design.revision}. DST ${bundle.provenance.dst_sha256.slice(0,12)} and original ${original.sha256.slice(0,12)}. Approval and machine release remain unavailable.`;
  $('proof-status').textContent=proof.binding.actual_size_panel?'The stored PDF includes a 1:1 panel. Print at Actual size / 100% and measure the 100 mm bar. This screen is not that measurement.':'The design does not fit the 1:1 panel. It was not shrunk. Use a tiled proof before treating the page as actual size.';
 }else if(original?.route.route==='unrecognised'){
  $('compare-status').textContent=`${original.name} is retained. It is not renamed to .dst and it is not executed.`;
  clearProofFrame();
 }else if(dirty){
  $('compare-status').textContent=`Revision ${design.revision} is not compiled. The machine file, PDF, and twin are not current.`;
  clearProofFrame();
 }else if(!original){
  $('compare-status').textContent='No original is attached. The design and DST are not evidence of source artwork.';
  clearProofFrame();
 }else{
  $('compare-status').textContent='Original retained. Confirm its physical width, then generate a proof for this revision.';
  clearProofFrame();
 }
}
function mountProof(url){
 const slot=$('proof-slot');if(!slot)return;
 const frame=document.createElement('iframe');
 frame.id='proof-frame';frame.className='proof-frame';frame.title='Reopened PDF proof';frame.src=url;
 slot.replaceChildren(frame);
}
function clearProofFrame(){const slot=$('proof-slot');if(slot)slot.replaceChildren();}
function drawOverlay(state){
 if($('compare-canvas').hidden)return;
 drawCompare($('compare-canvas'),{dark,design,bundle:bundle&&!dirty?bundle:null,image:originalImage,placement:readPlacement(),showSource:!!originalImage&&!overlay.flicker,showDesign:true,showTrace:!!bundle&&!dirty,opacity:Number($('overlay-opacity').value)/100,wipe:Number($('overlay-wipe').value)/100,colour:overlay.colour,followClock:!overlay.independent,completedStitches:state.completed_stitches,camera:compareCamera});
}
function bytesToBase64(bytes){let binary='';for(let i=0;i<bytes.length;i+=0x8000)binary+=String.fromCharCode(...bytes.subarray(i,i+0x8000));return btoa(binary);}
async function adoptOriginal(name,bytes){
 if(bytes.length>8*1024*1024)throw new Error('Original exceeds 8 MiB. Attach a smaller file.');
 const route=filenameRoute(name);
 if(original?.previewUrl)URL.revokeObjectURL(original.previewUrl);
 const sha=await digest(bytes);
 let previewUrl=null;originalImage=null;
 if(route.route==='source_raster'){previewUrl=URL.createObjectURL(new Blob([bytes]));originalImage=new Image();originalImage.onload=()=>{redraw=true;renderCompare();};originalImage.src=previewUrl;}
 original={name:route.filename,bytes,sha256:sha,route,previewUrl};
 $('scale-confirm').checked=false;proof=null;clearProofFrame();
 const preview=$('original-preview');
 if(previewUrl){preview.src=previewUrl;preview.hidden=false;preview.alt=`Original artwork, ${route.filename}`;}
 else{preview.removeAttribute('src');preview.hidden=true;preview.alt='';}
 renderCompare();redraw=true;
}
async function generateProof(){
 if($('proof-generate').disabled)return;
 const placement=readPlacement();
 const expected={revision:design.revision,artifact:bundle.artifact_id,source:original.sha256};
 proofBusy=true;renderCompare();
 message('Writing the PDF from this revision’s stored DST and the attached original. Rust is not run again, and the coordinates are not re-decoded.');
 try{
  const response=await fetch('/api/proof',{method:'POST',headers:{'Content-Type':'application/json','X-Twin-Token':session.token},body:JSON.stringify({artifact_id:expected.artifact,source_name:original.name,source_base64:bytesToBase64(original.bytes),source_width_mm:placement.width_mm,source_x_mm:placement.x_mm,source_y_mm:placement.y_mm,page:placement.page,scale_confirmed:true})});
  const value=await response.json();
  if(!response.ok)throw new Error(value.error||'Proof failed.');
  if(dirty||design.revision!==expected.revision||bundle?.artifact_id!==expected.artifact||original?.sha256!==expected.source){message('The design or original changed while the proof was written. That PDF is not current.',true);return;}
  proof={id:value.proof_id,binding:value.binding};
  mountProof(`/api/proofs/${value.proof_id}.pdf`);
  showCompare('pdf');
  message(value.binding.actual_size_panel?'PDF proof stored and reopened. Print it at Actual size / 100%.':'PDF proof stored. The 1:1 panel was withheld because the design does not fit. It was not shrunk.');
 }catch(error){message(error.message,true);}
 finally{proofBusy=false;renderCompare();redraw=true;}
}
async function reopenProof(){
 if(!proof||dirty||!bindingMatches(proof.binding,compareFacts()))return;
 try{
  const response=await fetch(`/api/proofs/${proof.id}.pdf`);
  if(!response.ok)throw new Error('Stored proof is no longer available. Generate it again.');
  const bytes=new Uint8Array(await response.arrayBuffer());
  if(await digest(bytes)!==proof.binding.pdf_sha256)throw new Error('Reopened PDF does not match the stored hash. It was not shown as current.');
  if(dirty||!bindingMatches(proof.binding,compareFacts()))throw new Error('The revision changed. This PDF is not current.');
  mountProof(`/api/proofs/${proof.id}.pdf?opened=${proof.binding.pdf_sha256.slice(0,12)}`);
  showCompare('pdf');
  message('Reopened the stored PDF bytes. The hash matches this revision.');
 }catch(error){message(error.message,true);}
}
async function downloadProof(){
 if(!proof||dirty||!bindingMatches(proof.binding,compareFacts()))return;
 const id=proof.id,expected=proof.binding.pdf_sha256;
 try{
  const response=await fetch(`/api/proofs/${id}.pdf`);
  if(!response.ok)throw new Error('Stored proof is no longer available. Generate it again.');
  const bytes=new Uint8Array(await response.arrayBuffer());
  if(await digest(bytes)!==expected)throw new Error('PDF hash mismatch. Download refused.');
  if(proof?.id!==id||dirty||!bindingMatches(proof.binding,compareFacts()))throw new Error('The revision changed. This PDF is not current.');
  saveFile(`${design.name}-proof.pdf`,bytes,'application/pdf');
 }catch(error){message(error.message,true);}
}
function wireUI(){
 $('compile').onclick=compileDesign;$('name').onchange=()=>edit(()=>design.name=$('name').value);
 for(const kind of ['fill','satin','running'])$('add-'+kind).onclick=()=>edit(()=>{const id=Math.max(...design.objects.map(o=>o.id))+1;design.objects.push({id,label:`New ${kind}`,kind,points:kind==='fill'?[{x:-15,y:-15},{x:15,y:-15},{x:15,y:15},{x:-15,y:15}]:[{x:-20,y:0},{x:20,y:0}],rgb:[55,112,211],spacing_mm:.6,angle_degrees:30,max_stitch_mm:3,width_mm:3,compensation_mm:.1,underlay_mm:0});selected=id;vertex=0;});
 $('remove').onclick=()=>edit(()=>{design.objects=design.objects.filter(o=>o.id!==selected);selected=design.objects[0].id;vertex=0;});
 for(const [id,offset] of [['up',-1],['down',1]])$(id).onclick=()=>{const i=design.objects.findIndex(o=>o.id===selected),j=i+offset;if(j>=0&&j<design.objects.length)edit(()=>{[design.objects[i],design.objects[j]]=[design.objects[j],design.objects[i]];});};
 for(const [id,source,target] of [['undo',history,future],['redo',future,history]])$(id).onclick=()=>{const from=id==='undo'?history:future,to=id==='undo'?future:history;if(!from.length)return;const rev=design.revision;to.push(copy(design));design=from.pop();design.revision=rev+1;selected=design.objects[0].id;vertex=0;invalidate();renderObjects();renderInspector();persist();};
 $('save').onclick=()=>saveFile(`${design.name}.json`,JSON.stringify(design,null,2),'application/json');$('open').onclick=()=>$('file-input').click();
 $('file-input').onchange=async()=>{try{const file=$('file-input').files[0];if(!file)return;if(file.size>262144)throw new Error('Native project exceeds 256 KiB.');const d=JSON.parse(await file.text());if(d.schema_version!==1||!Array.isArray(d.objects)||!d.objects.length||d.objects.length>16||!d.objects.every(o=>Array.isArray(o.points)&&o.points.length>=2&&o.points.length<=256&&o.points.every(p=>Number.isFinite(p.x)&&Number.isFinite(p.y))&&Array.isArray(o.rgb)&&o.rgb.length===3&&o.rgb.every(n=>Number.isInteger(n)&&n>=0&&n<=255)))throw new Error('Invalid native design. SVG, image and machine-file import are not part of M1.');history.push(copy(design));future=[];design=d;selected=d.objects[0].id;vertex=0;invalidate();renderObjects();renderInspector();persist();}catch(e){message(e.message,true);}finally{$('file-input').value='';}};
 $('play').onclick=()=>{if(!bundle)return;if(clock.time>=bundle.trace.duration_ms)clock.seek(0);clock.playing=!clock.playing;clock.last=null;redraw=true;updateControls();};
 $('start').onclick=()=>{clock.playing=false;clock.seek(0);redraw=true;};$('back').onclick=()=>{clock.step(-1);redraw=true;};$('next').onclick=()=>{clock.step(1);redraw=true;};
 $('scrub').oninput=()=>{clock.playing=false;clock.seek(Number($('scrub').value));redraw=true;};$('speed').onchange=()=>clock.speed=Number($('speed').value);
 $('download').onclick=async()=>{if(!bundle||dirty)return;const id=bundle.artifact_id,expected=bundle.provenance.dst_sha256;try{const response=await fetch(`/api/artifacts/${id}.dst`);if(!response.ok)throw new Error('Artifact expired. Recompile.');const bytes=new Uint8Array(await response.arrayBuffer());if(await digest(bytes)!==expected)throw new Error('Export hash mismatch. Download refused.');if(bundle?.artifact_id!==id||dirty)throw new Error('Design changed during export. Recompile.');saveFile(`${design.name}.dst`,bytes,'application/octet-stream');}catch(e){message(e.message,true);}};
 $('evidence').onclick=()=>bundle&&!dirty&&saveFile(`${design.name}.replay.json`,JSON.stringify(bundle,null,2),'application/json');
 $('theme').onclick=()=>{dark=!dark;document.body.classList.toggle('dark',dark);redraw=true;};
 document.querySelectorAll('[data-view]').forEach(button=>button.onclick=()=>{document.querySelectorAll('[data-view]').forEach(b=>b.classList.toggle('chosen',b===button));$('viewports').className='viewports '+(button.dataset.view==='split'?'split':'only-'+button.dataset.view);redraw=true;});
 const cameras=['orbit','top','front'];$('camera').onclick=()=>{cameraIndex=(cameraIndex+1)%3;scene?.setView(cameras[cameraIndex]);$('camera').textContent='Camera: '+cameras[cameraIndex];redraw=true;};
 $('wire').onclick=()=>{if(scene){scene.wireframe=!scene.wireframe;$('wire').setAttribute('aria-pressed',scene.wireframe);redraw=true;}};
 $('fullscreen').onclick=async()=>{try{await $('viewports').parentElement.requestFullscreen();redraw=true;}catch{message('Fullscreen is unavailable in this browser.');}};
 new ResizeObserver(()=>redraw=true).observe($('viewports'));
 let drag=null;const canvas=$('design-canvas');
 const position=e=>{const r=canvas.getBoundingClientRect(),t=canvasTransform();return [(e.clientX-r.left-t.cx)/t.scale,(e.clientY-r.top-t.cy)/t.scale];};
 canvas.onpointerdown=e=>{const p=position(e),t=canvasTransform();let hit=null;for(const o of design.objects)for(let i=0;i<o.points.length;i++){const q=o.points[i];if(Math.hypot(q.x-p[0],q.y-p[1])*t.scale<9)hit=[o.id,i];}if(!hit)return;history.push(copy(design));future=[];selected=hit[0];vertex=hit[1];drag=true;canvas.setPointerCapture(e.pointerId);clearTimeout(compileTimer);invalidate();clearTimeout(compileTimer);renderObjects();renderInspector();};
 canvas.onpointermove=e=>{if(!drag)return;const p=position(e);const q=current().points[vertex];q.x=Math.round(clamp(p[0],-60,60)*10)/10;q.y=Math.round(clamp(p[1],-60,60)*10)/10;redraw=true;};
 const finish=()=>{if(!drag)return;drag=null;design.revision++;invalidate();renderInspector();renderObjects();persist();};canvas.onpointerup=finish;canvas.onpointercancel=finish;
 document.addEventListener('keydown',e=>{if(['INPUT','TEXTAREA','SELECT'].includes(e.target.tagName))return;if(e.code==='Space'){e.preventDefault();$('play').click();}if((e.ctrlKey||e.metaKey)&&e.key==='z'){e.preventDefault();$(e.shiftKey?'redo':'undo').click();}});
 $('proof-generate').onclick=generateProof;$('proof-reopen').onclick=reopenProof;$('proof-download').onclick=downloadProof;
 $('choose-original').onclick=()=>$('original-input').click();
 $('original-input').onchange=async()=>{try{const file=$('original-input').files[0];if(!file)return;await adoptOriginal(file.name,new Uint8Array(await file.arrayBuffer()));message(original.route.route==='unrecognised'?original.route.action:'Original retained. Later crops and redraws stay separate from these bytes. Confirm the physical width before a proof.');}catch(error){message(error.message,true);}finally{$('original-input').value='';}};
 $('attach-reference').onclick=async()=>{try{const response=await fetch('/reference-square.png');if(!response.ok)throw new Error('Engineering reference did not load.');await adoptOriginal('reference-square.png',new Uint8Array(await response.arrayBuffer()));$('source-width').value='14';$('source-x').value='-2';$('source-y').value='-2';$('scale-confirm').checked=false;renderCompare();redraw=true;message(design?.name==='Square'?'14 mm test reference attached at -2, -2 mm. It is a constructed fixture. Confirm the width before a proof.':'14 mm test reference attached. It is a constructed fixture, and it is not evidence that it produced the open design. Confirm the width before a proof.');}catch(error){message(error.message,true);}};
 $('open-square').onclick=async()=>{try{const response=await fetch('/square.json');if(!response.ok)throw new Error('Engineering square did not load.');const next=await response.json();history.push(copy(design));future=[];design=next;selected=next.objects[0].id;vertex=0;invalidate();renderObjects();renderInspector();persist();message('Engineering square opened. Rust must compile it before the machine file, PDF, or twin can bind to this revision.');}catch(error){message(error.message,true);}};
 document.querySelectorAll('[data-compare]').forEach(button=>button.onclick=()=>showCompare(button.dataset.compare));
 $('compare-tabs').addEventListener('keydown',event=>{if(event.key!=='ArrowRight'&&event.key!=='ArrowLeft')return;const buttons=[...$('compare-tabs').querySelectorAll('[role="tab"]')];const index=Math.max(0,buttons.findIndex(button=>button.dataset.compare===compareView));const next=(index+(event.key==='ArrowRight'?1:-1)+buttons.length)%buttons.length;event.preventDefault();buttons[next].focus();showCompare(buttons[next].dataset.compare);});
 for(const id of ['source-width','source-x','source-y','source-page','source-dpi'])$(id).addEventListener('input',()=>{renderCompare();redraw=true;});
 $('scale-confirm').addEventListener('change',()=>{renderCompare();redraw=true;});
 $('overlay-opacity').addEventListener('input',()=>{overlay.opacity=Number($('overlay-opacity').value)/100;redraw=true;});
 $('overlay-wipe').addEventListener('input',()=>{overlay.wipe=Number($('overlay-wipe').value)/100;redraw=true;});
 $('overlay-independent').addEventListener('change',()=>{overlay.independent=$('overlay-independent').checked;redraw=true;});
 $('overlay-colour').addEventListener('change',()=>{overlay.colour=$('overlay-colour').value==='all'?'all':Number($('overlay-colour').value);redraw=true;});
 $('overlay-flicker').onclick=()=>{overlay.flicker=!overlay.flicker;$('overlay-flicker').setAttribute('aria-pressed',String(overlay.flicker));redraw=true;};
 $('compare-reset').onclick=()=>{compareCamera={panX:0,panY:0,zoom:null};redraw=true;};
 let comparePan=null;const overlayCanvas=$('compare-canvas');
 overlayCanvas.onpointerdown=event=>{comparePan={x:event.clientX,y:event.clientY};overlayCanvas.setPointerCapture(event.pointerId);};
 overlayCanvas.onpointermove=event=>{if(!comparePan)return;const scale=Number(overlayCanvas.dataset.pxPerMm)||1;compareCamera.panX+=(event.clientX-comparePan.x)/scale;compareCamera.panY+=(event.clientY-comparePan.y)/scale;comparePan={x:event.clientX,y:event.clientY};redraw=true;};
 const endPan=()=>{comparePan=null;};overlayCanvas.onpointerup=endPan;overlayCanvas.onpointercancel=endPan;
 overlayCanvas.addEventListener('wheel',event=>{event.preventDefault();const current=compareCamera.zoom||Number(overlayCanvas.dataset.pxPerMm)||1;compareCamera.zoom=clamp(current*(event.deltaY<0?1.12:0.89),0.15,40);redraw=true;},{passive:false});
}
async function init(){
 try{design=await(await fetch('/example.json')).json();
  // A stored project is never loaded silently; explicit Open preserves predictable demos.
  selected=design.objects[0].id;wireUI();renderObjects();renderInspector();
  try{scene=new MachineScene($('machine-canvas'),()=>redraw=true);$('view3d').querySelector('.view-unit').textContent=scene.backend==='webgl'?'WebGL':'Software 3D';}catch(e){$('gl-warning').hidden=false;$('gl-warning').textContent=e.message;}
  session=await(await fetch('/api/session')).json();if(session.mode==='transport-test-fixture'){$('compile').textContent='Reload fixture';$('download').textContent='Download fixture .dst';}
  if(session.engine_available)await compileDesign();else{message('Rust binary not found. From the repository root run: cargo build -p embroidery-twin. No preview compiler is substituted.',true);updateControls();}
  function frame(t){const wasPlaying=clock.playing;const state=clock.tick(t);if(wasPlaying||redraw)paint(state);requestAnimationFrame(frame);}requestAnimationFrame(frame);
  Object.defineProperty(window,'__twinlab',{value:Object.freeze({snapshot:()=>({design:copy(design),dirty,artifact_id:bundle?.artifact_id||null,provenance:bundle?.provenance||null,trace:bundle?.trace||null,state:clock.sample(),webgl:!!scene?.gl,renderer:scene?.backend||'unavailable',scene_ready:scene?.available||false,mode:session.mode,compare:{original_sha256:original?.sha256||null,original_name:original?.name||null,proof_id:proof?.id||null,proof_sha256:proof?.binding?.pdf_sha256||null,binding_current:!!(proof&&!dirty&&bindingMatches(proof.binding,compareFacts())),release_allowed:false,approval_allowed:false}})})});
 }catch(e){message('Startup failed: '+e.message,true);}
}
init();
