#!/usr/bin/env python3
"""Wrap official chain logo SVGs into 48x48 public/chains/*.svg tiles."""

from __future__ import annotations

import base64
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "public" / "chains"
SCRATCH = Path(
    sys.argv[1]
    if len(sys.argv) > 1
    else "/var/folders/k7/_r0bjtp12dngr0ncryvtt4mc0000gn/T/grok-goal-11e98898edeb/implementer/raw-logos"
)

TILE = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" role="img" aria-label="{label}">
  <rect width="48" height="48" rx="8" fill="#fff"/>
  <g transform="translate(24 24) scale({scale}) translate({tx} {ty})">
    {inner}
  </g>
</svg>
"""

BASE_SQUARE = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" role="img" aria-label="Base">
  <rect width="48" height="48" rx="8" fill="#fff"/>
  <rect x="9.6" y="9.6" width="28.8" height="28.8" rx="1.44" fill="#0052FF"/>
</svg>
"""

SUI_DROPLET = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" role="img" aria-label="Sui">
  <rect width="48" height="48" rx="8" fill="#fff"/>
  <g transform="translate(6 4) scale(0.045)">
    <path fill-rule="evenodd" clip-rule="evenodd" d="M626.027 417.029C666.817 468.244 691.209 533.014 691.209 603.469C691.209 673.925 666.076 740.673 624.214 792.176L620.588 796.626L619.641 790.981C618.817 786.201 617.869 781.34 616.757 776.478C595.785 684.349 527.471 605.365 415.03 541.378C339.095 498.28 295.626 446.448 284.213 387.487C276.838 349.375 282.318 311.098 292.907 278.301C303.496 245.545 319.235 218.063 332.626 201.541L376.383 148.06C384.046 138.666 398.426 138.666 406.09 148.06L626.068 417.029H626.027ZM695.206 363.59L402.01 5.12968C396.407 -1.70989 385.942 -1.70989 380.338 5.12968L87.184 363.59L86.2363 364.784C32.3026 431.738 0 516.821 0 609.444C0 825.138 175.151 1000 391.174 1000C607.198 1000 782.349 825.138 782.349 609.444C782.349 516.821 750.046 431.738 696.112 364.826L695.165 363.631L695.206 363.59ZM157.351 415.876L183.556 383.779L184.339 389.712C184.957 394.409 185.74 399.106 186.646 403.844C203.622 492.883 264.23 567.088 365.546 624.565C453.637 674.708 504.934 732.35 519.684 795.554C525.864 821.924 526.936 847.881 524.258 870.584L524.093 871.985L522.816 872.603C483.055 892.009 438.351 902.927 391.133 902.927C225.459 902.927 91.1394 768.855 91.1394 603.428C91.1394 532.396 115.902 467.172 157.269 415.793L157.351 415.876Z" fill="#4DA2FF"/>
    <path d="M157.351 415.876L183.556 383.779L184.339 389.712C184.957 394.409 185.74 399.106 186.646 403.844C203.622 492.883 264.23 567.088 365.546 624.565C453.637 674.708 504.934 732.35 519.684 795.554C525.864 821.924 526.936 847.881 524.258 870.584L524.093 871.985L522.816 872.603C483.055 892.009 438.351 902.927 391.133 902.927C225.459 902.927 91.1394 768.855 91.1394 603.428C91.1394 532.396 115.902 467.172 157.269 415.793L157.351 415.876Z" fill="#4DA2FF"/>
  </g>
</svg>
"""


def read_svg_body(path: Path) -> str:
    text = path.read_text(encoding="utf-8", errors="replace")
    if "<svg" not in text:
        raise ValueError(f"not svg: {path}")
    # Strip outer svg wrapper; keep defs + content.
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


def wrap_file(name: str, label: str, src: Path, padding: float = 4.0) -> str:
    raw = src.read_text(encoding="utf-8", errors="replace")
    body = read_svg_body(src)
    _, _, vw, vh = parse_viewbox(raw)
    inner_w = 48.0 - 2 * padding
    scale = inner_w / max(vw, vh)
    tx = -vw / 2
    ty = -vh / 2
    inner = f'<svg viewBox="0 0 {vw} {vh}" width="{vw}" height="{vh}" overflow="visible">{body}</svg>'
    return TILE.format(label=label, scale=f"{scale:.6f}", tx=f"{tx:.3f}", ty=f"{ty:.3f}", inner=inner)


def wrap_png(name: str, label: str, src: Path) -> str:
    raw = src.read_bytes()
    if len(raw) < 200 or not raw.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError(f"invalid png for {name}: {src}")
    data = base64.b64encode(raw).decode("ascii")
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" role="img" aria-label="{label}">
  <rect width="48" height="48" rx="8" fill="#fff"/>
  <image href="data:image/png;base64,{data}" xlink:href="data:image/png;base64,{data}" x="6" y="6" width="36" height="36" preserveAspectRatio="xMidYMid meet"/>
</svg>
"""


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    BOB_ICON = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" role="img" aria-label="BOB">
  <rect width="48" height="48" rx="8" fill="#fff"/>
  <g transform="translate(8 10) scale(0.31)">
    <rect x="259.4" y="413.5" fill="#343536" width="101.6" height="101.6"/>
    <rect x="373.8" y="413.5" fill="#F58B00" width="101.6" height="101.6"/>
    <rect x="373.8" y="299.2" fill="#F58B00" width="101.6" height="101.6"/>
    <rect x="259.4" y="299.2" fill="#343536" width="101.6" height="101.6"/>
    <rect x="259.4" y="184.8" fill="#343536" width="101.6" height="101.6"/>
  </g>
</svg>
"""

    mapping = {
        "bitcoin": ("Bitcoin", SCRATCH / "bitcoin_official.svg"),
        "ethereum": ("Ethereum", SCRATCH / "ethereum_official.svg"),
        "solana": ("Solana", SCRATCH / "solana_official.svg"),
        "arbitrum": ("Arbitrum", SCRATCH / "arbitrum_official.svg"),
        "bsc": ("BNB Chain", SCRATCH / "bsc_official.svg"),
        "avalanche": ("Avalanche", SCRATCH / "avalanche_official.svg"),
        "polygon": ("Polygon", SCRATCH / "polygon_official.svg"),
        "zksync": ("zkSync", SCRATCH / "zksync_official.svg"),
    }
    written: list[str] = []
    for slug, (label, path) in mapping.items():
        if not path.exists():
            raise SystemExit(f"missing source for {slug}: {path}")
        out = OUT / f"{slug}.svg"
        out.write_text(wrap_file(slug, label, path), encoding="utf-8")
        written.append(slug)

    (OUT / "bob.svg").write_text(BOB_ICON, encoding="utf-8")
    written.append("bob")

    (OUT / "base.svg").write_text(BASE_SQUARE, encoding="utf-8")
    written.append("base")

    (OUT / "sui.svg").write_text(SUI_DROPLET, encoding="utf-8")
    written.append("sui")

    op_svg = SCRATCH / "optimism_official.svg"
    if not op_svg.exists():
        raise SystemExit(f"missing optimism svg: {op_svg}")
    op_body = read_svg_body(op_svg)
    if "#FF0421" not in op_body and "#FF0420" not in op_body:
        raise SystemExit(f"optimism svg missing brand red: {op_svg}")
    (OUT / "optimism.svg").write_text(wrap_file("optimism", "Optimism", op_svg), encoding="utf-8")
    written.append("optimism")

    print("wrapped:", ", ".join(sorted(written)))


if __name__ == "__main__":
    main()