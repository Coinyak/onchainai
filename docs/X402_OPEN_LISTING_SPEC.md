# x402 오픈 리스팅 + 수익화 — 구현 핸드오프

다른 AI/개발자에게 전달하는 명세. 코드는 없음. 무엇을·왜·어디에 만들지만 기술.
[[X402_REFERRAL_SPEC]]의 후속이며 [[LAUNCH_READINESS_SPEC]] §4(x402 검증 잡, PR30 체인에서 구현 완료)를 전제로 얹는다 — 배관(017/018/028, verify, claim)은 그대로 두고 **공급·등록·수익 제품**만 추가.

---

## 0. 결정 요약 (2026-07-04, 오너 확정)

1. **x402 등록은 오픈 셀프서브.** 사람 심사 없음 — 업계 표준(Bazaar=정산 시 자동 등재, 402 Index=제출 시 프로브)과 동일하게 "사람 심사 제로 + 프로토콜 자동 검사".
2. **등록 = 수수료 동의.** 셀프 등록 약관에 "OnchainAI 어트리뷰션 결제분의 N% referral" 포함. 크롤링 도구는 동의가 없으므로 referral 없음(공급·트래픽용) — 클레임 시 동의로 전환.
3. **Verified는 게이트가 아니라 사다리.** 일반 크립토 유저가 오를 수 있는 자동 단계: Live(프로브 통과) → Price-match(가격 일치) → Claimed/Domain-verified(선택). 017 원칙("검증 플래그는 노출 게이트 아님") 유지.
4. **수익 3상품**: ① 등록 약관 referral(계약 정산→split 자동화), ② Featured 입점료·유료 검증(OnchainAI 자신이 x402 셀러), ③ 프리미엄 MCP(발견 무료 / 신뢰 데이터 유료).
5. **커스터디 금지 유지.** payTo 바꿔치기·결제 프록시·게이트웨이 금지. 어트리뷰션은 문서화된 경로만 — Base Builder Codes(ERC-8021, 2026-06 x402 공식 지원)가 그 경로.

## 0.6 동시 작업 조율 (2026-07-04 갱신 — Grok 67f4c21 푸시 반영)

- **Grok 완료분** (`origin/grok/ui-audit-p0p1` 67f4c21, PR 리뷰 대기): Agent Sync P0–P2(migration **030** `agent_tokens`·device flow·`bookmarks.source`·`agent_sync_log`, MCP **Bearer 인증** + `save_to_toolkit`/`save_stack_to_blueprint`/`link_status`, `/connect#agent-sync`, plugin 0.2.0), Blueprint v2(migration **029** edges), dashboard admin 가드, dev 워크플로 Next.js 전환(cargo-leptos 제거). Rust 테스트 427 passed.
- **진행 순서(오너 확정)**: PR 리뷰 → 머지 → Railway+Vercel 배포 → `post-deploy-verify.sh`. **본 스펙 L 페이즈는 그 머지 후 main에서 브랜치** — migration 번호(031)·`api_v2/mod.rs` 라우터 충돌 회피.
- **x402 활성화 스펙**([[2026-07-03-x402-activation-spec]], Grok 워크트리에 의도적 미커밋): 문서 상태 라벨은 "오너 패킷 대기"지만 **2026-07-04 프로덕션 실측 결과 §4 필수값 대부분 반영 완료** — site_settings에 bps=250·payout 주소(0x9626…)·builder code(bc_…) 저장됨, Base.dev 도메인 인증 meta 라이브(28d38c2), X2(site fallback)는 33177d3으로 구현됨. **실제 잔여**: ① X4 웹 어트리뷰션·X1/X3 Admin UI = 구현 작업(referral_events 0행인 이유), ② 고지 문구 EN/KO 확정(패킷 5·6), ③ `allow_x402_registration` 현재 **false** → L1 배포와 동시에 true 전환(런칭 스위치), ④ 파일럿 표(패킷 7)는 본 스펙의 Bazaar 시드+셀프 등록이 대체 — 불필요.
- **정본 라벨은 `PRODUCT_ENHANCEMENT_SPEC.md` §K** (K1 어트리뷰션 / K2 프리미엄 MCP / K3 유료 노출·검증 / K4 가드레일). 본 문서 M2=K3, M3=K2 — 새 라벨이 아니라 그 항목의 상세 설계. §K의 2026-06-29 수익화 보류는 **2026-07-04 오너 결정으로 해제**(§K에 해제 주석 반영) — 본 스펙이 실행 스펙.
- **`allow_x402_registration` 통합**: 이미 `site_settings`에 존재(활성화 스펙 X8 지적 "저장만, 미강제"). **L1이 이 갭을 메운다** — false면 x402 타입 제출 차단. 중복 구현 금지.

## 0.5 전제 실측 (2026-07-04)

- 라이브 공개 도구 214개 중 **x402 = 0개**. pricing 전부 free. 사이드바에 x402 필터(Type/Pricing)가 노출돼 있어 클릭 시 빈 결과 — L5에서 처리.
- 이미 있는 배관 (재사용, 재구현 금지):
  - 마이그레이션 017/018: `tools.referral_*`, `x402_pay_to_address`, `x402_builder_code`, 검증 플래그 3종, `site_settings.default_referral_bps/default_referral_payout_address/x402_builder_code`, `referral_events`(attribution_session).
  - **x402 검증 스택 전체가 main에 머지 완료(be49680 → #30/#31, 2026-07-04)**: `src/server/x402_verify.rs`(`probe_x402_endpoint`, SSRF 가드·5s 타임아웃·64KB 캡), `migrations/028_x402_verification.sql`(`tools.x402_endpoint`/`x402_last_checked_at`/`x402_check_failures`), 일일 크론(`X402_VERIFY_CRON`, 세마포어 4), `POST /api/v2/admin/tools/{id}/x402-verify` + admin Re-verify 버튼. 연속 3회 실패 시 `x402_endpoint_verified` 강등 규칙 포함. **이 스펙에서 프로브를 재설계하지 말 것 — 재사용만.**
  - 제출 플로우: `POST /api/v2/submit`(`src/server/api_v2/submissions.rs`) → auth + `UserRateLimitAction::SubmitTool` + relevance scan + slug 중복 차단 → `tool_submissions(status='pending', payload JSONB)`.
  - 크롤러: `src/crawler/sources/mod.rs`의 `SourceCrawler` trait + `default_crawlers()` 등록.
  - 설치 가이드: `src/public_install_guide.rs::x402_notice_for_tool` — UI/MCP 공용.
  - 클레임: `request_tool_claim`(007) + `reports_claims.rs`.
  - Base 도메인 검증 meta(28d38c2), install guide referral 기본값(33177d3).

---

## Phase L1 — 셀프 등록: URL 프로브 → 자동 게시 (P0)

**선행: 없음(x402_verify는 main에 있음). 착수 시점은 Agent Sync PR 머지 후 main 브랜치에서(§0.6).**

### 제출 입력 확장 (`SubmitToolInput`, `src/server/functions/submissions_workbench/submission_intake.rs`)
- `x402_endpoint_url: Option<String>` 추가 — `tool_type='x402'`면 필수. `validate_optional_https_url` 재사용(https 강제).
- 약관 동의: `listing_terms_version: Option<String>` — x402 제출 시 필수(§M1 약관 버전 문자열).
- payload는 JSONB라 `tool_submissions` 스키마 변경 불요. 동의 감사는 migration 031(§DB).

### 프로브 → 자동 채움 → 자동 게시
1. 제출 시 `probe_x402_endpoint`(x402_verify.rs) 실행(비동기 job 가능): 유효한 402 + PaymentRequirements 파싱 → `x402_price`·자산·체인·`x402_pay_to_address`·설명 자동 채움. **제출자 입력보다 프로브 결과 우선.** 게시 시 URL을 `tools.x402_endpoint`(028)에 저장 — 일일 크론이 자동으로 이어받는다.
2. 프로브 통과 시 **x402 타입에 한해** 운영자 큐를 건너뛴다: `approval_status='approved'`, `relevance_status='accepted'`(x402 엔드포인트는 정의상 크립토 결제 도구), trust tier=community, 라벨 "Community-listed · auto-probed".
   - `PUBLIC_TOOL_WHERE`/RLS는 **변경 없음** — 게이트가 아니라 승인 데이터가 자동으로 채워지는 것.
3. 프로브 실패 시 pending 유지 + 제출자에게 실패 사유 표시("endpoint did not return a valid 402").

### 런칭 스위치
- 프로덕션 `site_settings.allow_x402_registration`은 현재 **false**(2026-07-04 실측). L1 배포 시 운영자가 true로 전환 — 이게 오픈 리스팅의 온/오프 스위치이며, L1은 이 플래그를 제출 경로에서 강제한다(§0.6, 활성화 스펙 X8 갭 해소).

### 어뷰징 레일 (심사가 아니라 스팸 방지)
- GitHub 로그인 필수 유지(클레임·referral 연속성). 익명 제출 없음.
- 기존 레이트리밋 + URL 정규화 dedupe(deduper 패턴) + 도메인 블록리스트(admin 관리).
- 사후 모더레이션: 기존 quarantine + 신고. **정기 라이브니스 재검사(§L4) 연속 실패 시 자동 내리기.**

## Phase L2 — Bazaar 크롤링 시드 (P0, L1과 병행 가능)

- 새 소스 `src/crawler/sources/bazaar.rs` — `SourceCrawler` 구현, `default_crawlers()` 등록.
- 소스: CDP Bazaar discovery `GET https://api.cdp.coinbase.com/platform/v2/x402/discovery/resources` (페이지네이션; limit/offset). 응답의 payment requirements·스키마·메타를 normalizer로 매핑, `pricing='x402'`.
- **크롤링 도구는 `referral_enabled=false` 고정**(동의 없음). 상세 페이지에 "이 도구의 주인이신가요? 클레임" CTA(§L5) → 클레임 승인 시 약관 동의 → referral 전환(§M1). 도구 주인용 안내는 LAUNCH_READINESS_SPEC이 예고한 `docs/TOOL_OWNERS.md`(P1)와 합쳐 작성.
- 크롤링 도구도 프로브를 거쳐 Live/Price-match 사다리 라벨을 받는다.
- 인제스트는 기존 운영자 승인 큐를 따른다(자동 게시는 셀프 등록 한정 — 크롤링 노이즈에 대한 기존 방어 유지). 초기 시드만 운영자가 일괄 승인.

## Phase L3 — 어트리뷰션 계량 (P0)

수수료의 청구 근거. "OnchainAI 경유 사용"을 측정한다.

- **Builder Code(ERC-8021, buyer-side)**: install guide가 뱉는 x402 클라이언트 설정/스니펫에 `site_settings.x402_builder_code`를 포함 — 그 설정으로 결제하는 모든 트랜잭션에 코드가 붙어 온체인 집계 가능. 33177d3의 기본값 배선을 ERC-8021 실측 스펙(docs.base.org builder-codes)에 맞춰 검증·보정. **문서화된 필드만 사용, referrer류 필드 발명 금지.**
- **로컬 어트리뷰션**: `referral_events`에 `view`/`install_guide`/`click_out` 기록(레이트리밋 필수). 이미 스키마 존재 — insert 경로가 없으면 `get_install_guide`·클릭아웃에 추가.
- **인증 어트리뷰션(Agent Sync 연동, §A1)**: MCP 호출이 Bearer(agent token, migration 030)로 인증된 경우 `attribution_session`에 익명 해시 대신 **토큰 기반 안정 식별자(해시)** 기록 — 익명 fallback 유지. 인증된 "발견→설치 가이드→툴킷 저장" 체인은 M1 정산·협상 근거 중 최상급 증거.
- **admin 대조 뷰**: 도구별 attributed 이벤트 수 + (수동 입력) 온체인 집계 → 월별 청구 근거. v1은 조회 화면이면 충분.
- 한계 명시(UI 카피에도): 에이전트가 우리 설정을 쓰지 않으면 어트리뷰션이 붙지 않는다 → 원클릭 설치 UX가 곧 매출.

## Phase L4 — 재검사 잡 확장 (P1) — 잡 자체는 이미 있음(028 크론)

028 크론이 플래그 갱신(Live/Price-match 사다리)을 이미 수행한다. 이 페이즈가 **추가**하는 것 두 가지뿐:
- **이력 적재**: 프로브 결과를 `x402_probe_history`(§DB)에 insert — 프리미엄 MCP(§M3)의 원료. 기존 tracing 로깅은 유지.
- **자동 내리기 정책**: 028은 연속 3회 실패 시 뱃지 강등만 한다. 여기에 연속 실패 14일(기본값) → 기존 quarantine 재사용으로 delist 추가(오픈 결정 #3). 복구 프로브 성공 시 quarantine 해제 경로 포함.

## Phase A — Agent Sync × x402 시너지 (2026-07-04 추가, Grok 67f4c21 인프라 재사용)

Agent Sync가 만든 것(agent_tokens·Bearer MCP·`agent_sync_log`·`bookmarks.source`·Blueprint edges)은 x402 수익화의 부족한 조각들을 정확히 채운다. 전부 기존 인프라 재사용 — 신규 배관 없음.

| ID | 항목 | 내용 | 우선순위 |
|---|---|---|---|
| A1 | 인증 어트리뷰션 | §L3 — `attribution_session`에 agent token 식별자(해시). "누가 경유했나"가 익명 추정에서 계정 단위 사실로 승격 | P0 (L3에 포함) |
| A2 | 전환 증거 승격 | `save_to_toolkit`/`agent_sync_log`를 M1 대조 뷰의 전환 지표로 — "에이전트 N개 계정이 이 도구를 툴킷에 저장" = 뷰 카운트보다 강한 정산·협상 근거 | P1 (M1과 함께) |
| A3 | 에이전트 네이티브 등록 | 새 MCP 도구 `submit_x402_endpoint`(Bearer 필수) — L1과 동일한 프로브→자동 게시→약관 동의 경로를 MCP로. x402 셀러는 곧 에이전트 유저이므로 웹 폼보다 이 채널이 본류가 될 수 있음. 동일 레이트리밋·`allow_x402_registration` 체크 | P2 (L1 웹 플로우 검증 후) |
| A4 | 프리미엄 과금 식별 | M3 유료 MCP 도구의 과금 식별을 agent token 계정 미터링으로 — x402 호출당 결제(무계정)와 병행 옵션. Bearer 인증이 이미 있어 추가 인증 배관 불요 | P2 (M3와 함께) |
| A5 | Blueprint × x402 | 블루프린트 노드에 x402 가격 메타·고지 표시 + "예상 실행 비용/run"(노드별 `x402_price` 합, 클라이언트 계산). 블루프린트 공유 = x402 도구 발견 채널 → 어트리뷰션 source에 `blueprint` 추가 | P2 |

주의: A1/A3/A4는 Grok 소유 레인(`src/server/mcp/`, `mcp/auth.rs`)을 건드림 — **Agent Sync PR 머지 후 착수**, one-writer 규칙(MULTI_AGENT_COORDINATION §2) 준수.

## Phase M1 — 등록 약관 referral: v1 계약 정산 (P1)

- **약관**: 등록 페이지에 명시 — "OnchainAI 어트리뷰션이 확인된 결제 볼륨의 N%(기본 250bps)를 referral로 정산. 미정산 시 delist·Featured 제외." 투명 고지(수수료 받는다는 사실)는 유저 페이지에도.
- 동의 저장: migration 031 `listing_agreements`(감사 추적) + `tools.referral_enabled=true, referral_model='attribution', referral_bps=합의값`.
- **v1 정산은 계약(수동)**: 월별 admin 대조 뷰 → 청구(청구 자체를 x402 결제로 받을 수 있음, §M2 인프라) → 미납 시 delist. 자금 이동 코드 없음.
- **v2 (별도 사이클)**: x402 V2 dynamic payTo 라우팅 기반 — 도구 측에 "OnchainAI 어트리뷰션 호출만 payTo를 split 컨트랙트로" 미들웨어 스니펫 제공. 온체인 split이라 커스터디 없음. **구현 전 V2 스펙 실측 필수.**

## Phase M2 — OnchainAI가 x402 셀러가 된다: Featured 입점료 + 유료 검증 (P1)

- 판매 대상(전부 우리 자신의 서비스 — 남의 자금 아님): Featured 카루셀 슬롯(기간제), 유료 검증 티어(정기 프로브 SLA + 뱃지), M1 청구 수납.
- 구현: Axum에 x402 리소스 서버 미들웨어(402 응답 + facilitator 검증). **Rust SDK/미들웨어 성숙도 실측 조사 선행** — 미성숙 시 CDP facilitator API 직접 호출로 검증만 위임.
- 보너스: CDP facilitator 사용 시 OnchainAI 자체가 Bazaar에 자동 등재 → 유입.
- **거버넌스 게이트**: AGENTS.md 하드룰 "x402 is attribution/trust metadata only"는 M2와 충돌(우리가 결제를 받게 됨). **M2 착수 전 오너가 해당 하드룰 문구를 개정해야 한다** — "자신의 서비스 판매는 허용, 타인 자금 커스터디·프록시·게이트웨이는 계속 금지"로. 개정 전 M2 코드 작성 금지.

## Phase M3 — 프리미엄 MCP (P2, 데이터 축적 후)

기준 한 줄: **발견은 무료, 신뢰 데이터는 유료.** 기존 무료 도구 5종(search/detail/install guide/categories/dashboard)은 영구 무료 — 트래픽 엔진.

| 신규 MCP 도구 | 내용 | 과금 근거 | 가격 초기값 |
|---|---|---|---|
| `check_endpoint_health` | 라이브 여부·30일 업타임·최종 프로브 시각 | 결제 직전 보험 | $0.001/호출 |
| `price_history` | 가격 변경 이력·광고가=청구가 검증 | 러그/바가지 방지 | $0.001 |
| `x402_trends` | 주간 신규·상승·카테고리 통계 | 빌더 분석 | $0.01 |

- 선행: L4 프로브 이력 누적(최소 30일) + M2 미들웨어. MCP 무료/유료 도구 분리는 402 응답으로 — 유료 도구 호출 시 402, 결제 후 응답. 과금 식별은 x402 호출당 결제 또는 agent token 계정 미터링(§A4) 병행.

## Phase L5 — 공개 UI (P0~P1, L1·L2와 함께)

- **`/x402` 허브 페이지**(frontend `app/x402/page.tsx`): x402 5초 설명 + Live 도구 그리드 + "List your endpoint"(URL 한 칸 → 프로브 미리보기 → 게시) + MCP 연결 CTA. SEO 타깃 "x402 tools directory".
- **submit 폼**(`frontend/app/submit/page.tsx`): `type=x402` 선택 시 조건부 필드(endpoint URL 필수 + 약관 동의 체크박스). 프로브 진행 스피너 → 자동 채움 미리보기 카드 → 게시.
- **빈 필터 함정 수리(즉시)**: 사이드바 x402 Type/Pricing 카운트 0일 때 빈 상태에 "첫 x402 도구를 등록해 보세요 → /x402" CTA.
- **TopNav 정책**: 공급 10개+ 확보 전 TopNav 변경 금지. 이후 x402를 **텍스트 링크**로(GitHub 옆) — Submit primary 버튼은 유일성 유지. 홈 PromoCards에 3번째 카드 "List your x402 endpoint".
- 도구 카드/상세: Live·Price-match·Claimed 사다리 뱃지, 크롤링 도구엔 클레임 CTA, referral 활성 도구엔 투명 고지 1줄.

---

## DB — migration 031_x402_open_listing.sql (029=blueprint_edges·030=agent_sync가 이미 선점, 67f4c21)

- `tool_submissions`: `terms_version TEXT`, `terms_accepted_at TIMESTAMPTZ` (x402 셀프 등록 감사).
- 새 테이블 `x402_probe_history`(id, tool_id FK, probed_at, status, http_status, advertised_price, actual_price, latency_ms) — L4 적재, M3 원료. RLS: public read 불요(서버 전용), 운영자만.
- 새 테이블 `listing_agreements`(id, tool_id FK, user_id FK, terms_version, referral_bps, model, accepted_at, revoked_at NULL) — M1 감사 추적.
- 프로브 상태 컬럼은 **추가하지 않는다** — 028의 `x402_last_checked_at`/`x402_check_failures` + 검증 플래그로 충분.
- RLS 동기화 + `sqlx prepare`.

## 하지 말 것 (기존 하드룰 유지·재확인)

- payTo 바꿔치기, 결제 프록시/게이트웨이, 유저·제공자 자금 보관 일체 금지.
- 문서화되지 않은 `referrer`/`split` 결제 필드 발명 금지 — 어트리뷰션은 ERC-8021 Builder Codes와 `x402_builder_code` 메타 경로만.
- `PUBLIC_TOOL_WHERE`/RLS에 검증 플래그를 노출 게이트로 추가 금지(017 원칙).
- M2 착수 전 AGENTS.md 하드룰 개정 없이 결제 수신 코드 작성 금지.

## 검증 (DoD)

- `cargo test --features ssr`(프로브 mock 402 유닛 테스트 포함: 성공/실패/SSRF 차단/가격 불일치), clippy/fmt, 마이그레이션 후 `sqlx prepare`.
- 셀프 등록 e2e: 유효 402 mock → 자동 게시 확인, 무효 URL → pending+사유. 레이트리밋·중복 차단 테스트.
- 크롤링 도구 `referral_enabled=false` 불변 테스트. 어트리뷰션 insert 레이트리밋 테스트.
- UI는 `dev-watch.sh`로 반복, `ui-change-gate.sh`로 마감. 스모크: /x402 허브·submit x402 플로우 데스크톱 1280+모바일 375 스크린샷.
- MCP: 무료 5종 회귀 없음(`search_tools` 등 기존 계약 유지).

## 오픈 결정 (기본값 채택 후 결정 로그 남기고 진행)

| # | 결정 | 기본값 |
|---|---|---|
| 1 | referral 기본 bps | 250 (2.5%) — `site_settings.default_referral_bps` |
| 2 | 자동 게시 대상 | 셀프 등록 x402만 (크롤링·타 타입 제외) |
| 3 | 데드 엔드포인트 자동 내리기 | 연속 실패 14일 → 기존 quarantine 재사용 |
| 4 | 프로브 타임아웃/재시도 | x402_verify 기존 값 유지 |
| 5 | 약관 문구 | v1은 "합의 라벨" 수준 명시 + 법률 검토 전 과장 금지 |
| 6 | M3 가격 | 표 초기값, M2 후 운영 조정 |
| 7 | 고지 문구 (2026-07-04 채택) | EN: "OnchainAI may receive referral fees from tools discovered through this directory. We never process payments or hold funds — attribution metadata only." / KO: "OnchainAI는 이 디렉토리를 통해 발견된 도구로부터 레퍼럴 수수료를 받을 수 있습니다. 결제 처리나 자금 보관은 하지 않으며, 어트리뷰션 메타데이터만 기록합니다." |

### 한 줄 요약
등록은 열고(프로브가 심사) 크롤링으로 시드하되, 수수료 동의는 등록 약관에서 받는다. 어트리뷰션은 Builder Codes로 온체인 증빙, v1 정산은 계약, 유료 상품(Featured·검증·프리미엄 MCP)은 OnchainAI 자신이 x402 셀러가 되어 판다. 커스터디는 계속 금지.
