# 공식 출범 준비 스펙 — GitHub 공개 + 에이전트 온보딩 + x402 활성화

> 작성일: 2026-07-03 · 목적: 프라이빗 레포를 **공개 출범 가능한 상태**로 만드는 전 범위 분석 + 실행 스펙.
> 관련: [SKILL_PLUGIN_SPEC.md](SKILL_PLUGIN_SPEC.md) · [X402_REFERRAL_SPEC.md](X402_REFERRAL_SPEC.md) · [CONNECT.md](CONNECT.md) · [UI 통합 감사 스펙](superpowers/specs/2026-07-03-visual-ui-audit-16-agent-spec.md)
> 표기: ✅ = 2026-07-03 기준 구현/완료, 🔲 = 남은 작업. 우선순위 P0(공개 전 필수) > P1(공개 직후 2주) > P2(후속).

---

## 0. 전체 진단 요약

| 표면 | 상태 | 비고 |
|---|---|---|
| MCP 서버 (`POST /mcp`) | ✅ 운영 중 | 도구 5개, IP 레이트리밋, critical 설치명령 차단, sanitize |
| `/connect` 허브 (웹) | ✅ 운영 중 | ChatGPT/Claude/Cursor/VS Code 카드 + 딥링크 + universal 명령 |
| Claude Code 플러그인 | ✅ 이번 턴 재구성 | `plugin/onchainai/` 분리 — dev MCP(vercel/railway) 유출 버그 수정 |
| Skill | ✅ | 플러그인에 동봉, 단독 설치 경로 문서화 |
| x402 스키마/메타/어트리뷰션 | ✅ | migrations 017–018, MCP 응답 referral 메타, `referral_events` 기록 |
| x402 검증 잡 (엔드포인트/가격) | ✅ `be49680` | migration 028 + SSRF 가드 프로브 + 일일 cron + admin 재검증 route (§4) |
| UI 시각 품질 (통합 감사) | ◐ 막바지 | Phase A/B/C/D 랜딩(`bfcb662`·`f81c78e`·`ae8e9aa`); 잔여 = 회귀 검증 + G2(P0) + P1 6건 — [통합 감사 스펙 v3.1](superpowers/specs/2026-07-03-visual-ui-audit-16-agent-spec.md) |
| 레퍼럴 어드민 API | ✅ | `/api/v2/admin/tool-referral`, `/api/v2/admin/referral-stats` |
| README/LICENSE/공개 문서 | ✅ 이번 턴 | README 전면 개편, LICENSE/SECURITY/CONTRIBUTING/CONNECT/llms.txt 신설 |
| GitHub 레포 | 🔲 PRIVATE | 공개 전환 절차 §2 |
| 레지스트리 등재 (MCP registry 등) | 🔲 | §5 |
| 어댑션 계측 | 부분 | `install_guide` 이벤트만 기록 — §6 |

**아키텍처 사실 확인** (문서 낡음 수정 완료): 프론트엔드는 Leptos SSR이 아니라 **Next.js(Vercel)**,
Rust 바이너리는 API/MCP/크롤러(Railway), DB는 Supabase. Vercel이 `/api`·`/auth`·`/mcp`를 Railway로 rewrite.

---

## 1. 공개 범위 재정비 (결정 기록)

### 1.1 이번 턴에서 수정한 공개 차단 요소
- ✅ **LICENSE 없음** → MIT LICENSE 추가 (README 배지·Cargo.toml `license="MIT"`와 정합).
- ✅ **README 낡음** → 아키텍처/사용법/플러그인/x402 정책 반영 전면 개편.
- ✅ **frontend/README.md** create-next-app 보일러플레이트 → 실제 문서로 교체.
- ✅ **플러그인 dev MCP 유출**: 마켓플레이스 `source: "./"`가 레포 루트를 번들 → 개발용 루트
  `.mcp.json`(vercel·railway)이 설치 사용자에게 자동 연결될 버그. `plugin/onchainai/`로 분리,
  플러그인 `.mcp.json`은 OnchainAI HTTP 엔드포인트 단일. `claude plugin validate` 통과,
  `spec-verify.sh J1 J2` 게이트에 `J2-devmcp`(유출 회귀 가드) 추가.
- ✅ `.gitignore` 꼬리의 `.env*`가 `!.env.example` 부정 규칙을 뒤에서 덮던 문제 수정
  (`git check-ignore`로 검증).
- ✅ `NEXT_PUBLIC_GITHUB_REPO` 기본값이 존재하지 않는 org(`onchain-ai/onchainai`) →
  실제 레포로 수정 (Vercel env가 있으면 그쪽이 우선).
- ✅ 루트 `SECURITY.md`(신고 채널) / `CONTRIBUTING.md` 신설 — GitHub 표준 파일.

### 1.2 공개 유지 결정 (스캔 결과 이상 없음)
- 추적 파일 시크릿 스캔: 토큰 패턴(ghp_/sk-/JWT/AKIA/service key) 0건. 지갑주소는 테스트
  벡터·제로주소·dev seed뿐. `.env`류가 히스토리에 add된 적 없음(`git log --diff-filter=A` 확인).
- `docs/superpowers/`, `.cursor/.factory/.grok/.agents` 등 내부
  에이전트 운영 파일: **공개 유지**. 민감정보 없고 "에이전트 네이티브 레포" 증거 가치가 있음.
- ✅ 완료된 일회성 작업 지시서(`GROK_FULL_SPEC_TASK.md`, `UI_UX_IMPROVEMENT_SPEC.md` 등)와
  일회성 검증 스크립트는 `docs/archive/`, `scripts/archive/`로 이동 (목록: `docs/archive/README.md`).
- `seeds/`, `supabase/config.toml`, `scripts/dns-records.txt`: 공개 무해 확인.
- `ADMIN_GITHUB_LOGINS`는 서버 env로만 동작 — 코드/례시에 남은 값은 공개 GitHub 핸들이라 무해.

### 1.3 🔲 공개 전환(visibility flip) 실행 체크리스트 — P0
순서대로, 모두 수동 확인:
1. **히스토리 시크릿 정밀 스캔**: `gitleaks git .` (미설치 시 `brew install gitleaks`).
   발견 시: 키 즉시 로테이트 → 히스토리 재작성(BFG) 여부 판단. *커밋 이력이 많아 수동 grep은 불충분.*
2. **키 로테이션(예방)**: 공개 직전 `JWT_SECRET`, `SUPABASE_SERVICE_KEY`, `GITHUB_CLIENT_SECRET`,
   `GITHUB_API_TOKEN` 로테이트 권장 — 과거 스크린샷/세션 공유로 새어 나갔을 가능성 차단.
3. **GitHub 레포 설정** (Settings): Secret scanning + Push protection ON → Dependabot alerts ON →
   Private vulnerability reporting ON (SECURITY.md가 이 경로 안내) → About: 설명/website
   `https://www.onchain-ai.xyz`/topics(`mcp`, `crypto`, `x402`, `ai-agents`, `rust`, `nextjs`) →
   Social preview에 `frontend/public/og-default.png` 업로드.
4. **브랜치 보호 재적용 확인**: `docs/BRANCH_PROTECTION.md` + `scripts/configure-branch-protection.sh`
   (공개 전환 시 보호 규칙이 유지되는지 확인).
5. **CI 예산 가드 확인**: `ci.yml`은 `workflow_dispatch` 전용(외부 PR이 Actions를 못 태움) — 유지.
   fork PR 정책: 공개 직후에는 수동 dispatch 리뷰 플로 유지, P2에서 paths-filter 기반 자동 CI 검토.
6. **Visibility flip** → 직후 스모크: README 배지/링크, `/plugin marketplace add Coinyak/onchainai`
   → `/plugin install onchainai@onchainai` 실설치, `claude mcp add --transport http ...` 연결,
   `https://www.onchain-ai.xyz/llms.txt` 200.
7. 실패 항목 있으면 되돌리지 말고 fast-follow 수정(플러그인은 version bump 필요 주의).

---

## 2. 사용자 온보딩 표면 — "유저가 내 MCP/플러그인/스킬을 쓰려면"

### 2.1 완성된 경로 (이번 턴 포함)
| 경로 | 진입점 | 상태 |
|---|---|---|
| MCP 직결 | `claude mcp add --transport http onchainai https://www.onchain-ai.xyz/mcp` | ✅ README·CONNECT.md·/connect |
| MCP (커넥터형) | Claude Desktop/Web·ChatGPT 커넥터에 URL 등록 | ✅ 상동 |
| MCP (딥링크) | Cursor/VS Code 원클릭 | ✅ /connect |
| MCP (stdio 브리지) | `npx mcp-remote <url>`, `npx add-mcp <url>` | ✅ 상동 |
| 플러그인 | `/plugin marketplace add` + `/plugin install` | ✅ `plugin/onchainai/` + README/CONNECT |
| 스킬 단독 | 스킬 폴더를 `~/.claude/skills/` 복사 또는 런타임 업로드 | ✅ CONNECT.md §skill |
| 에이전트 자동 발견 | `https://www.onchain-ai.xyz/llms.txt` | ✅ 이번 턴 신설 |

### 2.2 남은 온보딩 작업
- ✅ **/connect 허브 "Plugin & Skill" 섹션** — 이 브랜치에서 구현됨(`connect-plugin-card`
  testid, 설치 2-명령 + 복사 버튼, 스킬 단독 설치 안내, 기존 클라이언트 카드 9개 불변).
  프로덕션 빌드 + 브라우저 스냅샷/콘솔 검증 완료. 배포 전 `ui-change-gate.sh` 최종 확인만.
- ✅ **푸터에 llms.txt·Connect MCP 링크** — 구현·브라우저 확인 완료.
- ✅ **P2 · `GET /mcp` 안내 응답** (2026-07-05): `handle_mcp_info`가 JSON `{name, version, description,
  protocolVersion, endpoint, transport, docs, tools[]}` 200 반환 — POST(JSON-RPC)는 불변. 라우트에
  `.get(...)` 추가(`src/lib.rs`), 유닛 테스트 `mcp_info_lists_public_tools_and_endpoint`. *배포 전까지 프로덕션은 405.*
- ✅ **P2 · initialize의 `protocolVersion` 에코** (2026-07-05): `negotiate_protocol_version`이 클라이언트
  요청 버전이 지원목록(`2024-11-05`/`2025-03-26`/`2025-06-18`)이면 에코, 아니면 기본값 폴백
  (`src/server/mcp.rs`). 유닛 테스트 `protocol_version_echoes_supported_and_falls_back`.
- ✅ **P2 · serverInfo.version을 Cargo 버전과 동기화**: 이미 `env!("CARGO_PKG_VERSION")` 적용 상태.

---

## 3. 배포·디스커버리 채널 — "잘 쓰게 만들기 (외부)" — P1

등재는 전부 무료·메타데이터 제출형. 순서 = 효과 순.
1. **공식 MCP Registry** (registry.modelcontextprotocol.io): `server.json` 작성
   (name `xyz.onchain-ai/onchainai`, remote `streamable-http` + URL), 도메인 DNS TXT 검증,
   `mcp-publisher publish`. 레포에 `server.json` 커밋 → Claude 디렉터리/서드파티 클라이언트가 수확.
2. **Smithery** 원격 서버 등재, **PulseMCP / mcp.so / Glama / Cursor Directory** 제출 폼.
3. **awesome-mcp-servers** PR (crypto 섹션).
4. **x402 생태계 등재**: x402.org/Coinbase x402 레포의 ecosystem 리스트에 "discovery/index"
   카테고리로 PR — *결제 서비스가 아니라 디렉터리임을 명시* (커스터디 없음 정책 인용).
5. **Claude Code 플러그인**: 자체 마켓플레이스는 완료. 커뮤니티 마켓플레이스 목록에 제출.
6. 출범 포스트: X/Farcaster + dev.to 아키텍처 글("agent-native Rust MCP directory") — 레포 공개가 전제.

각 등재 후 `docs/CONNECT.md`에 등재 위치 목록 추가(신뢰 앵커).

---

## 4. x402 활성화 — 검증 잡 — ✅ 구현 완료 (`be49680`)

> 원칙 재확인(AGENTS.md 하드룰): attribution/trust 메타데이터만. 커스터디·facilitator·게이트웨이·
> 자금이동·비문서화 `referrer`/`split` 필드 금지. 검증 플래그는 **공개 게이트가 아니라 신뢰 뱃지**
> (`PUBLIC_TOOL_WHERE`/RLS에 추가 금지 — X402_REFERRAL_SPEC 2026-06-27 결정 유지).

> **2026-07-03 저녁 구현됨**: `src/server/x402_verify.rs`(유닛 테스트 8건 포함), migration 028,
> `X402_VERIFY_CRON` 일일 잡(SKIP_CRAWLER 시 미등록), `POST /api/v2/admin/tools/{id}/x402-verify`.
> 아래 4.1–4.5는 설계 기록으로 유지. **잔여**: 운영 DB 마이그레이션 + `sqlx prepare`, 도구별
> `x402_endpoint` 등록(운영자), admin UI 재검증 버튼/뱃지 노출, x402 실스펙 필드명 재실측(§4.3 ⚠️).

### 4.1 당시 갭 (구현 전 기록)
`payment_verified`·`x402_endpoint_verified`·`price_verified`는 스키마/노출/어드민 API까지 있으나
**갱신 수단이 수동뿐**. 자동 검증 잡이 없어 "operator verified" 뱃지가 실질 항상 false.
또한 프로브 대상 URL 컬럼이 없음(`x402_price`는 텍스트, 엔드포인트 필드 부재).

### 4.2 스키마 — `migrations/028_x402_verification.sql`
```sql
ALTER TABLE tools
  ADD COLUMN IF NOT EXISTS x402_endpoint TEXT,               -- 프로브할 결제 엔드포인트(도구 주인이 제공)
  ADD COLUMN IF NOT EXISTS x402_last_checked_at TIMESTAMPTZ, -- 마지막 프로브 시각
  ADD COLUMN IF NOT EXISTS x402_check_failures INTEGER NOT NULL DEFAULT 0;
-- RLS: 기존 public 정책 그대로 (컬럼 추가만; 게이트 불변). sqlx prepare 재실행.
```

### 4.3 모듈 — `src/server/x402_verify.rs` (ssr 전용)
- `probe_x402_endpoint(client, url) -> ProbeOutcome`:
  - **SSRF 가드(필수)**: `https` 스킴만, 호스트 DNS 해석 후 사설/루프백/링크로컬/메타데이터 IP 거부,
    리다이렉트 금지, 타임아웃 5s, 응답 64KB 캡. (`install_safety.rs`의 위험 판정 패턴 참고, 신규 유틸로.)
  - 요청: 결제 헤더 없이 `POST`(빈 바디) 또는 `GET` — **402 응답이 오는지**가 라이브니스.
  - 402 응답 바디의 x402 `PaymentRequirements`(`accepts[]`: `scheme`/`network`/`maxAmountRequired`/
    `payTo`/`asset`) 파싱. ⚠️ 구현 직전 x402 최신 스펙 필드명 실측(스펙 진화 중 — X402_REFERRAL_SPEC §9).
- 플래그 갱신 규칙:
  - `x402_endpoint_verified` ← 402 + 파싱 성공. 연속 3회 실패 시 false로 강등(`x402_check_failures`).
  - `price_verified` ← 파싱된 금액/자산이 `x402_price` 표기와 일치(정규화 비교)할 때만 true.
  - `payment_verified`(소유권) — **자동 갱신 금지.** 클레임 플로(`request_tool_claim`)에서 도구 주인이
    `x402_pay_to_address` 소유를 서명으로 증명했을 때 운영자가 수동 승인 (기존 스펙 §4 표 유지).
- 실행 경로 2개:
  1. **크론 잡**: `X402_VERIFY_CRON`(기본 매일 1회), `SKIP_CRAWLER=1`이면 미등록. 대상:
     `pricing='x402' AND x402_endpoint IS NOT NULL` + public 게이트 통과 도구. 동시성 세마포어 4,
     도구당 1req — 외부 서비스에 예의.
  2. **어드민 수동 재검증**: `POST /api/v2/admin/tools/{id}/x402-verify` (admin guard + rate limit)
     → 즉시 프로브, 갱신된 플래그 반환. 프론트 admin/tools에 "Re-verify" 버튼(뱃지 3종 표시).
- 로깅: tracing으로 결과 기록. `referral_events`에는 넣지 않음(어트리뷰션 로그 순수성 유지).

### 4.4 테스트 (수용 기준)
- wiremock: 402+올바른 PaymentRequirements → endpoint_verified true / 가격 불일치 → price_verified false
  / 200 응답(402 아님) → endpoint_verified false / 3연속 실패 강등.
- SSRF 유닛: `http://`, `https://127.0.0.1`, `https://169.254.169.254`, 사설대역, 리다이렉트 → 전부 거부.
- 회귀: 검증 false여도 public 노출 불변(기존 `PUBLIC_TOOL_WHERE` 테스트 유지), `cargo test --features ssr`
  + clippy/fmt 통과, 마이그레이션 후 `sqlx prepare`.

### 4.5 운영 문서
- `OPERATOR_GUIDE.md`에 "x402 검증/레퍼럴 운영" 섹션: 플래그 의미, 수동 재검증, 레퍼럴 합의 절차.
- 🔲 P1 `docs/TOOL_OWNERS.md`(영문): 도구 주인용 — 클레임 → `x402_endpoint`/가격 등록 → 검증 뱃지
  획득 → (선택) 레퍼럴 합의(`referral_model` attribution/split은 운영 합의 라벨임을 고지).

---

## 5. 어댑션 계측 — "잘 쓰게 만들기 (내부 시스템)" — P1

목표: **발견 → 연결 → 호출 → 설치** 퍼널을 측정 가능하게.

| 단계 | 신호 | 현재 | 계획 |
|---|---|---|---|
| 발견 | /connect·llms.txt 조회 | Vercel analytics만 | `funnel_events` 기록 (아래) |
| 연결 | 연결 명령/JSON 복사 | 없음 | `connect_copy` 이벤트 (client) |
| 호출 | MCP tools/call 볼륨 | tracing 로그만 | 일별 집계 카운터 (Railway 로그 기반 P1, 테이블 P2) |
| 설치 안내 | `get_install_guide` | ✅ `referral_events` | 유지 |
| 외부 이탈 | 상세페이지 click-out | 없음 | `click_out` 이벤트 |

- **신규 테이블** `funnel_events`(migration 029): `id, event_type CHECK(IN ('connect_view','connect_copy','click_out')), tool_id UUID NULL, client TEXT NULL, session_hash TEXT NULL, created_at`.
  `referral_events`와 분리 이유: referral은 x402/레퍼럴 어트리뷰션 전용(tool_id NOT NULL, 정산 대조용) —
  섞으면 정산 근거가 오염됨.
- **수집 endpoint**: `POST /api/v2/events` — 익명, 바디 `{event_type, tool_id?, client?}`,
  IP 레이트리밋(기존 `rate_limit.rs` 패턴), 세션은 해시만(PII 저장 금지), event_type 허용목록 강제.
- **어드민 대시보드 위젯**: 기존 `referral-stats` 옆에 funnel 요약(주간 연결/복사/이탈 수).
- **주간 KPI 정의**: MCP 연결 클라이언트 수(추정), tools/call 수, install_guide 수, 플러그인 설치 프록시
  (GitHub stars/clone), x402 도구 중 verified 비율.

---

## 6. 로드맵 요약

| 순번 | 항목 | 우선순위 | 규모 | 상태 |
|---|---|---|---|---|
| 1 | README/LICENSE/SECURITY/CONTRIBUTING/CONNECT/llms.txt | P0 | — | ✅ 이번 턴 |
| 2 | 플러그인 번들 분리 + 검증 게이트 | P0 | — | ✅ 이번 턴 |
| 3 | gitleaks 히스토리 스캔 + 키 로테이트 + repo 설정 + flip (§1.3) | P0 | 0.5d 수동 | 🔲 |
| 4 | flip 직후 스모크(플러그인 실설치·MCP 연결·llms.txt) | P0 | 1h | 🔲 |
| 5 | /connect Plugin&Skill 섹션 + 푸터 링크 | P1 | — | ✅ 이 브랜치 (배포 전 ui-change-gate만) |
| 6 | x402 검증 잡 (§4: 028 마이그레이션 + 모듈 + cron + admin route + 테스트) | P1 | — | ✅ `be49680` (운영 DB migrate + admin UI 노출 잔여) |
| 7 | MCP Registry/Smithery/등재 일괄 (§3) | P1 | 1d 수동 | 🔲 |
| 8 | funnel_events + /api/v2/events + 대시보드 위젯 (§5) | P1 | 1–2d | 🔲 |
| 9 | TOOL_OWNERS.md + OPERATOR_GUIDE x402 섹션 | P1 | 0.5d | 🔲 |
| 10 | GET /mcp 안내 응답, protocolVersion 에코, 버전 동기화 | P2 | 0.5d | ✅ 2026-07-05 (배포 대기) |
| 11 | 컬렉션→플러그인 export (SKILL_PLUGIN_SPEC §3 J3) | P2 | 별도 스펙 | 🔲 |
| 12 | UI 통합 감사 잔여 — P0: 🔧A0/A2 운영 정렬 · B/C/D 랜딩 회귀 검증(슬라이스 0) · G2 featured 겹침 → P1: J1–J2 빈상태·H3 CTA·I 검색·K1 배지·L1/L4 (상세: [통합 감사 스펙 = Grok 실행 패킷 v3.1](superpowers/specs/2026-07-03-visual-ui-audit-16-agent-spec.md)) | P0/P1 | 1–2d | ◐ Phase A/B/C/D ✅(`bfcb662`·`f81c78e`·`ae8e9aa`) · 잔여 Grok 위임 (결정 완료 §0.3) |

## 7. 비범위 (변경 없음)
- x402 커스터디/facilitator/게이트웨이/자금이동 일체 (X402_REFERRAL_SPEC §7).
- 검증 플래그의 public 게이트 편입.
- 자동 CI 상시화(예산) — 수동 dispatch 유지.

## 8. 이번 턴 검증 기록 (2026-07-03)
- `./scripts/spec-verify.sh J1 J2` → 7 PASS (신설 `J2-market`·`J2-devmcp` 포함).
- `claude plugin validate plugin/onchainai` / `claude plugin validate .` → 둘 다 통과.
- 매니페스트 3종 `JSON.parse` OK. `git check-ignore`로 `.env.example` 추적 가능·`.env*` 무시 확인.
- 시크릿 패턴 grep 0건, `git log --diff-filter=A`에 env류 추가 이력 없음 (정밀 스캔은 §1.3-1).
- `./scripts/agent-harness-check.sh` → PASS (`cargo check --features ssr` 포함; 이 셸에선
  `OPENSSL_DIR="$HOME/.local/openssl" OPENSSL_STATIC=1` 지정 필요 — `.zshrc` 값).
- `./scripts/check-mcp-config-parity.sh` PASS, `node scripts/sync-ui-watch-paths.mjs` 재동기화(115 paths).
- frontend: `npx tsc --noEmit` OK, `next build` 성공(15/15 정적 생성), `next start`(:3100) 브라우저
  검증 — /connect 플러그인 카드·슬래시 명령(프리픽스 없음)·복사 버튼, 푸터 llms.txt·Connect 링크,
  `/llms.txt` 200, TopNav GitHub 링크 = 실제 레포. 콘솔 에러 0.
