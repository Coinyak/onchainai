#!/usr/bin/env bash
# Base Builder Code operator checklist (dashboard.base.org — browser login required).
set -euo pipefail
cat <<'EOF'
== Base Builder Code (onchain attribution) ==

1. Open https://dashboard.base.org and sign in
2. Register app: OnchainAI
3. Add + verify domain: onchain-ai.xyz (and www if prompted)
4. Settings → Builder Codes → copy code (pattern: bc_xxxxxxxx or similar)
5. Admin → Site settings → x402_builder_code = <your code>
6. When premium x402 ships: include builder-code extension in 402 PaymentRequirements

Validate a settlement tx:
  https://buildercode-checker.vercel.app/

Docs:
  https://docs.cdp.coinbase.com/x402/core-concepts/builder-codes

Suggested app code slug (if free-form): onchainai
EOF
if command -v open >/dev/null 2>&1; then
  open "https://dashboard.base.org"
fi