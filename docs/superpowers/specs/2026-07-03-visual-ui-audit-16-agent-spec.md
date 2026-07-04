# 프로덕션 UI 통합 감사 — 최종 실행 스펙 (Grok 실행 패킷 v3)

> Related: [[../../UI_UX_IMPROVEMENT_SPEC]] | [[2026-06-30-auth-admin-navigation-preview-final-spec]] | [[../../UI_UX_DESIGN]] | [[../../../DESIGN]] | [[../../LAUNCH_READINESS_SPEC]] | [[../../BUILD_DEPLOY_RULES]] | [[../../../AGENTS.md]]
>
> Date: 2026-07-03 (v3.1 — 외부 에이전트 실행 패킷으로 최종화)
> Status: **Final — Grok 실행용.** 두 감사 스펙(시각 16-에이전트 + 인증·Connect·레이아웃)의 병합 정본이며,
> `2026-07-03-production-auth-connect-layout-audit-spec.md`를 대체한다(커밋 금지·삭제 대상).
> 오픈 질문은 §0.3에서 **전부 결정 완료** — 구현자는 질문 없이 진행 가능하다.
> Evidence: 16-zone 소스·curl 감사 + 16 서브에이전트 Playwright 프로덕션 감사 (1280×900 / 375×812).
> 상태 기준선: 브랜치 `claude/nostalgic-dubinsky-661e17` **HEAD `ae8e9aa`** — Phase A(`bfcb662`),
> Phase B/C(`f81c78e`), Phase D(`ae8e9aa`)까지 랜딩된 코드로 2026-07-03 21:20 재대조 완료.
> ⚠️ B/C/D 커밋은 아직 UI 게이트·브라우저 회귀를 거치지 않았을 수 있다 — **슬라이스 0(회귀 검증)이 최우선**.
> 표기: ✅ 완료(코드 확인) · ◐ 부분 · 🔲 미구현 · 🔧 오너 수동 운영 작업(구현 에이전트 범위 아님).

---

## 0. 실행 지시 (Grok 전용 — 먼저 읽기)

### 0.1 환경·브랜치·사전 준비

1. 레포: `/Users/hoyeon/OnchainAI` (메인 체크아웃). 이 스펙과 선행 구현은 브랜치
   `claude/nostalgic-dubinsky-661e17`에 있다.
2. **사전 정리(1회)** — 메인 체크아웃에 같은 경로의 **untracked 구버전 스펙 2개**가 남아 있으면 체크아웃이 충돌한다:
   ```bash
   rm -f docs/superpowers/specs/2026-07-03-production-auth-connect-layout-audit-spec.md
   rm -f docs/superpowers/specs/2026-07-03-visual-ui-audit-16-agent-spec.md   # tracked 병합본으로 대체됨
   ```
3. 작업 브랜치 생성 (해당 브랜치는 다른 워크트리가 점유 중이므로 **분기**만):
   ```bash
   git fetch --all
   git checkout -b grok/ui-audit-p0p1 claude/nostalgic-dubinsky-661e17
   git status --short   # 깨끗하지 않으면 중단하고 오너에게 보고
   ```
4. 프론트 준비: `cd frontend && npm ci`. 개발 서버 `API_PROXY_TARGET=<로컬 API 또는 Railway URL> npm run dev`.
   로컬 Rust API가 없으면 `API_PROXY_TARGET=https://onchainai-production.up.railway.app`로 실데이터 확인 가능(읽기 전용 확인만, 쓰기 액션 금지).
5. 시작 전 상태 검증(스펙 §4가 맞는 코드인지): `frontend/lib/siwx.ts` 존재, `frontend/app/globals.css`에 `.login-modal-overlay` 존재, `style/output.css`에 `.toolbar-type-chip` 존재해야 한다. 없으면 잘못된 브랜치다 — 중단.
6. **prebuild 복사본 동기화**: `ae8e9aa`가 `style/output.css` 원본만 커밋해 `frontend/styles/site-output.css` 복사본이 stale일 수 있다. 첫 `npm run build`(prebuild) 후 복사본에 diff가 생기면 **단독 커밋** `chore(ui): sync prebuilt site-output.css`로 먼저 커밋하고 진행한다.

### 0.2 절대 규칙

1. **범위**: 이 스펙의 P0+P1 슬라이스만(§6). `src/`(Rust)·`migrations/`·`plugin/`·`docs/`(본 스펙 체크박스 제외) **수정 금지**. Rust 무변경이므로 cargo 게이트 불필요.
2. **스타일 파일 규칙**:
   - **신규 CSS는 전부 `frontend/app/globals.css`**에 추가한다(Phase A 선례).
   - `frontend/styles/site-output.css`는 `style/output.css`의 **prebuild 복사본** — 직접 수정 금지.
   - 기존 규칙 수정(D2 등)은 **원본 `style/output.css`**를 고치고 `cd frontend && npm run prebuild`로 복사본을 갱신해 **둘 다 커밋**한다.
3. **`data-testid` 보존**(§9). 제거·개명 시 같은 커밋에서 `scripts/smoke-test-frontend.sh`를 갱신한다.
4. 영어 UI 카피, 오렌지 primary CTA 화면당 1개, 8px radius, lucide 아이콘 (`DESIGN.md`).
5. UI 검증: 슬라이스마다 `cd frontend && npm run lint && npm run build`, 마감 시 `./scripts/ui-change-gate.sh`. 브라우저 확인 없이 "QA 통과" 주장 금지 (`AGENTS.md`).
6. 커밋은 **로컬에서 슬라이스 단위**로, 메시지에 이슈 ID 포함(§6 제안 메시지 사용). **push는 오너 지시가 있을 때만** 하고, push하게 되면 메시지에 `[skip ci]`를 포함한다. CI/CodeRabbit/qodo **직접 트리거 금지**.
7. 배포 스크립트(`deploy-railway.sh`, `vercel-prod-setup.sh`) 실행 금지 — 배포는 오너가 한다.
8. 막히면 추측으로 스코프를 넓히지 말고 해당 슬라이스를 SKIP으로 보고하고 다음 슬라이스 진행.

### 0.3 확정 결정 (구 오픈 질문 — 재질문 금지)

| # | 결정 |
|---|------|
| D-1 | **G2 featured 겹침**: 스키마 변경 없음. `headline`이 **명시적 빈 문자열/공백**이면 overlay 텍스트 블록을 렌더하지 않는다(그라디언트는 유지). 현재 코드는 `headline \|\| tool_name` 폴백이라 항상 텍스트가 그려짐 — 폴백을 "headline이 `null`일 때만 tool_name" + "trim 결과 빈 문자열이면 텍스트 미렌더"로 바꾼다. dot의 `aria-label` 폴백(`headline?.trim() \|\| tool_name`)은 유지. 클린 에셋 교체는 오너 운영 작업(🔧). |
| D-2 | **J1 사이드바 카운트**: API 변경 없음. empty state 카피로 완화 — "N Bridge tools exist in other types." 형식 + Clear filters. scoped count는 후속(범위 밖). |
| D-3 | **H7 체인 tag 중복**: 코드 변경 없음 — 유지(정보 밀도). |
| D-4 | **C1 Safe install 1차 탭**: ✅ `f81c78e`가 변형으로 구현 — `CONNECT_CARD_CLIENTS = ["codex","chatgpt","claude","cursor","vscode"]`(5탭, codex 1차·default), 탭 라벨 "ChatGPT connector". **이 변형을 수용한다**(재작업 금지). 잔여 P2 선택: `/connect` 그리드 카드 label(`mcp-connect.ts` L326)만 "(connector)" 표기 통일. |
| D-5 | **gate 경로 nav CTA(H4 잔여)**: 현상 유지 — `/submit`·`/toolkit`에서 TopNav Sign in과 `GuestSignInPrompt` 병존 허용(둘 다 `/login` 수렴, 폼 중복 아님). 추가 작업 없음. H4 종결. |
| D-6 | **L3 카테고리 라우트**: 단기 보강만 — H1 제목 + breadcrumb. 라우트 통합은 범위 밖. |

### 0.4 Grok 소유 슬라이스 (이 순서 권장 — v3.1에서 재산정)

Phase B/C/D가 `f81c78e`·`ae8e9aa`로 랜딩되어 **G1·D2·H1·H2·C1·C3·L2·D3–D5는 코드 완료**다.
Grok의 일은 ① 그 커밋들의 **회귀 검증**, ② 잔여 이슈 구현이다.

| 순서 | Slice | ID | 우선순위 |
|------|-------|----|---------|
| **0** | **B/C/D 랜딩 회귀 검증(코드 수정 없음)**: `npm run lint && npm run build` → `ui-change-gate.sh` → 375/1280 스크린샷으로 §4의 ✅ 항목 수용 기준 실측 — G1 칩 pill·D2 preview install 노출·H1/H2 브랜드(워드마크 **최소 1회** 노출 확인 — TopNav 제거로 0회가 되면 결함으로 보고)·C1 Codex 1차 탭·C3 More 탭·L2 overflow. 실패 항목은 결함 슬라이스로 승격해 수정 | 검증 | **P0** |
| 1 | G2 featured overlay 로직 — **유일한 P0 코드 잔여** | G2 | **P0** |
| 2 | J2 EmptyState filterLines (+J1 카피) | J1·J2 | P1 |
| 3 | H3 Submit CTA 단일화 | H3 | P1 |
| 4 | I 검색·타입어헤드 | I1–I3 | P1 |
| 5 | K1 배지 구분 | K1 | P1 |
| 6 | L1 체인 `+N` + L4 admin redirect | L1·L4 | P1 |
| 7 | (시간 남으면) `/connect` 그리드 ChatGPT 카드 라벨 "(connector)", G3–G4, J3–J5, K2–K5, M4–M5 | — | P2 |

**범위 밖(하지 말 것)**: A0/A2(🔧 오너), scoped count API, H7, L3 라우트 통합, M1–M3·M6–M7(디자인 토큰 대공사), E1–E3, x402/plugin/Rust 전부.

### 0.5 완료 보고 형식

슬라이스별로: 이슈 ID / 변경 파일 / 실행한 검증 명령과 결과(원문) / 375·1280 스크린샷 경로 / 남긴 리스크.
전체 완료 시 §7 매트릭스 체크 상태 + `git log --oneline` 목록.

---

## 1. 제품 목표

1. **로그인 왕복 성공** — GitHub·지갑·이메일이 `www.onchain-ai.xyz`에서 왕복 (코드 ✅, 운영 확인 🔧).
2. **글자 깨짐·텍스트 합침 없음** — 모바일 툴바 칩, featured 캐러셀, listing actions.
3. **브랜드·CTA 중복 최소화** — 뷰포트당 로고+워드마크 1벌, Sign in/Submit 1곳, 오렌지 primary 1개.
4. **필터·검색 상태 = 결과** — 카운트·empty state·Clear 경로 일치.
5. **신뢰 신호 구분** — Verified/Official 색·아이콘 구분.
6. **Connect vs Safe install 역할 분리** — 허브=OnchainAI MCP, Safe install=해당 툴.
7. **모바일 터치·가독성** — 컨트롤 ≥44px, 잘림·대비 미달 없음.
8. **로딩 신뢰** — skeleton-only flash 금지 (P2).

## 2. 비목표

WCAG 전면 인증 · Leptos 롤백 · `/categories`↔`/tools` 라우트 통합 · x402 결제 UI · 자동 CI/봇 트리거 · Connect 카피 전면 재작성 · Compare/Blueprints 재설계 · 감사 아티팩트 커밋 · **(Grok) §0.4 범위 밖 전부**.

## 3. 아키텍처 전제

| 계층 | 경로 | 비고 |
|------|------|------|
| 프론트 | Vercel — `frontend/` Next.js 16 | 본 스펙의 유일한 코드 표면 |
| 스타일 | `frontend/app/globals.css`(신규 규칙) ← / → `style/output.css`(원본) → prebuild → `frontend/styles/site-output.css`(복사본, 직접 수정 금지) | 셋 다 git tracked |
| API | Railway — `/api/v2`, `/auth`, `/onboarding`, `/mcp` | 변경 없음 |
| OAuth | live `client_id=Ov23liJjqFxXDtFZfmJi`, callback `https://www.onchain-ai.xyz/auth/callback` | 🔧 오너 |
| 디자인 | `DESIGN.md`, `docs/UI_UX_DESIGN.md` | 토큰·불변 규칙 |

---

## 4. 이슈 레지스트리 (상태는 2026-07-03 브랜치 재대조)

### Phase A — 인증 (P0) — ✅ `bfcb662` 완료 (Grok 작업 없음)

| ID | 내용 | 상태 |
|----|------|------|
| A0 | Vercel `API_PROXY_TARGET`·OAuth 앱 정렬 | 🔧 오너 (§8) |
| A1 | 온보딩 form POST `/onboarding/complete\|skip` | ✅ |
| A2 | GitHub 앱 www callback 등록 | 🔧 오너 |
| A3 | Vercel 빌드 시 Railway 기본 프록시 | ✅ |
| A4 | SIWX 지갑 (`lib/siwx.ts`, `wallet-sign-in`) | ✅ |
| A5=H5 | 로그인 모달 fixed overlay + scroll lock | ✅ |
| A6 | `/login?auth=` 오류 배너 (`lib/auth-errors.ts`) | ✅ |
| A7=H4 | 게이트 페이지 Sign in 정리 (`GuestSignInPrompt`) | ✅ (D-5로 종결) |

### Phase F — 인프라·스모크 — ✅ 완료·유지

`smoke-test-api.sh` / `smoke-test-frontend.sh`(wallet assert 포함) / `smoke-test.sh` 유지. Grok은 testid 변경 시에만 갱신.

### Phase G — 글자 깨짐 (P0)

| ID | 증상 | 검증된 원인 | 수정안 | 상태 |
|----|------|-------------|--------|------|
| **G1=D1** | 375px `/tools`에서 `MCPCLIAPISDKSkillx402Verifi…` 연속 문자열 | `.toolbar-*` 클래스 CSS 부재였음 | **✅ `ae8e9aa`** — `style/output.css`에 chip/select 스타일(44px touch, active) 랜딩. 잔여: 슬라이스 0에서 375px 실측 + (선택) `toolbar-type-chip-<id>` testid·스모크 assert 추가 | ✅ 재검증 |
| **G2** | featured BOB 슬라이드에서 overlay headline이 이미지 내 마케팅 카피와 겹침 | `FeaturedCarousel.tsx` L109 `const title = card.headline \|\| card.tool_name` — headline이 비어도 tool_name으로 **항상** overlay 텍스트 렌더; `headline`은 DB에서 nullable(`migrations/009` L7) | **D-1 결정 적용**: headline이 `null`이면 기존대로 tool_name 폴백, **빈/공백 문자열이면 overlay 텍스트 블록 미렌더**(그라디언트 div는 유지, `featured-carousel-overlay`에 modifier class). 접근성: 링크 `aria-label`은 tool_name 유지 | 🔲 |
| G3 | 히어로 placeholder 모바일 잘림(`…DeFi, ch`) | 57자 vs ~279px | 모바일 전용 placeholder ≤40자 (`"Search tools, chains, DeFi…"`) — CSS 미디어로는 불가, 컴포넌트에서 viewport 분기 또는 짧은 문구로 통일 | 🔲 P2 |
| G4 | 상세 `CompareSuggest similar` 붙음 | `.listing-actions-row` CSS 0건 | `globals.css`: `display:flex; gap:12px; flex-wrap:wrap` | 🔲 P2 |

**수용 기준**: 375px 칩 개별 pill·연결 문자열 0건·칩 높이 ≥44px / headline 비운 카드에서 overlay 텍스트 없음(스크린샷) / (P2) placeholder 의미 전달·actions 간격 ≥8px.

### Phase H — 중복 가시성·브랜드 (P1)

| ID | 증상 | 수정안 | 상태 |
|----|------|--------|------|
| **H1=B1** | TopNav 로고+워드마크 + `Sidebar.tsx` `SidebarBrand` 중복 | **✅ `f81c78e`** — Option A 채택: TopNav는 액션만, 브랜드는 SidebarBrand 단일. ⚠️ 슬라이스 0 확인 필수: **sidebar 없는 라우트**(`/login`·`/connect` 등)와 375px에서 워드마크가 **0회가 되지 않는지** — 0회면 결함으로 승격 | ✅ 재검증 |
| **H2=B2** | 모바일 collapsed sidebar 로고+햄버거 2행(~109px) | **✅ `ae8e9aa`/`f81c78e`** — 모바일 61px 단일 행 + `.sidebar-brand-text` 숨김. 슬라이스 0에서 375px 스택 해소 실측 | ✅ 재검증 |
| **H3** | Submit CTA 3중(헤더/프로모/EmptyState) | 헤더 Submit=유일 오렌지 filled. 프로모 카드 CTA→outline/text, `EmptyState` `.empty-state-submit-btn`→tertiary(기존 CSS 토큰 사용) | 🔲 |
| H6 | 프리뷰 열림 시 카드 액션+프리뷰 액션바 중복 | 프리뷰 open 상태에서 해당 카드 `.card-action-btn` 숨김 | 🔲 P2 |
| H7 | 체인 스트립↔카드 tag 중복 | **D-3: 변경 없음** | ✖ 종결 |

**수용 기준**: 1280px `/tools` "OnchainAI" 워드마크 1회 / 375px 로고 마크 1회 / 홈 오렌지 filled 1개 / sidebar 없는 라우트에서 브랜드 1회 존재.

### Phase I — 검색·타입어헤드 (P1) — 파일: `frontend/components/tools/ToolSearchCombobox.tsx`

| ID | 증상 | 검증된 원인 | 수정안(행동 계약) | 상태 |
|----|------|-------------|-------------------|------|
| **I1** | 입력 중 typeahead 목록 미노출(API 200인데) | debounced URL sync(`router.push(...?q=)` 계열 useEffect)가 리렌더로 `isOpen` 리셋 | 입력 중에는 URL 갱신 금지 — hero/toolbar 모두 **submit(Enter)·선택·blur 후에만** `?q=` 반영. 목록은 결과 ≥1이면 `aria-expanded=true`로 유지 | 🔲 |
| **I2** | 드롭다운 안 보이는데 Enter → 첫 매칭 **상세로 직행**(L231 `selectActive`, L195 `router.push(/tools/slug)`) | Enter가 활성 항목 select로 처리 | Enter 계약: 드롭다운 **가시 + 항목 명시 하이라이트** 시에만 상세 이동; 그 외 Enter는 검색 목록(`?q=`) 이동(L211 경로 유지) | 🔲 |
| **I3** | 모바일 검색 오버레이 포커스 실패 | `mobileExpanded` 후 `inputRef.focus()` 미호출 | overlay 마운트 후 `focus({preventScroll:true})`; overlay 수명과 URL sync 분리 | 🔲 |
| I4 | placeholder 불일치 | 의도 분리 | G3과 함께 문구만 정리 | 🔲 P2 |

**수용 기준**: `wallet` 입력 → 목록 ≥1건·`aria-expanded=true` / 드롭다운 닫힘 Enter → `/?q=wallet`(상세 직행 금지) / 375px 포커스=`activeElement`가 input / `/?q=wallet` 가로 스크롤 없음.

### Phase J — 필터·빈 상태 (P1)

| ID | 증상 | 검증된 사실 | 수정안 | 상태 |
|----|------|-------------|--------|------|
| **J1** | Bridge(26)+MCP → 0 tools인데 사이드바 26 유지 | 카운트는 전역(`/api/v2` categories) | **D-2**: empty state 첫 줄에 `No Bridge + MCP tools yet. 26 Bridge tools exist in other types.` 형식 카피(활성 function 카운트 사용 가능할 때) — API 변경 금지 | 🔲 |
| **J2** | Empty state에 Clear filters 버튼 없음 | `EmptyState.tsx`는 `filterLines`·`clearHref`·`.empty-state-clear-btn`(CSS 존재, `style/output.css` L1773–1786)까지 **완비**. `ToolsBrowser.tsx` L397이 `clearHref`만 전달 → `hasFilters=false`로 버튼 미노출. `describeActiveFilters` 유틸은 프론트에 **없음**(신규 작성) | `ToolsBrowser`에서 활성 필터를 문자열 배열로 생성해 `filterLines` 전달. 형식: `Function: Bridge` / `Type: MCP` / `Chain: Base` / `Status: Verified` / `Search: "wallet"`. Clear 버튼에 `data-testid="empty-state-clear-filters"` 추가 | ◐ |
| J3 | Clear가 function 축만 초기화 | | J2의 clearHref가 모든 축 제거 URL인지 확인, 아니면 전체 초기화로 | 🔲 P2 |
| J4 | 0-count 카테고리 클릭 가능 | | `count===0` → `aria-disabled` + 클릭 무시 | 🔲 P2 |
| J5 | 모바일 `N tools` 비가시 | | count를 strip 바깥 행으로 | 🔲 P2 |

**수용 기준**: `/tools?function=bridge&type=mcp`(또는 교집합 0인 아무 조합) empty panel에 필터 요약 + Clear filters 버튼 → 클릭 시 전체 목록 복귀.

### Phase K — 신뢰 배지 (P1/P2)

| ID | 증상 | 검증된 사실 | 수정안 | 상태 |
|----|------|-------------|--------|------|
| **K1** | Verified ≡ Official 동일 스타일 | `style/output.css` L684–685 두 클래스 **완전 동일**(cream+dark) | `.badge-verified`를 green tint(예: `#EDF7ED` bg / `#1E7B34` border·text 계열, 대비 ≥4.5:1) 또는 ✓ 아이콘으로 분리 — **원본 `style/output.css` 수정 + prebuild** | 🔲 |
| K2 | Quick Facts Type/Status 배지 중복 | | 배지 존재 시 facts에서 해당 2필드 제거 | 🔲 P2 |
| K3 | Sort 행에 status chip 혼재 | | 2행 분리 또는 `Filter:` prefix | 🔲 P2 |
| K4 | 북마크 라벨 불일치 | | `Save to Toolkit`/`Remove from Toolkit` 통일 | 🔲 P2 |
| K5 | `GitHub 1001 GitHub stars` | | `GitHub · 1,001 stars` | 🔲 P2 |

### Phase C/L — Connect·Safe install·체인·Admin (P1/P2)

| ID | 증상 | 검증된 사실 | 수정안 | 상태 |
|----|------|-------------|--------|------|
| **C1** | 홈/툴 Safe install 1차 탭 ChatGPT | — | **✅ `f81c78e`** — D-4 변형 구현: `CONNECT_CARD_CLIENTS = ["codex","chatgpt","claude","cursor","vscode"]`(5탭, codex 1차), default `codex`, 탭 라벨 "ChatGPT connector". 잔여(P2 선택): `/connect` 그리드 카드 label(`mcp-connect.ts` L326 "ChatGPT")도 "(connector)" 표기 | ✅ |
| **C3** | Safe install **More** → `/connect` 페이지 이탈 | — | **✅ `f81c78e`** — More가 탭(`install-more-tab` testid), 패널 내 링크만 `/connect`. 슬라이스 0에서 탭 전환 실측 | ✅ 재검증 |
| **L1** | 모바일 `+16` chain pill 뷰포트 밖 | `ChainStrip.tsx`, scroll 1224px vs 311px | trailing fade(우측 그라디언트) + `+N` pill을 스트립 우측 sticky로 | 🔲 |
| **L2=C2** | `/connect` install-cmd overflow | — | **✅ `f81c78e`** — compare 패리티 containment 랜딩(globals.css). 슬라이스 0에서 1280px Claude Code 카드 실측 | ✅ 재검증 |
| L3 | `/categories/bridge` H1 없음 | | **D-6**: H1 + breadcrumb만 추가 | 🔲 P2 |
| **L4** | `/admin/tools` 비인증 무음 redirect | `admin/layout.tsx` L17 `redirect("/")` | `redirect("/login?return_to=" + 요청 경로)`; `/login`이 `return_to` 존중(로그인 후 복귀), 배너 "Admin access required" | 🔲 |

### Phase D — 프리뷰·모바일 — ✅ `ae8e9aa` (슬라이스 0 재검증 대상)

| ID | 증상 | 검증된 사실 | 수정안 | 상태 |
|----|------|-------------|--------|------|
| **D2** | 데스크톱 preview에서 install 단계 숨김 | `.install-steps { display:none }` 규칙이었음 | **✅ `ae8e9aa`** — 숨김 제거, install steps + x402 메타 노출. ⚠️ 원본만 커밋됨 — §0.1-6 복사본 sync 커밋 선행. 슬라이스 0에서 데스크톱 프리뷰 실측 | ✅ 재검증 |
| D3 | 모바일 preview에서 install 아래로 밀림 | | **✅ `ae8e9aa`** — Bottom sheet가 `PreviewPanelContent` 공유 | ✅ 재검증 |
| D4 | 모바일 preview 액션바 없음 | | **✅ `ae8e9aa`** — `PreviewActionBar` 포함 | ✅ 재검증 |
| D5 | sticky 겹침 ≤767px | | **✅ `ae8e9aa`** — toolbar가 61px sidebar bar 아래로 | ✅ 재검증 |

### Phase E (P2 — Grok 범위 밖) / Phase M (P2 — M4·M5만 선택 허용)

| ID | 내용 | 상태 |
|----|------|------|
| E1–E3 | Compare `?tools=`·Blueprints flash·404 분리 | 🔲 범위 밖 |
| **M4** | `#999999` muted 대비 2.85:1 (`style/output.css` 7건) | 🔲 선택 — 일괄 `#767676` 치환 + prebuild |
| **M5** | `:focus-visible` 불일치 — preview/compare엔 적용됨(globals.css), `.card-action-btn`·chain-tile 누락 | 🔲 선택 — 동일 패턴 추가 |
| M1–M3, M6–M7 | H1 스케일/CTA 색/거터/skeleton | 🔲 범위 밖(후속) |

---

## 5. 필수 사용자 동작 (구현 후 이 시나리오가 참이어야 함)

1. **모바일 필터(G1)**: 375px `/tools` — HOT 셀렉트 옆 MCP·CLI·API·SDK·Skill·x402·Verified·Official이 개별 pill, 탭 시 URL 반영·오렌지 active. 연결 문자열 금지.
2. **Featured(G2)**: headline을 비운 카드(카피 내장 이미지)에서 overlay 텍스트가 안 그려지고 이미지 카피만 보인다. dot·키보드 포커스 정상.
3. **검색(I)**: `wallet` 입력 → 목록 노출 → 방향키로 선택+Enter면 상세, 그냥 Enter면 `?q=wallet` 목록. 모바일 포커스 시 키보드 뜸.
4. **빈 결과(J)**: 교집합 0 → 필터 요약 + Clear filters 버튼 → 클릭 한 번에 복귀. (J1 카피: 다른 type에 N개 존재 안내.)
5. **브랜드(H)**: 어느 뷰포트에서도 "OnchainAI" 워드마크 1회.
6. **Safe install(C)**: 첫 탭 Codex CLI; More 탭은 페이지 이탈 없이 터미널 명령+copy; "More clients →"만 `/connect`로.
7. **Admin(L4)**: 게스트가 `/admin/tools` → `/login?return_to=/admin/tools` + 안내 배너, 로그인 후 복귀.

## 6. 슬라이스·커밋 계획 (v3.1 잔여분)

| 순서 | 커밋(제안 메시지) | 파일 | ID |
|------|-------------------|------|----|
| 0 | `chore(ui): sync prebuilt site-output.css` — **첫 빌드 후 diff가 있을 때만** (§0.1-6). 회귀 검증 자체는 무커밋; 결함 발견 시 `fix(ui): regression <ID> from f81c78e/ae8e9aa` | `frontend/styles/site-output.css` | 슬라이스 0 |
| 1 | `fix(ui): featured overlay skips empty headline G2` | `FeaturedCarousel.tsx` | G2 |
| 2 | `fix(ui): empty-state filter summary + clear J1 J2` | `ToolsBrowser.tsx`, (필요시 `EmptyState.tsx` 카피) | J1·J2 |
| 3 | `fix(ui): single orange primary CTA H3` | promo/EmptyState CTA CSS·컴포넌트 | H3 |
| 4 | `fix(ui): typeahead visibility + enter contract I1-I3` | `ToolSearchCombobox.tsx` | I1–I3 |
| 5 | `fix(ui): verified badge distinct K1` | `style/output.css`(+prebuild 복사본 동커밋) | K1 |
| 6 | `fix(ui): chain +N visibility + admin login redirect L1 L4` | `ChainStrip.tsx`/CSS, `admin/layout.tsx`, `/login` return_to | L1·L4 |
| 7 | (P2 선택) `polish(ui): chatgpt connector grid label, muted contrast, focus-visible` | `mcp-connect.ts`, `style/output.css` | 그리드 라벨·M4·M5 |

병렬 금지: `style/output.css`(+복사본)와 `globals.css` 각각 동시 2 writer 금지. G2↔J↔I는 파일 겹침 없어 병렬 가능.

## 7. 검증 매트릭스 (슬라이스마다)

| Gate | 명령 | 비고 |
|------|------|------|
| Lint+Build | `cd frontend && npm run lint && npm run build` | 매 슬라이스 |
| UI 게이트 | `./scripts/ui-change-gate.sh` | 슬라이스 묶음 마감 시 (레포 루트) |
| 스모크 | `./scripts/smoke-test-frontend.sh <base-url>` | testid 변경 슬라이스 |
| 스크린샷 | `node scripts/visual-snapshots.mjs <base-url> --out .playwright-cli/ui-audit-post-fix` | P0 완료 후 1회 + 최종 1회 |
| 수동 | 1280×900 + 375×812 스크린샷 (해당 화면) | 매 슬라이스, 보고에 경로 첨부 |

배포 후(오너): `./scripts/post-deploy-verify.sh https://www.onchain-ai.xyz`.
감사 원본 재현 스크립트(`.audit-agent*.mjs`)는 메인 체크아웃 워크스페이스에만 있음 — 있으면 재실행, 없으면 visual-snapshots로 대체.

## 8. 🔧 오너 수동 체크리스트 (Grok 실행 금지 — 보고서에 재출력만)

```bash
curl -sI 'https://www.onchain-ai.xyz/auth/github' | grep -i location   # 307 + www redirect_uri
./scripts/smoke-test-api.sh https://onchainai-production.up.railway.app
./scripts/smoke-test-frontend.sh https://www.onchain-ai.xyz
```
GitHub OAuth 앱: callback `https://www.onchain-ai.xyz/auth/callback`, client_id=Railway `GITHUB_CLIENT_ID`.
Vercel: `API_PROXY_TARGET`=Railway URL 명시. Featured: BOB 카드 headline 비우기(또는 클린 에셋 업로드) — G2 배포 후.

## 9. ID 매핑 + `data-testid`

**Canonical ID ↔ 감사 원문 별칭** (중복 구현 방지):

| Canonical | 별칭 | 영역 |
|-----------|------|------|
| G1 | D1 | 모바일 toolbar CSS |
| H1·H2 | B1·B2 | 브랜드 중복 |
| H4·H5 | A7·A5 | 로그인 UI (둘 다 ✅) |
| L2 | C2 | Connect overflow |
| 나머지 (G2–G4, H3·H6, I*, J*, K*, C1·C3, L1·L3·L4, D2–D5, M*) | — | 단일 출처 |

**`data-testid` 계약**:

| testid | 조치 |
|--------|------|
| `top-nav-sign-in`, `sidebar-brand`, `github-sign-in`, `wallet-sign-in`, `guest-sign-in`, `toolkit-sign-in` | 유지 — H1에서 브랜드 숨김/제거 시 `smoke-test-frontend.sh` 같은 커밋 갱신 |
| `connect-page`, `connect-client-*`, `connect-plugin-card`, `toolbar-search-bar`, `home-search-bar` | 유지 |
| `install-more-clients-link` | C3에서 `install-more-tab`로 대체 + 스모크 갱신 |
| 신규 추가 | `toolbar-type-chip-<id>` (G1), `empty-state-clear-filters` (J2) |

## 10. 완료 정의 (DoD — v3.1)

- **P0 (출시 전 필수)**: 슬라이스 0 회귀 검증 전 항목 PASS(실패분 수정 포함) · G2 코드 완료 · §7 게이트 통과. 🔧 A0/A2는 오너 확인 대기 상태로 보고.
- **P1 (본 패킷 완료 조건)**: J1–J2 · H3 · I1–I3 · K1 · L1·L4 — 각 수용 기준 + 스크린샷 증거.
- **P2**: 선택(그리드 ChatGPT 라벨, M4·M5, G3–G4, J3–J5, K2–K5) — 미착수 시 "후속" 표기만.
- ✅ 재검증 항목(G1·D2·H1·H2·C1·C3·L2·D3–D5)은 슬라이스 0 증거(스크린샷/게이트 로그)로 닫는다.
- 모든 커밋에서 `data-testid` 계약(§9) 위반 0건, 스모크 그린.
- 전사 로드맵 연결: [[../../LAUNCH_READINESS_SPEC]] §6 항목 12.

## 11. 감사 출처 인덱스 (참고 — 아티팩트는 메인 체크아웃 전용, 비커밋)

| Agent | 담당 표면 | 핵심 발견 → ID | Agent | 담당 표면 | 핵심 발견 → ID |
|-------|-----------|----------------|-------|-----------|----------------|
| #1 | 홈 데스크톱 히어로·캐러셀 | G2, H1 | #9 | 툴 프리뷰·상세 | G4, K2 |
| #2 | 홈 모바일 | G2, G3, H2 | #10 | 검색·타입어헤드 | I1–I3 |
| #3 | `/tools` 데스크톱 카드 | K1, D2 | #11 | MCP 프로모 카드 | H3 |
| #4 | `/tools` 모바일 | **G1 (P0)** | #12 | 카드 액션·비교 | H6, K4 |
| #5 | 체인 스트립 | L1, H7 | #13 | 접근성 기본 | M4, K1 |
| #6 | 사이드바 필터 | J3, J4 | #14 | 크로스페이지 일관성 | M1–M3 |
| #7 | `/login`·`/submit` | A5–A7(✅) | #15 | 카테고리·Connect·Admin | L2–L4 |
| #8 | 빈 상태·필터 조합 | J1, J2 | #16 | 성능·로딩 신호 | M6, M7 |

아티팩트: `.playwright-cli/ui-audit-prod-2026-07-03/`, `.audit-agent{4..16}-output/`, `.audit-screenshots*/`, `/tmp/onchain-sidebar-*.png`. 재현 스크립트: `.audit-agent4-tools-mobile.mjs`, `.audit-agent5-chain-strip.mjs`, `.audit-agent7-login.mjs`, `.audit-agent10-search.mjs`, `.audit-agent12-tool-card-actions.mjs`, `.audit-agent15-category-connect-admin.mjs`, `.audit-agent16-performance.mjs`, `scripts/ui-audit-agent1.mjs` (없으면 `visual-snapshots.mjs`로 대체 — §7).
