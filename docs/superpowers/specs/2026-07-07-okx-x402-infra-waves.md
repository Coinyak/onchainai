# OKX AI 등재 + x402 자사 수익화 인프라 Wave 스펙

> Related: [[2026-07-07-product-a-verified-api]] | [[2026-07-07-s-group-strategy-memo]] | [[../../X402_OPEN_LISTING_SPEC]] | [[../../X402_REFERRAL_SPEC]] | [[../../CONNECT]] | [[../../OPERATOR_GUIDE]] | [[../../SECURITY]] | [[2026-07-04-free-tier-guardian-spec]] | [[2026-07-03-x402-activation-spec]] | [[../../../AGENTS.md]]
>
> Date: 2026-07-07
> Status: Final spec — **오너 입력 대기(§6 OKX 월렛/신원·W7 facilitator 호환)** — Wave 1·2 착수 가능. 본 문서는 인프라 Wave(0~3) + OKX A2MCP 등재만 다룬다. Product A(§9→별도)·S-group(§10→별도)은 Wave 4+ 로드맵 문서로 분리.
> Scope: ① OKX AI Agent Marketplace에 OnchainAI 자사 x402 서비스 등재(A2MCP/pay-per-call) ② 내부 x402 카탈로그·셀프등록·L4 자동내리기·어트리뷰션 정합화 ③ Free Tier Guardian(OD-FTG) 코드 정합(회귀 방지) ④ K2 전환 훅(free→paid 워크플로). **결제 실행·커스터디·facilitator 프록시·타인 자금 이동 범위 밖.**
> Evidence: OKX AI Agent Marketplace User Agreement(2026-06-18, okx.ai/help/okx-ai-agent-marketplace-user-agreement) + `src/server/mcp_x402.rs` + `src/server/x402_payment.rs` + `docs/X402_OPEN_LISTING_SPEC.md` §K2 + `2026-07-04-free-tier-guardian-spec`(OD-FTG) 대조

**본 문서는 구현 코드를 포함하지 않는다.** 목표 동작·오너 입력·수용 기준·검증·금지 사항만 정의한다.

---

## 0. 세션 요약

| # | 갭 | 스펙 ID | 심각도 |
|---|-----|---------|--------|
| 1 | x402 카탈로그 61건 중 공개 2건, 59건 pending | W1 | P0 |
| 2 | `allow_x402_registration=false` — 셀프등록 막힘 | W2 | P1 |
| 3 | X4 웹 어트리뷰션 미기록·Admin 토글 UI 없음 | W3 | P0 |
| 4 | L4 probe history 미사용·14일 auto-quarantine 미구현·SKIP_CRAWLER 시 no-op | W4 | P0(선행) |
| 5 | ~~`compare_tools` 코드가 OD-FTG 위반으로 프리미엄 게이트에 잔류~~ → **W5 완료(2026-07-07)**: `PREMIUM_MCP_TOOLS`에서 제거·단위테스트 전환·`spec-verify.sh` 코드 가드(FTG-D2) 추가·Admin 토글 문구 정정 | W5 | P0 ✅ |
| 6 | OKX AI 등재 미실행 — 외부 유통 채널 부재 | W6 | P1 |
| 7 | OKX x402 facilitator ↔ CDP Facilitator 호환 미확정 — **SPOF, Plan B 필수(§3.4)** | W7 | 블로커(등재 전) |
| 8 | K2 전환 훅 부재 — discovery 무료는 맞지만 "에이전트가 언제 check_endpoint_health를 켜야 하는지" 워크플로 약함 | W8 | P1 |

---

## 0.5 discovery 무료 원칙의 경제적 근거 (정책이 아닌 전략)

x402 = "API 키/신용카드 대신 지갑으로 호출당 결제"라는 메커니즘 이해는 맞으나, "에이전트 호출에 어차피 LLM 크레딧이 드니 discovery도 x402로 유료화해도 된다"는 결론은 성립하지 않는다:

- **수취인 상이·비용은 추가**: LLM 크레딧은 LLM 제공자에게, x402는 OnchainAI에게. discovery에 x402를 걸면 호출자는 LLM 크레딧 + OnchainAI 톨을 **이중 지불**. 대체가 아닌 가산.
- **퍼널 파괴**: search가 top-of-funnel. 발견 단계 유료화는 paid 프리미엄(trust data) 도달 전 이탈.
- **대체재 풍부**: 검색/발견은 GitHub·공식 MCP Registry·Smithery·타 디렉터리가 무료로 제공. abundant 재화에 톨을 걸면 라우팅 이탈.
- **scarce value는 trust data**: 30일 liveness·probe uptime·검증 가격은 복제 어려움. K2(`check_endpoint_health`)가 과금 지점.
- **agent 툴 선택은 비용 민감**: LLM 크레딧은 어떤 MCP를 부르든 드는 sunk cost. 우리가 톨을 얹으면 무료 대비 순증 비용.

→ 과금은 scarciest 지점(K2 trust data)에만. discovery는 무료로 둬야 전환 경로가 산다. **단, "무료"가 "K2로 가는 길을 안 보여준다"는 뜻은 아님 — §3.6 전환 훅으로 보완.**

---

## 1. 제품 목표

1. **OKX AI(A2MCP/x402)** 에 OnchainAI 자사 K2 서비스(`check_endpoint_health`)를 pay-per-call로 등재하여 **우리 지갑으로 직접 수취**. discovery는 OKX 마켓에도 무료 MCP로 별도 노출.
2. x402 카탈로그를 선별 승인으로 채운다(공개 2 → 점진적 확대). `relevance_status=accepted` + `referral_enabled=false` 유지, 20건 단계적.
3. L4(probe history 기반 14일 연속 실패 auto-quarantine)가 프로덕션에서 동작한다(`SKIP_CRAWLER`와 무관하게).
4. 어트리뷰션 이벤트가 웹 경로에도 기록되고, Admin에서 site/per-tool referral·x402 토글을 조작한다.
5. **OD-FTG 정합**: `compare_tools`를 프리미엄 게이트에서 제거, 코드를 영구 무료 정책과 일치시킨다. ✅ W5 완료.
6. K2(`check_endpoint_health`)가 prod에서 CDP Facilitator 정산까지 end-to-end 검증된다.
7. **W7 비호환 Plan B**(§3.4) — OKX facilitator가 CDP와 안 맞을 때 폐기가 아니라 ① discovery-only OKX 등재 ② Registry/Smithery/Bazaar 다채널 ③ referral(M1) 우선 수익 레버로 우회. W7을 싱글 포인트 오브 페일러로 두지 않는다.
8. **K2 전환 훅(§3.6)** — 무료 discovery 응답이 stale 신호·스킵 비용을 노출해 on-demand fresh probe(K2)로의 전환 사유를 응답마다 제시.
9. OnchainAI는 **결제·지갑 연결·자금 이동·커스터디·facilitator 프록시를 하지 않음** — 자사 서비스 직접 수취 + attribution metadata만.

---

## 2. 비목표 / 금지

- **discovery에 x402/402/로그인/지갑 게이트 적용 금지** — `search_tools`·`get_tool_detail`·`get_install_guide`·`list_categories`·`get_dashboard_snapshot`·`compare_tools`·웹 `/compare`·`/x402` 허브는 OD-FTG §2 영구 무료. 근거(§0.5).
- `compare_tools`를 K2 유료로 분류 — **OD-FTG(FTG-1) 폐기 확정**. W5 완료.
- OKX A2A(escrow) — per-call K2/A 자사 수취에는 A2MCP/x402 직결이 더 단순하므로 **A2A 비적용**. 단, advisory·납품형 S-group은 A2A가 자연스러운 채널 → **A2A는 S-group 범위에 한해 in-scope**(별도 문서 `2026-07-07-s-group-strategy-memo`).
- 커스터디, third-party payment proxy/gateway, 타인 자금 이동, 문서화되지 않은 `referrer`/`split` 필드.
- `payment_verified` 등을 `PUBLIC_TOOL_WHERE`/RLS visibility gate에 추가.
- 자동 CI/리뷰 봇 트리거.

---

## 3. OKX AI 등재 설계 (W6)

### 3.1 팩트 (OKX AI Agent Marketplace User Agreement, 2026-06-18)

- **비커스토디얼**: "OKX does not take custody of any asset… is not a party to any transaction."
- **결제 레일**: escrow(A2A, 온체인 ownerless 컨트랙트) + non-escrow **HTTP 402(x402)** pay-per-call(A2MCP).
- **역할**: Client User / **AI Agent Provider**(OnchainAI) / Arbitrator. 디지털 월렛(self-custody) 연결 필수.
- 사이트: okx.ai — `/agents`(에이전트 마켓), `/tasks`(태스크 마켓), "JOIN OKX.AI (BETA)". 등록 폼은 월렛 로그인 뒤 SPA(curl 비접근).

### 3.2 등재 대상 (K2 only, A2MCP)

| MCP tool | OKX 등재 | 과금 | 비고 |
|----------|----------|------|------|
| `search_tools`·`get_tool_detail`·`get_install_guide`·`list_categories`·`get_dashboard_snapshot`·`compare_tools` | 무료 MCP로 **별도 노출(트래픽용)** | 무료 | OD-FTG §2 |
| `check_endpoint_health` | **A2MCP/x402 유료 서비스로 등재** | x402 $0.001/호출 | "결제 직전 보험" — 제3자 x402 도구 호출 전 liveness 보증 |
| `export_toolkit` | 보류 | 미정 | OD-FTG가 명시 유료/무료 결정 안 함 — 별도 창업자 결정 필요 |

### 3.3 등재 메타데이터 (제출용, `docs/listings/directory-forms.md`에 추가)

| 필드 | 값 |
|------|-----|
| Provider | OnchainAI |
| Service | OnchainAI Trust Probe (x402 endpoint liveness) |
| Endpoint | `https://www.onchain-ai.xyz/mcp` (POST JSON-RPC, streamable-http) |
| 결제 모델 | A2MCP / x402 (HTTP 402) |
| 과금 도구 | `check_endpoint_health` ($0.001/call, USDC) |
| 수취 지갑 | `0x2af05c1661da38a2919dc27b4c8b71cb91c30017` (Base USDC) — K2 `X402_PAY_TO_ADDRESS`·`default_referral_payout_address`와 **동일**(오너 확정 2026-07-07) |
| 무료 도구(동일 엔드포인트) | search_tools, get_tool_detail, compare_tools 등 6종 |
| Repo | https://github.com/Coinyak/onchainai |
| Registry 교차등재 | io.github.Coinyak/onchainai v0.2.0 (server.json) |

### 3.4 블로커: facilitator 호환 (W7) + Plan B + FacilitatorProvider trait

우리 x402 구현은 **CDP Facilitator**(`https://api.cdp.coinbase.com/platform/v2/x402`, `src/server/x402_payment.rs::facilitator_client`)로 PaymentRequirements 검증. OKX AI의 x402가 동일 facilitator인지 자체인지 미확정(OKX dev docs 월렛 로그인 뒤, curl 비접근). W7을 SPOF로 두지 않는다:

- **Plan B(비호환 시, §1 목표7에 명시)**:
  1. **discovery-only OKX 등재** — K2 과금 없이 무료 MCP만 OKX 마켓에 노출(트래픽·신뢰 앵커).
  2. **다채널 분산** — Registry/Smithery/mcp.so/Bazaar에 K2는 그대로, OKX는 discovery만.
  3. **referral(M1) 우선 수익 레버** — OKX 유통이 K2 수취를 못 지원해도 어트리뷰션 수익은 OKX 트래픽으로 증가.
- **FacilitatorProvider trait(아키텍처 해소 경로)** — CDP/OKX verify를 URL 주입형 멀티클라이언트 trait으로 분리. **프록시/커스터디 아님** — 단지 facilitator verify 엔드포인트를 설정 주입으로 전환. 비호환 판명 시 OKX 전용 client 구현체만 추가(결제 라우팅·자금 이동 없음). 별도 구현 스펙 필요.
- **해소 조건**: OKX 로그인 후 dev docs 확인(오너 수동·§6) 또는 브라우저 세션으로 agent 팩트체크. **오너 30분 OKX dev docs 팩트체크를 Wave 1부터 병렬(등재 자체는 Wave 3)**.

### 3.5 문서 연동

- `docs/CONNECT.md` "Listed on (external discovery)" 표에 OKX AI 행 추가 — status "Operator register (W7 해소 후; 비호환 시 discovery-only)".
- `docs/listings/directory-forms.md`에 OKX AI 섹션 추가(§3.3 메타데이터).

### 3.6 K2 전환 훅 — free → paid 워크플로 (W8, 핵심 추가)

"discovery 무료"가 "K2로 가는 길을 숨긴다"가 되면 전환 경로가 죽는다. 무료 응답이 **K2 전환 사유를 응답마다 제시**하도록 보완:

- **Stale Trust Badge(무료, OD-FTG 합치)** — `get_tool_detail`/`compare_tools`에 L4 기반 "마지막 프로브 시각·LIVE 여부(24h stale 임계)" 메타만 노출. on-demand fresh probe = K2(`check_endpoint_health`) 전환 사유를 응답에 포함. 게이트 아님(무료 응답에 정보 추가만).
- **스킵 비용 노출(B, §0.5의 자연 귀결)** — 무료 응답에 "죽은 엔드포인트 평균 손실액 vs 프로브 $0.001" 명시. 프로브 *안* 돌리는 비용을 보여줘 전환 압력. 에이전트 비용 민감성과 정합.
- **Probe Receipt(K2 성공 시 반환)** — `check_endpoint_health` 성공 응답에 timestamp·endpoint hash·요금 일치(advertised vs actual) attestation 포함. "결제 직전 보험" 가치를 합성 가능하게.
- **Probe Receipt → K1 어트리뷰션 앵커(A)** — 에이전트가 3rd-party 호출 *직전* 우리에게 유료 프로브를 받았다는 Receipt = "우리가 중개했다"는 강한 어트리뷰션 증거. K1(구조적으로 soft)의 징수 근거를 강화하는 유일한 레버. K2→K1 플라이휠. **v1 정산은 여전히 계약+delist threat(자금 이동 금지 유지); Receipt는 증빙 강화일 뿐 자동 징수 아님.**
- **OKX 3-step SOP 카피** — `search_tools` → stale badge 확인 → 유료 `check_endpoint_health` → 3rd-party 호출. OKX A2MCP 리스팅을 "제품"이 아니라 **에이전트 운영 레시피**로 포지셔닝("Dead endpoint에 $10 쓰기 전 $0.001").

---

## 4. 내부 인프라 정합화 (Wave)

### Wave 0 — 즉시 단독(W5, 완료 2026-07-07)

W5는 W1/W3/W4/W2 어디에도 기술적으로 의존하지 않는 독립 수정이며, **지금 관리자 토글 체크박스 하나로 OD-FTG(compare_tools 영구무료)를 깰 수 있는 live 상태**였으므로 최우선 단독 처리. W3(Admin UI)보다 먼저 배포해 노출 창을 줄임.

- `PREMIUM_MCP_TOOLS`에서 `compare_tools` 제거 → `&["export_toolkit"]`(`src/server/mcp_x402.rs`).
- 단위테스트 전환: `is_premium_mcp_tool("compare_tools")==false` + 전용 가드 `compare_tools_is_free_forever_odftg`(코드 레벨 회귀 방지).
- `spec-verify.sh` FTG-D2: Rust 상수 grep 가드 추가(기존 FTG-D는 문서 문구만 검사).
- `src/server/mcp.rs` 주석·`frontend/app/admin/settings/page.tsx` 토글 문구 정정("Charge for export_toolkit … compare_tools is Free Forever").
- 검증: `cargo test --features ssr --lib premium_tool_names_are_stable` PASS · `compare_tools_is_free_forever_odftg` PASS · `spec-verify.sh` FTG-D2 PASS.

### Wave 1 — 개발(병렬 착수 가능)

| 역할 | 작업 | 스펙 ID |
|------|------|---------|
| Backend | L4: `x402_probe_history` 일일 크론 적재 + 14일 연속 실패 → auto-quarantine. `SKIP_CRAWLER=1`과 무관하게 verify 크론 동작(별도 스케줄/플래그) | W4 |
| Backend | Stale Trust Badge: `get_tool_detail`/`compare_tools` 응답에 L4 기반 last-probe/LIVE(24h) 메타 추가(무료, 게이트 아님) + 스킵 비용 노출 | W8 |
| Backend | Probe Receipt: `check_endpoint_health` 성공 응답에 timestamp·endpoint hash·요금 일치 attestation + K1 어트리뷰션 앵커 필드 | W8 |
| Frontend | X1 Admin site x402 토글 UI + X3 per-tool referral 폼 | W3 |
| Frontend | X4: 웹 install guide 경로 `referral_events` 기록(MCP와 동일) + Stale Badge UI | W3/W8 |
| Security | X4 rate limit + bulk approve 가드, `referral_enabled=true`만 billable | W3/W1 |
| Ops | Bazaar pending 59건 중 양질 선별 승인(루브릭 §6, 5건→48h→15건 canary) | W1 |
| Ops(오너) | OKX dev docs 30분 팩트체크(W7) — Wave 1부터 병렬, 등재는 Wave 3 | W7 |

### Wave 2 — 프로덕션 스위치(직렬, 순서 강제)

1. **W1** Bazaar 선별 승인 → 카탈로그 채움(2 → ~20+).
2. **W4** L4 배포 + `SKIP_CRAWLER` 정리 → 죽은 엔드포인트 자동 정리 활성.
3. **W8** Stale Badge + Probe Receipt 배포(무료 응답 강화, K2 전환 훅 활성).
4. **W3** Admin UI + 어트리뷰션 하드닝 배포.
5. **W2** `allow_x402_registration=true` 전환(W4 먼지 후에만 안전).
6. _(W5는 Wave 0에서 완료.)_

### Wave 3 — K2 prod 검증 + OKX 등재

1. **K2** `check_endpoint_health` prod end-to-end(CDP Facilitator 정산) 검증 + Probe Receipt 실측.
2. **W7** OKX facilitator 호환 확정(또는 Plan B 전환 결정).
3. **W6** OKX AI 등재(§3) — K2 정산 + W7 해소 후. 비호환 시 discovery-only 등재(Plan B-1).

> **순서 불변**: W2 스위치는 W4(L4) 머지 후. W6 등재는 K2 prod 정산 + W7 해소 후. discovery 무료 정책은 모든 Wave에서 회귀 금지. W8(전환 훅)은 W4 이후(프로브 history 의존).

---

## 5. 수용 기준

- [ ] `compare_tools` 호출이 402를 반환하지 않는다(OD-FTG 정합). 단위테스트 + prod smoke. ✅ W5.
- [ ] `check_endpoint_health`만 x402 402를 반환하고, 유료 응답 후 CDP 정산이 성공한다(prod).
- [ ] L4: 14일 연속 probe 실패 도구가 `status=quarantined`로 전환되는 로직 + 테스트.
- [ ] `SKIP_CRAWLER=1` 환경에서도 x402 verify/L4 크론이 동작한다(별도 스케줄 확인).
- [ ] Bazaar 선별 승인(루브릭 12+/16) 후 공개 x402 도구가 `/x402` 필터에 노출된다.
- [ ] 웹 install guide에서 `referral_enabled=true` 도구에 `referral_events` 행이 생성된다.
- [ ] Admin UI에서 site x402 토글·per-tool referral을 변경할 수 있다.
- [ ] **W8**: `get_tool_detail`/`compare_tools` 응답에 last-probe 시각·LIVE(24h)·스킵 비용 메타가 포함된다(무료, 게이트 아님).
- [ ] **W8**: `check_endpoint_health` 성공 응답에 Probe Receipt(timestamp·hash·요금일치) 포함.
- [ ] OKX AI 등재 메타데이터가 `docs/CONNECT.md`·`docs/listings/directory-forms.md`에 추가됨(status: W7 해소 또는 discovery-only).
- [ ] discovery 6툴 + 웹 `/compare`·`/x402`에 402/로그인/지갑 게이트가 없다(회귀 가드 FTG-D2).

---

## 6. 오너 입력 대기

1. **OKX 수취 지갑 주소** — ✅ `0x2af05c1661da38a2919dc27b4c8b71cb91c30017` (K2 `X402_PAY_TO_ADDRESS`·레퍼럴 payout과 동일; OKX Provider 등록 폼에도 동일 주소 사용).
2. **OKX 가입·월렛 연결·Provider 등록·서비스 리스트** — 월렛/신원/서명 수반 수동 웹 플로우. agent 대리 불가.
3. **`export_toolkit` 유료/무료 결정(OD-FTG-4 제안)** — ≤10 slug = 무료, 대량/webhook = 유료. OKX 등재 전 결정.
4. **OKX dev docs 접근(W7)** — 30분 팩트체크. 브라우저 세션 제공 또는 오너 직접 확인. Wave 1부터 병렬.
5. **Bazaar 선별 승인 루브릭(16점 중 12+)**: 402 핸드셰이크(필수 4) + 가격 일치 ±10%(필수 3) + stars/npm + registry 교차등재 + install_risk — 1차 5건 → 48h 관찰 → 15건 canary. `OPERATOR_GUIDE`에 반영 권장.
6. **수취 지갑 실측** — ✅ `0x2af05…0017` = prod `X402_PAY_TO_ADDRESS` + `site_settings.default_referral_payout_address` (2026-07-07 반영).

---

## 7. 리스크

1. **W2를 W4 없이 켜면** 죽은 x402 엔드포인트가 공개 카탈로그에 누적 → 신뢰 하락. (순서 강제로 완화)
2. **W1 무분별 bulk approve** — 402 핸드셰이크 통과 ≠ 안전/정품. (루브릭 12+/16 + 단계적 canary)
3. **W7 비호환** — Plan B(discovery-only / 다채널 / M1 referral)로 우회. FacilitatorProvider trait로 멀티클라이언트화(프록시 아님).
4. **compare_tools 게이트 회귀** — FTG-D2 코드 가드 + OD-FTG §2 PR 거부 룰. README/AGENTS/llms.txt 프리미엄 문구 금지 grep(FTG-D3) 추가 권장.
5. **OKX 약관 변경** — 본 스펙은 2026-06-18 약관 기준. 등재 전 재확인.
6. **W8 과광고 리스크** — Stale Badge/스킵 비용이 "광고"로 느껴지면 무료 응답 신뢰 훼손. 정보성 메타 한정, 강제 CTA 금지.

---

## 8. 검증

- `cargo test --features ssr` — `mcp_x402` 프리미엄 분류(compare_tools=false) + L4 quarantine + Stale Badge/Probe Receipt 단위테스트.
- `cargo clippy --features ssr -- -W clippy::all` / `cargo fmt --check`.
- `./scripts/spec-verify.sh` — FTG-D2(PREMIUM_MCP_TOOLS 코드 가드) PASS.
- prod smoke: discovery 6툴 402 미반환 + `check_endpoint_health` 402 반환/정산 + Probe Receipt 필드 존재.
- `./scripts/agent-harness-check.sh` + split-deploy smoke에 x402·Stale Badge 케이스 추가.
- OKX 등재 후: OKX 마켓에서 OnchainAI 서비스 발견 + 무료 도구 호출 + `check_endpoint_health` x402 호출 정상. 비호환 시 discovery-only 검증.
