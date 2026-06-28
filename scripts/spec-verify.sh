#!/usr/bin/env bash
# spec-verify.sh — acceptance harness for docs/PRODUCT_ENHANCEMENT_SPEC.md.
#
# Read-only. Runs the automatable acceptance checks from docs/VERIFICATION.md and
# prints PASS / FAIL / MANUAL / SKIP per spec item. Many items are SUPPOSED to be
# FAIL until implemented (red -> green). Exit 1 if any selected auto-check FAILs.
#
# Two modes:
#   report (default when no IDs)  — show full status, ALWAYS exit 0 (dashboard; red is fine)
#   gate   (default when IDs given) — only the listed IDs must PASS, exit 1 if any fail
#
# Usage:
#   ./scripts/spec-verify.sh                # report mode, all checks, exit 0
#   ./scripts/spec-verify.sh full           # report + cargo test/clippy/fmt
#   ./scripts/spec-verify.sh C3 J1 D7       # gate mode: these IDs MUST pass (exit 1 if not)
#   ./scripts/spec-verify.sh report C3 J1   # force report even with IDs (exit 0)
#   ./scripts/spec-verify.sh gate           # force gate on everything (exit 1 if any fail)
#   PROD_URL=https://staging.x ./scripts/spec-verify.sh
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT" || exit 2

PROD_URL="${PROD_URL:-https://www.onchain-ai.xyz}"
APEX_URL="${APEX_URL:-https://onchain-ai.xyz}"

FULL=0
MODE=auto   # auto | report | gate
SELECT=()
for a in "$@"; do
  case "$a" in
    full|--full) FULL=1 ;;
    fast|--fast) FULL=0 ;;
    report|--report) MODE=report ;;
    gate|--gate) MODE=gate ;;
    *) SELECT+=("$a") ;;
  esac
done
# auto: gate if specific IDs were named, else report
[ "$MODE" = auto ] && { [ ${#SELECT[@]} -gt 0 ] && MODE=gate || MODE=report; }

P=0; F=0; M=0; S=0
FAILED_IDS=()

want() { # want ID -> 0 if selected (or no selection given)
  [ ${#SELECT[@]} -eq 0 ] && return 0
  local id="$1"; for s in "${SELECT[@]}"; do [[ "$id" == "$s"* || "$s" == "$id"* ]] && return 0; done
  return 1
}
ok()   { printf '  \033[32m[PASS]\033[0m %-10s %s\n' "$1" "$2"; P=$((P+1)); }
no()   { printf '  \033[31m[FAIL]\033[0m %-10s %s\n' "$1" "$2"; F=$((F+1)); FAILED_IDS+=("$1"); }
man()  { printf '  \033[33m[MAN ]\033[0m %-10s %s\n' "$1" "$2"; M=$((M+1)); }
skip() { printf '  \033[90m[SKIP]\033[0m %-10s %s\n' "$1" "$2"; S=$((S+1)); }

# present ID "desc" PATTERN FILE...  -> PASS if pattern found in an existing file
present() { local id="$1" d="$2" pat="$3"; shift 3; want "$id" || return
  local found=1 existed=0
  for f in "$@"; do [ -e "$f" ] && { existed=1; grep -Eq -- "$pat" "$f" && found=0; }; done
  [ $existed -eq 0 ] && { no "$id" "$d (파일 없음)"; return; }
  [ $found -eq 0 ] && ok "$id" "$d" || no "$id" "$d"
}
# absent ID "desc" PATTERN FILE...  -> PASS if pattern NOT found (missing file = pass)
absent() { local id="$1" d="$2" pat="$3"; shift 3; want "$id" || return
  for f in "$@"; do [ -e "$f" ] && grep -Eq -- "$pat" "$f" && { no "$id" "$d"; return; }; done
  ok "$id" "$d"
}
exists() { local id="$1" d="$2" path="$3"; want "$id" || return
  [ -e "$path" ] && ok "$id" "$d" || no "$id" "$d ($path 없음)"; }
manual() { local id="$1" d="$2"; want "$id" || return; man "$id" "$d"; }

# curl_has ID "desc" URL PATTERN  (-L follow; SKIP on no network)
curl_has() { local id="$1" d="$2" url="$3" pat="$4"; want "$id" || return
  command -v curl >/dev/null || { skip "$id" "$d (no curl)"; return; }
  local resp rc status body
  resp="$(curl -sS -L --max-time 15 -w '\n%{http_code}' "$url" 2>/dev/null)"; rc=$?
  [ $rc -ne 0 ] && { skip "$id" "$d (무응답/네트워크)"; return; }
  status="${resp##*$'\n'}"
  body="${resp%$'\n'*}"
  [[ "$status" =~ ^2[0-9][0-9]$ ]] || { no "$id" "$d (HTTP ${status:-unknown})"; return; }
  [ -z "$body" ] && { no "$id" "$d (빈 응답)"; return; }
  printf '%s' "$body" | grep -Eq -- "$pat" && ok "$id" "$d" || no "$id" "$d"
}
# curl_loc_has ID "desc" URL PATTERN  -> PASS if the redirect Location header matches (no -L)
curl_loc_has() { local id="$1" d="$2" url="$3" pat="$4"; want "$id" || return
  command -v curl >/dev/null || { skip "$id" "$d (no curl)"; return; }
  local loc; loc="$(curl -sS -I --max-time 15 "$url" 2>/dev/null | tr -d '\r')"
  [ -z "$loc" ] && { skip "$id" "$d (무응답/네트워크)"; return; }
  printf '%s' "$loc" | grep -Eqi -- "$pat" && ok "$id" "$d" || no "$id" "$d"
}
# header_redirect ID "desc" URL  -> PASS if 3xx to www
header_redirect() { local id="$1" d="$2" url="$3"; want "$id" || return
  command -v curl >/dev/null || { skip "$id" "$d (no curl)"; return; }
  local h; h="$(curl -sS -I --max-time 15 "$url" 2>/dev/null)"
  [ -z "$h" ] && { skip "$id" "$d (무응답)"; return; }
  printf '%s' "$h" | grep -Eqi '^HTTP/.* 30[1278]' && \
  printf '%s' "$h" | grep -Eqi '^location: https://www\.' && ok "$id" "$d" || no "$id" "$d"
}

echo "── spec-verify [$MODE] (PROD_URL=$PROD_URL, full=$FULL) ──"

# cargo_gate ID "desc" -- <cargo args...>  : SKIP on env errors (e.g. missing OpenSSL), else PASS/FAIL
cargo_gate() { local id="$1" d="$2"; shift 3; want "$id" || return
  local out; out="$(cargo "$@" 2>&1)"; local rc=$?
  if [ $rc -eq 0 ]; then ok "$id" "$d"; return; fi
  if printf '%s' "$out" | grep -qiE 'development packages of openssl|Could not find directory of OpenSSL|linker `cc` not found|could not find native static library'; then
    skip "$id" "$d (환경: 시스템 라이브러리 부재 — 코드 무관)"; return
  fi
  no "$id" "$d"; printf '%s\n' "$out" | tail -4 | sed 's/^/        /'
}
echo "[0] 품질 게이트"
cargo_gate Q1 "cargo check --features ssr" -- check --features ssr
if [ $FULL -eq 1 ]; then
  cargo_gate Q2 "cargo test --features ssr" -- test --features ssr
  cargo_gate Q3 "cargo clippy --features ssr" -- clippy --features ssr -- -W clippy::all
  cargo_gate Q4 "cargo fmt --check" -- fmt --check
else
  want Q2 && skip Q2 "cargo test (full 에서)"; want Q3 && skip Q3 "cargo clippy (full)"; want Q4 && skip Q4 "cargo fmt (full)"
fi

echo "[1] 완료 항목 회귀 가드"
absent A1 "rmcp 의존성 제거 유지" 'rmcp' Cargo.toml Cargo.lock
if want E1; then
  if git ls-files 2>/dev/null | grep -Eq 'playwright-.*\.yaml|railway-config-pull'; then no E1 "stray 아티팩트 재유입"; else ok E1 "stray 아티팩트 없음"; fi
fi
present E1g ".gitignore 패턴 유지" 'playwright-\*\.yaml' .gitignore

echo "[2] 트랙1 인증·canonical"
curl_loc_has F1-app "앱이 www redirect_uri 전송(307 Location)" "$PROD_URL/auth/github" 'location:.*redirect_uri=https%3A%2F%2Fwww\.onchain-ai\.xyz%2Fauth%2Fcallback'
manual F1-gh "GitHub OAuth 앱에 www 콜백 등록 + 실제 로그인 왕복 확인"
header_redirect H1 "apex→www 301" "$APEX_URL/"
manual F2 "지갑(SIWX)+Safari 세션 유지 수동 확인"
absent C3 "About 카피·개인핸들 정리" 'MVP does not include self-service|hoyeon4315-cpu' src/app.rs

echo "[3] 트랙2 발견 커버리지"
absent B1 "SourceCrawler dead-code 해킹 제거" '_SourceCrawlerSealed' src/crawler/sources/mod.rs
if want B2; then
  n=$(ls src/crawler/sources/*.rs 2>/dev/null | grep -vc '/mod.rs')
  [ "${n:-0}" -gt 4 ] && ok B2 "신규 크롤러 소스 추가 (소스 ${n}개)" || no B2 "신규 소스 미추가 (소스 ${n:-0}개, >4 필요)"
fi

echo "[4] 트랙3 검색 품질 Tier0"
present A4-desc "search_tools description 보강" 'bridge USDC to Base|Uniswap MCP server' src/server/mcp.rs
present A4-params "search_tools inputSchema에 sort/cursor 키" '"(sort|cursor|next_cursor)"[[:space:]]*:' src/server/mcp.rs
manual A4-params2 "MCP tools/list POST로 search_tools inputSchema 확인"
absent A4-rank "MCP 검색 정렬이 stars 고정 아님" 'ORDER BY stars DESC LIMIT 50' src/server/mcp.rs

echo "[5] 트랙4 채택 동선"
present D4 "에이전트 연결 CTA(mcpServers 노출)" 'mcpServers|mcp-remote' src/components/promo_cards.rs src/pages/home.rs src/app.rs
exists J1 "OnchainAI Skill 파일" skills/onchainai-crypto-tools/SKILL.md
present J1-rule "Skill 안전규칙(critical/x402)" 'critical|x402' skills/onchainai-crypto-tools/SKILL.md
exists J2 "Plugin manifest" .claude-plugin/plugin.json
exists J2-mcp ".mcp.json 번들" .mcp.json
present J2-mcp2 ".mcp.json에 OnchainAI 엔드포인트" 'onchain-ai\.xyz/mcp' .mcp.json

echo "[6] 트랙5 신뢰 카드"
absent D7-icon "북마크 ☆ 글리프 제거(SVG화)" '☆' src/components/tool_card.rs
present D7-trust "카드가 install_risk 시각화" 'install_risk' src/components/tool_card.rs

echo "[7] Next (구현 시)"
present C4-tradfi "TradFi asset_class 분기" 'tradfi' src/crawler/normalizer.rs
present C4-ws "WebSocket type 분기" 'websocket' src/crawler/normalizer.rs
if want E2; then big=$(find src/server -name '*.rs' 2>/dev/null -exec wc -l {} \; | awk '$1>800{print $2}'); [ -z "$big" ] && ok E2 "서버 .rs 800LOC 초과 0" || no E2 "800LOC 초과: $(echo "$big" | tr '\n' ' ')"; fi
curl_has H2-sitemap "sitemap.xml 200" "$PROD_URL/sitemap.xml" '<urlset|<sitemapindex'
curl_has H2-robots "robots.txt 존재" "$PROD_URL/robots.txt" 'User-agent|Sitemap'
if want E3; then c=$(grep -rIlE --include='*.rs' 'unwrap\(\)|expect\(' src 2>/dev/null | wc -l | tr -d ' '); man E3 "패닉 감사: unwrap/expect 포함 파일 ${c}개(테스트 포함) — 런타임 경로 수동 확인"; fi

echo
echo "── 요약: ${P} PASS · ${F} FAIL · ${M} MANUAL · ${S} SKIP ──"
[ ${#FAILED_IDS[@]} -gt 0 ] && echo "FAIL: ${FAILED_IDS[*]}"
if [ "$MODE" = report ]; then
  echo "(report 모드: 전체 현황만, exit 0. FAIL=미구현이라 정상. 게이트는 'gate' 또는 ID 지정.)"
  exit 0
else
  echo "(gate 모드: 선택 ID는 반드시 PASS. PASS/SKIP 수는 네트워크·full 여부로 달라짐.)"
  [ $F -eq 0 ] && exit 0 || exit 1
fi
