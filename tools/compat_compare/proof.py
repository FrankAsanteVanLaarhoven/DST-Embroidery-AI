"""Create a source-vs-M1-replay PDF proof without a second stitch codec.

Trusted local developer utility, NOT a server for arbitrary uploads. It validates
SHA-256 and exact record-byte binding of an existing M1 bundle; it DOES NOT run
Rust or authenticate/redecode coordinates. Hash-consistent fabricated coordinates
are outside this transport check. Use the native gate before production claims.

Raster source intake is deliberately restricted to PNG/JPEG/WebP/TIFF/BMP.
PDF source pages use an installed PyMuPDF reader, if available. Original bytes
are retained unchanged by this tool; all previews are explicit derivatives.
"""
from __future__ import annotations
import argparse, base64, hashlib, io, json, math, re
from pathlib import Path
from PIL import Image, ImageOps
from reportlab.pdfgen import canvas
from reportlab.lib.pagesizes import A4, landscape
from reportlab.lib.utils import ImageReader

MAX_INPUT=16*1024*1024
MAX_EVENTS=60_000
HEX=re.compile(r'^[a-f0-9]{64}$')
MM=72/25.4


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def bounded_read(path: Path, limit: int=MAX_INPUT) -> bytes:
    if path.stat().st_size > limit:
        raise ValueError(f'{path.name} exceeds {limit} bytes')
    with path.open('rb') as f:
        data=f.read(limit+1)
    if len(data)>limit: raise ValueError('input grew beyond limit')
    return data


def finite(value, name, positive=False):
    if isinstance(value,bool) or not isinstance(value,(int,float)) or not math.isfinite(value) or (positive and value<=0):
        raise ValueError(f'{name} must be finite'+(' and positive' if positive else ''))
    return float(value)


def validate_bundle(bundle: dict) -> bytes:
    if not isinstance(bundle,dict) or bundle.get('protocol') not in ('twinlab-m1/1','embroidery-project-replay/1'):
        raise ValueError('expected a twinlab-m1/1 or embroidery-project-replay/1 evidence bundle')
    if bundle.get('kernel',{}).get('mode') not in ('native-rust','transport-test-fixture'):
        raise ValueError('unsupported provenance mode')
    payload=bundle.get('dst_base64')
    if not isinstance(payload,str) or len(payload)>4*(512+3*MAX_EVENTS):
        raise ValueError('invalid DST payload')
    try: data=base64.b64decode(payload,validate=True)
    except (ValueError,TypeError) as exc: raise ValueError('invalid base64') from exc
    if len(data)<515 or len(data)>512+3*MAX_EVENTS or not data.startswith(b'LA:'):
        raise ValueError('invalid M1 DST envelope')
    expected=bundle.get('provenance',{}).get('dst_sha256')
    if not isinstance(expected,str) or not HEX.fullmatch(expected) or sha(data)!=expected:
        raise ValueError('DST SHA-256 mismatch')
    t=bundle.get('trace',{}); events=t.get('events')
    if not isinstance(events,list) or not events or len(events)>MAX_EVENTS or len(data)!=512+3*len(events) or t.get('record_count')!=len(events):
        raise ValueError('record cardinality mismatch')
    palette=bundle.get('palette')
    if not isinstance(palette,list) or not 1<=len(palette)<=256: raise ValueError('palette required')
    for col in palette:
        rgb=col.get('rgb')
        if not isinstance(rgb,list) or len(rgb)!=3 or not all(type(v) is int and 0<=v<=255 for v in rgb): raise ValueError('invalid RGB palette')
    prior=[0,0]; end=0; needle_count=0
    for i,e in enumerate(events):
        if not isinstance(e,dict): raise ValueError('event object required')
        if e.get('index')!=i or e.get('byte_offset')!=512+3*i or e.get('raw')!=list(data[512+3*i:515+3*i]):
            raise ValueError('raw byte binding mismatch')
        if e.get('kind') not in ('stitch','jump','color_change','end'): raise ValueError('unsupported event kind')
        for key in ('from_units','to_units'):
            p=e.get(key)
            if not isinstance(p,list) or len(p)!=2 or not all(type(v) is int and abs(v)<=1_000_000 for v in p):
                raise ValueError('bounded integer coordinates required')
        if e['from_units']!=prior: raise ValueError('discontinuous supplied trace')
        prior=e['to_units']
        start=finite(e.get('start_ms'),'start_ms'); stop=finite(e.get('end_ms'),'end_ms')
        if start!=end or stop<=start: raise ValueError('non-monotonic timeline')
        end=stop
        c=e.get('color_index')
        if type(c) is not int or not 0<=c<len(palette): raise ValueError('colour index outside palette')
        if e['kind']=='end' and i!=len(events)-1: raise ValueError('end before final record')
        needle_count+=e['kind']=='stitch'
    if events[-1]['kind']!='end' or events[-1]['raw']!=[0,0,243]: raise ValueError('canonical final END required')
    if t.get('needle_placements')!=needle_count or t.get('duration_ms')!=end: raise ValueError('summary differs from supplied trace')
    return data


def source_image(path: Path, page_number:int=1):
    raw=bounded_read(path)
    if raw.startswith(b'%PDF-'):
        try: import fitz
        except ImportError as e: raise ValueError('PDF source requires PyMuPDF; raster source remains available') from e
        with fitz.open(stream=raw,filetype='pdf') as doc:
            if doc.needs_pass: raise ValueError('encrypted PDF source is not supported')
            if not 1<=len(doc)<=100 or not 1<=page_number<=len(doc): raise ValueError('PDF page outside bounded range')
            page=doc[page_number-1]; rect=page.rect
            if rect.is_empty or not all(math.isfinite(v) for v in [rect.width,rect.height]): raise ValueError('invalid PDF page bounds')
            scale=min(2.0,1600/max(rect.width,rect.height))
            pix=page.get_pixmap(matrix=fitz.Matrix(scale,scale),alpha=False)
            im=Image.open(io.BytesIO(pix.tobytes('png'))).convert('RGB')
        return im, {'kind':'source_pdf_page_raster_derivative','page':page_number,'source_sha256':sha(raw),'filename':path.name}
    with Image.open(io.BytesIO(raw)) as opened:
        if opened.format not in ('PNG','JPEG','WEBP','TIFF','BMP'): raise ValueError('unsupported raster format')
        if opened.width*opened.height>20_000_000: raise ValueError('raster exceeds 20 megapixels')
        opened.seek(0)
        oriented=ImageOps.exif_transpose(opened)
        rgba=oriented.convert('RGBA'); bg=Image.new('RGBA',rgba.size,'white'); bg.alpha_composite(rgba); im=bg.convert('RGB')
        im.thumbnail((1600,1600))
    return im, {'kind':'source_raster_derivative','page':1,'source_sha256':sha(raw),'filename':path.name}


def safe_text(value, limit=100):
    return str(value).replace('\n',' ').encode('ascii','replace').decode('ascii')[:limit]


def make_proof(bundle_path:Path,source_path:Path,out:Path,source_width_mm:float,source_x_mm:float=0,source_y_mm:float=0,page:int=1,overwrite:bool=False):
    for v,n,pos in [(source_width_mm,'source width',True),(source_x_mm,'source X',False),(source_y_mm,'source Y',False)]: finite(v,n,pos)
    manifest_path=out.with_suffix('.manifest.json')
    inputs={bundle_path.resolve(),source_path.resolve()}
    if out.resolve() in inputs or manifest_path.resolve() in inputs: raise ValueError('output cannot overwrite an input')
    if not overwrite and (out.exists() or manifest_path.exists()): raise ValueError('output exists; use --overwrite explicitly')
    raw_bundle=bounded_read(bundle_path)
    bundle=json.loads(raw_bundle); dst=validate_bundle(bundle)
    im,source=source_image(source_path,page)
    h_mm=source_width_mm*im.height/im.width
    if max(source_width_mm,h_mm)>2000: raise ValueError('source dimensions exceed 2000 mm proof limit')
    events=bundle['trace']['events']
    coords=[p for e in events for p in [e['from_units'],e['to_units']]]
    lo_x=min([p[0]/10 for p in coords]+[source_x_mm]); hi_x=max([p[0]/10 for p in coords]+[source_x_mm+source_width_mm])
    lo_y=min([p[1]/10 for p in coords]+[source_y_mm]); hi_y=max([p[1]/10 for p in coords]+[source_y_mm+h_mm])
    pad=max(1.0,(max(hi_x-lo_x,hi_y-lo_y))*.08); lo_x-=pad;lo_y-=pad;hi_x+=pad;hi_y+=pad
    W,H=landscape(A4); c=canvas.Canvas(str(out),pagesize=(W,H),invariant=1,pageCompression=1)
    c.setTitle('EmbroideryOS - Source and byte-bound DST replay proof')
    c.setAuthor('EmbroideryOS proof utility')
    mode=bundle['kernel']['mode']; fixture=mode=='transport-test-fixture'
    def header(title,number):
        c.setFillColorRGB(.07,.10,.16);c.setFont('Helvetica-Bold',21);c.drawString(36,H-43,title)
        c.setFont('Helvetica',9);c.setFillColorRGB(.28,.33,.40)
        c.drawString(36,H-62,'RECORDED FIXTURE - NOT LIVE RUST' if fixture else 'SUPPLIED NATIVE REPLAY - RUST NOT RE-RUN BY THIS UTILITY')
        c.setFont('Helvetica',8);c.drawRightString(W-36,22,f'{number} / 2 | Engineering review only - not a machine release')
    def panel(x,y,w,h,draw_source=True,draw_trace=False,actual=False):
        c.setStrokeColorRGB(.81,.84,.88);c.setFillColorRGB(.975,.98,.985);c.roundRect(x,y,w,h,8,stroke=1,fill=1)
        scale=MM if actual else min((w-26)/(hi_x-lo_x),(h-26)/(hi_y-lo_y))
        ox=x+(w-(hi_x-lo_x)*scale)/2-lo_x*scale; oy=y+(h-(hi_y-lo_y)*scale)/2+hi_y*scale
        c.saveState();clip=c.beginPath();clip.rect(x+1,y+1,w-2,h-2);c.clipPath(clip,stroke=0,fill=0)
        if draw_source: c.drawImage(ImageReader(im),ox+source_x_mm*scale,oy-(source_y_mm+h_mm)*scale,width=source_width_mm*scale,height=h_mm*scale)
        if draw_trace:
            for e in events:
                if e['kind'] not in ('stitch','jump'):continue
                a,b=e['from_units'],e['to_units'];col=bundle['palette'][e['color_index']]['rgb']
                c.setStrokeColorRGB(*[v/255 for v in col]);c.setLineWidth(.55 if e['kind']=='stitch' else .35);c.setDash([] if e['kind']=='stitch' else [2,2])
                c.line(ox+a[0]/10*scale,oy-a[1]/10*scale,ox+b[0]/10*scale,oy-b[1]/10*scale)
            c.setDash([])
        c.restoreState()
        return scale
    header('Original artwork / exported stitch view',1)
    pw=(W-88)/2;py=H-386;ph=288
    c.setFillColorRGB(.10,.14,.20);c.setFont('Helvetica-Bold',11);c.drawString(36,H-88,'01  Source artwork');c.drawString(52+pw,H-88,'02  DST replay view')
    scale=panel(36,py,pw,ph,True,False);panel(52+pw,py,pw,ph,False,True)
    c.setFont('Helvetica',9);c.setFillColorRGB(.21,.26,.33)
    lines=[f"Source: {safe_text(source['filename'],64)} | page/frame {source['page']} | explicit width {source_width_mm:g} mm",
           f"Common viewport: X {lo_x:.2f} to {hi_x:.2f} mm; Y {lo_y:.2f} to {hi_y:.2f} mm | display scale {scale/MM:.2f}:1",
           f"Records: {len(events)} | needle placements: {bundle['trace']['needle_placements']} | trace timing: illustrative",
           'Raster resolution is not calibration. Thread colours are supplied palette metadata, not recovered original colours.',
           'SHA-256 and raw-record binding are checked. Coordinate semantics and physical stitch quality are not revalidated.',
           'The attached source is user-selected; this tool does not assert it is the historical artwork that produced the DST.']
    for i,line in enumerate(lines): c.drawString(36,py-24-i*16,line)
    c.setFont('Helvetica',8);c.drawString(36,py-136,'Source SHA-256: '+source['source_sha256']);c.drawString(36,py-150,'DST SHA-256: '+sha(dst))
    c.showPage();header('Overlay / physical-scale proof',2)
    c.setFont('Helvetica-Bold',11);c.setFillColorRGB(.10,.14,.20);c.drawString(36,H-88,'03  Registered overlay');c.drawString(52+pw,H-88,'04  Physical scale 1:1')
    panel(36,py,pw,ph,True,True)
    fits=(hi_x-lo_x)*MM<=pw-26 and (hi_y-lo_y)*MM<=ph-26
    if fits: panel(52+pw,py,pw,ph,False,True,True)
    else:
        c.setFillColorRGB(.21,.26,.33);c.setFont('Helvetica',10)
        c.drawString(60+pw,py+ph/2,'Design exceeds this 1:1 proof area.')
        c.drawString(60+pw,py+ph/2-17,'No automatic downscaling. Use tiled proof implementation.')
    barx=52+pw+20;bary=py-30;c.setStrokeColorRGB(.10,.14,.20);c.setLineWidth(.8);c.line(barx,bary,barx+100*MM,bary)
    for k in range(11): xx=barx+k*10*MM;c.line(xx,bary-3,xx,bary+(7 if k in (0,10) else 3))
    c.setFillColorRGB(.21,.26,.33);c.setFont('Helvetica',9);c.drawString(barx,bary-18,'100 mm - print at Actual size / 100%; measure after printing')
    c.setFont('Helvetica',9)
    for i,line in enumerate(['Overlay uses explicitly entered source position and scale; registration is not inferred.',
        'No fabric simulation, device calibration, printer calibration, machine transfer or ISO certification.',
        'PDF page coordinates are physical units; monitor zoom is not guaranteed actual size.',
        'Full byte-binding and source/proof hashes are in the accompanying manifest.']):c.drawString(36,py-62-i*16,line)
    c.save()
    manifest={'schema':'embroideryos-compare-proof/0.1','source':dict(source,width_mm=source_width_mm,height_mm=h_mm,x_mm=source_x_mm,y_mm=source_y_mm),'bundle_file_sha256':sha(raw_bundle),'dst_sha256':sha(dst),'pdf_sha256':sha(out.read_bytes()),'kernel_mode_reported':mode,'checks':{'dst_hash':True,'record_byte_binding':True,'trace_structure':True,'rust_executed':False,'decoder_semantics_independently_verified':False,'historical_source_association_verified':False,'physical_qualification':False},'actual_size_panel':fits,'page_size_pt':[W,H],'provenance_note':'Integrity-only review; no authenticated release authority.'}
    manifest_path.write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
    return manifest


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--bundle',type=Path,required=True);p.add_argument('--source',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    p.add_argument('--source-width-mm',type=float,required=True);p.add_argument('--source-x-mm',type=float,default=0);p.add_argument('--source-y-mm',type=float,default=0);p.add_argument('--page',type=int,default=1);p.add_argument('--overwrite',action='store_true')
    a=p.parse_args()
    try:
        result=make_proof(a.bundle,a.source,a.output,a.source_width_mm,a.source_x_mm,a.source_y_mm,a.page,a.overwrite)
        print(json.dumps(result,indent=2))
    except (OSError,ValueError,KeyError,TypeError) as e:p.exit(2,f'Proof refused: {e}\n')
if __name__=='__main__':main()
