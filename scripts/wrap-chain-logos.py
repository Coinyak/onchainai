#!/usr/bin/env python3
"""Wrap official chain logo SVGs into 48x48 public/chains/*.svg tiles (manifest-driven, harness-round-11)."""

from __future__ import annotations

import base64
import json
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "public" / "chains"
MANIFEST = ROOT / "scripts" / "chain-logo-manifest.json"

PROVENANCE = "<!-- official-brand-asset harness-round-11 -->"
TILE_VIEWBOX = 'viewBox="0 0 48 48"'
WRAP_CENTER = "translate(24 24)"
MAX_DIRECT_COORD = 48.0

TILE = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" role="img" aria-label="{label}">
  {provenance}
  <rect width="48" height="48" rx="8" fill="#fff"/>
  <g transform="translate(24 24) scale({scale}) translate({tx} {ty})">
    {inner}
  </g>
</svg>
"""

BASE_SQUARE = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" role="img" aria-label="Base">
  {PROVENANCE}
  <rect width="48" height="48" rx="8" fill="#fff"/>
  <rect x="9.6" y="9.6" width="28.8" height="28.8" rx="1.44" fill="#0052FF"/>
</svg>
"""

SUI_DROPLET = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" role="img" aria-label="Sui">
  {PROVENANCE}
  <rect width="48" height="48" rx="8" fill="#fff"/>
  <g transform="translate(6 4) scale(0.045)">
    <path fill-rule="evenodd" clip-rule="evenodd" d="M626.027 417.029C666.817 468.244 691.209 533.014 691.209 603.469C691.209 673.925 666.076 740.673 624.214 792.176L620.588 796.626L619.641 790.981C618.817 786.201 617.869 781.34 616.757 776.478C595.785 684.349 527.471 605.365 415.03 541.378C339.095 498.28 295.626 446.448 284.213 387.487C276.838 349.375 282.318 311.098 292.907 278.301C303.496 245.545 319.235 218.063 332.626 201.541L376.383 148.06C384.046 138.666 398.426 138.666 406.09 148.06L626.068 417.029H626.027ZM695.206 363.59L402.01 5.12968C396.407 -1.70989 385.942 -1.70989 380.338 5.12968L87.184 363.59L86.2363 364.784C32.3026 431.738 0 516.821 0 609.444C0 825.138 175.151 1000 391.174 1000C607.198 1000 782.349 825.138 782.349 609.444C782.349 516.821 750.046 431.738 696.112 364.826L695.165 363.631L695.206 363.59ZM157.351 415.876L183.556 383.779L184.339 389.712C184.957 394.409 185.74 399.106 186.646 403.844C203.622 492.883 264.23 567.088 365.546 624.565C453.637 674.708 504.934 732.35 519.684 795.554C525.864 821.924 526.936 847.881 524.258 870.584L524.093 871.985L522.816 872.603C483.055 892.009 438.351 902.927 391.133 902.927C225.459 902.927 91.1394 768.855 91.1394 603.428C91.1394 532.396 115.902 467.172 157.269 415.793L157.351 415.876Z" fill="#4DA2FF"/>
    <path d="M157.351 415.876L183.556 383.779L184.339 389.712C184.957 394.409 185.74 399.106 186.646 403.844C203.622 492.883 264.23 567.088 365.546 624.565C453.637 674.708 504.934 732.35 519.684 795.554C525.864 821.924 526.936 847.881 524.258 870.584L524.093 871.985L522.816 872.603C483.055 892.009 438.351 902.927 391.133 902.927C225.459 902.927 91.1394 768.855 91.1394 603.428C91.1394 532.396 115.902 467.172 157.269 415.793L157.351 415.876Z" fill="#4DA2FF"/>
  </g>
</svg>
"""

BOB_ICON = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" role="img" aria-label="BOB">
  {PROVENANCE}
  <rect width="48" height="48" rx="8" fill="#fff"/>
  <g transform="translate(13 7)">
    <rect x="0" y="0" width="10" height="10" fill="#343536"/>
    <rect x="0" y="12" width="10" height="10" fill="#343536"/>
    <rect x="12" y="12" width="10" height="10" fill="#F58B00"/>
    <rect x="0" y="24" width="10" height="10" fill="#343536"/>
    <rect x="12" y="24" width="10" height="10" fill="#F58B00"/>
  </g>
</svg>
"""

INLINE = {
    "inline_bob": BOB_ICON,
    "inline_base": BASE_SQUARE,
    "inline_sui": SUI_DROPLET,
}


def read_svg_body(path: Path) -> str:
    text = path.read_text(encoding="utf-8", errors="replace")
    if "<svg" not in text:
        raise ValueError(f"not svg: {path}")
    text = re.sub(r"<\?xml[^>]*\?>", "", text, flags=re.I)
    text = re.sub(r"<!DOCTYPE[^>]*>", "", text, flags=re.I)
    m = re.search(r"<svg[^>]*>(.*)</svg>", text, flags=re.S | re.I)
    if not m:
        raise ValueError(f"no svg body: {path}")
    return m.group(1).strip()


def parse_viewbox(svg_text: str) -> tuple[float, float, float, float]:
    m = re.search(r'viewBox=["\']([^"\']+)["\']', svg_text, re.I)
    if not m:
        w = re.search(r'width=["\']([0-9.]+)', svg_text, re.I)
        h = re.search(r'height=["\']([0-9.]+)', svg_text, re.I)
        if w and h:
            return 0.0, 0.0, float(w.group(1)), float(h.group(1))
        return 0.0, 0.0, 48.0, 48.0
    parts = [float(x) for x in m.group(1).replace(",", " ").split()]
    return parts[0], parts[1], parts[2], parts[3]


def wrap_raster(label: str, src: Path, padding: float = 4.0) -> str:
    data = src.read_bytes()
    suffix = src.suffix.lower()
    if suffix == ".webp":
        mime = "image/webp"
    elif suffix == ".png":
        mime = "image/png"
    elif suffix in {".jpg", ".jpeg"}:
        mime = "image/jpeg"
    else:
        raise ValueError(f"unsupported raster: {src}")
    b64 = base64.b64encode(data).decode("ascii")
    inner = 48.0 - 2 * padding
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" role="img" aria-label="{label}">
  {PROVENANCE}
  <rect width="48" height="48" rx="8" fill="#fff"/>
  <image href="data:{mime};base64,{b64}" x="{padding}" y="{padding}" width="{inner}" height="{inner}" preserveAspectRatio="xMidYMid meet"/>
</svg>
"""


def wrap_file(label: str, src: Path, padding: float = 4.0) -> str:
    raw = src.read_text(encoding="utf-8", errors="replace")
    body = read_svg_body(src)
    xmin, ymin, vw, vh = parse_viewbox(raw)
    inner_w = 48.0 - 2 * padding
    scale = inner_w / max(vw, vh)
    tx = -(xmin + vw / 2)
    ty = -(ymin + vh / 2)
    inner = f'<svg viewBox="0 0 {vw} {vh}" width="{vw}" height="{vh}" overflow="visible">{body}</svg>'
    return TILE.format(
        label=label,
        provenance=PROVENANCE,
        scale=f"{scale:.6f}",
        tx=f"{tx:.3f}",
        ty=f"{ty:.3f}",
        inner=inner,
    )


def load_manifest() -> dict:
    return json.loads(MANIFEST.read_text(encoding="utf-8"))


def validate_forbidden(text: str, slug: str, forbidden: list[str]) -> None:
    for bad in forbidden:
        if bad in text:
            raise SystemExit(f"{slug} public svg contains forbidden content: {bad!r}")


def _attr_num(tag: str, name: str) -> float | None:
    m = re.search(rf'\b{name}=["\']([0-9.]+)', tag)
    return float(m.group(1)) if m else None


def validate_tile_layout(text: str, slug: str) -> None:
    if TILE_VIEWBOX not in text:
        raise SystemExit(f'{slug}: root tile must use viewBox="0 0 48 48"')
    wrapped = WRAP_CENTER in text
    for m in re.finditer(r"<rect\b[^>]*>", text, re.I):
        tag = m.group(0)
        if re.search(r'\bwidth=["\']48', tag):
            continue
        x, y = _attr_num(tag, "x"), _attr_num(tag, "y")
        if x is None or y is None:
            continue
        if (x > MAX_DIRECT_COORD or y > MAX_DIRECT_COORD) and not wrapped:
            raise SystemExit(
                f"{slug}: rect at ({x},{y}) outside 48x48 tile without wrap centering "
                f"({WRAP_CENTER}); use wrap_file() or fix inline template"
            )


def validate_public_tile(
    path: Path,
    slug: str,
    markers: list[str],
    forbidden: list[str],
    *,
    require_vector: bool = False,
) -> str:
    if not path.exists():
        raise SystemExit(f"missing public svg for {slug}: {path}")
    text = path.read_text(encoding="utf-8")
    if "<svg" not in text:
        raise SystemExit(f"not svg: {path}")
    if markers and not any(m in text for m in markers):
        raise SystemExit(f"{slug} public svg missing brand markers in {path}")
    validate_forbidden(text, slug, forbidden)
    if require_vector:
        body = read_svg_body(path)
        if not any(m in body for m in markers):
            raise SystemExit(f"{slug} vector tile missing brand markers in {path}")
    validate_tile_layout(text, slug)
    return text


def write_if_changed(path: Path, text: str) -> bool:
    if path.exists() and path.read_text(encoding="utf-8") == text:
        return False
    path.write_text(text, encoding="utf-8")
    return True


def finalize_tile(text: str, slug: str) -> str:
    validate_tile_layout(text, slug)
    return text


def resolve_raw_root(argv: list[str]) -> Path | None:
    if "--public-fallback" in argv:
        return None
    positional = [arg for arg in argv if not arg.startswith("-")]
    if positional:
        return Path(positional[0])
    scratch = os.environ.get("ONCHAINAI_SCRATCH", "").strip()
    if scratch:
        candidate = Path(scratch) / "raw-logos"
        if candidate.is_dir():
            return candidate
    return None


def preserve_public_tile(
    out: Path,
    slug: str,
    markers: list[str],
    forbidden: list[str],
    *,
    require_vector: bool = False,
) -> str:
    text = validate_public_tile(
        out, slug, markers, forbidden, require_vector=require_vector
    )
    write_if_changed(out, text)
    return text


def main() -> None:
    raw_root = resolve_raw_root(sys.argv[1:])
    data = load_manifest()
    forbidden = data.get("forbidden", [])
    OUT.mkdir(parents=True, exist_ok=True)
    written: list[str] = []
    preserved: list[str] = []
    regenerated: list[str] = []

    for entry in data["entries"]:
        slug = entry["id"]
        label = entry["label"]
        kind = entry["kind"]
        out = OUT / f"{slug}.svg"
        markers = entry.get("markers", [])
        require_vector = bool(entry.get("require_vector"))

        if kind in INLINE:
            if raw_root is None:
                if not out.exists():
                    text = finalize_tile(INLINE[kind], slug)
                    validate_forbidden(text, slug, forbidden)
                    write_if_changed(out, text)
                    regenerated.append(slug)
                else:
                    preserve_public_tile(
                        out,
                        slug,
                        markers,
                        forbidden,
                        require_vector=require_vector,
                    )
                    preserved.append(slug)
            else:
                text = finalize_tile(INLINE[kind], slug)
                validate_forbidden(text, slug, forbidden)
                if write_if_changed(out, text):
                    regenerated.append(slug)
        elif kind == "wrap":
            if raw_root is None:
                preserve_public_tile(
                    out,
                    slug,
                    markers,
                    forbidden,
                    require_vector=require_vector,
                )
                preserved.append(slug)
            else:
                src = raw_root / entry["source"]
                if not src.exists():
                    if out.exists():
                        preserve_public_tile(
                            out,
                            slug,
                            markers,
                            forbidden,
                            require_vector=require_vector,
                        )
                        preserved.append(slug)
                        written.append(slug)
                        continue
                    raise SystemExit(f"missing source for {slug}: {src}")
                if require_vector:
                    body = read_svg_body(src)
                    if not any(m in body for m in markers):
                        raise SystemExit(f"{slug} svg missing brand markers in {src}")
                text = wrap_file(label, src)
                validate_forbidden(text, slug, forbidden)
                finalize_tile(text, slug)
                if write_if_changed(out, text):
                    regenerated.append(slug)
        elif kind == "wrap_raster":
            if raw_root is None:
                preserve_public_tile(out, slug, markers, forbidden)
                preserved.append(slug)
            else:
                src = raw_root / entry["source"]
                if not src.exists():
                    if out.exists():
                        preserve_public_tile(out, slug, markers, forbidden)
                        preserved.append(slug)
                        written.append(slug)
                        continue
                    raise SystemExit(f"missing raster for {slug}: {src}")
                text = wrap_raster(label, src)
                if markers and not any(m in text for m in markers):
                    raise SystemExit(f"{slug} raster tile missing markers")
                validate_forbidden(text, slug, forbidden)
                finalize_tile(text, slug)
                if write_if_changed(out, text):
                    regenerated.append(slug)
        elif kind == "public_svg":
            preserve_public_tile(out, slug, markers, forbidden)
            preserved.append(slug)
        else:
            raise SystemExit(f"unknown kind for {slug}: {kind}")

        written.append(slug)

    mode = "public-fallback" if raw_root is None else f"raw:{raw_root}"
    print(f"mode: {mode}")
    print(f"regenerated: {', '.join(sorted(regenerated)) or '(none)'}")
    print(f"preserved: {', '.join(sorted(preserved)) or '(none)'}")
    print(f"wrapped: {', '.join(sorted(written))}")


if __name__ == "__main__":
    main()