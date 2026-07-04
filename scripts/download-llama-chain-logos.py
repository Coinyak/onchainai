#!/usr/bin/env python3
"""Download DeFiLlama chain icons into scripts/raw-logos/{id}_llama.png."""
from __future__ import annotations

import subprocess
import urllib.request
from pathlib import Path

RAW = Path(__file__).resolve().parents[1] / "scripts" / "raw-logos"

CHAINS: list[tuple[str, list[str]]] = [
    ("stellar", ["https://icons.llama.fi/stellar.jpg", "https://icons.llamao.fi/icons/chains/rsz_stellar?w=48&h=48"]),
    ("algorand", ["https://icons.llama.fi/algorand.jpg", "https://icons.llama.fi/algorand.png"]),
    ("filecoin", ["https://icons.llama.fi/filecoin.jpg", "https://icons.llama.fi/filecoin.png"]),
    ("ronin", ["https://icons.llama.fi/ronin.jpg", "https://icons.llama.fi/ronin.png"]),
    ("worldchain", ["https://icons.llamao.fi/icons/chains/rsz_world-chain?w=48&h=48", "https://icons.llama.fi/world-chain.jpg"]),
    ("hedera", ["https://icons.llama.fi/hedera.jpg", "https://icons.llamao.fi/icons/chains/rsz_hedera?w=48&h=48"]),
    ("xrpl", ["https://icons.llama.fi/ripple.jpg", "https://icons.llama.fi/xrpl.jpg"]),
    ("thorchain", ["https://icons.llamao.fi/icons/chains/rsz_thorchain?w=48&h=48", "https://icons.llama.fi/thorchain.jpg"]),
    ("katana", ["https://icons.llama.fi/katana.jpg", "https://icons.llamao.fi/icons/chains/rsz_katana?w=48&h=48"]),
    ("dydx", ["https://icons.llama.fi/dydx.jpg", "https://icons.llama.fi/dydx-chain.jpg"]),
    ("fraxtal", ["https://icons.llama.fi/fraxtal.jpg", "https://icons.llama.fi/fraxtal.png"]),
    ("tezos", ["https://icons.llama.fi/tezos.jpg", "https://icons.llama.fi/tezos.png"]),
    ("mezo", ["https://icons.llama.fi/mezo.jpg", "https://icons.llamao.fi/icons/chains/rsz_mezo?w=48&h=48"]),
    ("bittensor", ["https://icons.llama.fi/bittensor.jpg", "https://icons.llamao.fi/icons/chains/rsz_bittensor?w=48&h=48"]),
    ("pulsechain", ["https://icons.llama.fi/pulsechain.jpg", "https://icons.llama.fi/pulsechain.png"]),
    ("provenance", ["https://icons.llama.fi/provenance.jpg", "https://icons.llamao.fi/icons/chains/rsz_provenance?w=48&h=48"]),
    ("fluent", ["https://icons.llama.fi/fluent.jpg", "https://icons.llamao.fi/icons/chains/rsz_fluent?w=48&h=48"]),
    ("hydration", ["https://icons.llama.fi/hydradx.jpg", "https://icons.llamao.fi/icons/chains/rsz_hydration?w=48&h=48"]),
    ("mixin", ["https://icons.llama.fi/mixin.jpg", "https://icons.llamao.fi/icons/chains/rsz_mixin?w=48&h=48"]),
    ("vaulta", ["https://icons.llama.fi/eos.jpg", "https://icons.llama.fi/vaulta.jpg"]),
    ("ethereal", ["https://icons.llama.fi/ethena-usde.jpg", "https://icons.llamao.fi/icons/chains/rsz_ethereal?w=48&h=48"]),
    ("stable", ["https://icons.llama.fi/stable-2.jpg", "https://icons.llamao.fi/icons/chains/rsz_stable?w=48&h=48"]),
    ("xpr", ["https://icons.llama.fi/proton.jpg", "https://icons.llamao.fi/icons/chains/rsz_xpr?w=48&h=48"]),
]


def main() -> None:
    RAW.mkdir(parents=True, exist_ok=True)
    missing: list[str] = []
    for cid, urls in CHAINS:
        out = RAW / f"{cid}_llama.png"
        if out.exists() and out.stat().st_size > 500:
            print(f"skip {cid}")
            continue
        ok = False
        for url in urls:
            try:
                req = urllib.request.Request(url, headers={"User-Agent": "OnchainAI/1.0"})
                data = urllib.request.urlopen(req, timeout=20).read()
                src = RAW / f"{cid}_llama.src"
                src.write_bytes(data)
                if data[:8].startswith(b"\x89PNG"):
                    out.write_bytes(data)
                else:
                    subprocess.run(
                        ["sips", "-s", "format", "png", str(src), "--out", str(out)],
                        check=True,
                        capture_output=True,
                    )
                src.unlink(missing_ok=True)
                if out.stat().st_size > 200:
                    print(f"ok {cid} <- {url}")
                    ok = True
                    break
            except Exception as exc:
                print(f"try fail {cid} {url}: {exc}")
        if not ok:
            missing.append(cid)
            print(f"MISSING {cid}")
    if missing:
        raise SystemExit(f"failed: {', '.join(missing)}")


if __name__ == "__main__":
    main()