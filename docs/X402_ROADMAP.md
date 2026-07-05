# OnchainAI x402 로드맵 (Living Plan)

> **이 문서는 자문·체크리스트용이다. 언제든 변동될 수 있으며, AGENTS.md 하드룰이나 구현 강제 규칙이 아니다.**
>
> Related: [X402_MONETIZATION_SPEC](X402_MONETIZATION_SPEC.md) (수익·제품 **정본**) · [X402_REFERRAL_SPEC](X402_REFERRAL_SPEC.md) · [2026-07-04-free-tier-guardian-spec](superpowers/specs/2026-07-04-free-tier-guardian-spec.md) · [LAUNCH_READINESS_SPEC](LAUNCH_READINESS_SPEC.md) · [REVENUE_FORECAST](REVENUE_FORECAST.md) · [AGENTS](../AGENTS.md)
>
> Last updated: **2026-07-05** (Bazaar L2 crawler shipped — PR-5)  
> Owner: 창업자 — 우선순위·가격·일정은 이 문서보다 **최신 창업자 지시**가 이긴다.

---

## 0. 에이전트·창업자용 — 언제 읽나

| 질문 유형 | 먼저 읽을 문서 | 이 로드맵 역할 |
|-----------|----------------|----------------|
| “돈 어디서?” “가격 맞아?” “왜 paywall 안 해?” | `X402_MONETIZATION_SPEC.md` | 맥락·단계·체크리스트로 **설명** |
| “이번 스프린트 뭐 넣어?” “다음은?” | **본 문서** §2–§4 | **현재 범위**·후보 우선순위 (변동 가능) |
| 코드/게이트가 막히면 | `AGENTS.md` 하드룰 + FTG 스펙 | 로드맵은 **우회 근거가 될 수 없음** |
| 창업자가 “로드맵 바꿔” | 본 문서 수정 + 필요 시 정본 스펙 동기화 | **여기가 먼저** 갱신 |

**답변 톤:** 로드맵 항목은 “현재 계획”으로 말하고, Locked 정책은 정본 §1·§7·AGENTS.md로 구분한다.

---

## 1. 문서 계층 (무엇이 구속력인가)

```
┌────────────────────────────────────────────────────────────┐
│  AGENTS.md Hard Rules + FTG 스펙                            │
│  → discovery 무료, compare 무료, no custody, no proxy       │
├────────────────────────────────────────────────────────────┤
│  X402_MONETIZATION_SPEC.md (정본)                           │
│  → 제품·가격·금지·DoD — 구현·리뷰 시 기준                    │
├────────────────────────────────────────────────────────────┤
│  X402_ROADMAP.md (본 문서) ← Living, advisory only          │
│  → 단계·체크리스트·스프린트·KPI·상태 — 창업자와 상의용        │
└────────────────────────────────────────────────────────────┘
```

| 구분 | 예시 | 바뀌면 |
|------|------|--------|
| **Locked** (정본) | 발견 100% 무료, discovery paywall 금지, compare 무료 | 정본 + AGENTS.md **명시적** 수정 필요 |
| **Plan** (로드맵) | Phase 2 Bazaar 30건, vet SKU 다음 주 | 창업자 말 한마디로 **즉시** 변경 가능 |
| **Reference** (로드맵) | Launch 가격표, 90일 매출 구간 | 협상·시장 반응에 따라 조정 |

---

## 2. 북극성 (한 줄 — 방향만, 일정은 아님)

> 일반·x402 **발견은 전부 무료**로 모으고, **에이전트 신뢰(1¢)** 와 **제공자 B2B**에서만 P&L을 낸다. 2.5% 어트리뷰션은 협상 칩(P&L $0).

---

## 3. 현재 스프린트 범위 (2026-07 기준)

**목표:** `feat/x402-open-listing` → main, 첫 유료 Agent SKU 1개 live.

### 3.1 IN (이번에 한다)

- [ ] `feat/x402-open-listing` → `main` 머지
- [ ] migration `031` (open listing / premium 인프라 — 브랜치 실제 파일명 확인)
- [ ] L1 x402 셀프등록 (probe → auto-publish)
- [ ] L5 `/x402` 허브 + submit UI
- [ ] **`check_endpoint_health` only** — $0.001, Base mainnet, CDP facilitator
- [ ] 스모크 후 `site_settings.allow_x402_registration = true`
- [ ] DoD: `scripts/x402-premium-e2e.mjs` prod 1건, FTG 게이트, MCP discovery 402 없음

### 3.2 OUT (의도적 제외 — 다음 페이즈 후보)

- [ ] `vet_before_pay`, `price_history`, `probe_now`, `bulk_vet`
- [ ] 무료 쿼터 5회/일 (payer 기준)
- [ ] 제공자 x402 결제 (등록 $5, Verification SLA, Featured)
- ~~Bazaar L2 크롤 시드~~ — **shipped (PR-5)**; Phase 2 KPI(승인 Bazaar ≥30)는 밀도 목표이지 크롤러 배선 블로커가 아님
- [ ] 어트리뷰션 Tier A 수동 청구
- [ ] x402 discovery paywall (영구 **Rejected** — 로드맵에도 재검토 안 함)

### 3.3 스프린트 체크리스트 (배포)

| # | 항목 | 확인 |
|---|------|------|
| S1 | `cargo test --features ssr` / clippy 관련 | ☐ |
| S2 | Railway + Vercel 배포, migration 적용 | ☐ |
| S3 | `/x402` SSR 200, submit E2E 1건 | ☐ |
| S4 | MCP `search_tools` / `get_tool_detail` 402 없음 | ☐ |
| S5 | `check_endpoint_health` 402→pay→200 | ☐ |
| S6 | `allow_x402_registration` 켜기 (운영자) | ☐ |
| S7 | CDP·지갑 키 로테이션 (노출 이력 있으면) | ☐ |

---

## 4. 페이즈 로드맵 (체크리스트 — 일정·순서 변동 가능)

### Phase 0 — Merge & stabilize (D0–14)

| Ship | KPI | 상태 |
|------|-----|------|
| main ← open-listing 브랜치 | deploy verify PASS | ☐ 진행 중 |
| x402 payment 모듈 prod | E2E $0.001 1건 | ☐ worktree 완료, main 대기 |
| MCP 회귀 0 | discovery 402 = 0 | ☐ |

### Phase 1 — Open listing (D14–35)

| Ship | KPI | 상태 |
|------|-----|------|
| `allow_x402=true` | 셀프등록 ≥3 | ☐ |
| `/x402` SEO 허브 | x402 카탈로그 툴 ≥10 | ☐ |
| Admin 설정 UI | 등록 스위치 문서화 | ☐ |

**후보 (1b):** Founding 등록 $0 90일 — **정본 가격, 로드맵에서 “언제 켤지”만 조정**

### Phase 2 — Supply density (D35–60)

| Ship | KPI | 상태 |
|------|-----|------|
| Bazaar 크롤 시드 (L2) — **crawler shipped** | 승인된 Bazaar 툴 ≥30 (밀도 KPI) | ☐ crawler ☑ / KPI 진행 중 |
| L3/L4 어트리뷰션 메타 | `referral_events` 증가 | ☐ |
| x402 verify cron 안정화 | 프로브 실패율 모니터 | ☐ |

### Phase 3 — Agent Trust expansion (D60–90)

| Ship | KPI | 상태 |
|------|-----|------|
| `vet_before_pay` | 주간 premium 호출 ≥10 | ☐ |
| `x402_probe_history` + `price_history` | 30일+ 이력 | ☐ |
| 무료 쿼터 5/일 | abuse 모니터 | ☐ |
| M1 프로브 히스토리 UI (선택) | — | ☐ |

### Phase 4 — Provider B2B (후행 — 순서 바뀔 수 있음)

| Ship | KPI | 상태 |
|------|-----|------|
| `PROV-REG` x402 checkout | 유료 등록 1건 | ☐ |
| `PROV-VERIFY-SLA` | 월 구독 1팀 | ☐ |
| `PROV-FEATURED` + Sponsored 라벨 | 슬롯 ≤5 | ☐ |
| Founding Pack / Trust Launch 번들 | — | ☐ |

### Phase 5 — Attribution (Non-P&L, 매출 기대 없음)

| Ship | KPI | 상태 |
|------|-----|------|
| Tier A 서면 합의 템플릿 | Tier A 전환 ≥1 | ☐ |
| 수동 정산 프로세스 | tx 증빙 워크플로 | ☐ |
| 전제: x402≥10, install_guide 월≥100 | — | ☐ |

### 병행 백로그 (우선순위 미정)

- [ ] MCP `compare_tools` **무료** 구현 (A2)
- [ ] `export_toolkit` 무료 (대량 slug 유료는 **미결** — FTG와 충돌 시 무료 유지)
- [ ] Agent Sync Bearer → +5회/일 보너스 (선택)

---

## 5. 제품·가격 참고표 (Launch — **협상·시장에 따라 변경 가능**)

정본과 동일하지만, **로드맵 관점에서는 “현재 가정”**이다. CFO·구현은 정본을 따른다.

### 5.1 에이전트 신뢰 (축 B)

| SKU | Tool | Launch 가격 | 구현 |
|-----|------|-------------|------|
| AGT-HEALTH | `check_endpoint_health` | $0.001/호출 | **이번 스프린트** |
| AGT-VET | `vet_before_pay` | $0.001 | Phase 3 |
| AGT-PRICE-HIST | `price_history` | $0.001 | Phase 3 |
| AGT-PROBE-NOW | `probe_now` | $0.005 | Phase 3+ |
| AGT-BULK-VET | `bulk_vet` | $0.008 | Phase 3+ |

**무료 쿼터 (권장, 미구현):** health/vet/price 각 3/일, 합산 5/일; probe/bulk 0.

### 5.2 제공자 B2B (축 B)

| SKU | Launch | Mature | 비고 |
|-----|--------|--------|------|
| PROV-REG-FOUNDING | $0 (90d, ≤50팀) | — | GTM 칩 |
| PROV-REG | $5/등록 | $10 | x402 셀프등록만 |
| PROV-VERIFY-SLA | $15/월 | $20/월 | 일 1 프로브 |
| PROV-FEATURED | $5/주 | $10/주 | 캐러셀 ≤5슬롯 |

### 5.3 어트리뷰션 (축 A — P&L $0)

| Tier | 모델 | 로드맵 기대 |
|------|------|-------------|
| B | attribution | 이벤트·협상 데이터만 |
| A | split (라벨) | 서면 합의 후 수동 인식 |

---

## 6. 구현 상태 스냅샷 (갱신: 2026-07-04)

| 영역 | main | feat/x402-open-listing (worktree) |
|------|------|-----------------------------------|
| L1 셀프등록 | ☐ | ☑ 구현 |
| L5 `/x402` | ☐ | ☑ 구현 |
| `check_endpoint_health` | ☐ | ☑ E2E prod |
| `x402_payment` / premium | ☐ | ☑ |
| `allow_x402_registration` 설정 | ☑ 필드만 | ☐ submit 게이트 |
| Bazaar L2 crawler (`bazaar.rs`) | ☑ | ☑ PR-5 |
| Provider checkout | ☐ | ☐ |
| `compare_tools` MCP | ☐ PLANNED | — |

> 브랜치 머지 후 이 표를 **직접** 갱신한다. 스냅샷이 오래되면 “확인 필요”라고 답한다.

---

## 7. 90일 KPI (매출 후순위 — 목표치 조정 가능)

| Phase | 1차 KPI | 매출 기대 |
|-------|---------|-----------|
| 0 | deploy + MCP 회귀 0 | — |
| 1 | x402 툴 ≥10, 셀프 ≥3 | — |
| 2 | Bazaar ≥30, referral_events ↑ | — |
| 3 | premium 호출 ≥10/주, E2E 안정 | 90d **$342–$1,140** (어트리뷰션 제외, 참고) |

상세 시나리오: `REVENUE_FORECAST.md`

---

## 8. 명시적 폐기·금지 (로드맵에서도 재오픈 안 함)

정본 §7과 동일 — 창업자가 **명시적으로** 되돌리지 않는 한:

- x402 discovery paywall
- 일반 MCP에서 x402 숨김
- `compare_tools` / discovery API 402
- 제3자 x402 호출 프록시·커스터디
- 2.5%를 P&L 매출로 계상

---

## 9. 창업자 자문 FAQ (짧은 답변 앵커)

| 질문 | 답변 앵커 |
|------|-----------|
| “1센트 헬스가 뭐야?” | 제3자 x402 API **결제 직전** 엔드포인트 alive/프로브 스냅샷. `check_endpoint_health`. |
| “이번에 뭐 나가?” | §3 IN만. vet·제공자 결제는 OUT. Bazaar L2 **크롤러**는 shipped; Phase 2 밀도 KPI는 별도. |
| “등록료 언제?” | Phase 4 후보; Founding $0는 GTM 기간 칩. |
| “2.5% 받아?” | 협상 칩, 장부 $0. Tier A만 수동 인식. |
| “compare 유료?” | **아니오**, 영구 무료 (Locked). |
| “로드맵이랑 정본이 다르면?” | **정본 + AGENTS.md** 우선. 로드맵은 계획일 뿐. |
| “x402 어디에 등록?” | §10.3 — **디렉터리(X1–X4)** vs **Bazaar 판매자(X5–X8)** vs **공급(L1/L2)** 구분 |
| “Bazaar 등록 폼?” | **없음** — CDP settle + discovery 메타. Seller: [CDP Bazaar doc](https://docs.cdp.coinbase.com/x402/bazaar) |

---

## 10. 외부 홍보·등재 체크리스트 (창업자 수동 — 순서·대상 변동 가능)

> **목적:** OnchainAI MCP·플러그인·스킬·x402를 *다른 플랫폼*에서 찾을 수 있게 한다.  
> 상세 절차: `docs/LAUNCH_READINESS_SPEC.md` §3 · 온보딩: `docs/CONNECT.md` · 패키징: `docs/SKILL_PLUGIN_SPEC.md`  
> 등재 후 `CONNECT.md`에 “Listed on …” 링크 추가 권장 (신뢰 앵커).

### 10.1 이미 갖춘 것 (재확인만)

| 항목 | 상태 | 액션 |
|------|------|------|
| MCP 엔드포인트 | ✅ `https://www.onchain-ai.xyz/mcp` | URL 일관성 유지 |
| `/connect` 허브 | ✅ | 클라이언트별 딥링크 점검 |
| Claude Code 플러그인 | ✅ `plugin/onchainai/` | `claude plugin validate` |
| 스킬 (플러그인 동봉) | ✅ `onchainai-crypto-tools` | 단독: `~/.claude/skills/` 복사 |
| `llms.txt` | ✅ `/llms.txt` | 에이전트 발견용 — 내용 갱신 시 재배포 |
| GitHub 레포 | 🔲 private 가능 | 공개 전 §1.3 스캔·키 로테이션 선행 |

**플러그인 설치 명령 (홍보용 복붙):**
```
/plugin marketplace add Coinyak/onchainai
/plugin install onchainai@onchainai
```

### 10.2 MCP·에이전트 디렉터리 (효과 순 — P1)

| # | 채널 | 무엇을 등록 | 상태 | 메모 |
|---|------|-------------|------|------|
| D1 | **공식 MCP Registry** [registry.modelcontextprotocol.io](https://registry.modelcontextprotocol.io) | `server.json` + publish | ☑ | `io.github.Coinyak/onchainai` v0.2.0 — published 2026-07-04 |
| D2 | **Smithery** | 원격 HTTP MCP | ☐ | 스킬 보유 디렉터리 — MCP+skill 함께 노출 |
| D3 | **PulseMCP** / **mcp.so** / **Glama** | 제출 폼 | ☐ | 무료 메타데이터 |
| D4 | **Cursor Directory** | MCP URL | ☐ | `/connect` 딥링크와 동일 URL |
| D5 | **awesome-mcp-servers** | GitHub PR, crypto 섹션 | ☐ | 한 줄 pitch + 링크 |
| D6 | **web3-mcp-hub** registry | `registry.json` PR | ☑ | [rudazy/web3-mcp-hub#1](https://github.com/rudazy/web3-mcp-hub/pull/1) |

### 10.3 x402 전용 등록 (MCP 디렉터리와 **별도**)

x402는 “폼 하나 제출” 채널이 여러 갈래다. **역할별로 나눠 등록**한다.

#### A. 우리가 **디렉터리**로 홍보 (발견·인덱스)

| # | 채널 | 등록 방식 | 상태 | 메모 |
|---|------|-----------|------|------|
| X1 | **생태계 리스트 PR** | GitHub 등 markdown 리스트에 OnchainAI 추가 | ☐ | 카테고리: `discovery` / `index` / `directory` — **결제·커스터디 아님** 명시. `x402.org/ecosystem` 페이지는 404(2026-07) → **GitHub·CDP 문서·커뮤니티** 쪽이 현실적 |
| X2 | **x402 Foundation** | [slack.x402.org](http://slack.x402.org) · [github.com/x402-foundation/x402](https://github.com/x402-foundation/x402) · WG | ☐ | 제품 등록 폼이 아니라 **커뮤니티·스펙** 참여 — “discovery directory” 소개 |
| X3 | **자체 카탈로그 self-list** | [onchain-ai.xyz/submit](https://www.onchain-ai.xyz/submit) 또는 L1 x402 셀프등록 | ☑ | `scripts/seed-onchainai-listing.mjs` prod 적용 (slug `onchainai`) |
| X4 | **`/x402` SEO 허브** | 랜딩·sitemap | ☐ | open-listing 스프린트 IN |

#### B. 우리가 **유료 x402 API 판매자**로 노출 (`check_endpoint_health` 등)

| # | 채널 | 등록 방식 | 상태 | 메모 |
|---|------|-----------|------|------|
| X5 | **CDP Bazaar** | **별도 등록 폼 없음** | ☐ | CDP Facilitator로 **settle 1회 성공** + `paymentPayload.resource` + Bazaar `declareDiscoveryExtension` 메타 → [Bazaar discovery](https://docs.cdp.coinbase.com/x402/bazaar)에 자동 인덱스. 30일 무호출 시 목록에서 빠질 수 있음 |
| X6 | **Base Builder Code** | [dashboard.base.org](https://dashboard.base.org) | ☑ | `bc_ljttbnhv` — `site_settings` + Admin UI (`data-testid=x402-builder-code`) |
| X7 | **CDP Seller Quickstart** | CDP API 키 + Facilitator URL `https://api.cdp.coinbase.com/platform/v2/x402` | ☐ | Rust 경로는 SDK 없음 — 402 응답·verify/settle·Bazaar extension을 **직접** 맞춤 (worktree `x402_payment.rs` 참고) |
| X8 | **Merchant discovery 조회** | `GET …/discovery/merchant?payTo=<우리지갑>` | ☐ | Bazaar 인덱스 후 **자가 확인**용 — API 키 불필요 |

#### C. **타인 x402 도구**를 우리 카탈로그에 넣기 (공급)

| # | 채널 | 등록 방식 | 상태 | 메모 |
|---|------|-----------|------|------|
| X9 | **Bazaar 크롤 시드 (L2)** | CDP discovery API → 운영자 승인 큐 | ☑ crawler | `bazaar.rs` + forced pending; Phase 2 KPI ≥30 승인은 운영자 심사 후 |
| X10 | **제3자 셀프등록 (L1)** | 제공자가 `/x402` submit | ☐ | `allow_x402_registration=true` 후 |

#### x402 등록 체크리스트 (창업자 수동)

```
디렉터리 홍보:
  ☐ X3 자체 카탈로그에 OnchainAI MCP + /x402 허브 등록
  ☐ X1 생태계 GitHub 리스트 PR (discovery/index)
  ☐ X2 Slack/WG에 한 줄 소개

유료 SKU (check_endpoint_health) Bazaar 노출:
  ☐ X7 CDP Facilitator verify/settle prod 동작 확인
  ☐ X5 Bazaar extension 메타 + settle 1건 → discovery/search에 검색
  ☐ X6 dashboard.base.org Builder Code 발급 → 402 응답·정산에 반영
  ☐ X8 merchant?payTo= 로 인덱스 확인

공급 확대 (후행):
  ☐ X10 L1 셀프등록 오픈
  ☑ X9 Bazaar 크롤 시드 (crawler wired — `/admin/tools` 심사로 밀도 확보)
```

**x402 홍보 한 줄 (외부 폼용):**  
*Free discovery for x402 APIs; paid Agent Trust (`check_endpoint_health`) before you pay third parties. OnchainAI never holds funds.*

**흔한 오해:** Bazaar는 “신청서”가 아니라 **CDP로 결제가 한 번 정산되면** 올라간다. 디렉터리(OnchainAI) 홍보와 **판매 엔드포인트**(1¢ health) Bazaar 인덱스는 **작업이 다르다**.

### 10.4 스킬·플러그인 추가 채널

| # | 채널 | 상태 | 메모 |
|---|------|------|------|
| P1 | Claude **커뮤니티** 플러그인 마켓 목록 | ☐ | 자체 마켓플레이스(`Coinyak/onchainai`)는 완료 |
| P2 | **cryptoskill.org** 등 skill 레지스트리 | ☐ | 크롤러 소스로 이미 참조 — *역등록* 검토 |
| P3 | dev.to / X / Farcaster 출범 포스트 | ☐ | GitHub 공개·D1 완료 후가 효율적 |
| P4 | GitHub **About topics** | ☑ | `gh repo edit` 적용 |

### 10.5 권장 순서 (한 번에 다 안 해도 됨)

```
1. GitHub 공개 + topics + 소셜 프리뷰 (LAUNCH §1.3)
2. X3 자체 카탈로그 self-list + /x402 허브
3. D1 MCP Registry (MCP 홍보)
4. X6 Base Builder Code (dashboard.base.org)
5. X5 Bazaar 인덱스 — check_endpoint_health settle 1건 + discovery 검색
6. X1 x402 생태계 GitHub 리스트 PR
7. D2–D4 + D5 (Smithery, mcp.so, Cursor, awesome-mcp-servers)
8. P3 출범 포스트 (discovery 무료 + 1¢ health + Bazaar 각도)
```

### 10.6 등재 시 공통 카피 (초안 — 수정 가능)

| 필드 | 값 |
|------|-----|
| Name | OnchainAI |
| URL | https://www.onchain-ai.xyz |
| MCP | https://www.onchain-ai.xyz/mcp |
| Docs | https://www.onchain-ai.xyz/connect |
| Repo | https://github.com/Coinyak/onchainai |
| One-liner | Crypto tool directory for AI agents — MCP, CLI, SDK, API, x402 discovery; trust + install-risk gating. |

---

## 11. 갱신 로그

| 날짜 | 변경 |
|------|------|
| 2026-07-04 | 초안: 10-agent 합의·open-listing 스프린트·페이즈 체크리스트 |
| 2026-07-04 | §10 외부 홍보·등재 체크리스트 (MCP registry, x402 ecosystem, skill/plugin) |
| 2026-07-04 | §10.3 x402 전용 등록 상세 (Bazaar·Builder Code·self-list·L1/L2) |
| 2026-07-04 | 등재 실행: server.json, prod self-list, web3-mcp-hub#1, awesome-crypto#209, CONNECT §listed |
| 2026-07-05 | §3.2 Bazaar L2 crawler shipped (PR-5); Phase 2 ≥30 KPI는 밀도 목표로 유지 |

**다음 갱신 트리거:** main 머지 완료, Phase 1 KPI 달성/미달, 가격·순서 창업자 결정.

---

### 한 줄

**이 파일은 “지금 뭐 하고, 다음에 뭘 할지” 적어 두는 메모장이다. 규칙이 아니다.**