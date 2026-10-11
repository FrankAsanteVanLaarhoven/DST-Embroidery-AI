/** Transport/view fixture only. Binary copied verbatim from the pinned repository golden square. */
import {createHash} from 'node:crypto';
const base64=`TEE6U3F1YXJlICAgICAgICAgIA1TVDogICAgICA0DUNPOiAgMA0rWDogIDEw
MA0tWDogICAgMA0rWTogIDEwMA0tWTogICAgMA1BWDorICAgIDANQVk6LSAg
MTAwDU1YOisgICAgMA1NWTorICAgIDANUEQ6KioqKioqDRogICAgICAgICAg
ICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg
ICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg
ICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg
ICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg
ICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg
ICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg
ICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg
ICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg
ICAgICAgICAgICAgICAgICAJBAdgEBMGCAsAAPM=`;
export const dst=Buffer.from(base64,'base64');
export const BLOB_SHA='5d279f970901f5e2960b904166ba5f4f19fd7312';
const gitBlob=createHash('sha1').update(`blob ${dst.length}\0`).update(dst).digest('hex');
if(gitBlob!==BLOB_SHA)throw new Error('Copied golden DST does not match the GitHub blob SHA.');
export const design={schema_version:1,name:'Square',revision:1,objects:[{id:1,label:'Golden codec square',kind:'running',points:[{x:0,y:0},{x:10,y:0},{x:10,y:10},{x:0,y:10}],rgb:[53,106,210],spacing_mm:.5,angle_degrees:0,max_stitch_mm:10,width_mm:3,compensation_mm:0,underlay_mm:0}]};
// Expected values from the existing repository's square oracle; NOT a second decoder.
const to=[[100,0],[100,100],[0,100],[0,100]],from=[[0,0],[100,0],[100,100],[0,100]];
export function fixtureResult(request={profile_id:'generic-flat-120-v1',design}){
 const events=to.map((point,i)=>({index:i,byte_offset:512+i*3,raw:[...dst.subarray(512+i*3,515+i*3)],kind:i===3?'end':'stitch',from_units:from[i],to_units:point,color_index:0,start_ms:i*100,end_ms:i===3?301:(i+1)*100,object_id:i===3?null:1}));
 return {protocol:'twinlab-m1/1',design:request.design,profile:{id:request.profile_id,name:'Generic 6-needle · flat frame',units:'mm',width_mm:120,height_mm:120,calibrated:false,hardware_connected:false},kernel:{version:'fixture',base_commit:'98303601416a3b0d68eae37e939031732bd77c45',generation:'Recorded codec fixture',export:'Pinned repository bytes',mode:'transport-test-fixture'},generated_stitch_count:3,warnings:['This fixture does not execute the new Rust bridge.'],palette:[{rgb:[53,106,210],needle:1}],trace:{units:'0.1 mm; X right; Y down',decoder:'Expected positions from baseline square oracle (not executed here)',trim_policy:'raw jumps; no trim inference',timing:'illustrative only',events,record_count:4,needle_placements:3,duration_ms:301},dst_bytes:[...dst]};
}
