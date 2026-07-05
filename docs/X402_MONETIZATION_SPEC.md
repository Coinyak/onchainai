# OnchainAI x402 수익화 — 최종 스펙

> Related: [X402_ROADMAP](X402_ROADMAP.md) (living plan, advisory) | [X402_REFERRAL_SPEC](X402_REFERRAL_SPEC.md) | [X402_OPEN_LISTING_SPEC](X402_OPEN_LISTING_SPEC.md) (worktree) | [PRODUCT_ENHANCEMENT_SPEC](PRODUCT_ENHANCEMENT_SPEC.md) §K | [2026-07-04-free-tier-guardian-spec](superpowers/specs/2026-07-04-free-tier-guardian-spec.md) | [2026-07-03-x402-activation-spec](superpowers/specs/2026-07-03-x402-activation-spec.md) | [AGENTS](../AGENTS.md)
>
> Date: 2026-07-04  
> Status: **Final — 창업자 확정 (10-agent 최종 검토)**  
> Scope: OnchainAI **수익화 정본** — 가격·제품·무료 티어·금지·GTM·KPI. 구현 핸드오프는 OPEN_LISTING L페이즈·코드 경로 참조.

**본 문서는 최종 검토 합의문이다.** L1 셀프등록·Bazaar 시드·UI 상세는 `X402_OPEN_LISTING_SPEC.md`를 따른다.

---

## 0. 한 줄 북극성

> **크립토 툴 디렉터리(일반 MCP)는 영구 무료로 모으고, x402는 신뢰 레이어로 차별화한다. 돈은 (1) 에이전트 신뢰 데이터, (2) 제공자 B2B, (3) 장기 어트리뷰션 협상에서만 낸다.**

---

## 1. 확정 결정 (Locked — 재협상 없음)

| # | 결정 |
|---|------|
| L1 | **발견 100% 무료** — 웹·MCP·REST. 일반 MCP **및** x402 카탈로그 메타 전부 노출 |
| L2 | **x402 discovery paywall 금지** — `search_tools` 등에 402 게이트 불가 |
| L3 | **일반 MCP에서 x402 숨김 금지** — 필터·검색·비교에 x402 포함 |
| L4 | **`compare_tools` 영구 무료** — 웹 `/compare` + MCP + API (§K2 유료안 **폐기**) |
| L5 | **에이전트 유료 = 신뢰 데이터만** — 결제·설치 **직전** 마이크로페이먼트 |
| L6 | **제공자 유료 = OnchainAI 서비스** — 등록·검증·Featured (payee = OnchainAI) |
| L7 | **2.5% 어트리뷰션 = P&L $0** — 협상 칩·데이터만 (`referral_events`) |
| L8 | **커스터디·제3자 결제 프록시·라우팅 마진 금지** |
| L9 | **검증 플래그 = 뱃지** — `PUBLIC_TOOL_WHERE`에 결제 검증 추가 금지 |

---

## 2. 수익 아키텍처 (두 축)

```
┌─────────────────────────────────────────────────────────────┐
│  축 A — 발견 어트리뷰션 (Non-P&L)                            │
│  에이전트 → 제3자 x402 도구 직접 결제 (OnchainAI 경유 없음)   │
│  OnchainAI: builder_code 메타 + referral_events            │
│  수익: 협상·수동 정산만 (Phase 3+, KPI≠매출)                │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  축 B — 자기 서비스 x402 판매 (P&L)                          │
│  B-에이전트: check_endpoint_health, vet_before_pay …       │
│  B-제공자: 등록료, Verification SLA, Featured              │
│  payee = OnchainAI 지갑 · 제3자 자금 미보관                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. 영구 무료 제품 (Free Forever)

정책 상세·회귀 게이트: `docs/superpowers/specs/2026-07-04-free-tier-guardian-spec.md`

### 3.1 웹

| 표면 | 경로/API |
|------|----------|
| 검색·필터·상세 | `/tools`, `/tools/[slug]`, `/api/v2/tools/*` |
| **비교** | `/compare`, `GET /api/v2/tools/compare` |
| x402 필터·메타 | `?pricing=x402`, Type=x402 사이드바 |
| 설치 가이드 | 툴 상세·비교 패널 |
| SEO 허브 (구현 예정) | `/x402` — SSR, 결제 UI 없음 |
| Connect·Plugin | `/connect`, `plugin/onchainai` |

### 3.2 MCP (무인증 공개 티어)

| Tool | 상태 |
|------|------|
| `search_tools` | LIVE |
| `get_tool_detail` | LIVE |
| `get_install_guide` | LIVE |
| `list_categories` | LIVE |
| `get_dashboard_snapshot` | LIVE |
| **`compare_tools`** | **PLANNED — 반드시 무료** |
| `export_toolkit` | PLANNED — **무료** (일반 번들; 100+ slug 대량은 후행 검토) |

### 3.3 수용 기준

```bash
./scripts/spec-verify.sh gate FTG-A FTG-B FTG-C FTG-D FTG-E
```

- 발견 API/MCP에 HTTP 402·지갑 선행 조건 **0건**

---

## 4. 유료 제품 — 에이전트 신뢰 (축 B-에이전트)

**원칙:** 발견은 무료. **제3자 x402 API에 지갑 열기 직전** OnchainAI 신뢰 스냅샷만 유료.

**결제자:** 호출하는 에이전트/유저 → **OnchainAI** (`X402_PAY_TO_ADDRESS`)  
**제3자 API 호출비:** 별도 — 유저 → 도구 제공자 (OnchainAI 비경유)

### 4.1 SKU · 가격표

| SKU | MCP / API | Launch | Mature | 단위 |
|-----|-----------|--------|--------|------|
| `AGT-HEALTH` | `check_endpoint_health` | **$0.001** | $0.001 | /호출 |
| `AGT-VET` | `vet_before_pay` | **$0.001** | $0.001 | /호출 |
| `AGT-PRICE-HIST` | `price_history` | $0.001 | $0.001 | /호출 |
| `AGT-PROBE-NOW` | `probe_now` (온디맨드 프로브) | $0.005 | $0.005 | /호출 |
| `AGT-BULK-VET` | `bulk_vet` (≤10 slug) | $0.008 | $0.008 | /호출 |

### 4.2 제품 설명

| Tool | 제공 데이터 |
|------|-------------|
| `check_endpoint_health` | 라이브 여부, 마지막 프로브, 업타임(이력 축적 후) |
| `vet_before_pay` | health + 가격 일치 + 검증 플래그 + install_risk **한 방** |
| `price_history` | `x402_probe_history` 기반 가격 변경·불일치 이력 |
| `probe_now` | 실시간 `probe_x402_endpoint` 1회 (크론 캐시 아님) |
| `bulk_vet` | 스택/워크플로용 다건 vet |

### 4.3 결제 플로우

```
무결제 호출 → HTTP 402 + PAYMENT-REQUIRED
→ 클라이언트 USDC 결제 (Base mainnet, CDP facilitator verify/settle)
→ 200 + JSON + PAYMENT-RESPONSE
```

구현 참고: `feat/x402-open-listing` — `src/server/x402_payment.rs`, `x402_premium.rs`, `scripts/x402-premium-e2e.mjs`

### 4.4 무료 쿼터 (권장)

| Tool | 무료/지갑/일 |
|------|-------------|
| health, price_history, vet | 각 3회 (합산 상한 **5회/일**) |
| probe_now, bulk_vet | **0회** |

식별: x402 정산 `payer` 주소. Agent Sync Bearer 연동 시 **+5회/일** 보너스(선택).

### 4.5 구현 상태

| Tool | 상태 |
|------|------|
| `check_endpoint_health` | worktree 구현·prod E2E 완료 → **main 머지 대기** |
| 나머지 | PLANNED (L4 `x402_probe_history` 선행) |

---

## 5. 유료 제품 — 제공자 B2B (축 B-제공자)

**결제자:** 툴 메이커 → OnchainAI. **Sponsored** 라벨 필수. **품질 게이트 우회 불가.**

### 5.1 SKU · 가격표

| SKU | Launch | Mature | 단위 | 비고 |
|-----|--------|--------|------|------|
| `PROV-REG-FOUNDING` | **$0** | — | /툴 | 런치 후 **90일**, 최대 50팀 |
| `PROV-REG` | **$5** | **$10** | /등록 1회 | x402 셀프등록만 |
| `PROV-VERIFY-SLA` | **$15/월** | **$20/월** | /툴·월 | 일 1회 프로브 + 3종 뱃지 |
| `PROV-FEATURED` | **$5/주** | **$10/주** | /슬롯·주 | 홈 캐러셀, 동시 ≤5슬롯 |

### 5.2 번들 (선택)

| 패키지 | 가격 | 포함 |
|--------|------|------|
| **Founding Builder Pack** | $49 1회 | 등록 + 검증 1회 + Featured 1주 |
| **Trust Launch** | $79 1회 | 익스프레스 검증 + Featured 2주 |

### 5.3 등록 정책

| 타입 | 등록료 |
|------|--------|
| MCP / CLI / SDK / API (일반) | **무료** |
| x402 (`pricing=x402` 또는 type=x402) | Founding $0 → 이후 표준 SKU |
| Bazaar 크롤 시드 | **무료** · `referral_enabled=false` · 운영자 승인 큐 |

런칭 스위치: `site_settings.allow_x402_registration` (L1 배포·스모크 후 `true`)

### 5.4 구현 상태

전 SKU **PLANNED** (Admin 수동 Featured·검증만 LIVE). M2 x402 결제 수납은 Phase 3.

---

## 6. 어트리뷰션 (축 A — Non-P&L)

**2.5% (250 bps)는 협상 목표가이지 인식 수익이 아니다.**

| Tier | `referral_model` | 수익 |
|------|------------------|------|
| **B — 기본** | `attribution` | $0 · 이벤트만 |
| **A — 협조** | `split` (비즈니스 라벨) | 합의·입금 후만 인식 |

**KPI (매출 아님):** `referral_events` 수, referral-enabled 도구 수, install_guide 전환, Tier A 전환 수.

**수동 정산 시작:** Tier A 서면 합의 + tx 증빙 + Phase 3 (x402 ≥10, 월 install_guide ≥100).

상세: `X402_REFERRAL_SPEC.md`, `2026-07-03-x402-activation-spec.md`

---

## 7. 명시적 금지 (REJECTED)

| 항목 | 이유 |
|------|------|
| x402 discovery paywall | Bazaar·SEO·MCP 신뢰 붕괴 |
| 일반 MCP에서 x402 결과 제거 | 디렉터리 완전성 |
| 제3자 x402 **호출** 프록시·라우팅 마진 | 커스터디·AGENTS.md 위반 |
| `compare_tools` / `export_toolkit` 호출 과금 | OD-FTG 영구 무료 |
| 어트리뷰션 2.5%를 CFO 시트 매출로 계상 | 회수율 8–20% |
| 검증 플래그를 노출 게이트로 사용 | 017 원칙 |
| 문서화되지 않은 `referrer`/`split` 필드 | 하드룰 |

> **참고:** `docs/MCP_X402_MONETIZATION_SPEC.md`의 `compare_tools`/`export_toolkit` 유료안은 **본 스펙에 의해 폐기**. 해당 코드 경로는 default off 유지·제거 검토.

---

## 8. OPEN_LISTING 정렬 (L ↔ 수익)

| L단계 | 수익화 |
|-------|--------|
| L1 셀프등록 | 공급 확대 (등록료는 Phase 1b) |
| L2 Bazaar 시드 | 카탈로그 밀도 (무료) |
| L3/L4 어트리뷰션 | 축 A 데이터 |
| L5 `/x402` 허브 | SEO · **무료** |
| M3 Agent Trust | 축 B-에이전트 |
| M2 Featured/검증 | 축 B-제공자 |

---

## 9. 90일 GTM (KPI — 매출 후순위)

| Phase | 기간 | Ship | 1차 KPI |
|-------|------|------|---------|
| **0** | D0–14 | `feat/x402-open-listing` → main | deploy verify, MCP 회귀 0 |
| **1** | D14–35 | `allow_x402=true`, `/x402`, L1 | **x402 툴 ≥10**, 셀프 ≥3 |
| **2** | D35–60 | Bazaar 크롤, L3/L4 | Bazaar 승인 ≥30, events ↑ |
| **3** | D60–90 | `check_endpoint_health` prod, M1 뷰 | E2E PASS, premium 호출 ≥10/주 |

**의도적 후순위:** MRR, referral 청구액 (v1 수동).

---

## 10. 매출 전망 (참고 — 어트리뷰션 $0)

| 기간 | 보수 | 기본 |
|------|------|------|
| **90일** | $342 | $1,140 |
| **12개월** | $4,182 | $11,863 |

주력: 제공자 Featured·검증 (초기) + 에이전트 trust (K2 머지 후). 상세: `docs/REVENUE_FORECAST.md` (있을 경우).

---

## 11. AGENTS.md 정합

하드룰 문구 (머지 시 동기화):

- **제3자** x402 = 메타·어트리뷰션만 (커스터디·프록시·자금이동 금지)
- **예외 허용:** OnchainAI **자기** Agent Trust·제공자 B2B 서비스 x402 수취
- **금지 유지:** 발견 API 402, 제3자 결제 경로 중계

---

## 12. 검증 (DoD)

| ID | 검사 |
|----|------|
| MON-1 | 무료 MCP 5종 + compare(구현 시) 402 없음 |
| MON-2 | `check_endpoint_health` 402→결제→200 E2E 1건 (`x402-premium-e2e.mjs`) |
| MON-3 | Featured 카드 Sponsored 라벨 (유료화 시) |
| MON-4 | `PUBLIC_TOOL_WHERE`에 x402 검증 플래그 없음 |
| MON-5 | FTG-A~E 스펙 게이트 PASS |
| MON-6 | 일반+x402 웹 비교 `/compare` 비인증 200 |

---

## 13. 구현 우선순위 (요약)

```
1. main ← feat/x402-open-listing (L1/L5 + check_endpoint_health)
2. allow_x402_registration=true (스모크 후)
3. Bazaar 시드 (L2)
4. vet_before_pay + 무료 쿼터
5. price_history (probe_history 30일+)
6. Provider x402 checkout (M2)
7. compare_tools MCP 무료 구현 (A2)
```

---

## 14. 10-agent 최종 합의 (2026-07-04)

| 관점 | 결론 |
|------|------|
| UX | discovery paywall **Bad** · compare 무료 **Good** |
| 경쟁 | Bazaar 무료 → 숨김 해자 **2/10** |
| 매출 | 축 B > 어트리뷰션 · 90일 **$200–1.1K** 현실적 |
| 규칙 | 등록·trust·Featured **OK** · discovery 숨김 **금지** |
| GTM | 공급→데이터→프리미엄 순 · attribution 매출 기대 **$0** |
| 무료 티어 | OD-FTG 확정 · compare 영구 무료 |
| 제공자 | Founding $0 90d → 검증·Featured 유료 |
| Agent Trust | 1센트 헬스 = 킬러 SKU · vet 병행 |
| 어트리뷰션 | 협상 칩 · Tier A/B 분리 |
| 스펙 구조 | 본 문서 = 수익 정본 · OPEN_LISTING = 공급 정본 |

---

### 한 줄 요약

**일반·x402 발견은 전부 무료, compare도 무료, 돈은 에이전트 1센트 헬스와 제공자 등록·검증·Featured에서만 받고, 2.5% 어트리뷰션은 장부에 넣지 않는다.**