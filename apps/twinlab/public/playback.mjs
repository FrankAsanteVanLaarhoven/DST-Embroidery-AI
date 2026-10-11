/** One deterministic clock drives both views. No renderer owns a second clock. */
export const clamp=(x,a,b)=>Math.max(a,Math.min(b,x));
export function sampleTrace(trace,requestedMs){
  if(!trace?.events?.length)return {time_ms:0,event_index:-1,phase:0,position_mm:[0,0],needle_lift_mm:12,completed_stitches:0,color_index:0,kind:'idle'};
  const time=clamp(Number.isFinite(requestedMs)?requestedMs:0,0,trace.duration_ms);
  let lo=0,hi=trace.events.length-1;
  while(lo<hi){const mid=(lo+hi)>>1;if(trace.events[mid].end_ms<=time)lo=mid+1;else hi=mid;}
  const e=trace.events[lo],phase=clamp((time-e.start_ms)/(e.end_ms-e.start_ms),0,1);
  const move=e.kind==='stitch'?clamp(phase/0.45,0,1):phase;
  const position_mm=e.from_units.map((p,i)=>(p+(e.to_units[i]-p)*move)/10);
  let lift=12;
  if(e.kind==='stitch'){
    if(phase>=0.45&&phase<0.70)lift=12*(1-(phase-0.45)/0.25);
    else if(phase>=0.70&&phase<0.95)lift=12*(phase-0.70)/0.25;
  }
  // Prefix array is derived once from events, not a second source of motion.
  let completed=trace.stitch_prefix?trace.stitch_prefix[lo]:trace.events.slice(0,lo).filter(x=>x.kind==='stitch').length;
  if(e.kind==='stitch'&&phase>=0.70)completed++;
  return {time_ms:time,event_index:lo,phase,position_mm,needle_lift_mm:lift,completed_stitches:completed,color_index:e.color_index,kind:e.kind,raw:e.raw,object_id:e.object_id};
}
export function prepareTrace(trace){
  let n=0;const stitch_prefix=trace.events.map(e=>{const before=n;if(e.kind==='stitch')n++;return before;});
  return {...trace,stitch_prefix};
}
export function frameTranslation(position_mm){return [-position_mm[0]/1000,0,-position_mm[1]/1000];}
export function designPointToScene(point_mm){return [point_mm[0]/1000,0,point_mm[1]/1000];}
export function pointUnderNeedle(point_mm){const p=designPointToScene(point_mm),f=frameTranslation(point_mm);return p.map((v,i)=>v+f[i]);}
export class Playback {
  constructor(){this.trace=null;this.time=0;this.speed=1;this.playing=false;this.last=null;}
  load(trace){this.trace=prepareTrace(trace);this.time=0;this.playing=false;this.last=null;return this.sample();}
  sample(){return sampleTrace(this.trace,this.time);}
  seek(ms){this.time=clamp(ms,0,this.trace?.duration_ms||0);this.last=null;return this.sample();}
  step(direction){if(!this.trace)return this.sample();const s=this.sample();const index=clamp(s.event_index+direction,0,this.trace.events.length-1);this.playing=false;return this.seek(this.trace.events[index].start_ms);}
  tick(now){if(this.last!==null&&this.playing&&this.trace){this.time=clamp(this.time+(now-this.last)*this.speed,0,this.trace.duration_ms);if(this.time===this.trace.duration_ms)this.playing=false;}this.last=now;return this.sample();}
}
