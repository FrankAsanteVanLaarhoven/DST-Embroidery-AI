"""Native Compare workspace checks. Requires the served Rust pipeline."""
from __future__ import annotations
import hashlib
import os
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
    url = os.environ.get('TWINLAB_URL', 'http://127.0.0.1:4175/')
    out = Path(os.environ.get('COMPARE_OUTPUT', ROOT / 'evidence' / 'compare'))
    out.mkdir(parents=True, exist_ok=True)
    errors = []
    console = []
    with sync_playwright() as p:
        options = {'headless': True, 'args': ['--no-sandbox', '--disable-dev-shm-usage']}
        if os.environ.get('BROWSER_EXECUTABLE'):
            options['executable_path'] = os.environ['BROWSER_EXECUTABLE']
        browser = p.chromium.launch(**options)
        page = browser.new_page(viewport={'width': 1440, 'height': 1000}, device_scale_factor=1, accept_downloads=True)
        page.on('pageerror', lambda error: errors.append(str(error)))
        page.on('console', lambda message: console.append(message.type + ':' + message.text) if message.type == 'error' else None)
        page.goto(url, wait_until='networkidle')
        page.wait_for_function('() => window.__twinlab && window.__twinlab.snapshot().artifact_id', timeout=30000)
        first = page.evaluate('window.__twinlab.snapshot()')
        assert first['mode'] == 'native-rust', first['mode']
        assert page.locator('#compare-heading').inner_text() == 'Compare'
        page.locator('#attach-reference').click()
        page.wait_for_function('() => window.__twinlab.snapshot().compare.original_sha256', timeout=10000)
        page.locator('#scale-confirm').check()
        page.locator('#proof-generate').click()
        page.wait_for_function('() => window.__twinlab.snapshot().compare.binding_current', timeout=30000)
        bound = page.evaluate('() => window.__twinlab.snapshot()')
        assert bound['compare']['binding_current'] is True
        assert bound['compare']['release_allowed'] is False
        assert bound['compare']['proof_sha256']
        assert bound['provenance']['dst_sha256'] == first['provenance']['dst_sha256']
        assert 'Approval and machine release remain unavailable' in page.locator('#compare-status').inner_text()
        page.locator('#tab-machine').click()
        assert 'DST' in page.locator('#machine-summary').inner_text()
        page.locator('#tab-twin').click()
        page.locator('#scrub').evaluate('(el)=>{el.value=0;el.dispatchEvent(new Event("input",{bubbles:true}));}')
        page.wait_for_timeout(80)
        assert page.locator('#design-canvas').get_attribute('data-event-index') == '0'
        assert 'record 1 /' in page.locator('#twin-event').inner_text()
        page.locator('#tab-pdf').click()
        assert page.locator('#proof-frame').is_visible()
        old_proof = bound['compare']['proof_sha256']
        page.locator('#spacing').fill('1.4')
        page.locator('#spacing').dispatch_event('change')
        page.wait_for_timeout(50)
        stale = page.evaluate('window.__twinlab.snapshot()')
        assert stale['dirty'] is True
        assert stale['compare']['binding_current'] is False
        assert page.locator('#download').is_disabled()
        assert page.locator('#proof-download').is_disabled()
        assert 'not current' in page.locator('#compare-status').inner_text()
        page.wait_for_function('(old)=>window.__twinlab.snapshot().artifact_id && window.__twinlab.snapshot().artifact_id!==old', arg=first['artifact_id'], timeout=30000)
        revised = page.evaluate('window.__twinlab.snapshot()')
        assert revised['provenance']['dst_sha256'] != first['provenance']['dst_sha256']
        assert revised['compare']['binding_current'] is False
        page.locator('#scale-confirm').check()
        page.locator('#proof-generate').click()
        page.wait_for_function('() => window.__twinlab.snapshot().compare.binding_current', timeout=30000)
        current = page.evaluate('window.__twinlab.snapshot()')
        assert current['compare']['proof_sha256'] != old_proof
        assert current['compare']['binding_current'] is True
        with page.expect_download() as dst_info:
            page.locator('#download').click()
        dst_path = out / 'revised.dst'
        dst_info.value.save_as(dst_path)
        assert hashlib.sha256(dst_path.read_bytes()).hexdigest() == current['provenance']['dst_sha256']
        with page.expect_download() as pdf_info:
            page.locator('#proof-download').click()
        pdf_path = out / 'revised-proof.pdf'
        pdf_info.value.save_as(pdf_path)
        assert hashlib.sha256(pdf_path.read_bytes()).hexdigest() == current['compare']['proof_sha256']
        page.locator('#proof-reopen').click()
        page.wait_for_function('() => document.getElementById("message").textContent.includes("Reopened")', timeout=10000)
        page.locator('#tab-original').click()
        page.locator('#tab-original').focus()
        page.keyboard.press('ArrowRight')
        assert page.locator('#tab-editable').get_attribute('aria-selected') == 'true'
        page.screenshot(path=str(out / 'compare-desktop.png'), full_page=True)
        page.locator('#theme').click()
        page.wait_for_timeout(80)
        page.screenshot(path=str(out / 'compare-dark.png'), full_page=True)
        page.set_viewport_size({'width': 768, 'height': 1024})
        page.wait_for_timeout(100)
        page.screenshot(path=str(out / 'compare-tablet.png'), full_page=True)
        page.set_viewport_size({'width': 390, 'height': 844})
        page.wait_for_timeout(100)
        assert page.evaluate('document.documentElement.scrollWidth<=innerWidth+1')
        page.locator('#tab-machine').click()
        assert page.locator('#machine-window').is_visible()
        page.screenshot(path=str(out / 'compare-mobile.png'), full_page=True)
        page.set_viewport_size({'width': 1440, 'height': 1000})
        page.locator('#theme').click()
        dts = out / 'kept-name.dts'
        dts.write_bytes(b'not a dst file')
        page.locator('#original-input').set_input_files(dts)
        page.wait_for_function('() => window.__twinlab.snapshot().compare.original_name==="kept-name.dts"', timeout=10000)
        assert 'not renamed' in page.locator('#compare-status').inner_text()
        assert page.evaluate('window.__twinlab.snapshot().compare.binding_current') is False
        missing = page.goto(url + 'missing-compare-page', wait_until='networkidle')
        assert missing.status == 404
        assert page.locator('h1').inner_text() == 'Page not found'
        page.locator('a').click()
        page.wait_for_function('() => window.__twinlab && window.__twinlab.snapshot().artifact_id', timeout=30000)
        assert not errors, errors
        unexpected = [line for line in console if 'missing-compare-page' not in line and '404' not in line]
        assert not unexpected, unexpected
        browser.close()
    print('compare browser checks passed')
    print('pdf', pdf_path)
    print('dst', dst_path)


if __name__ == '__main__':
    main()
