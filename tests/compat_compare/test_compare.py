from __future__ import annotations
import base64, copy, hashlib, importlib.util, json, math, sys, tempfile, unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/compat_compare'))
from policy import Probe, choose_mode, mm_per_pixel, effective_dpi, pixels_to_mm, format_policy
from proof import make_proof, validate_bundle, source_image
from PIL import Image

class PolicyTests(unittest.TestCase):
    def test_254_sampling(self):self.assertAlmostEqual(mm_per_pixel(254),.1)
    def test_300_sampling(self):self.assertAlmostEqual(mm_per_pixel(300),25.4/300)
    def test_enlargement(self):self.assertEqual(effective_dpi(300,2),150)
    def test_units(self):self.assertAlmostEqual(pixels_to_mm(1000,254),100,places=10)
    def test_bad_dpi(self):
        for d in (0,-1,float('nan'),float('inf'),True):
            with self.assertRaises(ValueError):mm_per_pixel(d)
    def test_bad_scale(self):
        with self.assertRaises(ValueError):effective_dpi(300,0)
    def test_legacy_not_direct(self):self.assertEqual(choose_mode(Probe('Windows XP','x86',1,'unmaintained'))['mode'],'isolated_capture_gateway')
    def test_old_machine_not_assumed(self):self.assertEqual(choose_mode(Probe('Old Apple','x86_64',4))['mode'],'manual_assessment')
    def test_light(self):self.assertEqual(choose_mode(Probe('Linux','x86_64',4,'maintained',True,True))['mode'],'lightweight_2d_candidate')
    def test_3d(self):self.assertEqual(choose_mode(Probe('Windows','x64',8,'maintained',True,True,True,True))['mode'],'interactive_3d_candidate')
    def test_gpu_smoke_required(self):self.assertEqual(choose_mode(Probe('Windows','x64',8,'maintained',True,True,True,False))['mode'],'lightweight_2d_candidate')
    def test_webgpu_alone_insufficient(self):self.assertEqual(choose_mode(Probe('macOS','arm64',16,'maintained',True,True,False,False,True))['mode'],'lightweight_2d_candidate')
    def test_no_canvas(self):self.assertEqual(choose_mode(Probe('Linux','arm64',16,'maintained',True,False))['mode'],'gateway_or_static_review')
    def test_low_memory(self):self.assertEqual(choose_mode(Probe('Linux','x86',1,'maintained',True,True))['mode'],'gateway_or_static_review')
    def test_never_enables_control(self):
        r=choose_mode(Probe('Windows','x64',64,'maintained',True,True,True,True,True,True))
        self.assertFalse(r['machine_control_enabled']);self.assertFalse(r['production_release_enabled']);self.assertFalse(r['local_ai_enabled']);self.assertEqual(r['qualification'],'unqualified_candidate')
    def test_dts_not_renamed(self):self.assertFalse(format_policy('.DTS')['rename_automatically'])
    def test_pdf_not_machine_file(self):self.assertFalse(format_policy('pdf')['machine_executable'])
    def test_other_format_unknown(self):self.assertEqual(format_policy('pes')['qualification'],'unknown')
    def test_invalid_probe(self):
        with self.assertRaises(ValueError):choose_mode(Probe('Windows','x64',math.nan,'maintained'))

class ProofTests(unittest.TestCase):
    def setUp(self):self.bundle=json.loads((ROOT/'fixtures/compat/square.replay.json').read_text())
    def reject(self,mutate):
        b=copy.deepcopy(self.bundle);mutate(b)
        with self.assertRaises(ValueError):validate_bundle(b)
    def test_valid_fixture(self):self.assertEqual(len(validate_bundle(self.bundle)),524)
    def test_native_project_replay_protocol(self):
        bundle=copy.deepcopy(self.bundle)
        bundle['protocol']='embroidery-project-replay/1'
        self.assertEqual(bundle['protocol'],'embroidery-project-replay/1')
        self.assertEqual(len(validate_bundle(bundle)),524)
        self.assertEqual(self.bundle['protocol'],'twinlab-m1/1')
    def test_wrong_hash(self):self.reject(lambda b:b['provenance'].update(dst_sha256='0'*64))
    def test_changed_raw(self):self.reject(lambda b:b['trace']['events'][0].update(raw=[0,0,3]))
    def test_cardinality(self):self.reject(lambda b:b['trace'].update(record_count=5))
    def test_bad_base64(self):self.reject(lambda b:b.update(dst_base64='!'))
    def test_timing(self):self.reject(lambda b:b['trace']['events'][1].update(start_ms=10))
    def test_discontinuous(self):self.reject(lambda b:b['trace']['events'][1].update(from_units=[1,1]))
    def test_bad_kind(self):self.reject(lambda b:b['trace']['events'][1].update(kind='sequin'))
    def test_palette(self):self.reject(lambda b:b['palette'][0].update(rgb=[999,0,0]))
    def test_count(self):self.reject(lambda b:b['trace'].update(needle_placements=999))
    def test_final(self):self.reject(lambda b:b['trace']['events'][-1].update(kind='jump'))
    def test_coordinates(self):self.reject(lambda b:b['trace']['events'][1].update(to_units=[float('nan'),0]))
    def test_synthetic_coordinates_not_authenticated(self):
        # Intentional limitation assertion: byte-consistent metadata is not proof
        # of semantic decoding or a signature. Do not conceal this trust boundary.
        b=copy.deepcopy(self.bundle)
        b['trace']['events'][0]['to_units']=[80,0]
        b['trace']['events'][1]['from_units']=[80,0]
        self.assertEqual(len(validate_bundle(b)),524)
    def test_raster_source(self):
        im,meta=source_image(ROOT/'fixtures/compat/reference-square.png');self.assertEqual(im.size,(560,560));self.assertEqual(meta['kind'],'source_raster_derivative')
    def test_pdf_proof_and_physical_bar(self):
        import fitz
        with tempfile.TemporaryDirectory() as t:
            out=Path(t)/'proof.pdf';m=make_proof(ROOT/'fixtures/compat/square.replay.json',ROOT/'fixtures/compat/reference-square.png',out,14,-2,-2)
            self.assertFalse(m['checks']['rust_executed']);self.assertTrue(m['checks']['record_byte_binding']);self.assertTrue(m['actual_size_panel'])
            with fitz.open(out) as doc:
                self.assertEqual(len(doc),2);self.assertIn('NOT LIVE RUST',doc[0].get_text())
                spans=[s for b in doc[1].get_text('dict')['blocks'] if 'lines' in b for l in b['lines'] for s in l['spans']]
                labels=[s for s in spans if '100 mm - print' in s['text']]
                self.assertTrue(labels);self.assertTrue(all(s['color']<0x999999 for s in labels))
                lines=[item for p in doc[1].get_drawings() for item in p['items'] if item[0]=='l']
                lengths=[abs(i[2].x-i[1].x) for i in lines if abs(i[2].y-i[1].y)<1e-4]
                self.assertTrue(any(abs(v-100*72/25.4)<.02 for v in lengths))
    def test_source_pdf_derivative(self):
        import fitz
        with tempfile.TemporaryDirectory() as t:
            p=Path(t)/'src.pdf';doc=fitz.open();doc.new_page(width=200,height=100);doc.save(p);doc.close()
            image,meta=source_image(p);self.assertEqual(meta['kind'],'source_pdf_page_raster_derivative');self.assertEqual(image.width,2*image.height)
    def test_existing_output_refused(self):
        with tempfile.TemporaryDirectory() as t:
            out=Path(t)/'p.pdf';out.write_bytes(b'keep')
            with self.assertRaises(ValueError):make_proof(ROOT/'fixtures/compat/square.replay.json',ROOT/'fixtures/compat/reference-square.png',out,14)
            self.assertEqual(out.read_bytes(),b'keep')
    def test_missing_scale(self):
        with tempfile.TemporaryDirectory() as t:
            with self.assertRaises(ValueError):make_proof(ROOT/'fixtures/compat/square.replay.json',ROOT/'fixtures/compat/reference-square.png',Path(t)/'p.pdf',0)
    def test_no_auto_downscale(self):
        with tempfile.TemporaryDirectory() as t:
            m=make_proof(ROOT/'fixtures/compat/square.replay.json',ROOT/'fixtures/compat/reference-square.png',Path(t)/'p.pdf',500)
            self.assertFalse(m['actual_size_panel'])

    def test_saved_lettering_proof_shows_the_selected_row(self):
        """Binding hash stays. Parsed labels and the rendered replay are extra."""
        import fitz
        batch_path=ROOT/'evidence/lettering-v2/name-drop-batch.json'
        before=batch_path.read_bytes()
        batch=json.loads(before)
        rows={item['item_id']:item for item in batch['items']}
        expected={
            'N-03':'Café',
            'N-04':'Cafe\u0301',
            'N-17':'O\u2019Neil',
            'N-18':"O'Neil",
        }
        rendered={}
        for item_id, source in expected.items():
            item=rows[item_id]
            self.assertEqual(item['text'], source)
            project_text=next(obj['text']['content'] for obj in item['project']['objects'] if obj['kind']=='text')
            self.assertEqual(project_text, source)
            pdf=base64.b64decode(item['pdf_base64'])
            self.assertTrue(pdf.startswith(b'%PDF'))
            self.assertEqual(hashlib.sha256(pdf).hexdigest(), item['binding']['pdf_sha256'])
            self.assertEqual(item['binding']['dst_sha256'], item['dst_sha256'])
            self.assertEqual(item['binding']['item_id'], item_id)
            with fitz.open(stream=pdf, filetype='pdf') as doc:
                self.assertEqual(len(doc), 2)
                text=doc[0].get_text()+doc[1].get_text()
                self.assertIn('DST replay view', text)
                self.assertIn('Engineering review only', text)
                self.assertIn('SUPPLIED NATIVE REPLAY', text)
                # Stitch lettering is drawn. Extraction is not the stored source.
                self.assertNotEqual(text, source)
                rendered[item_id]=self.replay_panel(doc[0])
            self.assertGreater(rendered[item_id][0], 20)
        self.assertEqual(rows['N-03']['pdf_base64'], rows['N-04']['pdf_base64'])
        self.assertEqual(rendered['N-03'][1], rendered['N-04'][1])
        self.assertNotEqual(rows['N-03']['text'], rows['N-04']['text'])
        self.assertNotEqual(rows['N-03']['proof_id'], rows['N-04']['proof_id'])
        self.assertNotEqual(rendered['N-17'][1], rendered['N-18'][1])
        self.assertIn('\u0301', rows['N-04']['text'])
        standalone=ROOT/'evidence/lettering-v2/N-03.proof.pdf'
        self.assertEqual(standalone.read_bytes(), base64.b64decode(rows['N-03']['pdf_base64']))
        self.assertEqual(batch_path.read_bytes(), before)

    @staticmethod
    def replay_panel(page):
        import fitz
        box=page.rect
        width, height=box.width, box.height
        panel_width=(width-88)/2
        panel_height=288
        reportlab_y=height-386
        clip=fitz.Rect(52+panel_width, height-(reportlab_y+panel_height), 52+panel_width*2, height-reportlab_y)
        pix=page.get_pixmap(matrix=fitz.Matrix(2, 2), clip=clip, alpha=False)
        ink=0
        samples=pix.samples
        for index in range(0, len(samples), 3):
            if samples[index]<230 or samples[index+1]<230 or samples[index+2]<230:
                ink+=1
        return ink, bytes(samples)


class RegisterTests(unittest.TestCase):
    def setUp(self):self.register=json.loads((ROOT/'contracts/compatibility-coverage.json').read_text())
    def test_all_supplied_items_preserved(self):
        ids={r['id'] for r in self.register['requirements']}
        self.assertTrue({f'{p}{i:02d}' for p in ('B','R') for i in range(1,13)}<=ids)
        self.assertEqual(self.register['baseline_obligation_count'],24)
    def test_unique_ids(self):
        ids=[r['id'] for r in self.register['requirements']];self.assertEqual(len(ids),len(set(ids)))
    def test_acceptance_owners_present(self):
        self.assertTrue(all(r['acceptance'] and r['owner_role'] and r['coverage_route'] for r in self.register['requirements']))
    def test_no_passed_qualification(self):
        self.assertTrue(all(r['implementation_status'] not in ('qualified','passed','certified') for r in self.register['requirements']))

if __name__=='__main__':unittest.main(verbosity=2)
