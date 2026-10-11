// Explicitly a transport/view fixture server. Never used by the production start command.
import {createTwinServer} from '../../apps/twinlab/server.mjs';
import {design,fixtureResult} from './fixture.mjs';
const runner=async request=>{
 const a=structuredClone(request.design),b=structuredClone(design);a.revision=1;b.revision=1;
 if(JSON.stringify(a)!==JSON.stringify(b))throw new Error('Fixture mode cannot recompile edited geometry. Build the Rust bridge to exercise the full milestone.');
 return fixtureResult(request);
};
const server=createTwinServer({runner,enginePath:process.execPath,engineSha:'0'.repeat(64),exampleDesign:design});
server.listen(Number(process.env.PORT||4174),'127.0.0.1',()=>console.log('FIXTURE-ONLY transport/view server at http://127.0.0.1:'+(process.env.PORT||4174)));
