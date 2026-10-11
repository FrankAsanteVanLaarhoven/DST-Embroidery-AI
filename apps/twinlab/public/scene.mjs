/** Original parametric 3D rig with WebGL and a low-fidelity software fallback. Not OEM CAD. */
import {frameTranslation,clamp} from './playback.mjs';
const identity=()=>new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]);
export function multiply(a,b){const r=new Float32Array(16);for(let c=0;c<4;c++)for(let row=0;row<4;row++)for(let k=0;k<4;k++)r[c*4+row]+=a[k*4+row]*b[c*4+k];return r;}
function perspective(fov,aspect,near,far){const f=1/Math.tan(fov/2),r=new Float32Array(16);r[0]=f/aspect;r[5]=f;r[10]=(far+near)/(near-far);r[11]=-1;r[14]=2*far*near/(near-far);return r;}
const normal=v=>{const n=Math.hypot(...v);return v.map(x=>x/n);};
const cross=(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
const dot=(a,b)=>a.reduce((v,x,i)=>v+x*b[i],0);
function lookAt(eye,target){const z=normal(eye.map((v,i)=>v-target[i])),x=normal(cross([0,1,0],z)),y=cross(z,x);return new Float32Array([x[0],y[0],z[0],0,x[1],y[1],z[1],0,x[2],y[2],z[2],0,-dot(x,eye),-dot(y,eye),-dot(z,eye),1]);}
function model(p,s){const r=identity();r[0]=s[0];r[5]=s[1];r[10]=s[2];r[12]=p[0];r[13]=p[1];r[14]=p[2];return r;}
function cube(){
 const positions=[],normals=[];
 const faces=[[[1,0,0],[[.5,-.5,-.5],[.5,.5,-.5],[.5,.5,.5],[.5,-.5,.5]]],[[-1,0,0],[[-.5,-.5,.5],[-.5,.5,.5],[-.5,.5,-.5],[-.5,-.5,-.5]]],[[0,1,0],[[-.5,.5,-.5],[-.5,.5,.5],[.5,.5,.5],[.5,.5,-.5]]],[[0,-1,0],[[-.5,-.5,.5],[-.5,-.5,-.5],[.5,-.5,-.5],[.5,-.5,.5]]],[[0,0,1],[[-.5,-.5,.5],[.5,-.5,.5],[.5,.5,.5],[-.5,.5,.5]]],[[0,0,-1],[[.5,-.5,-.5],[-.5,-.5,-.5],[-.5,.5,-.5],[.5,.5,-.5]]]];
 for(const [n,vs] of faces)for(const i of [0,1,2,0,2,3]){positions.push(...vs[i]);normals.push(...n);}return {positions,normals};
}
function cylinder(){const positions=[],normals=[];for(let i=0;i<20;i++){const a=i*Math.PI/10,b=(i+1)*Math.PI/10;const u=[Math.cos(a)*.5,0,Math.sin(a)*.5],v=[Math.cos(b)*.5,0,Math.sin(b)*.5];for(const [q,y] of [[u,-.5],[u,.5],[v,.5],[u,-.5],[v,.5],[v,-.5]]){positions.push(q[0],y,q[2]);normals.push(q[0]*2,0,q[2]*2);}for(const y of [-.5,.5])for(const q of (y>0?[[0,0,0],v,u]:[[0,0,0],u,v])){positions.push(q[0],y,q[2]);normals.push(0,y*2,0);}}return {positions,normals};}
function compileShader(gl,type,source){const s=gl.createShader(type);gl.shaderSource(s,source);gl.compileShader(s);if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw new Error(gl.getShaderInfoLog(s));return s;}
export class MachineScene{
 constructor(canvas,onChange=()=>{}){
  this.canvas=canvas;this.onChange=onChange;this.theta=.70;this.phi=.97;this.radius=.58;this.target=[0,.075,0];this.available=false;this.bundle=null;this.wireframe=false;
  const gl=canvas.getContext('webgl',{antialias:true,preserveDrawingBuffer:true});this.gl=gl;this.backend=gl?'webgl':'software-3d';this.context2d=gl?null:canvas.getContext('2d');if(!gl&&!this.context2d)throw new Error('No 3D rendering backend is available.');
  if(gl){
  const vs=compileShader(gl,gl.VERTEX_SHADER,`attribute vec3 aPosition;attribute vec3 aNormal;attribute vec3 aColor;uniform mat4 uMvp;uniform vec3 uScale;varying vec3 vNormal;varying vec3 vColor;void main(){gl_Position=uMvp*vec4(aPosition,1.);vNormal=normalize(aNormal/uScale);vColor=aColor;}`);
  const fs=compileShader(gl,gl.FRAGMENT_SHADER,`precision mediump float;varying vec3 vNormal;varying vec3 vColor;uniform vec3 uColor;uniform float uAlpha;uniform float uUnlit;void main(){float light=.42+.58*max(dot(normalize(vNormal),normalize(vec3(.4,1.,.65))),0.);gl_FragColor=vec4(uColor*vColor*mix(light,1.,uUnlit),uAlpha);}`);
  this.program=gl.createProgram();gl.attachShader(this.program,vs);gl.attachShader(this.program,fs);gl.linkProgram(this.program);if(!gl.getProgramParameter(this.program,gl.LINK_STATUS))throw new Error(gl.getProgramInfoLog(this.program));
  this.loc={};for(const n of ['uMvp','uScale','uColor','uAlpha','uUnlit'])this.loc[n]=gl.getUniformLocation(this.program,n);for(const n of ['aPosition','aNormal','aColor'])this.loc[n]=gl.getAttribLocation(this.program,n);
  }
  this.box=this.mesh(cube());this.cylinder=this.mesh(cylinder());this.thread=null;
  this.available=true;
  let drag=null;canvas.addEventListener('pointerdown',e=>{drag=[e.clientX,e.clientY];canvas.setPointerCapture(e.pointerId);});
  canvas.addEventListener('pointermove',e=>{if(!drag)return;this.theta+=(e.clientX-drag[0])*.007;this.phi=clamp(this.phi+(e.clientY-drag[1])*.007,.12,1.48);drag=[e.clientX,e.clientY];onChange();});
  canvas.addEventListener('pointerup',()=>drag=null);canvas.addEventListener('pointercancel',()=>drag=null);
  canvas.addEventListener('wheel',e=>{e.preventDefault();this.radius=clamp(this.radius*Math.exp(e.deltaY*.001),.18,.8);onChange();},{passive:false});
  canvas.addEventListener('webglcontextlost',e=>{e.preventDefault();this.available=false;onChange();});
  canvas.addEventListener('webglcontextrestored',()=>location.reload());
 }
 mesh({positions,normals,colors}){const gl=this.gl,put=data=>{if(!gl)return null;const b=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,b);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(data),gl.STATIC_DRAW);return b;};return {positions,normals,colors,position:put(positions),normal:put(normals),color:colors?put(colors):null,count:positions.length/3};}
 dispose(mesh){if(!mesh||!this.gl)return;for(const b of [mesh.position,mesh.normal,mesh.color])if(b)this.gl.deleteBuffer(b);}
 setBundle(bundle){
  this.bundle=bundle;this.dispose(this.thread);this.thread=null;
  if(!bundle)return;
  const positions=[],normals=[],colors=[];
  for(const e of bundle.trace.events){if(e.kind!=='stitch')continue;const rgb=bundle.palette[e.color_index].rgb.map(n=>n/255);for(const point of [e.from_units,e.to_units]){positions.push(point[0]/10000,.0012,point[1]/10000);normals.push(0,1,0);colors.push(...rgb);}}
  this.thread=this.mesh({positions,normals,colors});
 }
 setView(name){if(name==='top'){this.phi=.12;this.theta=0;}else if(name==='front'){this.phi=1.4;this.theta=0;}else{this.theta=.70;this.phi=.97;}this.onChange();}
 render(state,dark=false){
  if(!this.available)return;const gl=this.gl,canvas=this.canvas,dpr=Math.min(devicePixelRatio||1,2);
  const w=Math.max(1,Math.round(canvas.clientWidth*dpr)),h=Math.max(1,Math.round(canvas.clientHeight*dpr));if(canvas.width!==w||canvas.height!==h){canvas.width=w;canvas.height=h;}
  if(gl){gl.viewport(0,0,w,h);gl.clearColor(...(dark?[.066,.077,.10,1]:[.931,.940,.953,1]));gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT);gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LEQUAL);gl.enable(gl.BLEND);gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);gl.useProgram(this.program);}
  const primitives=[];
  const eye=[this.radius*Math.sin(this.phi)*Math.sin(this.theta),this.target[1]+this.radius*Math.cos(this.phi),this.radius*Math.sin(this.phi)*Math.cos(this.theta)];
  const vp=multiply(perspective(.70,w/h,.005,3),lookAt(eye,this.target));
  const draw=(mesh,p,s,color,alpha=1,unlit=0,lines=false,count=mesh.count)=>{
   if(!gl){this.collectSoftware(primitives,mesh,multiply(vp,model(p,s)),s,color,alpha,unlit,lines,count,w,h);return;}
   for(const [attr,b] of [['aPosition',mesh.position],['aNormal',mesh.normal]]){gl.bindBuffer(gl.ARRAY_BUFFER,b);gl.enableVertexAttribArray(this.loc[attr]);gl.vertexAttribPointer(this.loc[attr],3,gl.FLOAT,false,0,0);}
   if(mesh.color){gl.bindBuffer(gl.ARRAY_BUFFER,mesh.color);gl.enableVertexAttribArray(this.loc.aColor);gl.vertexAttribPointer(this.loc.aColor,3,gl.FLOAT,false,0,0);}else{gl.disableVertexAttribArray(this.loc.aColor);gl.vertexAttrib3f(this.loc.aColor,1,1,1);}
   gl.uniformMatrix4fv(this.loc.uMvp,false,multiply(vp,model(p,s)));gl.uniform3fv(this.loc.uScale,s);gl.uniform3fv(this.loc.uColor,color);gl.uniform1f(this.loc.uAlpha,alpha);gl.uniform1f(this.loc.uUnlit,unlit);gl.drawArrays(lines?gl.LINES:gl.TRIANGLES,0,count);
  };
  const box=(p,s,c)=>draw(this.box,p,s,c,1,0,this.wireframe);
  const metal=[.68,.72,.78],body=dark?[.56,.60,.67]:[.88,.90,.94],trim=[.13,.16,.22];
  // Rigid machine base, upright, arm and casing. Units are metres.
  box([0,-.022,-.025],[.255,.022,.215],trim);box([.095,.070,-.104],[.040,.180,.048],body);
  box([.039,.153,-.055],[.150,.040,.145],body);box([0,.115,-.014],[.065,.082,.044],body);
  box([.098,.084,-.076],[.003,.058,.012],[.16,.40,.78]);
  // Carriage/fabric/stitch geometry share exactly one inverse XY transform.
  const f=frameTranslation(state.position_mm);const at=(x,y,z)=>[x+f[0],y,z+f[2]];
  box(at(0,-.0015,0),[.15,.002,.15],dark?[.72,.70,.64]:[.91,.87,.78]);
  for(const sign of [-1,1]){box(at(sign*.066,.003,0),[.007,.009,.14],metal);box(at(0,.003,sign*.066),[.14,.009,.007],metal);}
  box(at(.078,0,-.01),[.028,.008,.027],trim);
  // Six-needle selector is illustrative. Only the active needle descends.
  const palette=this.bundle?.palette||[{rgb:[54,111,214],needle:1}];const needle=(palette[state.color_index]?.needle||1)-1;
  const needleColors=new Map(palette.map(p=>[p.needle-1,p.rgb.map(x=>x/255)]));
  for(let i=0;i<6;i++){
   const x=(i-needle)*.008,active=i===needle;const lift=active?state.needle_lift_mm/1000:.014;
   draw(this.cylinder,[x,.035+lift,0],[.002,.050,.002],metal);
   draw(this.cylinder,[x,.006+lift,0],[.0007,.012,.0007],metal);
   draw(this.cylinder,[(i-2.5)*.013,.193,-.047],[.010,.024,.010],needleColors.get(i)||[.46,.48,.52]);
  }
  box([0,.0045,.003],[.009,.002,.008],metal);
  // The exact decoded trace is rendered on the moving fabric, not on the casing.
  if(this.thread){draw(this.thread,f,[1,1,1],[1,1,1],.12,1,true);draw(this.thread,f,[1,1,1],[1,1,1],1,1,true,Math.min(this.thread.count,state.completed_stitches*2));}
  // Datum grid on the fixed table, drawn as thin geometry.
  for(let i=-6;i<=6;i++){box([i*.02,-.009,.014],[.00025,.00025,.20],dark?[.22,.25,.30]:[.73,.77,.82]);box([0,-.009,.014+i*.015],[.24,.00025,.00025],dark?[.22,.25,.30]:[.73,.77,.82]);}
  if(!gl)this.paintSoftware(primitives,w,h,dark);
  canvas.dataset.renderer=this.backend;canvas.dataset.eventIndex=String(state.event_index);canvas.dataset.completedStitches=String(state.completed_stitches);canvas.dataset.frameX=String(f[0]);canvas.dataset.frameZ=String(f[2]);
 }
 collectSoftware(out,mesh,mvp,scale,color,alpha,unlit,lines,count,w,h){
  const project=i=>{const x=mesh.positions[i*3],y=mesh.positions[i*3+1],z=mesh.positions[i*3+2];const q=mvp[3]*x+mvp[7]*y+mvp[11]*z+mvp[15];if(q<=0)return null;return [(mvp[0]*x+mvp[4]*y+mvp[8]*z+mvp[12])/q*w/2+w/2,h/2-(mvp[1]*x+mvp[5]*y+mvp[9]*z+mvp[13])/q*h/2,(mvp[2]*x+mvp[6]*y+mvp[10]*z+mvp[14])/q];};
  const stride=lines?2:3,light=normal([.4,1,.65]);
  for(let i=0;i+stride<=count;i+=stride){
   const points=Array.from({length:stride},(_,k)=>project(i+k));if(points.some(p=>!p))continue;
   // Cull back faces in the software view, matching outward-wound scene meshes.
   if(!lines){const [a,b,c]=points;if((b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])>=0)continue;}
   const n=normal(mesh.normals.slice(i*3,i*3+3).map((v,k)=>v/scale[k]));
   const illumination=unlit?1:.42+.58*Math.max(dot(n,light),0);
   const rgb=color.map((v,k)=>Math.round(clamp(v*(mesh.colors?mesh.colors[i*3+k]:1)*illumination,0,1)*255));
   out.push({points,depth:points.reduce((a,p)=>a+p[2],0)/stride,color:`rgba(${rgb.join(',')},${alpha})`,lines});
  }
 }
 paintSoftware(primitives,w,h,dark){
  const ctx=this.context2d;ctx.setTransform(1,0,0,1,0,0);ctx.fillStyle=dark?'#11141a':'#edf0f4';ctx.fillRect(0,0,w,h);
  primitives.sort((a,b)=>b.depth-a.depth);
  for(const p of primitives){ctx.beginPath();ctx.moveTo(p.points[0][0],p.points[0][1]);for(const v of p.points.slice(1))ctx.lineTo(v[0],v[1]);if(p.lines){ctx.strokeStyle=p.color;ctx.lineWidth=Math.min(devicePixelRatio||1,2);ctx.stroke();}else{ctx.closePath();ctx.fillStyle=p.color;ctx.fill();}}
 }

}
