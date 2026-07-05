# 프로덕션 워크스루 QA 감사 — 실행 스펙 (2026-07-05)

> Related: [[2026-07-03-visual-ui-audit-16-agent-spec]] | [[2026-06-30-auth-admin-navigation-preview-final-spec]] | [[2026-07-04-blueprint-v2-ux-agent-export-spec]] | [[2026-07-03-x402-activation-spec]] | [[../../X402_OPEN_LISTING_SPEC]] | [[../../OPERATOR_GUIDE]] | [[../../UI_UX_DESIGN]] | [[../../../DESIGN]] | [[../../../AGENTS.md]]
>
> Date: 2026-07-05
> Status: **In progress — `qa/prod-walkthrough-fixes`.** 프로덕션 `www.onchain-ai.xyz`를 실제 계정(지갑 + GitHub operator)으로 수동 워크스루하며 발견한 25개 이슈의 정본 레지스트리.
> Evidence: claude-in-chrome 실브라우저 조작(1494×812) + 프로덕션 API `curl`/`fetch` 프로브 + 소스 코드 대조. 상태 기준선: 브랜치 `main` HEAD `e5bdd10`.
> 표기: ✅ 완료 · ◐ 부분 · 🔲 미구현(전부 신규 발견) · 🔧 오너 수동 작업(브랜드 에셋 소싱·정책 결정 등 구현 에이전트 범위 밖).
> ⚠️ 모든 파일:라인은 `main@e5bdd10` 기준 실측. 구현 전 해당 라인이 이동했는지 재대조할 것.

---

## 0. 실행 지시 (구현자 먼저 읽기)

### 0.1 환경·브랜치

1. 레포: `/Users/hoyeon/OnchainAI` (메인 체크아웃). 기준선 `main@e5bdd10`.
2. 작업 브랜치 분기:
   ```bash
   git status --short   # 깨끗하지 않으면 중단하고 오너에게 보고
   git checkout -b qa/prod-walkthrough-fixes main
   ```
3. 프론트: `cd frontend && npm ci`. 개발 서버는 `./scripts/dev-watch.sh`(Next.js + API 동시). 로컬 Rust API 없으면 `API_PROXY_TARGET=https://onchainai-production.up.railway.app`로 읽기 전용 확인만(쓰기 액션 금지).
4. Rust 표면 변경 슬라이스는 `cargo check --features ssr` → 마감 시 `cargo test --features ssr` + `cargo clippy --features ssr -- -W clippy::all` + `cargo fmt --check`.

### 0.2 절대 규칙

1. **스타일 파일 규칙**(선례: 16-에이전트 스펙 §0.2):
   - 신규/수정 CSS는 원칙적으로 `frontend/app/globals.css`.
   - `frontend/styles/site-output.css`는 `style/output.css`의 **prebuild 복사본** — 직접 수정 금지. 원본 수정 후 `cd frontend && npm run prebuild`로 복사본 갱신, **둘 다 커밋**.
2. **`data-testid` 보존.** 제거·개명 시 같은 커밋에서 `scripts/smoke-test-frontend.sh` 갱신.
3. UI 카피 영어, 오렌지 primary CTA 화면당 1개, 8px radius, lucide 아이콘 (`DESIGN.md`).
4. UI/auth/routing 슬라이스: `./scripts/dev-watch.sh`로 반복, 마감 `./scripts/ui-change-gate.sh`. 브라우저 확인 없이 "QA 통과" 주장 금지.
5. 스키마 변경 시 마이그레이션 + `sqlx prepare`.
6. 커밋은 로컬 슬라이스 단위(§6 제안 메시지). **push는 오너 지시가 있을 때만**, push 시 `[skip ci]` 포함. CI/CodeRabbit/qodo 직접 트리거 금지.
7. 배포 스크립트(`deploy-railway.sh` 등) 실행 금지 — 배포는 오너.
8. 막히면 추측으로 스코프 넓히지 말고 SKIP 보고 후 다음 슬라이스.

### 0.3 확정 결정 (2026-07-05 — 재질문 금지)

| # | 결정 | 관련 이슈 |
|---|------|-----------|
| Q-1 | **placeholder 제거** — Google 미구성 안내 블록 삭제 | AU4 |
| Q-2 | **프론트 제거** — 이메일 폼 삭제; 백엔드 `/auth/email` 라우트는 후속(범위 밖) | AU5 |
| Q-3 | **"Connect Wallet"** 라벨; `data-testid="wallet-sign-in"` 유지 | AU6 |
| Q-4 | **LG2/LG4 좌표 수정**는 구현; **LG1/LG3 실로고**는 오너 에셋 공급 대기(🔧) | LG* |
| Q-5 | **UX-only** — `GET /api/v2/settings.allow_x402_registration`으로 폼 진입 시 비활성 안내; DB 플래그 enable은 오너 | XP1 |

### 0.4 슬라이스 순서 (권장)

| 순서 | Slice | 이슈 | 우선순위 | 표면 |
|------|-------|------|---------|------|
| 1 | 인증 치명 결함 | AU1·AU2 | **P0** | Rust + FE |
| 2 | 운영자 편집 동선 | OP2·OP3·OP1 | **P0/P1** | FE |
| 3 | 에이전트 연결 버튼 | AS1 | **P0** | FE(CSS) |
| 4 | 블루프린트 엣지 선택 | BP1 | **P0** | FE |
| 5 | 검색 품질 | SR1·SR2·SR3 | **P1** | Rust + FE |
| 6 | 로고/브랜드 | LG1·LG2·LG4·LG5 | **P1** | FE + 에셋 |
| 7 | 툴 상세 패리티 | TD1·TD2 | **P1** | FE |
| 8 | x402 제출 UX | XP1·XP2·XP3 | **P1/P2** | Rust + FE |
| 9 | 로그인 카피·요청 변경 | AU3·AU4·AU5·AU6 | **P1/P2** | FE (Q-1~3 확정 후) |
| 10 | P2 잔여 | LG3·BP2 | **P2** | 에셋 + FE |

### 0.5 완료 보고 형식

슬라이스별: 이슈 ID / 변경 파일 / 실행한 검증 명령·결과(원문) / 스크린샷 경로 / 남긴 리스크. 전체 완료 시 §7 매트릭스 + `git log --oneline`.

---

## 1. 목표

1. **로그인 왕복이 사용자를 가두지 않는다** — 계정 전환·온보딩 이탈·재로그인이 무한 루프나 오류 페이지로 끝나지 않는다.
2. **운영자가 앱을 떠나지 않고 편집한다** — 카드에서 누른 편집/승인 버튼이 실제 편집 가능한 화면으로 이어진다.
3. **연결·제출 CTA가 보이고 작동한다** — Connect 버튼·x402 제출·에이전트 링크가 눌리는 대로 동작한다.
4. **검색이 타이핑에 반응한다** — 접두어 매칭 + 이름 우선 랭킹 + 드롭다운이 다른 UI 뒤로 숨지 않음.
5. **로고가 실제 브랜드를 나타낸다** — 제네릭 옥토캣·빈 화면·틀린 아이콘 없음.
6. **신뢰·위험 신호가 사람에게 보인다** — install risk 이유, 심사 컨텍스트가 UI에 노출.

## 2. 비목표

Google OAuth 신규 통합(Q-1 결정 전) · 블루프린트 캔버스 재설계 · x402 결제 정산 로직 · WCAG 전면 인증 · 브랜드 에셋 대량 재제작(오너 소싱 대상) · Compare/Dashboard 재설계 · 자동 CI/봇 트리거.

## 3. 아키텍처 전제 / 표면

| 계층 | 경로 | 본 스펙 관련 |
|------|------|-------------|
| 프론트 | Vercel — `frontend/` Next.js | 대부분 이슈의 코드 표면 |
| 스타일 | `frontend/app/globals.css`(신규) / `style/output.css`→prebuild→`frontend/styles/site-output.css`(복사본) | AS1·LG2 |
| API | Railway — `src/` Axum `/api/v2`·`/auth`·`/onboarding` | AU1·SR1·SR2·XP1·XP2 |
| 에셋 | `frontend/public/chains/*.svg` | LG3·LG4·LG5 |
| 로고 파이프 | `logo_url`(GitHub org avatar 유래) | LG1 |

---

## 4. 이슈 레지스트리

### 4.1 인증 · 온보딩

| ID | 증상 | 검증된 원인 | 수정안 | 우선 | 상태 |
|----|------|-------------|--------|------|------|
| **AU1** | 로그인 화면 "Sign out of GitHub" 클릭 → GitHub "What? Your browser did something unexpected" **오류 페이지**로 떨어짐. 다른 GitHub 계정 전환 경로가 **항상 실패**(간헐 아님) | `src/auth/routes.rs:335` `github_switch`가 `Redirect::temporary(github_logout_url())` → `github_logout_url()`(`routes.rs:305-307`)는 리터럴 `"https://github.com/logout"`. GitHub `/logout`은 **CSRF 보호 POST 전용**이라 authenticity_token 없는 GET/redirect는 무조건 거부. `github_switch_redirects_to_github_logout` 테스트(`routes.rs:469`)는 Location 헤더만 assert해 이 실패를 못 잡음 | github.com/logout 직접 리다이렉트 폐기. 대안 (a) 로컬 세션만 삭제 + 사용자가 **직접 클릭**할 GitHub 로그아웃 링크 안내(리다이렉트 체인 아님), 또는 (b) 다음 `/auth/github` 호출에 GitHub OAuth `prompt`/재인증 파라미터로 계정 재선택 강제 → `/logout` 자체를 안 침 | **P0** | 🔲 |
| **AU2** | 지갑 로그인 후 "Welcome to OnchainAI" 온보딩이 **매 로그인마다 재노출**되는 계정 존재 | `post_auth_redirect_path`(`src/auth/session_ssr.rs:637`)는 `onboarding_completed_at IS NULL`이면 `/onboarding/profile`로 보냄. 온보딩 화면의 "Skip for now"는 `/onboarding/skip`(완료 처리)이지만 **"Back to home" 링크는 `<Link href="/">`**(`frontend/app/onboarding/profile/page.tsx:52-55`)라 아무것도 POST하지 않고 이탈 → `onboarding_completed_at` 영원히 null → 재로그인마다 온보딩 반복 | "Back to home"을 (a) `/onboarding/skip` POST(자동 닉네임 + 완료 처리) 후 홈 이동으로 바꾸거나, (b) 링크 제거하고 Skip으로 단일화 | **P1** | 🔲 |
| **AU3** | "Use a different GitHub account? **Sign out of GitHub**, then return here…" 안내가 로그인 버튼 밑에 **상시 노출** — 최초 방문·미로그인 사용자에게도 보여 혼란 | `frontend/components/auth/LoginForm.tsx:150-162`. `signedOut` 상태와 무관하게 항상 렌더. (기능 자체는 AU1로 깨져 있음 — AU1 수정과 연동) | AU1 수정과 연동해 재작성. 최소한 `signedOut=1`(로그아웃 직후) 또는 재로그인 실패 시에만 노출하도록 조건부화, 또는 카피를 실제 동작하는 흐름으로 교체 | **P1** | 🔲 |
| **AU4** | "Google sign-in is not configured on this deployment yet" 안내만 표시(실제 Google 로그인 없음) | `LoginForm.tsx:163-180` — `getAuthProviders()`가 `providers.google=false` 반환 시 placeholder 렌더 | **Q-1 결정 후**: (a) Google OAuth 실제 통합 후 버튼 노출, 또는 (b) placeholder 안내 제거 | P2 | 🔧/🔲 |
| **AU5** | 이메일(매직링크) 로그인 폼 존재 — 오너 제거 요청 | `LoginForm.tsx:181-202`(`sendMagicLink` → POST `/auth/email`). 백엔드 `src/auth/email.rs` | **Q-2 결정 후**: 프론트 폼 제거(최소), 완전 제거면 백엔드 `/auth/email` 라우트·`supabase_auth` 의존까지 정리 | P2 | 🔲 |
| **AU6** | "Connect Wallet (**SIWX**)" — SIWX는 내부 라이브러리명(`frontend/lib/siwx.ts`), 사용자에게 무의미 | `LoginForm.tsx:211` 버튼 라벨. 지갑 로그인 기능 자체는 정상(왕복 확인됨) | **Q-3 확정(기능 유지)**: 라벨을 "Connect Wallet"로 개칭. `data-testid="wallet-sign-in"` 유지 | P2 | 🔲 |

**수용 기준(4.1)**: AU1 — 다른 GitHub 계정으로 실제 전환 가능(오류 페이지 없음). AU2 — "Back to home"으로 나가도 재로그인 시 온보딩 재노출 안 됨. AU3 — 최초 방문 로그인 화면에 계정 전환 카피 미노출(또는 동작하는 흐름). AU6 — 버튼 라벨 "Connect Wallet", 지갑 왕복 정상.

### 4.2 검색

| ID | 증상 | 검증된 원인 | 수정안 | 우선 | 상태 |
|----|------|-------------|--------|------|------|
| **SR1** | `unisw`·`uni`·`uniswa`(단어 미완성 접두어) 전부 **빈 결과** `[]`. `uniswap`(완전 단어)에서만 11건. 사용자는 타이핑 내내 "고장" 체감 | `src/server/tool_search.rs:12` FTS가 `plainto_tsquery`(완전 토큰만 매칭), `:*` 접두어 없음. 프론트 콤보박스도 결과 0일 때 드롭다운 자체를 안 열어(`aria-expanded=false`) "no results"조차 안 보임 | 백엔드: 짧은/부분 쿼리에 접두어 매칭(`to_tsquery`에 토큰별 `:*` suffix) 또는 trigram/ILIKE 폴백 추가. 프론트: 매칭 전에도 "no matches yet" 상태 노출(선택) | **P1** | 🔲 |
| **SR2** | `uniswap` 검색 시 이름에 "Uniswap"이 든 "Uniswap Official Viem Integration"이 이름엔 없고 설명에만 언급된 "AutoPilot DeFi Agent"·"Aegis"·"Ethskills"보다 **아래로** 랭크 | `tool_search.rs:8` tsvector가 `name || ' ' || description`를 `setweight` 없이 합침 → `ts_rank_cd`가 이름/설명 매치를 동일 취급(`TS_RANK_AND/OR`, `tool_search.rs:18-22`) | `setweight(to_tsvector(name),'A') \|\| setweight(to_tsvector(coalesce(description,'')),'B')`로 이름 매치 가중 | **P1** | 🔲 |
| **SR3** | `/?q=<쿼리>` 결과 페이지에서 검색창 재클릭 시 typeahead 드롭다운이 **sticky 툴바 뒤로** 깔림. 가려진 2~3번째 항목은 **클릭도 밑의 sort-link로 관통** | `.search-typeahead-list` z-index:40(absolute) < `.sticky-toolbar` z-index:90(sticky, top:56px). `elementFromPoint`로 관통 확인. 홈(툴바 없음)에선 재현 안 됨 | `.search-typeahead-list` z-index를 90 초과로(또는 portal/상위 stacking context) 올려 항상 툴바 위에 페인트 | **P1** | 🔲 |

**수용 기준(4.2)**: `unis` 입력 → Uniswap 관련 결과 노출(빈 배열 금지). `uniswap` → 이름 매치가 설명-only 매치보다 상위. `/?q=uniswap`에서 드롭다운 2~3번째 항목이 툴바 위에 보이고 클릭 시 해당 항목 이동(밑의 sort-link 관통 금지).

### 4.3 로고 · 브랜드 에셋

| ID | 증상 | 검증된 원인 | 수정안 | 우선 | 상태 |
|----|------|-------------|--------|------|------|
| **LG1** | Chainlink(`status=official`)·Circle 등 유명 official 툴 로고가 **GitHub 제네릭 회색 옥토캣** 실루엣 | `logo_url = https://avatars.githubusercontent.com/{org}` 그대로 사용. org가 커스텀 아바타 미설정 시 GitHub이 제네릭 placeholder 반환. ETag 대조로 smartcontractkit·circle-fin·reown-com·web3-mcp-hub가 **동일** placeholder etag `2ae73e12…` 확인(커스텀 있는 kukapay·rainbow-me·wevm는 상이) | official/verified(또는 homepage 있는) 툴은 공식 사이트 favicon/OG/brand-kit에서 실로고 폴백. 그리고/또는 이 placeholder etag/hash 감지 시 "no logo"로 처리해 `ToolLogo` 모노그램 폴백 유도 | **P1** | 🔧/🔲 |
| **LG2** | 40px 미만으로 렌더되는 비-brand 로고가 **잘림**(검색 콤보박스 32·Compare 36/28·블루프린트 32/36) | `frontend/styles/site-output.css:232-241` `.tool-logo-img`가 `width:40px;height:40px` 고정 + `.tool-logo` `overflow:hidden`. `size<40`이면 컨테이너가 크롭. `-brand` 변형(100%/contain)만 안전 | `.tool-logo-img`를 컨테이너 상대 크기(100%/100% + object-fit cover 또는 contain)로. 원본 `style/output.css` 수정 + prebuild | **P1** | 🔲 |
| **LG4** | Plasma 체인 로고 **완전 빈 화면**(흰 배경만) | `frontend/public/chains/plasma.svg` — 내부 nested `<svg viewBox="0 0 96 96">`의 배경 rect(`x=68 y=112`)·mark path(`x≈151 y≈160`)가 전부 viewBox **밖**. 추출 시 부모 translate 미반영으로 좌표가 안 옮겨짐 | path/rect 좌표를 0-96 viewBox 안으로 정정 또는 Plasma brand 소스에서 재추출 | **P1** | 🔧/🔲 |
| **LG5** | Scroll 체인 로고가 **아래 화살표(↓)** — 공식 로고 아님(파일 주석엔 "official logomark" 거짓 표기) | `frontend/public/chains/scroll.svg` path가 화살표 형상. 추출 오류 또는 잘못된 에셋 저장 | scroll.io/brand-kit에서 실 logomark 재소싱, 시각 확인 후 커밋 | **P1** | 🔧/🔲 |
| **LG3** | Aptos·Cosmos·NEAR·Starknet 로고가 **브랜드색 단색 원**(실로고 아님) | `frontend/public/chains/{aptos,cosmos,near,starknet}.svg` — `<circle cx=24 cy=24 r=14.4 fill=…>` 제네릭 패턴. 77개 중 이 4개만 해당(`grep -l` 확인) | 4개 체인 실 공식 로고 소싱·교체 | P2 | 🔧/🔲 |

**수용 기준(4.3)**: LG1 — Chainlink 등 official 툴 카드가 옥토캣이 아닌 실로고 또는 의미 있는 모노그램. LG2 — 32px 렌더 로고가 잘리지 않음. LG4 — Plasma 로고가 실제 마크로 보임. LG5 — Scroll 로고가 화살표 아님.

### 4.4 블루프린트 캔버스

| ID | 증상 | 검증된 원인 | 수정안 | 우선 | 상태 |
|----|------|-------------|--------|------|------|
| **BP1** | 캔버스에서 선(엣지)을 **실제 마우스로 클릭해도 선택 안 됨** → Solid/Arrow/색상/Delete link 인스펙터가 안 뜸 → **사용자가 링크를 지우거나 재스타일할 방법이 UI상 없음**(JS synthetic click으로만 선택됨을 실증) | `frontend/components/blueprint/BlueprintEditor.tsx:756-777` `handleCanvasPointerDown`이 `[data-port]`·`[data-testid='blueprint-node']` 위 클릭만 제외하고, 엣지 hit-line/handle circle(`BlueprintEdgesLayer.tsx`)은 제외 안 함 → pointerdown이 캔버스 팬(`setPointerCapture`) + `setSelectedEdgeId(null)`로 먼저 삼켜 실제 click이 엣지 onClick에 도달 못 함 | `handleCanvasPointerDown`의 팬/역선택 로직에서 엣지 레이어(예: `.blueprint-edge-hit`/`.blueprint-edge-handle` 또는 전용 data-attr)를 port/node처럼 **제외** | **P0** | 🔲 |
| **BP2** | 연결 자유도 낮음 — 노드마다 연결점 좌"in"/우"out" 1개씩뿐, 드래그도 **out에서만** 시작 가능(in에서 시작하면 무시) | 포트 2개 고정(`BlueprintNodeView.tsx:146-160,240-254`). `handlePortPointerDown`(`BlueprintEditor.tsx:637`) `if (readOnly \|\| side !== "out") return;` — in 포트 드래그 no-op | in 포트에서도 드래그 시작 허용(역방향) 및/또는 추가 앵커 포인트. UX 재설계 필요 — 오너 방향 확정 후 | P2 | 🔲 |

**수용 기준(4.4)**: BP1 — 실제 마우스로 선 클릭 → 인스펙터 노출 → Delete link로 삭제 가능.

### 4.5 툴 상세 페이지

| ID | 증상 | 검증된 원인 | 수정안 | 우선 | 상태 |
|----|------|-------------|--------|------|------|
| **TD1** | "INSTALL RISK: MEDIUM" 배지에 **왜 medium인지 설명 전무** | API `GET /api/v2/tools/{slug}`는 `install_risk_reasons`(예: `["no install command provided"]`) 반환하나, 프론트 어디서도 렌더 안 함 — `install_risk_reasons`는 `frontend/lib/install-guide.ts`에서 MCP 에이전트용 `get_install_guide` 응답 구성에만 소비, 사람용 페이지엔 미노출 | 상세 페이지 risk 배지 옆에 tooltip/expandable로 `install_risk_reasons` 표시 | **P1** | 🔲 |
| **TD2** | 검색/브라우즈 프리뷰 패널엔 "Save to Toolkit" 북마크가 있는데, "Open full page"로 간 **풀 상세 페이지엔 북마크 컨트롤 자체가 없음** | `PreviewPanelContent.tsx`엔 북마크 버튼 존재, `ToolDetail.tsx`엔 `grep` 0건(Bookmark/toolkit 없음). 프리뷰가 "풀 페이지 보라" 유도해놓고 거기선 저장 불가 | `ToolDetail.tsx`에 프리뷰와 동일한 Save to Toolkit 북마크 추가 | **P1** | 🔲 |

**수용 기준(4.5)**: TD1 — 상세 페이지에서 risk 이유 열람 가능. TD2 — 풀 상세 페이지에서 북마크 저장/해제 가능.

### 4.6 운영자 · Admin

| ID | 증상 | 검증된 원인 | 수정안 | 우선 | 상태 |
|----|------|-------------|--------|------|------|
| **OP2** | 툴 카드 옆 "**Review or edit**"(연필) 버튼 → 이미 발행된 툴은 편집 화면이 **안 뜸**(빈 new_candidate 큐만 보임). 카탈로그 대부분(224개 중 발행 툴)에서 사실상 무동작 | `frontend/components/tools/AdminToolCardActions.tsx:27` `reviewHref = /admin/tools?slug=…`가 `queue` 파라미터 없음 → 도착 페이지(`frontend/app/admin/tools/page.tsx:13`)가 기본 `new_candidate` 큐만 로드 → 발행 툴은 그 큐에 없어 패널 미확장(`page.tsx:51`). "not found" 메시지도 없이 무음 실패 | 버튼 링크에 실제 queue 포함, 또는 slug로 전역 툴 조회 모드 추가, 또는 심사 큐와 별도의 범용 툴 편집 뷰 신설 | **P0** | 🔲 |
| **OP3** | 홈 캐러셀 카드의 "**Edit**"/"**Add**" 버튼이 아무 편집도 못 함 | `FeaturedCarousel.tsx:37-46,120-126`가 `/admin/featured?edit=<id>`·`?new=1&tool=<slug>`로 링크하나, 도착 `frontend/app/admin/featured/page.tsx`는 `listFeaturedCardsAdmin()`만 호출하는 **읽기 전용 리스트** — `useSearchParams()`/edit·new 파라미터 미독해, `<form>`/`<input>`/`<textarea>` **0개**. 헤드라인/부제/이미지 수정·신규 카드 추가 UI가 아예 없음 | edit/new 쿼리에 연결된 실제 편집/생성 폼 구현(`docs/FEATURED_CARDS.md` 참조), 또는 구현 전까지 버튼 제거 | **P0** | 🔲 |
| **OP1** | 심사 패널이 승인/거부 전 **툴 컨텍스트 전무** — 이름 + 메모 textarea + 버튼 5개(approve/reject/mark_verified/mark_official/demote_verified)뿐 | `frontend/components/admin/AdminReviewDecisionPanel.tsx`가 액션만 렌더. `/api/v2/admin/review-queue`는 description·repo_url·official_team·`crypto_relevance_score`+reasons·**`relevance_status`(예 chainlink-local="rejected", score 13)**·install_risk를 반환하나 미노출. 운영자는 GitHub를 따로 열어야 판단 가능, 시스템 자동 red-flag도 못 보고 승인 위험 | 액션 버튼과 함께 description·repo/homepage 링크·crypto_relevance_score/reasons·install_risk_level/reasons·relevance_status 렌더 | **P1** | 🔲 |

**참고(정상 확인)**: `mark_verified`/`mark_official`은 AGENTS.md의 "hand-set 금지"와 무관 — `operator_review_transition.rs`의 게이트(claim + 검증 링크 2개 등)를 거치는 정식 경로. "Add MCP"·"Save to Toolkit"·"Compare" 카드 버튼은 정상 동작 확인.

**수용 기준(4.6)**: OP2 — 발행 툴 연필 클릭 → 해당 툴 편집/심사 화면 노출. OP3 — Edit/Add 클릭 → 실제 카드 편집/생성 폼. OP1 — 심사 패널에 후보 메타·관련성 판정 노출.

### 4.7 x402 제출

| ID | 증상 | 검증된 원인 | 수정안 | 우선 | 상태 |
|----|------|-------------|--------|------|------|
| **XP1** | Type=X402 선택 → Name/Description/endpoint 입력 → "Check endpoint"(정상 프로브 피드백) → 약관 체크 → 최종 "Probe and publish" **누른 뒤에야** "x402 self-listing is currently disabled" | `src/server/api_v2/x402_listing.rs:292-296`가 `settings.allow_x402_registration`를 **최종 submit 안에서만** 체크. 이 플래그를 프론트가 사전 조회할 경로 없어 폼을 다 채운 뒤 헛수고 판명 | **Q-5 결정 후**: 활성화하거나, 공개 settings/config 엔드포인트로 `allow_x402_registration` 노출 → 폼 로드 시 상단 비활성 안내(또는 X402 옵션 비활성) | **P1** | 🔲 |
| **XP2** | x402 프로브 실패 메시지가 실제 받은 **HTTP 상태를 버림** — 200/404/500/타임아웃 무관하게 "endpoint did not return a valid 402…" + `details:null` | `src/server/x402_verify.rs:311-317` `classify_probe_details(status, body)`가 실제 StatusCode를 받지만 `ProbeDetailsOutcome::NotPaymentRequired`가 unit variant라 미전달. `x402_listing.rs:54-59`가 제네릭 사유 반환. 실제 x402 운영자가 왜 실패인지 알 수 없음 | `NotPaymentRequired`에 수신 status(+ body snippet) 담아 프로브 응답/UI에 노출 | **P1** | 🔲 |
| **XP3** | x402 최초 제출자용 안내 부재 — "we probe it for a valid 402 handshake" 한 줄뿐, 유효 응답 예시·스펙 링크 없음 | 프론트 submit X402 뷰. 레포에 `docs/X402_MONETIZATION_SPEC.md`·`docs/X402_OPEN_LISTING_SPEC.md` 존재하나 미링크 | 예상 402 응답 형식 예시 + 스펙 문서 링크 추가 | P2 | 🔲 |

**수용 기준(4.7)**: XP1 — X402 폼 진입 시 비활성 상태를 **즉시** 안내(또는 활성화). XP2 — 프로브 실패 시 실제 수신 status 표시. XP3 — 402 형식 안내/링크 노출.

### 4.8 에이전트 연결(Connect)

| ID | 증상 | 검증된 원인 | 수정안 | 우선 | 상태 |
|----|------|-------------|--------|------|------|
| **AS1** | `/connect#agent-sync` "I already have a code"의 **"Connect" 제출 버튼이 안 보임** — 흰 배경에 흰 글씨(computed: `color:#fff`, `background:rgba(0,0,0,0)`, `border:none`). 코딩 에이전트(Claude Code/Cursor) 링크 1차 CTA인데 사실상 사용 불가 | `frontend/app/globals.css:1334` `.agent-link-approve { background: var(--color-accent); … }`. 그러나 **`--color-accent`는 어디에도 정의 안 됨** — globals.css에서 `var(--color-accent)` **7회** 사용되나 정의 0회(실제 액센트 토큰은 `--color-tertiary: #e76f00`, `globals.css:8`). 미정의 변수라 `background`가 투명으로 풀림. (버튼 하나가 아니라 `--color-accent` 참조 7개 지점 전부 영향: agent-link-status·icon 배경 등) | `--color-accent`를 `:root`에 정의(= `--color-tertiary` 값)하거나, 7개 `var(--color-accent)` 참조를 `var(--color-tertiary)`로 치환. 원본 스타일 수정 + prebuild | **P0** | 🔲 |

**수용 기준(4.8)**: AS1 — Connect 버튼이 오렌지 filled로 보이고, `--color-accent` 참조 지점(agent-link 배경 등) 정상 렌더.

---

## 5. 필수 사용자 시나리오 (수정 후 참이어야 함)

1. **계정 전환(AU1)**: 로그인 화면에서 GitHub 계정 전환 경로가 오류 페이지 없이 다른 계정 선택으로 이어진다.
2. **온보딩 이탈(AU2)**: 온보딩에서 "Back to home"으로 나가도 재로그인 시 온보딩이 다시 뜨지 않는다.
3. **검색(SR1/SR3)**: `unis` 타이핑 중 Uniswap 결과가 나오고, `/?q=uniswap`에서 드롭다운 항목이 툴바 위에 보이며 클릭이 그 항목으로 간다.
4. **엣지 삭제(BP1)**: 블루프린트에서 선을 마우스로 클릭 → 인스펙터 → Delete link로 지운다.
5. **운영자 편집(OP2/OP3)**: 카드 연필 → 해당 툴 편집 화면; 캐러셀 Edit/Add → 실제 편집/생성 폼.
6. **Connect(AS1)**: `/connect`에서 코드 입력 후 보이는 오렌지 Connect 버튼을 눌러 에이전트를 연결한다.
7. **x402 제출(XP1)**: X402 폼 진입 즉시 비활성 여부를 알거나(비활성 시), 활성 시 정상 제출된다.

---

## 6. 슬라이스 제안 커밋 메시지

| Slice | 이슈 | 제안 메시지 |
|-------|------|-------------|
| 1 | AU1·AU2 | `fix(auth): repair GitHub account switch + onboarding back-to-home completion` |
| 2 | OP2·OP3·OP1 | `fix(admin): wire card edit routes + featured edit form + review-queue context` |
| 3 | AS1 | `fix(ui): define --color-accent token so agent-link Connect button renders` |
| 4 | BP1 | `fix(blueprint): let real mouse clicks select canvas edges` |
| 5 | SR1·SR2·SR3 | `fix(search): prefix matching + name-weighted ranking + typeahead z-index` |
| 6 | LG1·LG2·LG4·LG5 | `fix(logos): brand-logo fallbacks, sub-40px clipping, Plasma/Scroll assets` |
| 7 | TD1·TD2 | `feat(tool-detail): surface install-risk reasons + bookmark parity` |
| 8 | XP1·XP2·XP3 | `fix(x402): upfront disabled notice + probe status detail + spec guidance` |
| 9 | AU3·AU4·AU5·AU6 | `chore(auth): login copy cleanup (switch/google/email/SIWX)` |
| 10 | LG3·BP2 | `chore(logos+blueprint): placeholder chain logos + edge-direction UX` |

---

## 7. 완료 매트릭스

| 영역 | 이슈 | P0 | P1 | P2/결정 |
|------|------|----|----|---------|
| 인증·온보딩 | AU1·AU2·AU3·AU4·AU5·AU6 | AU1 | AU2·AU3 | AU4·AU5·AU6 |
| 검색 | SR1·SR2·SR3 | — | SR1·SR2·SR3 | — |
| 로고·에셋 | LG1·LG2·LG3·LG4·LG5 | — | LG1·LG2·LG4·LG5 | LG3 |
| 블루프린트 | BP1·BP2 | BP1 | — | BP2 |
| 툴 상세 | TD1·TD2 | — | TD1·TD2 | — |
| 운영자 | OP1·OP2·OP3 | OP2·OP3 | OP1 | — |
| x402 | XP1·XP2·XP3 | — | XP1·XP2 | XP3 |
| Connect | AS1 | AS1 | — | — |

**P0(5)**: AU1 · BP1 · OP2 · OP3 · AS1 — 핵심 흐름을 완전히 막는 결함.
**P1(13)**: AU2·AU3 · SR1·SR2·SR3 · LG1·LG2·LG4·LG5 · TD1·TD2 · OP1 · XP1·XP2.
**P2·결정(7)**: AU4·AU5·AU6 · LG3 · BP2 · XP3.
