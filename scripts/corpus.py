#!/usr/bin/env python3
"""Freeze, fetch, and package Fastvid corpus source masters."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import gzip
import html
import json
import re
import sys
import tarfile
import urllib.parse
import urllib.request
from collections import Counter
from pathlib import Path

import numpy as np
import rawpy


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "corpus"
SOURCE_ROOT = CORPUS / "sources"
AI_PROMPTS = CORPUS / "ai-prompts.json"
MASTER_CATALOG = CORPUS / "master-sources-v1.json"
MASTER_CHECKSUMS = CORPUS / "master-source-checksums-v1.sha256"
AI_CHECKSUMS = CORPUS / "ai-source-checksums.sha256"
AI_ARCHIVE = CORPUS / "fastvid-corpus-v1-ai-sources.tar.gz"
RAW_PIXLS_REPOSITORY = "https://raw.pixls.us/json/getrepository.php?set=all"
POLY_HAVEN_ASSETS = "https://api.polyhaven.com/assets"
POLY_HAVEN_FILES = "https://api.polyhaven.com/files/{asset}"
RAW_MAKE_QUOTAS = {
    "Canon": 8,
    "Nikon": 8,
    "Sony": 8,
    "Panasonic": 8,
    "Fujifilm": 6,
    "Olympus": 4,
    "Pentax": 4,
    "Hasselblad": 2,
    "Phase One": 2,
}
RAW_UNSUPPORTED_MODELS = {
    ("Nikon", "Z50_2"),
    ("Nikon", "Z5_2"),
    ("Sony", "ILCE-7M5"),
    ("Sony", "DSC-RX1RM3"),
    ("Sony", "ILCE-1M2"),
    ("Panasonic", "DC-S1M2"),
    ("Panasonic", "DC-S1M2ES"),
    ("Panasonic", "DC-GH7"),
    ("Panasonic", "DC-S5M2"),
    ("Panasonic", "DC-S9"),
    ("Panasonic", "DC-S1RM2"),
    ("Nikon", "Z f"),
    ("Nikon", "Z 8"),
    ("Panasonic", "DC-G9M2"),
    ("Panasonic", "DC-S5M2X"),
}
GAME_FILES = {
    "veloren": "File:Veloren Screenshot 2023.02.09 - 16.12.11.63.png",
    "0ad": "File:Screenshot 0 A.D. Delenda Est 20230208.png",
    "minetest": "File:QiskitBlocks Minetest 5.6.1 screenshot 20230205 135130.png",
    "supertuxkart": "File:STK track 8.png",
    "wesnoth": "File:The Battle for Wesnoth 1.16.2 title screen.png",
    "endless-sky": "File:Endless Sky 0.9.12 title screen.png",
    "freeciv": "File:Fciv-net-screenshot-2023-03-05.png",
    "xonotic": "File:Screenshot of Xonotic main menu.jpg",
}
USER_AGENT = "FastvidCorpus/1.0 (https://github.com/fastvid; reproducible codec corpus)"
API = "https://commons.wikimedia.org/w/api.php"
LICENSES = {
    "CC0",
    "Public domain",
    "CC BY 2.0",
    "CC BY 1.0",
    "CC BY 2.5",
    "CC BY 3.0",
    "CC BY 4.0",
    "CC BY-SA 2.0",
    "CC BY-SA 2.5",
    "CC BY-SA 3.0",
    "CC BY-SA 4.0",
    "GPLv2",
    "GPLv2+",
    "GPLv3+",
    "GPL",
    "GPLv3",
}
AI_FILENAMES = [
    "ai-01-farmers-market.png",
    "ai-02-family-kitchen.png",
    "ai-03-alpine-lake.png",
    "ai-04-fox-woodland.png",
    "ai-05-neon-game-plaza.png",
    "ai-06-fantasy-game-city.png",
    "ai-07-glass-material-study.png",
    "ai-08-library-render.png",
    "ai-09-creative-ui.png",
    "ai-10-microscopy.png",
    "ai-11-storm-wave.png",
    "ai-12-night-festival.png",
    "ai-13-beetle-macro.png",
    "ai-14-generative-ribbons.png",
    "ai-15-aerial-farmland.png",
    "ai-16-painted-harbor.png",
    "ai-17-voxel-factory.png",
    "ai-18-material-still-life.png",
    "ai-19-winter-station.png",
    "ai-20-desert-observatory.png",
]


def request_json(params: dict[str, str | int]) -> dict:
    url = API + "?" + urllib.parse.urlencode(params)
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.load(response)


def clean_markup(value: str) -> str:
    value = html.unescape(value or "")
    value = re.sub(r"<[^>]+>", "", value)
    return " ".join(value.split())


def get_json(url: str) -> dict:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.load(response)


def discover_masters(_: argparse.Namespace) -> None:
    """Freeze the high-bit/lossless portion of corpus v1 before downloading."""
    items: list[dict] = []

    raw_rows = get_json(RAW_PIXLS_REPOSITORY)["data"]
    raw_candidates: dict[str, list[dict]] = {make: [] for make in RAW_MAKE_QUOTAS}
    allowed_extensions = {
        ".3fr", ".arw", ".cr2", ".cr3", ".dng", ".nef", ".nrw",
        ".iiq", ".orf", ".pef", ".raf", ".raw", ".rw2",
    }
    for row in raw_rows:
        make, model, mode, megapixels, remark, license_html, date, raw_html, _ = row
        if (
            make not in RAW_MAKE_QUOTAS
            or (make, model) in RAW_UNSUPPORTED_MODELS
            or not isinstance(megapixels, (int, float))
            or megapixels < 12
            or "creativecommons.org/publicdomain/zero" not in license_html
        ):
            continue
        url_match = re.search(r"href='([^']+)'", raw_html)
        sha_match = re.search(r"\b([0-9a-f]{64})\b", raw_html)
        size_match = re.search(r"\(([0-9.]+)MB\)", raw_html)
        if not url_match or not sha_match:
            continue
        url = html.unescape(url_match.group(1))
        extension = Path(urllib.parse.urlparse(url).path).suffix.lower()
        size_mb = float(size_match.group(1)) if size_match else 0
        if extension not in allowed_extensions or size_mb > 120:
            continue
        raw_candidates[make].append(
            {
                "provider": "raw.pixls.us",
                "source_group": f"camera:{make}:{model}",
                "make": make,
                "model": model,
                "mode": mode,
                "megapixels": megapixels,
                "remark": remark,
                "date": date,
                "license": "CC0",
                "license_url": "https://creativecommons.org/publicdomain/zero/1.0/",
                "url": url,
                "sha256": sha_match.group(1),
                "size_mb": size_mb,
                "extension": extension,
            }
        )
    for make, quota in RAW_MAKE_QUOTAS.items():
        chosen_models: set[str] = set()
        candidates = sorted(
            raw_candidates[make],
            key=lambda item: (
                item["mode"] not in ("", "RAW", "raw"),
                item["date"],
                item["model"],
                item["sha256"],
            ),
            reverse=True,
        )
        for candidate in candidates:
            if candidate["model"] in chosen_models:
                continue
            chosen_models.add(candidate["model"])
            candidate["id"] = f"raw-{len([x for x in items if x['provider'] == 'raw.pixls.us']) + 1:02d}"
            slug = re.sub(
                r"[^a-z0-9]+",
                "-",
                f"{candidate['make']}-{candidate['model']}".lower(),
            ).strip("-")
            candidate["source_path"] = (
                f"v1/raw-pixls/{slug}-{candidate['sha256'][:12]}"
                f"{candidate['extension']}"
            )
            items.append(candidate)
            if len(chosen_models) == quota:
                break
        if len(chosen_models) != quota:
            raise SystemExit(f"raw.pixls.us: {make} only supplied {len(chosen_models)}/{quota}")

    assets = get_json(POLY_HAVEN_ASSETS)
    hdri_candidates = [
        (asset_id, metadata)
        for asset_id, metadata in assets.items()
        if metadata.get("type") == 0
        and (metadata.get("max_resolution") or [0])[0] >= 4096
    ]
    # Round-robin broad top-level categories before taking a second from one.
    buckets: dict[str, list[tuple[str, dict]]] = {}
    for asset_id, metadata in sorted(hdri_candidates):
        category = (metadata.get("categories") or ["uncategorized"])[0].split("/")[0]
        buckets.setdefault(category, []).append((asset_id, metadata))
    selected_hdris: list[tuple[str, dict]] = []
    depth = 0
    while len(selected_hdris) < 24:
        added = False
        for category in sorted(buckets):
            if depth < len(buckets[category]):
                selected_hdris.append(buckets[category][depth])
                added = True
                if len(selected_hdris) == 24:
                    break
        if not added:
            raise SystemExit("Poly Haven supplied fewer than 24 qualifying HDRIs")
        depth += 1
    for number, (asset_id, metadata) in enumerate(selected_hdris, 1):
        files = get_json(POLY_HAVEN_FILES.format(asset=urllib.parse.quote(asset_id)))
        source = files.get("hdri", {}).get("4k", {}).get("exr")
        if not source:
            raise SystemExit(f"Poly Haven asset has no 4K EXR: {asset_id}")
        items.append(
            {
                "id": f"hdri-{number:02d}",
                "provider": "Poly Haven",
                "source_group": f"hdri:{asset_id}",
                "asset_id": asset_id,
                "name": metadata.get("name"),
                "categories": metadata.get("categories", []),
                "tags": metadata.get("tags", []),
                "license": "CC0",
                "license_url": "https://polyhaven.com/license",
                "url": source["url"],
                "md5": source["md5"],
                "size_bytes": source["size"],
                "source_path": f"v1/polyhaven/{asset_id}_4k.exr",
            }
        )

    netflix_license = (
        "http://download.opencontent.netflix.com.s3.amazonaws.com/"
        "TechblogAssets/creative-commons-attribution-4-intl-public-license.txt"
    )
    netflix_frames = [
        ("chimera-03000", "Chimera", "Chimera/tif_DCI4k2398p/Chimera_DCI4k2398p_HDR_P3PQ_03000.tif"),
        ("chimera-07500", "Chimera", "Chimera/tif_DCI4k2398p/Chimera_DCI4k2398p_HDR_P3PQ_07500.tif"),
        ("meridian-01000", "Meridian", "Meridian/tiffs/Meridian_UHD4k5994p_HDR_P3PQ_01000.tif"),
        ("meridian-05000", "Meridian", "Meridian/tiffs/Meridian_UHD4k5994p_HDR_P3PQ_05000.tif"),
        ("sparks-01000", "Sparks", "sparks/SPARKS_4K_P3_PQ_4000nits_EXR/SPARKS_P3_PQ_4000nit_01000.exr"),
        ("sparks-05000", "Sparks", "sparks/SPARKS_4K_P3_PQ_4000nits_EXR/SPARKS_P3_PQ_4000nit_05000.exr"),
    ]
    for item_id, title, key in netflix_frames:
        items.append(
            {
                "id": item_id,
                "provider": "Netflix Open Content",
                "source_group": f"sequence:{title}",
                "title": title,
                "license": "CC BY 4.0",
                "license_url": netflix_license,
                "url": "http://download.opencontent.netflix.com.s3.amazonaws.com/" + key,
                "source_path": "v1/netflix/" + key.replace("/", "_"),
            }
        )
    for title, filename, frames in [
        ("El Fuente Boat", "Netflix_Boat_4096x2160_60fps_10bit_420.y4m", [60, 240]),
        ("El Fuente Food Market", "Netflix_FoodMarket_4096x2160_60fps_10bit_420.y4m", [120, 480]),
    ]:
        url = "http://download.opencontent.netflix.com.s3.amazonaws.com/ElFuente/" + filename
        for frame in frames:
            items.append(
                {
                    "id": f"{title.lower().replace(' ', '-')}-{frame:04d}",
                    "provider": "Netflix Open Content",
                    "source_group": f"sequence:{title}",
                    "title": title,
                    "license": "CC BY 4.0",
                    "license_url": netflix_license,
                    "url": url,
                    "frame": frame,
                    "source_format": "YUV4MPEG2 4096x2160 C420p10",
                    "source_path": f"v1/netflix/{title.lower().replace(' ', '-')}-{frame:04d}.y4m",
                }
            )

    for item_id, title, base, filename in [
        ("xiph-sintel-01000", "Sintel", "https://media.xiph.org/sintel/sintel-4k-tiff16/", "00001000.tif"),
        ("xiph-sintel-13500", "Sintel", "https://media.xiph.org/sintel/sintel-4k-tiff16/", "00013500.tif"),
        ("xiph-tears-03000", "Tears of Steel", "https://media.xiph.org/tearsofsteel/tearsofsteel-4k-tiff/", "graded_edit_03000.tif"),
        ("xiph-tears-14000", "Tears of Steel", "https://media.xiph.org/tearsofsteel/tearsofsteel-4k-tiff/", "graded_edit_14000.tif"),
    ]:
        items.append(
            {
                "id": item_id,
                "provider": "Xiph.org",
                "source_group": f"sequence:{title}",
                "title": title,
                "license": "CC BY 3.0",
                "license_url": "https://creativecommons.org/licenses/by/3.0/",
                "url": base + filename,
                "source_path": f"v1/xiph/{item_id}.tif",
            }
        )

    game_data = request_json(
        {
            "action": "query",
            "format": "json",
            "formatversion": 2,
            "titles": "|".join(GAME_FILES.values()),
            "prop": "imageinfo|revisions",
            "iiprop": "size|mime|extmetadata|url|sha1",
            "rvprop": "ids|timestamp",
        }
    )
    pages_by_title = {
        page["title"]: page
        for page in game_data.get("query", {}).get("pages", [])
    }
    for game, title in GAME_FILES.items():
        page = pages_by_title.get(title)
        if not page or page.get("missing"):
            raise SystemExit(f"missing frozen game screenshot: {title}")
        info = page["imageinfo"][0]
        metadata = info.get("extmetadata", {})
        license_name = clean_markup(
            metadata.get("LicenseShortName", {}).get("value", "")
        )
        if license_name not in LICENSES:
            raise SystemExit(f"unsupported game screenshot license: {title}: {license_name}")
        extension = Path(urllib.parse.urlparse(info["url"]).path).suffix.lower()
        revision = (page.get("revisions") or [{}])[0]
        items.append(
            {
                "id": f"game-{game}",
                "provider": "Wikimedia Commons",
                "source_group": f"game:{game}",
                "game_title": game,
                "title": title,
                "page_id": page["pageid"],
                "revision_id": revision.get("revid"),
                "revision_timestamp": revision.get("timestamp"),
                "width": info["width"],
                "height": info["height"],
                "mime": info["mime"],
                "license": license_name,
                "license_url": clean_markup(
                    metadata.get("LicenseUrl", {}).get("value", "")
                ),
                "url": info["url"],
                "original_sha1": info.get("sha1"),
                "source_path": f"v1/games/{game}{extension}",
                "low_bit_exception": True,
            }
        )

    for index, pattern in enumerate(
        ("rgb-noise", "gradient-dither", "moire", "checker-impulses"),
        1,
    ):
        items.append(
            {
                "id": f"procedural-{index:02d}",
                "provider": "Fastvid deterministic generator",
                "source_group": f"procedural:{pattern}",
                "pattern": pattern,
                "seed": 0xFA57_1600 + index,
                "width": 3840,
                "height": 2160,
                "bit_depth": 16,
                "pixel_format": "RGB48LE",
                "license": "CC0",
                "source_path": f"v1/procedural/{pattern}-3840x2160-rgb48le.raw",
            }
        )

    groups = Counter(item["source_group"] for item in items)
    over_cap = sorted(group for group, count in groups.items() if count > 2)
    if over_cap:
        raise SystemExit(f"source group exceeds two-item cap: {over_cap}")
    payload = {
        "schema": 1,
        "status": "source-master selection complete; four procedural 16-bit patterns are generated locally",
        "counts": {
            "camera_raw": 50,
            "poly_haven_exr": 24,
            "netflix_high_bit": 10,
            "xiph_lossless": 4,
            "distinct_game_titles": 8,
            "procedural_16bit": 4,
            "frozen_ai_1080p": 20,
            "source_master_items": len(items),
        },
        "items": items,
    }
    MASTER_CATALOG.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n")
    print(f"wrote {MASTER_CATALOG} with {len(items)} frozen high-bit/lossless masters")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def md5(path: Path) -> str:
    digest = hashlib.md5()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url: str, destination: Path) -> None:
    if destination.exists():
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix(destination.suffix + ".part")
    parts = urllib.parse.urlsplit(url)
    safe_url = urllib.parse.urlunsplit(
        (
            parts.scheme,
            parts.netloc,
            urllib.parse.quote(urllib.parse.unquote(parts.path), safe="/%:@"),
            parts.query,
            parts.fragment,
        )
    )
    request = urllib.request.Request(safe_url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=180) as response:
        with temporary.open("wb") as output:
            while chunk := response.read(1024 * 1024):
                output.write(chunk)
    temporary.replace(destination)


def download_y4m_frame(item: dict, destination: Path) -> None:
    if destination.exists():
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    request = urllib.request.Request(
        item["url"],
        headers={"Range": "bytes=0-255", "User-Agent": USER_AGENT},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        prefix = response.read(256)
    header_end = prefix.index(b"\n") + 1
    header = prefix[:header_end]
    if header != b"YUV4MPEG2 W4096 H2160 F60:1 Ip A1:1 C420p10\n":
        raise RuntimeError(f"unexpected Y4M header for {item['id']}: {header!r}")
    frame_bytes = 4096 * 2160 * 3
    record_bytes = len(b"FRAME\n") + frame_bytes
    record_start = header_end + item["frame"] * record_bytes
    record_end = record_start + record_bytes - 1
    request = urllib.request.Request(
        item["url"],
        headers={
            "Range": f"bytes={record_start}-{record_end}",
            "User-Agent": USER_AGENT,
        },
    )
    temporary = destination.with_suffix(destination.suffix + ".part")
    with urllib.request.urlopen(request, timeout=180) as response:
        record = response.read()
    if len(record) != record_bytes or not record.startswith(b"FRAME\n"):
        raise RuntimeError(
            f"bad Y4M range for {item['id']}: {len(record)} bytes, "
            f"prefix={record[:16]!r}"
        )
    with temporary.open("wb") as output:
        output.write(header)
        output.write(record)
    temporary.replace(destination)


def generate_pattern16(item: dict, destination: Path) -> None:
    if destination.exists():
        return
    width, height = item["width"], item["height"]
    seed = item["seed"]
    pattern = item["pattern"]
    rng = np.random.default_rng(seed)
    y, x = np.ogrid[:height, :width]
    if pattern == "rgb-noise":
        array = rng.integers(0, 65536, (height, width, 3), dtype=np.uint16)
    elif pattern == "gradient-dither":
        r = np.broadcast_to((x * 65535) // (width - 1), (height, width))
        g = np.broadcast_to((y * 65535) // (height - 1), (height, width))
        b = ((x + y) * 65535) // (width + height - 2)
        dither = ((x ^ y) & 15) - 7
        array = np.stack(
            tuple(np.clip(channel + dither, 0, 65535) for channel in (r, g, b)),
            axis=2,
        ).astype("<u2")
    elif pattern == "moire":
        wave = 32767 + np.rint(
            30000 * np.sin(x * 0.37 + np.sin(y * 0.061) * 9)
        )
        array = np.stack(
            (
                wave,
                np.mod(wave + x * 29, 65536),
                np.mod(65535 - wave + y * 31, 65536),
            ),
            axis=2,
        ).astype("<u2")
    elif pattern == "checker-impulses":
        cells = (((x // 2) ^ (y // 2)) & 1).astype(bool)
        array = np.empty((height, width, 3), dtype="<u2")
        array[cells] = (62000, 4000, 47000)
        array[~cells] = (900, 59000, 12000)
        points_y = rng.integers(0, height, 50000)
        points_x = rng.integers(0, width, 50000)
        array[points_y, points_x] = rng.integers(
            0, 65536, (50000, 3), dtype=np.uint16
        )
    else:
        raise RuntimeError(f"unknown procedural pattern: {pattern}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix(destination.suffix + ".part")
    temporary.write_bytes(array.astype("<u2", copy=False).tobytes())
    temporary.replace(destination)


def fetch_masters(args: argparse.Namespace) -> None:
    catalog = json.loads(MASTER_CATALOG.read_text())
    items = catalog["items"]
    pinned = {}
    if MASTER_CHECKSUMS.exists() and not args.refresh_checksums:
        pinned = {
            path: digest
            for digest, path in (
                line.split("  ", 1)
                for line in MASTER_CHECKSUMS.read_text().splitlines()
                if line
            )
        }
        expected_paths = {item["source_path"] for item in items}
        if set(pinned) != expected_paths:
            raise SystemExit(
                "pinned master checksums do not match the catalog; "
                "maintainers must review and use --refresh-checksums explicitly"
            )

    def fetch_one(item: dict) -> tuple[str, str, str]:
        destination = SOURCE_ROOT / item["source_path"]
        if item["provider"] == "Fastvid deterministic generator":
            generate_pattern16(item, destination)
        elif item.get("source_format", "").startswith("YUV4MPEG2"):
            download_y4m_frame(item, destination)
        else:
            download(item["url"], destination)
        actual_sha256 = sha256(destination)
        if item.get("sha256") and actual_sha256 != item["sha256"]:
            raise RuntimeError(f"SHA-256 mismatch: {item['id']}")
        if item.get("md5") and md5(destination) != item["md5"]:
            raise RuntimeError(f"MD5 mismatch: {item['id']}")
        if pinned and actual_sha256 != pinned[item["source_path"]]:
            raise RuntimeError(f"pinned SHA-256 mismatch: {item['id']}")
        if item["provider"] == "raw.pixls.us":
            with rawpy.imread(str(destination)) as raw:
                if raw.sizes.raw_width <= 0 or raw.sizes.raw_height <= 0:
                    raise RuntimeError(f"invalid camera RAW dimensions: {item['id']}")
        return item["id"], item["source_path"], actual_sha256

    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as executor:
        futures = {executor.submit(fetch_one, item): item for item in items}
        for future in concurrent.futures.as_completed(futures):
            item_id, path, digest = future.result()
            results.append((path, digest))
            print(f"fetched {item_id}", file=sys.stderr)
    if args.refresh_checksums or not MASTER_CHECKSUMS.exists():
        lines = [f"{digest}  {path}" for path, digest in sorted(results)]
        MASTER_CHECKSUMS.write_text("\n".join(lines) + "\n")
    print(f"fetched and verified {len(results)} source masters")


def archive_ai(_: argparse.Namespace) -> None:
    ai_root = SOURCE_ROOT / "ai"
    paths = [ai_root / filename for filename in AI_FILENAMES]
    missing = [str(path) for path in paths if not path.is_file()]
    if missing:
        raise SystemExit(f"missing frozen AI originals: {missing}")
    checksums = [
        (str(Path("ai") / path.relative_to(ai_root)), sha256(path))
        for path in paths
    ]
    AI_CHECKSUMS.write_text(
        "".join(f"{digest}  {relative}\n" for relative, digest in checksums)
    )
    temporary_tar = AI_ARCHIVE.with_suffix("")
    with tarfile.open(temporary_tar, "w", format=tarfile.PAX_FORMAT) as tar:
        for path in paths:
            relative = Path("ai") / path.relative_to(ai_root)
            info = tar.gettarinfo(str(path), arcname=str(relative))
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            info.mtime = 0
            with path.open("rb") as source:
                tar.addfile(info, source)
        for metadata in (AI_PROMPTS, AI_CHECKSUMS):
            info = tar.gettarinfo(str(metadata), arcname=metadata.name)
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            info.mtime = 0
            with metadata.open("rb") as source:
                tar.addfile(info, source)
    with temporary_tar.open("rb") as source, AI_ARCHIVE.open("wb") as raw:
        with gzip.GzipFile(
            filename="", mode="wb", fileobj=raw, mtime=0, compresslevel=6
        ) as output:
            while chunk := source.read(1024 * 1024):
                output.write(chunk)
    temporary_tar.unlink()
    print(f"{sha256(AI_ARCHIVE)}  {AI_ARCHIVE}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    discover_masters_parser = subparsers.add_parser("discover-masters")
    discover_masters_parser.set_defaults(function=discover_masters)
    fetch_masters_parser = subparsers.add_parser("fetch-masters")
    fetch_masters_parser.add_argument("--jobs", type=int, default=6)
    fetch_masters_parser.add_argument(
        "--refresh-checksums",
        action="store_true",
        help="maintainer-only: replace pinned hashes after reviewing catalog changes",
    )
    fetch_masters_parser.set_defaults(function=fetch_masters)
    archive_ai_parser = subparsers.add_parser("archive-ai")
    archive_ai_parser.set_defaults(function=archive_ai)
    return parser.parse_args()


if __name__ == "__main__":
    arguments = parse_args()
    arguments.function(arguments)
