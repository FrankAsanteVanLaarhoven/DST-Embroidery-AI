"""Browser acceptance checks. --native requires the REAL served Rust pipeline.
--offline-fixture injects explicit test HTTP and SHA adapters; it cannot certify native mode.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
PUBLIC = ROOT / 'apps' / 'twinlab' / 'public'


def offline_html() -> str:
    # Node creates a provenance-bound fixture from the repository's exact golden DST.
    command = """import {fixtureResult,design} from './tests/twinlab/fixture.mjs';
import {sealResult} from './apps/twinlab/server.mjs';
const request={profile_id:'generic-flat-120-v1',design};
console.log(JSON.stringify(sealResult(fixtureResult(request),request,'0'.repeat(64),'transport-test-fixture').bundle));"""
    result = subprocess.run(['node', '--input-type=module', '-e', command], cwd=ROOT,
                            text=True, capture_output=True, check=True)
    bundle = json.loads(result.stdout)
    harness = """
const FIXTURE = __BUNDLE__;
window.__fixture_requests = 0;
window.__fixture_delay_ms = 0;
const canonical = value => Array.isArray(value) ? '['+value.map(canonical).join(',')+']' : value!==null&&typeof value==='object' ? '{'+Object.keys(value).sort().map(k=>JSON.stringify(k)+':'+canonical(value[k])).join(',')+'}' : JSON.stringify(value);
const hash=async value=>(await window.__testSha256(Array.from(new TextEncoder().encode(value)))).map(x=>x.toString(16).padStart(2,'0')).join('');
Object.defineProperty(window.crypto,'subtle',{value:{digest:async (_,bytes)=>Uint8Array.from(await window.__testSha256(Array.from(new Uint8Array(bytes)))).buffer}});
const artifacts=new Map();
const response=(value,status=200)=>new Response(JSON.stringify(value),{status,headers:{'Content-Type':'application/json'}});
window.fetch=async (url,options={})=>{
 if(url==='/example.json')return response(FIXTURE.design);
 if(url==='/api/session')return response({token:'offline-test-token',mode:'transport-test-fixture',engine_available:true,profile_id:'generic-flat-120-v1'});
 if(url==='/api/compile'){
  window.__fixture_requests++;
  if(window.__fixture_delay_ms)await new Promise(resolve=>setTimeout(resolve,window.__fixture_delay_ms));
  const req=JSON.parse(options.body),a=structuredClone(req.design),b=structuredClone(FIXTURE.design);a.revision=1;b.revision=1;
  if(canonical(a)!==canonical(b))return response({error:'Fixture mode cannot compile edited geometry. Build Rust to test the complete pipeline.'},422);
  const v=structuredClone(FIXTURE);v.design=req.design;v.provenance.design_sha256=await hash(canonical(req.design));
  v.artifact_id=await hash(canonical({...v.provenance,palette:v.palette,protocol:v.protocol}));artifacts.set(v.artifact_id,v);return response(v);
 }
 const m=String(url).match(/^\\/api\\/artifacts\\/([a-f0-9]{64})\\.(dst|json)$/);
 if(m&&artifacts.has(m[1])){const v=artifacts.get(m[1]);return m[2]==='json'?response(v):new Response(Uint8Array.from(atob(v.dst_base64),x=>x.charCodeAt(0)),{headers:{'Content-Type':'application/octet-stream'}});}
 throw new Error('Offline fixture attempted an unexpected network request: '+url);
};
""".replace('__BUNDLE__', json.dumps(bundle))
    modules=[]
    for filename in ['compare.mjs', 'playback.mjs', 'scene.mjs', 'app.mjs']:
        source=(PUBLIC/filename).read_text()
        source=re.sub(r'^import[^\n]*\n', '', source, flags=re.M)
        source=re.sub(r'\bexport (?=(?:const|function|class)\b)', '', source)
        modules.append(source)
    html=(PUBLIC/'index.html').read_text()
    html=re.sub(r'<script type="module" src="/app.mjs"></script>', '', html)
    html=re.sub(r'<link rel="icon"[^>]*>', '', html)
    html=html.replace('<link rel="stylesheet" href="/style.css">', '<style>'+(PUBLIC/'style.css').read_text()+'</style>')
    return html.replace('</body>', '<script>'+harness+'</script><script type="module">'+'\n'.join(modules)+'</script></body>')


def main() -> None:
    parser=argparse.ArgumentParser()
    mode=parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--native', action='store_true')
    mode.add_argument('--offline-fixture', action='store_true')
    parser.add_argument('--url', default='http://127.0.0.1:4173/')
    parser.add_argument('--output', default=str(ROOT/'evidence'))
    args=parser.parse_args()
    out=Path(args.output);out.mkdir(parents=True,exist_ok=True)
    checks=[];errors=[];skipped=[]
    def check(name: str, condition: bool) -> None:
        checks.append({'name':name,'passed':bool(condition)})
        if not condition: raise AssertionError(name)
    with sync_playwright() as p:
        options={'headless':True,'args':['--no-sandbox','--disable-dev-shm-usage']}
        if os.environ.get('BROWSER_EXECUTABLE'): options['executable_path']=os.environ['BROWSER_EXECUTABLE']
        browser=p.chromium.launch(**options)
        page=browser.new_page(viewport={'width':1600,'height':1030},device_scale_factor=1,accept_downloads=True)
        page.on('pageerror',lambda error:errors.append(str(error)))
        try:
            if args.offline_fixture:
                page.expose_function('__testSha256', lambda values:list(hashlib.sha256(bytes(values)).digest()))
                page.set_content(offline_html(),wait_until='load')
            else:
                page.goto(args.url,wait_until='networkidle')
            page.wait_for_function('window.__twinlab && window.__twinlab.snapshot().artifact_id',timeout=30000)
            state=page.evaluate('window.__twinlab.snapshot()')
            check('test mode is explicit',state['mode']==('native-rust' if args.native else 'transport-test-fixture'))
            check('3D scene initialises a real graphics backend',state['scene_ready'])
            check('current design has a bound artifact',not state['dirty'] and len(state['artifact_id'])==64)
            check('DST hash shown in evidence panel',page.locator('#hash').inner_text()==state['provenance']['dst_sha256'])
            check('exact DST export becomes available',page.locator('#download').is_enabled())
            total=state['trace']['duration_ms']
            for fraction in [0,.1,.25,.5,.75,1]:
                page.locator('#scrub').evaluate('(el,v)=>{el.value=v;el.dispatchEvent(new Event("input",{bubbles:true}));}',total*fraction)
                page.wait_for_timeout(40)
                s=page.evaluate('window.__twinlab.snapshot().state')
                two=page.locator('#design-canvas').get_attribute('data-event-index')
                three=page.locator('#machine-canvas').get_attribute('data-event-index')
                check(f'2D/3D same decoded event at {fraction:.0%}',two==three==str(s['event_index']))
                tx=float(page.locator('#machine-canvas').get_attribute('data-frame-x'))
                tz=float(page.locator('#machine-canvas').get_attribute('data-frame-z'))
                check(f'inverse hoop transform at {fraction:.0%}',abs(tx+s['position_mm'][0]/1000)<1e-10 and abs(tz+s['position_mm'][1]/1000)<1e-10)
            completed=int(page.locator('#design-canvas').get_attribute('data-completed-stitches'))
            check('end state reveals exactly the decoded needle placements',completed==state['trace']['needle_placements'])
            before=page.locator('#machine-canvas').screenshot()
            event_before=page.locator('#machine-canvas').get_attribute('data-event-index')
            page.locator('#camera').click();page.wait_for_timeout(80)
            after=page.locator('#machine-canvas').screenshot()
            check('camera switch changes rendered 3D pixels',hashlib.sha256(before).digest()!=hashlib.sha256(after).digest())
            check('camera does not change execution state',page.locator('#machine-canvas').get_attribute('data-event-index')==event_before)
            page.locator('#camera').click();page.locator('#camera').click()
            page.locator('[data-view="2d"]').click();check('2D-only view hides machine viewport',not page.locator('#view3d').is_visible())
            page.locator('[data-view="3d"]').click();check('3D-only view hides design viewport',not page.locator('#view2d').is_visible())
            page.locator('[data-view="split"]').click()
            with page.expect_download() as info: page.locator('#download').click()
            downloaded=out/'browser-downloaded.dst';info.value.save_as(downloaded)
            check('downloaded bytes match displayed DST SHA-256',hashlib.sha256(downloaded.read_bytes()).hexdigest()==state['provenance']['dst_sha256'])
            prefix='native' if args.native else 'fixture'
            page.screenshot(path=str(out/f'twinlab-{prefix}-desktop.png'),full_page=True)
            page.locator('#theme').click();page.wait_for_timeout(80)
            check('dark appearance applies',page.locator('body').evaluate('el=>el.classList.contains("dark")'))
            page.screenshot(path=str(out/f'twinlab-{prefix}-dark.png'),full_page=True)
            page.set_viewport_size({'width':390,'height':844});page.wait_for_timeout(100)
            check('mobile layout has no horizontal overflow',page.evaluate('document.documentElement.scrollWidth<=innerWidth+1'))
            page.screenshot(path=str(out/f'twinlab-{prefix}-mobile.png'),full_page=True)
            page.set_viewport_size({'width':1600,'height':1030});page.locator('#theme').click()
            revision_before=page.evaluate('window.__twinlab.snapshot().design.revision')
            page.locator('#point-x').fill('1');page.locator('#point-x').dispatch_event('change')
            page.locator('#point-x').dispatch_event('change')
            check('duplicate change events create only one revision',page.evaluate('window.__twinlab.snapshot().design.revision')==revision_before+1)
            check('editing invalidates the old artifact immediately',page.evaluate('window.__twinlab.snapshot().dirty'))
            check('editing disables stale export',page.locator('#download').is_disabled())
            check('editing disables stale playback',page.locator('#play').is_disabled())
            if args.native:
                old=state['artifact_id']
                page.wait_for_function('(old)=>window.__twinlab.snapshot().artifact_id && window.__twinlab.snapshot().artifact_id!==old',arg=old,timeout=30000)
                new=page.evaluate('window.__twinlab.snapshot()')
                check('edited geometry compiles through live Rust',new['provenance']['dst_sha256']!=state['provenance']['dst_sha256'])
            else:
                skipped.append('Live Rust recompilation and changed-DST verification: unavailable in fixture mode.')
                page.locator('#compile').click();page.wait_for_timeout(100)
                check('fixture cannot fake a successful recompile',page.locator('#download').is_disabled() and 'cannot compile' in page.locator('#message').inner_text())
                page.locator('#undo').click();page.locator('#compile').click()
                page.wait_for_function('window.__twinlab.snapshot().artifact_id',timeout=5000)
                check('undo restores the exact fixture geometry for replay',not page.evaluate('window.__twinlab.snapshot().dirty'))
                page.evaluate('window.__fixture_delay_ms=180')
                requests_before=page.evaluate('window.__fixture_requests')
                page.locator('#compile').click()
                page.wait_for_function('(n)=>window.__fixture_requests>n',arg=requests_before)
                page.locator('#point-x').fill('2');page.locator('#point-x').press('Tab')
                page.wait_for_timeout(250)
                check('late successful response cannot restore a stale artifact',page.evaluate('window.__twinlab.snapshot().dirty && !window.__twinlab.snapshot().artifact_id'))
                check('late response leaves export disabled',page.locator('#download').is_disabled())
                page.evaluate('window.__fixture_delay_ms=0')
                page.locator('#undo').click();page.locator('#compile').click()
                page.wait_for_function('window.__twinlab.snapshot().artifact_id',timeout=5000)
                page.locator('#scrub').evaluate('(el)=>{el.value=0;el.dispatchEvent(new Event("input",{bubbles:true}));}')
                page.wait_for_timeout(50)
                before_motion=page.locator('#machine-canvas').screenshot()
                page.locator('#scrub').evaluate('(el)=>{el.value=70;el.dispatchEvent(new Event("input",{bubbles:true}));}')
                page.wait_for_timeout(50)
                after_motion=page.locator('#machine-canvas').screenshot()
                check('decoded frame/needle movement changes rendered pixels',hashlib.sha256(before_motion).digest()!=hashlib.sha256(after_motion).digest())
                page.locator('#start').click();page.locator('#next').click();page.wait_for_timeout(40)
                check('single-step advances both views to raw record one',page.locator('#design-canvas').get_attribute('data-event-index')==page.locator('#machine-canvas').get_attribute('data-event-index')=='1')
            check('no uncaught page JavaScript errors',not errors)
            result={'mode':'native-served' if args.native else 'offline-fixture','renderer':state.get('renderer'),'passed':sum(c['passed'] for c in checks),'failed':sum(not c['passed'] for c in checks),'checks':checks,'skipped':skipped,'page_errors':errors,'limitations':[] if args.native else ['No Rust compilation/execution.','No served-origin browser test.','HTTP and SHA APIs are explicit test adapters.','WebGL path is unverified when renderer is software-3d.']}
        except Exception as error:
            result={'mode':'native-served' if args.native else 'offline-fixture','passed':sum(c['passed'] for c in checks),'failed':1+sum(not c['passed'] for c in checks),'checks':checks,'error':str(error),'page_errors':errors,'skipped':skipped}
            page.screenshot(path=str(out/'browser-failure.png'),full_page=True)
            (out/'browser-tests.json').write_text(json.dumps(result,indent=2)+'\n')
            raise
        finally:
            browser.close()
    (out/'browser-tests.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))

if __name__=='__main__': main()
