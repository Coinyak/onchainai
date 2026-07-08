# Free Tier Guardian — 현재 무료 티어 정책 스펙 (운영자 재량)

> Related: [[../../X402_OPEN_LISTING_SPEC]] | [[../../X402_REFERRAL_SPEC]] | [[2026-07-03-x402-activation-spec]] | [[../../UI_UX_IMPROVEMENT_SPEC]] Phase 2·5 | [[../../../AGENTS.md]] | [[../../../README.md]]
>
> Date: 2026-07-04 (updated 2026-07-08)  
> Status: **Advisory — 운영자 재량 (OD-FTG 완화)**  
> Scope: OnchainAI 자체 서비스에서 현재 무료로 유지하는 표면을 문서화. **영구 무료가 아닌 운영자 재량** — 필요 시 유료화 가능. 회귀 방지 가드레일은 advisory로 전환.  
> Evidence: `README.md`, `src/server/mcp.rs`, `src/server/api_v2/public_tools.rs`, `frontend/app/compare/`, `X402_OPEN_LISTING_SPEC` 정본 대조

**본 문서는 구현 코드를 포함하지 않는다.** 정책·수용 기준·검증·금지 사항만 정의한다.

---

## 0. 세션 요약

OnchainAI의 wedge는 **크립토 특화 × 큐레이션 × 신뢰/설치안전**이다. 에이전트·개발자가 *도구를 찾고 비교하고 설치 가이드를 받는* 핵심 발견 루프는 **현재 무료**로 유지한다 (운영자 재량, 영구 규칙 아님). 수익화(K1 어트리뷰션, 미래 K3 스폰서 노출)는 발견 루프 바깥 또는 제3자 x402 도구 호출에 적용한다. **예외:** OKX A2MCP Path A 활성 시 마켓플레이스 **단일 번들 SKU**로 모든 MCP `tools/call`이 미터링된다 (OD-FTG-5) — 기본 free-discovery 가이드라인의 의도적 예외이며, 상세는 `docs/listings/directory-forms.md` §Policy exception.

| # | 정책 갭 | 스펙 ID | 심각도 |
|---|---------|---------|--------|
| 1 | §K2 초안이 `compare_tools`를 유료 MCP로 분류 | FTG-1 | P0 |
| 2 | 웹 `/compare`·API와 MCP `compare_tools`(미구현) 간 무료 정책 불일치 위험 | FTG-2 | P0 |
| 3 | x402 **카탈로그 발견** vs **제3자 유료 호출** 경계가 공개 문서에 분산 | FTG-3 | P1 |
| 4 | SEO `/x402` 허브(미구현)가 유료 게이트·CSR 전용으로 출시될 위험 | FTG-4 | P1 |
| 5 | OnchainAI MCP에 402 핸드셰이크가 들어갈 위험(K2 deferred와 충돌) | FTG-5 | P0 |

**창업자 결정(OD-FTG, 2026-07-04; 완화 2026-07-08)**: 아래 §2 목록은 **현재 무료(운영자 재량)** — 영구 규칙이 아님. 향후 수익 실험은 §3 **명시적 유료 후보** 또는 운영자 판단으로 §2 항목을 유료화할 수 있다. 회귀 방지 가드(FTG-D2 등)는 advisory.

---

## 1. 제품 목표

1. 에이전트·웹 사용자가 **로그인·지갑·x402 결제 없이** 크립토 툴을 검색·상세·비교·설치 가이드·x402 메타데이터까지 이용할 수 있다.
2. **일반 MCP 5툴**과 **x402 카탈로그 축**(필터·배지·고지·검증 신호)이 동일한 무료 정책을 따른다.
3. `compare_tools`는 **웹·REST·MCP** 삼면 모두 무료이며, URL 공유·에이전트 자동 비교에 장벽이 없다.
4. SEO **`/x402` 허브**는 크롤러·AI 인용에 적합한 **공개 SSR 랜딩**으로 제공한다(결제 UI 없음).
5. OnchainAI는 제3자 x402 도구의 **가격 메타를 고지**할 뿐, 기본 모드에서는 자체 발견 API에 402를 반환하지 않는다. **OKX A2MCP Path A(OD-FTG-5) 활성 시**에는 마켓 단일 SKU로 `tools/call` 전부가 미터링될 수 있다.

---

## 2. 현재 무료(운영자 재량) — 제품 가이드라인 목록

> **정의**: 인증 불필요·결제 불필요·계정 불필요. IP/유저 **레이트리밋**은 허용(남용 방지 ≠ 유료화). Agent Sync Bearer 토큰은 **계정 연결**이지 결제가 아니다.

### 2.1 공통 원칙 (웹 + MCP + REST)

- **툴 발견**: 텍스트 검색, 카테고리·체인·타입·`pricing=x402` 필터, 정렬(`relevance`/`trust`/`stars`/`recent`), 페이지네이션.
- **툴 상세**: trust·install risk·chains·official links·x402 가격·검증 배지(`payment_verified` 등) — **표시만**, 노출 게이트 아님.
- **설치 가이드**: 플랫폼별 단계(claude/cursor/generic/cli), `critical` 위험 차단, x402 `x402_notice`·레퍼럴 disclosure.
- **비교**: 2~4개 slug 기준 비교 행렬(trust/chains/pricing/install risk/type/status 등).
- **공개 대시보드 스냅샷**: 커버리지·featured·x402 집계.
- **x402 카탈로그 축**: x402 타입/가격 메타 **발견·필터·비교·고지** — OnchainAI는 **메타데이터만** 노출, 결제 실행·지갑 연결·facilitator 프록시 없음.
- **품질 게이트**: `PUBLIC_TOOL_WHERE`·`install_risk=critical` 차단 — **신뢰는 무료 공공재**이며 유료로 우회 불가.

### 2.2 웹 (Next.js / Vercel)

| 표면 | 경로·API | 현재 무료 범위 |
|------|----------|----------------|
| 툴 브라우저 | `/tools`, `POST /api/v2/browser-data`, `GET /api/v2/tools/list` | 전 필터·미리보기·`compare_tools` 쿼리 파라미 |
| 툴 상세 | `/tools/[slug]`, `GET /api/v2/tools/:slug` | SSR 메타·JSON-LD·OG·본문 요약 |
| **비교** | `/compare?tools=`, `GET /api/v2/tools/compare` | 2~4 slug, URL 공유, 설치 가이드 접이식 |
| 대시보드 | `/dashboard`, `GET /api/v2/dashboard` | 공개 스냅샷 |
| 카테고리·체인 | `/categories/[id]`, `GET /api/v2/categories`, `GET /api/v2/chains` | 전부 |
| 연결 허브 | `/connect` | MCP URL·딥링크·플러그인 안내 |
| **x402 SEO 허브** | `/x402` (신규) | x402 툴 큐레이션 랜딩, 필터 딥링크, FAQ, `pricing=x402` 사전 렌더 |
| 설치 가이드 패널 | 툴 상세·비교 내 | x402 고지 포함, 결제 CTA 없음 |

### 2.3 MCP (`POST /mcp`, Bearer 없음 = 공개 티어)

| MCP tool | 현재 무료 범위 | 비고 |
|----------|----------------|------|
| `search_tools` | query·category·chain·sort·limit·cursor | x402 툴 포함 검색 |
| `get_tool_detail` | slug 1건 전체 공개 필드 | x402_price·검증 플래그 포함 |
| `get_install_guide` | slug+platform, x402_notice·referral 메타 | attribution 기록은 무료 동작 |
| `list_categories` | 전 카테고리+카운트 | — |
| `get_dashboard_snapshot` | limit≤12 공개 집계 | x402 섹션 포함 |
| **`compare_tools`** (A2 신규) | slugs 2~4, trust/x402/chains/pricing/install 비교 | **§K2 유료 분류 폐기** — 구현 시 반드시 공개 티어 |

**금지**: 위 6툴(및 동등 REST)에 대해 HTTP 402, `paymentRequired`, `X-Payment` 헤더 요구, API key·지갑 선행 조건.

### 2.4 MCP Agent Sync (Bearer = 계정 연결, **결제 아님**)

| MCP tool | 무료 정책 |
|----------|-----------|
| `save_to_toolkit` | 로그인 연동 무료; x402 과금 없음 |
| `save_stack_to_blueprint` | 동일 |
| `link_status` | 동일 |

### 2.5 REST 공개 API (`/api/v2/*`, 인증 없음)

- `GET /api/v2/tools/compare` — **현재 무료** (`compare_tools` MCP와 필드·한도 동일).
- `GET /api/v2/tools/search`, `GET /api/v2/tools/:slug`, `GET /api/v2/dashboard`, `GET /api/v2/categories`, `GET /api/v2/chains` — 현재 무료.
- 공개 응답은 기존처럼 `sanitize_tool_for_public_response` 적용(payout 주소 등 민감 필드 제거).

---

## 3. 명시적 유료·보류 후보 (§2 침범 금지)

> `X402_OPEN_LISTING_SPEC` §K **보류** 유지. 아래는 **미래 실험 후보**이며 §2와 **겹치면 안 된다**.

| 후보 | 설명 | §2와의 경계 |
|------|------|-------------|
| K1 어트리뷰션 | 제3자 x402 도구 **호출 시** 메이커↔디렉터리 수익 분배 협상 | OnchainAI **발견 API 과금 아님** |
| K3 스폰서 노출 | Featured·verified 신청 x402 결제 | **Sponsored** 라벨 필수; quality gate 우회 불가 |
| (미래) 대량 export | 컬렉션 100+ slug 일괄 export·웹hook | `compare_tools`·일반 검색·`/x402` 허브 **제외** |
| (미래) 고급 랭킹 티어 | 임베딩·커스텀 스코어 API | 기본 `relevance`/`trust` 정렬 **제외** |

**폐기(창업자 결정)**: §K2 초안의「`compare_tools`·핵심 검색 MCP 호출당 x402」— **채택하지 않음**.

---

## 4. SEO `/x402` 허브 스펙 (현재 무료 랜딩)

### 4.1 목적

- 검색·AI 크롤러 유입용 **x402 결제 가능 툴** 큐레이션 허브.
- [x402 Bazaar](https://docs.cdp.coinbase.com/x402/bazaar) 벤치마크: 발견·메타·신뢰 신호 — **결제 경로 비포함**.

### 4.2 라우트·콘텐츠

- **URL**: `https://www.onchain-ai.xyz/x402` (apex→www 301 후 서빙).
- **본문**: x402란(1단락)·OnchainAI 역할(메타만)·검증 배지 설명·대표 툴 카드 그리드·`/tools?pricing=x402` CTA.
- **SSR**: 서버 fetch로 첫 HTML에 툴 이름·가격 텍스트·검증 상태 포함(UI_UX Phase 2 패턴 재사용).
- **메타**: `title`: `x402 Payable Crypto Tools | OnchainAI`, `description`, OG/Twitter, canonical, JSON-LD `CollectionPage` + `ItemList`.
- **사이트맵**: `app/sitemap.ts`에 `/x402` 정적 엔트리 + x402 툴 slug 동적 엔트리.
- **내부 링크**: 홈 프로모·`/connect`·툴 상세 x402 배지 → `/x402` 역링크.

### 4.3 수용 기준

- [ ] `/x402` 비로그인 200, raw HTML에 ≥1개 x402 툴 이름 존재.
- [ ] `/x402` 응답에 402·결제 모달·지갑 연결 UI 없음.
- [ ] `curl`·Lighthouse SEO ≥90(Phase 2 게이트와 동일).
- [ ] `sitemap.xml`에 `/x402` 포함.
- [ ] `/tools?pricing=x402`와 허브 카드 데이터 소스 일치(동일 API·필터).

---

## 5. `compare_tools` 삼면 정합 (웹 · REST · MCP)

### 5.1 공통 계약

- **입력**: `slugs` 2~4개(웹: `?tools=` 또는 `?compare_tools=` 브라우저 컨텍스트).
- **출력 축**: name·slug·type·status·pricing·`x402_price`·install_risk·chains·stars·trust 신호·official 여부.
- **한도**: `MIN=2`, `MAX=4` (웹 `frontend/lib/compare.ts`와 동일).
- **가시성**: `PUBLIC_TOOL_WHERE` 동일; 비공개 slug는 soft skip+메시지.
- **인증**: 불필요.
- **과금**: 없음(현재 무료, 운영자 재량).

### 5.2 수용 기준

- [ ] `GET /api/v2/tools/compare?slugs=a,b` 비인증 200, 402 아님.
- [ ] `/compare?tools=a,b` 비로그인 렌더, 매트릭스·URL 공유 동작.
- [ ] MCP `compare_tools` 구현 시 `tools/list`에 **항상** 포함(Bearer 불필요).
- [ ] MCP·REST·웹 비교 행 **동일 slug set**에 대해 pricing/x402/install_risk 필드 일치(스냅샷 테스트).
- [ ] `scripts/smoke-test.sh`에 compare 공개 경로 검사 추가.
- [x] `X402_OPEN_LISTING_SPEC` 정본에 `compare_tools` 유료 문구 없음 — 본 스펙이 정책 출처.

---

## 6. x402 이중 축 가드레일 (General MCP ∩ x402 Catalog)

| 구분 | OnchainAI (무료) | 제3자 x402 도구 (유료 가능) |
|------|------------------|----------------------------|
| 검색·상세·비교 | ✅ 현재 무료 | 카탈로그 엔트리로 무료 **발견** |
| x402_price·endpoint 메타 | ✅ 무료 노출 | 제공자가 설정 |
| install_guide x402_notice | ✅ 무료 고지 | 호출 시 지갑 필요 안내 |
| 실제 API 호출·USDC 결제 | ❌ OnchainAI 미제공 | 제공자 MCP/API에서 발생 |
| referral/attribution | ✅ 무료 기록(K1) | 협상·split은 오프플랫폼 |

**수용 기준**

- [ ] 공개 MCP·REST·웹 경로에 `402 Payment Required` 응답 코드 없음(제3자 URL 프로브 제외).
- [ ] `get_install_guide`·웹 InstallGuide에「OnchainAI는 결제 처리 안 함」고지 유지.
- [ ] x402 미검증 툴도 public gate 통과 시 **무료 노출**(검증은 배지일 뿐).

---

## 7. Free Tier Guardian — 구현·리뷰 가드레일

### 7.1 코드 리뷰 체크리스트 (PR 필수)

- §2 경로에 `402`/`payment_required`/`require_x402_payment`/`premium_only` 추가 여부.
- `compare_tools`·`/compare`·`/api/v2/tools/compare` 분기에 인증·결제 조건 추가 여부.
- `/x402`에 `noindex`·로그인 wall·클라이언트 전용 fetch(빈 SSR) 여부.
- MCP `tool_definitions()`가 `compare_tools`를 authenticated-only로 넣었는지.
- `PUBLIC_TOOL_WHERE`에 유료 플래그를 visibility gate로 추가했는지.

### 7.2 자동 검증 (권장)

| ID | 검사 | 명령·패턴 |
|----|------|-----------|
| FTG-A | MCP 유료 키워드 부재 | `src/server/mcp.rs`에 `402`/`paymentRequired`/`x402_gate` 없음 |
| FTG-B | compare API 공개 | `curl -s -o /dev/null -w '%{http_code}' '$PROD_URL/api/v2/tools/compare?slugs=aave,uniswap'` → `200` |
| FTG-C | 정책 문서 존재 | `docs/superpowers/specs/2026-07-04-free-tier-guardian-spec.md`에 무료 정책 선언 |
| FTG-D | compare 유료 문구 제거 | `scripts/spec-verify.sh` `ftg_compare_free`: `compare_tools`/`/compare` 라인에 과금 키워드(`유료|paid|402|…`)가 있으나 `무료|폐기|채택하지` 등 부정·폐기 표기가 없으면 FAIL |
| FTG-E | README 무료 선언 | `README.md`에 `free`+`read-only` |

### 7.3 문서 동기화

- `README.md` — MCP tools 표에 `compare_tools`(구현 후) 추가, **free** 명시.
- `docs/INDEX.md` — 본 스펙 링크.
- `docs/X402_OPEN_LISTING_SPEC.md` — compare 유료 문구 없음·FTG 링크.
- `docs/superpowers/specs/2026-07-03-x402-activation-spec.md` §2 비목표 — K2 compare 과금 **영구 비목표** 명시.
- `plugin/onchainai/skills/` — compare·x402 발견은 OnchainAI 무료 MCP로 안내.

---

## 8. 비목표

- OnchainAI 자체 discovery API에 x402 micropayment 도입(K2 compare/search 게이트).
- `/x402`에서 제3자 facilitator·지갑 연결·결제 실행.
- `compare_tools`·`/compare` 로그인 필수화.
- Featured·스폰서(K3)가 quality gate·현재 무료 발견을 대체하는 것.
- Free tier 제거를 전제한「grace period 후 유료」플래그.

---

## 9. 오너 결정 로그

| ID | 날짜 | 결정 |
|----|------|------|
| OD-FTG | 2026-07-04 | §2 무료 목록 확정; `compare_tools`·`/x402` 허브·x402 카탈로그 발견 포함 |
| OD-FTG-4 | 2026-07-08 | **완화**: §2를 영구 규칙이 아닌 **현재 무료(운영자 재량)**으로 전환; FTG-D2·k2-prod-smoke discovery 체크 advisory |
| OD-FTG-5 | 2026-07-09 | **OKX Path A 예외**: OKX A2MCP 활성 시 단일 flat SKU로 **모든** MCP `tools/call`(discovery 포함) 미터링. free-discovery 가이드라인 예외. 코드 원복이 아닌 정책 기록. 정본: `docs/listings/directory-forms.md` §Policy exception, ASP #4609 |
| OD-FTG-2 | 2026-07-04 | §K2「compare_tools 유료」**폐기**; 수익 실험은 §3 후보만 |
| OD-FTG-3 | 2026-07-04 | 레이트리밋은 유지; 남용 방지 ≠ paywall |

---

## 10. 완료 정의 (Definition of Done)

- [ ] 본 스펙 `docs/INDEX.md` 등재.
- [x] `X402_OPEN_LISTING_SPEC`·§2와 정합(compare 현재 무료).
- [ ] `scripts/spec-verify.sh`에 FTG-A~E 추가(또는 `scripts/smoke-test.sh` 확장).
- [ ] MCP `compare_tools` 착수 시 §5 계약·테스트 먼저.
- [ ] `/x402` 허브 착수 시 §4 SSR·SEO 게이트 먼저.
- [ ] 프로덕션 smoke: compare·MCP `search_tools`·`get_install_guide` 비인증 성공.