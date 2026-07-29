#!/usr/bin/env python3
"""Deterministically extract frozen Fastvid masters into evaluator raw files."""
from __future__ import annotations
import argparse, concurrent.futures, hashlib, json, os, shutil, subprocess, sys
from pathlib import Path
import numpy as np
import rawpy

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "corpus/master-sources-v1.json"
MASTER_ROOT = ROOT / "corpus/sources"
DEFAULT_OUTPUT = ROOT / "artifacts/corpus-v1"
W4, H4, WH, HH = 3840, 2160, 1920, 1080
MATRIX = (("yuv422",8),("yuv422",10),("yuv422",16),("rgb444",10),("rgb444",16),("gray",8),("gray",10),("gray",16))
STRATIFIED_DEPTHS = {
    "yuv422": (8, 10, 16),
    "rgb444": (10, 16),
    "gray": (8, 10, 16),
}
REJECTION_CASES = {
    ("ai-01", "rgb444", 10),
    ("ai-05", "rgb444", 16),
    ("ai-13", "gray", 8),
    ("ai-13", "gray", 10),
    ("procedural-03", "gray", 16),
    ("procedural-03", "yuv422", 8),
    ("procedural-02", "yuv422", 10),
    ("procedural-02", "yuv422", 16),
}
AI_FILES = ("ai-01-farmers-market.png","ai-02-family-kitchen.png","ai-03-alpine-lake.png","ai-04-fox-woodland.png","ai-05-neon-game-plaza.png","ai-06-fantasy-game-city.png","ai-07-glass-material-study.png","ai-08-library-render.png","ai-09-creative-ui.png","ai-10-microscopy.png","ai-11-storm-wave.png","ai-12-night-festival.png","ai-13-beetle-macro.png","ai-14-generative-ribbons.png","ai-15-aerial-farmland.png","ai-16-painted-harbor.png","ai-17-voxel-factory.png","ai-18-material-still-life.png","ai-19-winter-station.png","ai-20-desert-observatory.png")

def sha256(path):
    digest=hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda:stream.read(1024*1024),b""): digest.update(block)
    return digest.hexdigest()

def rows():
    result=[]
    for item in json.loads(CATALOG.read_text())["items"]:
        result.append({**item,"path":MASTER_ROOT/item["source_path"],"ai":False})
    for name in AI_FILES:
        item_id="-".join(name.split("-",2)[:2])
        result.append({"id":item_id,"provider":"Fastvid frozen AI","source_group":"ai:"+item_id,"source_path":"ai/"+name,"path":MASTER_ROOT/"ai"/name,"ai":True})
    return result

def dimensions(row): return (WH,HH) if row["ai"] else (W4,H4)
def output_path(root,item_id,fmt,depth): return root/"raw"/item_id/f"{fmt}-{depth}.raw"
def expected_bytes(fmt,w,h): return w*h*({"gray":1,"yuv422":2,"rgb444":3}[fmt])*2

def stratified_cases(index):
    """Assign one depth per format using the frozen source-catalog order."""
    return {
        ("yuv422", STRATIFIED_DEPTHS["yuv422"][index % 3]),
        ("rgb444", STRATIFIED_DEPTHS["rgb444"][index % 2]),
        ("gray", STRATIFIED_DEPTHS["gray"][(index + 1) % 3]),
    }

def extraction_case_map(source_rows):
    """Return stratified cases plus every fixed rejection/performance input."""
    performance_ids = {
        row["id"]
        for row in [item for item in source_rows if dimensions(item) == (W4, H4)][:24]
    }
    result = {}
    for index, row in enumerate(source_rows):
        cases = stratified_cases(index)
        cases.update(
            (fmt, depth) for item_id, fmt, depth in REJECTION_CASES
            if item_id == row["id"]
        )
        if row["id"] in performance_ids:
            cases.update((("yuv422", 10), ("rgb444", 10)))
        result[row["id"]] = tuple(case for case in MATRIX if case in cases)
    return result

def ffmpeg_decode(path,w,h,ffmpeg,input_args=(),payload=None):
    vf=f"scale={w}:{h}:force_original_aspect_ratio=increase:flags=lanczos,crop={w}:{h},format=rgb48le"
    cmd=[ffmpeg,"-v","error",*input_args,"-i",("-" if payload is not None else str(path)),"-frames:v","1","-vf",vf,"-f","rawvideo","-pix_fmt","rgb48le","-"]
    done=subprocess.run(cmd,input=payload,check=True,capture_output=True)
    expected=w*h*6
    if len(done.stdout)!=expected: raise RuntimeError(f"{path}: FFmpeg emitted {len(done.stdout)}/{expected} bytes")
    return np.frombuffer(done.stdout,dtype="<u2").reshape(h,w,3).copy()

def decode(row,ffmpeg):
    w,h=dimensions(row); path=row["path"]
    if row["provider"]=="raw.pixls.us":
        with rawpy.imread(str(path)) as raw:
            rgb=raw.postprocess(output_bps=16,gamma=(1.,1.),no_auto_bright=True,use_camera_wb=True,output_color=rawpy.ColorSpace.sRGB)
        ih,iw,_=rgb.shape
        return ffmpeg_decode(path,w,h,ffmpeg,("-f","rawvideo","-pixel_format","rgb48le","-video_size",f"{iw}x{ih}"),rgb.astype("<u2",copy=False).tobytes())
    if row["provider"]=="Fastvid deterministic generator":
        return np.fromfile(path,dtype="<u2").reshape(H4,W4,3).copy()
    return ffmpeg_decode(path,w,h,ffmpeg)

def quantize(a,depth):
    if depth==16: return a.astype("<u2",copy=False)
    return ((a.astype(np.uint32)*((1<<depth)-1)+32767)//65535).astype("<u2")

def yuv422(rgb):
    a=rgb.astype(np.int64); r,g,b=a[...,0],a[...,1],a[...,2]
    y=(13933*r+46871*g+4732*b+32768)>>16
    cb=np.clip(((b-y)*35317+32768)>>16,-32768,32767)+32768
    cr=np.clip(((r-y)*41615+32768)>>16,-32768,32767)+32768
    return y.astype(np.uint16),((cb[:,::2]+cb[:,1::2]+1)>>1).astype(np.uint16),((cr[:,::2]+cr[:,1::2]+1)>>1).astype(np.uint16)

def write_planes(path,planes):
    path.parent.mkdir(parents=True,exist_ok=True); temporary=path.with_suffix(path.suffix+f".{os.getpid()}.part")
    with temporary.open("wb") as stream:
        for plane in planes: stream.write(plane.astype("<u2",copy=False).tobytes())
    temporary.replace(path); return sha256(path)

def extract_one(row,cases,root,ffmpeg,force):
    w,h=dimensions(row); outputs={}; complete=not force
    for fmt,depth in cases:
        path=output_path(root,row["id"],fmt,depth); complete &= path.is_file() and path.stat().st_size==expected_bytes(fmt,w,h)
    if not complete:
        rgb=decode(row,ffmpeg); y,cb,cr=yuv422(rgb)
        for fmt,depth in cases:
            path=output_path(root,row["id"],fmt,depth)
            if fmt=="rgb444": planes=tuple(quantize(rgb[...,i],depth) for i in range(3))
            elif fmt=="gray": planes=(quantize(y,depth),)
            else: planes=tuple(quantize(p,depth) for p in (y,cb,cr))
            outputs[f"{fmt}-{depth}"]={"path":path,"sha256":write_planes(path,planes)}
    else:
        for fmt,depth in cases:
            path=output_path(root,row["id"],fmt,depth); outputs[f"{fmt}-{depth}"]={"path":path,"sha256":sha256(path)}
    return {"id":row["id"],"provider":row["provider"],"source_group":row.get("source_group"),"source_path":row["source_path"],"source_sha256":sha256(row["path"]),"width":w,"height":h,"tiers":["full"],"outputs":outputs}

def sample(item,fmt,depth,root):
    output=item["outputs"][f"{fmt}-{depth}"]
    tiers = ["full", "rejection"] if (item["id"], fmt, depth) in REJECTION_CASES else ["full"]
    return {"id":f"{item['id']}-{fmt}-{depth}","path":str(output["path"].relative_to(root)),"sha256":output["sha256"],"width":item["width"],"height":item["height"],"format":fmt,"bit_depth":depth,"tiers":tiers,"source_id":item["id"],"source_sha256":item["source_sha256"]}

def performance(items,root):
    selected=[x for x in items if x["width"]==W4][:24]
    if len(selected)!=24: raise RuntimeError("fewer than 24 extracted 4K sources")
    result=[]
    for fmt in ("yuv422","rgb444"):
        key=f"{fmt}-10"
        result.append({"id":f"performance-4k-x24-{fmt}-10","paths":[str(x["outputs"][key]["path"].relative_to(root)) for x in selected],"sha256":[x["outputs"][key]["sha256"] for x in selected],"width":W4,"height":H4,"format":fmt,"bit_depth":10,"tiers":["rejection","full"]})
    ai=next(x for x in items if x["id"]=="ai-01"); output=ai["outputs"]["rgb444-10"]
    result.append({"id":"performance-1080p-rgb444-10","path":str(output["path"].relative_to(root)),"sha256":output["sha256"],"width":WH,"height":HH,"format":"rgb444","bit_depth":10,"tiers":["rejection","full"]})
    return result

def write_manifest(root,items,ffmpeg):
    samples=[
        sample(item,fmt,depth,root) for item in items for fmt,depth in MATRIX
        if f"{fmt}-{depth}" in item["outputs"]
    ]+performance(items,root)
    version=subprocess.run([ffmpeg,"-version"],check=True,capture_output=True,text=True).stdout.splitlines()[0]
    conversion={"schema_version":1,"source_catalog_sha256":sha256(CATALOG),"rawpy":rawpy.__version__,"numpy":np.__version__,"ffmpeg":version,"geometry":"center crop after Lanczos aspect-fill scaling","camera_raw":"16-bit linear gamma, camera white balance, no automatic brightness, sRGB primaries","other_sources":"FFmpeg decode to RGB48LE; HDR/PQ sources form an explicit high-bit SDR evaluation view","matrix":[f"{f}-{d}" for f,d in MATRIX],"stratification":"one depth per source per format, round-robin over frozen catalog order; fixed rejection and performance cases are unioned in","storage":"little-endian planar uint16 at every depth; RGB plane order R,G,B","yuv":"full-range BT.709 integer transform; horizontal two-tap box chroma downsample","bit_depth":"round(value16 * (2^depth-1) / 65535)"}
    manifest_sources=[]
    for item in items:
        public={key:value for key,value in item.items() if key!="outputs"}
        public["outputs"]={key:{"path":str(value["path"].relative_to(root)),"sha256":value["sha256"]} for key,value in item["outputs"].items()}
        manifest_sources.append(public)
    document={"schema_version":2,"revision":"fastvid-corpus-v1-extracted-2","conversion":conversion,"sources":manifest_sources,"samples":samples}
    path=root/"manifest.json"; temporary=path.with_suffix(".json.part")
    temporary.write_text(json.dumps(document,indent=2,sort_keys=True)+"\n"); temporary.replace(path); return path

def main():
    parser=argparse.ArgumentParser(description=__doc__); parser.add_argument("--output",type=Path,default=DEFAULT_OUTPUT); parser.add_argument("--ffmpeg",default="ffmpeg"); parser.add_argument("--jobs",type=int,default=2); parser.add_argument("--force",action="store_true"); args=parser.parse_args()
    if args.jobs<1: parser.error("--jobs must be positive")
    if shutil.which(args.ffmpeg) is None: parser.error(f"FFmpeg not found: {args.ffmpeg}")
    source_rows=rows(); case_map=extraction_case_map(source_rows); missing=[str(x["path"]) for x in source_rows if not x["path"].is_file()]
    if missing: parser.error(f"missing {len(missing)} source masters; first: {missing[0]}")
    root=args.output.resolve(); root.mkdir(parents=True,exist_ok=True); items=[]
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        futures={pool.submit(extract_one,row,case_map[row["id"]],root,args.ffmpeg,args.force):row for row in source_rows}
        for future in concurrent.futures.as_completed(futures):
            row=futures[future]; items.append(future.result()); print(f"extracted {row['id']}",file=sys.stderr,flush=True)
    order={row["id"]:i for i,row in enumerate(source_rows)}; items.sort(key=lambda x:order[x["id"]]); manifest=write_manifest(root,items,args.ffmpeg)
    print(json.dumps({"sources":len(items),"samples":len(json.loads(manifest.read_text())["samples"]),"manifest":str(manifest),"manifest_sha256":sha256(manifest)},indent=2)); return 0

if __name__=="__main__": raise SystemExit(main())
