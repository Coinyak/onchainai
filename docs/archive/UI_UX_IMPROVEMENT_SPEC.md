# Grok 작업 지시서: UI/UX 회귀 수리 + 발견성 회복 + 미리보기 재설계 + Stack Blueprint

> 작성: 2026-07-03 (v2 — 최종). 실행 주체: **Grok** (서브에이전트 수 제한 없음, 스킬·플러그인 자유 사용).
> 근거: 로컬 Next.js dev(프로덕션 API 프록시 `API_PROXY_TARGET=https://www.onchain-ai.xyz`) 데스크톱 1280·모바일 375×812 실측 + 소스 코드 대조 + 프로덕션 curl.
> 선행 문서: [[PRODUCT_ENHANCEMENT_SPEC]](Leptos 시절), [[USER_FRIENDLY_DISCOVERY_SPEC]], [[GROK_FULL_SPEC_TASK]](전환 지시서), [[../DESIGN]](디자인 토큰), [[UI_UX_DESIGN]](원설계).
> 기존 스펙과 겹치는 항목은 §10에서 승계/재정의를 명시한다.

## 목표

1. Next.js 전환(#27)이 만든 회귀·소실 19건 수리 (Phase 0)
2. 미리보기 패널을 모달에서 **비모달 도킹 패널**로 재설계 — 스크롤 점프 제거, 딤 제거, 내용 강화 (Phase 1, 오너 피드백 반영)
3. SEO 0 상태 복구 — 디렉토리 핵심 성장 레버 (Phase 2)
4. 검색·상세·비교·모바일 마감 (Phase 3~6)
5. 신규 기능 **Stack Blueprint** — 로그인 유저가 모눈종이 캔버스에 툴 카드를 드래그 배치하고 메모하는 설계도 (Phase 7, 오너 제안)
6. 백로그 신규 기능 N1~N6 (Phase 8)

## 절대 규칙 (모든 서브에이전트 공통)

1. **`frontend/AGENTS.md` 필독**: 이 Next.js 16은 훈련 데이터와 다르다. 코드 작성 전 `frontend/node_modules/next/dist/docs/`에서 해당 API(metadata, sitemap, 서버 컴포넌트, `headers()`) 규약 확인.
2. **디자인 불변식** ([DESIGN.md](../DESIGN.md)): 라이트 모드 온리, 오렌지 `#E76F00`는 화면당 주요 액션 1개만, 이모지 금지(lucide 단색 `#4B4B4B`), 카드 radius 8px, 모바일 본문 ≥16px·터치 44px, 그라데이션 금지, UI 텍스트 영어.
3. **x402는 메타데이터/신뢰 신호만** — 결제 실행·커스터디·지갑 UI 금지.
4. **기존 동작 보존**: 필터 URL 상태, 사이드바 접힘 저장, 바텀시트, `data-testid` 전부 (`tool-card-link`, `preview-panel`, `preview-bottom-sheet`, `profile-menu*`, `auth-*`, `github-sign-in`, `wallet-sign-in-link` 등). 제거/개명 금지.
5. **CI 자동 트리거 금지**: 푸시 시 `[skip ci]`. CodeRabbit/qodo 수동 전용. 검증은 로컬 명령으로.
6. **백엔드 변경 시**: sqlx 파라미터 쿼리, 마이그레이션 후 `sqlx prepare`, 인증 필수 라우트는 서버사이드 검사, `cargo check --features ssr` + clippy/fmt.
7. **비밀값 노출 금지**: `SUPABASE_SERVICE_KEY`/`JWT_SECRET` 클라이언트 금지, `.env` 커밋 금지.
8. **배포 가드레일**: Vercel은 main 푸시 시 **자동 배포**된다(`[skip ci]`는 GitHub Actions만 막음). 모든 작업은 피처 브랜치에서, main 병합은 해당 Phase의 §9 검증 통과 후에만. 병합 직후 프로덕션 스모크(홈/상세 1회) 확인.

## 오케스트레이션 가이드 (Grok 재량, 권장 분할)

서브에이전트·스킬·플러그인 자유. 권장 구조 — [docs/MULTI_AGENT_COORDINATION.md](MULTI_AGENT_COORDINATION.md)의 역할 분리 원칙 준용:

| 서브에이전트 | 담당 | 병렬성 |
|---|---|---|
| css-medic | Phase 0의 R2(캐스케이드 레이어) 선행 → R1/R3 | 최우선, 단독 |
| bug-squad ×N | Phase 0 나머지(R4~R19) — 파일 단위로 쪼개 병렬 | R2 이후 병렬 |
| preview-redesign | Phase 1 (I7) | R19와 같은 파일 — bug-squad와 조율 |
| seo-agent | Phase 2 (I1) | 독립 병렬 가능 |
| search-agent | Phase 3 (I2) | 독립 |
| detail-agent | Phase 4 (I3) — Phase 2 완료 후 | 순차 |
| compare-agent | Phase 5 (I4) | 독립 |
| canvas-agent | Phase 7 (N7) — 프론트+백엔드+마이그레이션, 가장 큼 | 독립, 조기 착수 가능 |
| qa-agent | 각 Phase 종료마다 스모크 (아래 검증 절) — `playwright-cli`/`visual-qa` 스킬 활용 | 상시 |

- 리포 스킬: `.agents/skills/onchainai-ui-workflow/`(디자인 불변식·검증 매트릭스 유효 — 단 dev-watch/ui-change-gate 등 **Leptos 명령은 frontend에 부적용**, `npm run dev`/`npm run lint` 사용), `web-design-guidelines`, `responsive-design`, `web-accessibility`, `tailwind`, `design-tokens`.
- MCP: `.grok/config.toml`에 vercel/railway/onchainai 구성 — 배포 관측은 [docs/MCP_AGENT_WORKFLOW.md](MCP_AGENT_WORKFLOW.md).
- 브랜치/워크트리 운용 재량. 단 Phase 단위로 검증 통과 후 통합.

## 현재 상태 & 전제 변화

- 프론트: `frontend/` Next.js 16.2 + React 19 + Tailwind v4 + TanStack Query 5. 전 페이지 `"use client"` + 브라우저 fetch — **서버 렌더 데이터 없음**.
- Leptos CSS 산출물을 `frontend/styles/site-output.css`로 복사, `app/globals.css:81`에서 Tailwind(1행) **뒤에** 임포트 → 레거시 규칙이 유틸리티를 상시 오버라이드.
- 백엔드: Railway의 Rust Axum `/api/v2` + `/auth` + `/mcp`. Vercel이 `next.config.ts` rewrites로 프록시.
- PRODUCT_ENHANCEMENT_SPEC의 `src/components/*.rs` 참조와 "SSR 정상(홈 HTML 228KB)" 진단은 Leptos 기준 — 무효.
- 도구 수 141(실측). apex→www 301은 해결 확인.

---

## Phase 0: 회귀·소실 수리 (트랙 R, P0)

확인 방법 — 브라우저 렌더 실측(R1~R11), 프로덕션 응답 실측(R13), 코드 확정(R12, R14~R19). 심각도 순.

| ID | 심각도 | 증상 | 원인 | 수정안 |
|---|---|---|---|---|
| R1 | **Critical** | 모바일(375px) 도구 상세의 체인 로고가 311×311px로 렌더 — 페이지 사용 불가 | `styles/site-output.css:334` `.chain-logo{width:100%;height:100%}`는 부모가 크기를 제약하던 Leptos 전제. `ToolDetail.tsx:141-153`의 `.detail-chain-tag`엔 제약 없음 | `.detail-chain-tag .chain-logo{width:24px;height:24px}` 또는 ChainLogo 인라인 사이즈. 데스크톱/모바일 확인 |
| R2 | **High** | Tailwind 유틸리티 조용히 무효 — `hidden md:flex`(`ToolCard.tsx:149`)가 모바일에서 `display:flex` 계산(실측) | `globals.css` 임포트 순서: tailwind(1행) → site-output.css(81행). 레거시 `.tool-install{display:flex}`(:594)가 항상 승리 | `site-output.css`를 `@layer legacy`로 감싸 유틸리티 아래 배치(단기). 중기: 레거시 규칙 Tailwind 이관·축소. **모든 스타일 작업 전 선행** |
| R3 | **High** | `/compare` 코드 블록이 칼럼을 뚫고 옆 칼럼을 덮음 | 칼럼 `min-width:0` 부재 + 한 줄 JSON 처리 없음 | `min-width:0` + `overflow-x:auto`(또는 `overflow-wrap:anywhere`) |
| R4 | **High** | 로그인 UI 렌더마다 하이드레이션 에러 반복 — `<form> in <p>` | `LoginForm.tsx:64-76` invalid HTML 중첩 | `<p>` → `<div>` |
| R5 | Medium | 상세 메타 "0 commentsupdated 4h ago" 붙음. 카드 `★ {stars}` 텍스트 글리프(`ToolCard.tsx:144`) — 구 D7 지적 잔존 | `.tool-detail-stats` 간격·구분자 부재 + 글리프 | gap + `·`. `★`→lucide `Star`, GitHub 스타임을 라벨/툴팁 명시 |
| R6 | Medium | 전체 페이지에 자기 자신 가리키는 "View full page" | `ToolDetail.tsx:178-184` 조건 `!compact && !addMode` 반전 | 명시 prop `showFullPageLink`로 호출부에서만 |
| R7 | Medium | 상세 링크 목록 2벌(설치 패널 하단 + Links 섹션) | `InstallGuidePanel` / `ToolDetail.tsx:155-174` 중복 | Links 섹션 단일화 |
| R8 | Low | "OFFICIAL" + "OFFICIAL: BOB" 배지 중복 | `ToolDetail.tsx:80-83` | official_team 있으면 status 배지 생략 |
| R9 | Low | 접힌 사이드바 브랜드 "Onch" 클리핑, 모바일 브랜드 2중 표시 | `SidebarBrand` 접힘/모바일 미처리 | 접힘·모바일에서 텍스트 숨김 |
| R10 | Low | 레일 약어 `Fn/Ac/Hu/Ty/St/Pr/Ri` 난해 | 약어 하드코딩 | lucide 아이콘+툴팁 (Phase 6 I6) |
| R11 | Low | 홈 오렌지 CTA 2개(Submit + Suggest→) — DESIGN.md 위반 | 독립 primary 스타일 | Suggest를 secondary로 |
| R12 | **High** | 로그아웃이 env 미설정 시 `http://localhost:3000`으로 POST | `TopNav.tsx:84` 폴백 — `api.ts:1`(`\|\| ""`)과 불일치 | 폴백 `""`(상대경로) 통일, Vercel env 실값 확인 |
| R13 | **High** (보안) | 응답 보안 헤더 HSTS뿐(실측). [SECURITY.md:166-172](SECURITY.md) 요구 5종 미적용 | Axum 미들웨어 헤더가 Vercel 분리로 소실 | `next.config.ts` `headers()`: 1차 X-Frame-Options/nosniff/Referrer-Policy/Permissions-Policy 즉시, CSP는 nonce 검토 후 후속 |
| R14 | **High** (기능 소실) | Tool Finder(구 Feature A) 미이식 — `ToolFinder.tsx` 사용처 0, `ToolFinderPanel` null 스텁 | 전환 배선 누락 | quick-match는 Phase 3(I2)가 흡수. 위저드 부활/폐기는 §11 |
| R15 | Medium | 홈 카테고리 그리드 미배선 — `CategoryGrid.tsx` 미사용, 카테고리 진입 사이드바뿐 | 전환 배선 누락 | 홈 검색 아래 복원(모바일 2칼럼) 또는 제거 확정+문서 갱신 (§11) |
| R16 | Medium | 임의 slug·오류 시 무브랜드 Next 기본 화면 | `not-found.tsx`/`error.tsx`/`global-error.tsx` 부재 | 브랜드 404(검색+인기 도구)/에러(재시도) 추가 |
| R17 | Low | 바텀시트 "Drag to expand" 라벨인데 클릭 토글만. `aria-modal`·포커스 트랩 부재 | `BottomSheet.tsx:62-68` | 라벨 "Tap to expand"+`aria-modal` 즉시, 드래그·트랩은 Phase 6 |
| R18 | Low | GitHub 링크 개인 핸들 하드코딩 | `TopNav.tsx:10` | 조직 리포/설정값 치환 |
| R19 | **High** (오너 보고) | **미리보기를 열면 화면이 맨 위로 점프** — 정렬/필터/Load more 클릭도 동일 | ① 모든 `<Link>`에 `scroll={false}` 부재(Next 기본 scroll-to-top) — `ToolCard previewHref`, 정렬/타입/체인/Load more/close 전부 ② `PreviewPanel.tsx:32-34` 열릴 때 `panel.focus()`가 스크롤 유발 | 쿼리 파라미터만 바꾸는 모든 Link에 `scroll={false}`. focus는 `{preventScroll:true}`. Load more는 현재 위치 유지 확인 |

**Phase 0 수용 기준**: 375px 상세 정상(로고 24px)·`.tool-install` 모바일 `display:none` 실측·`/compare` 겹침 0·콘솔 하이드레이션 에러 0·미리보기/정렬/필터 클릭 시 스크롤 위치 유지·로그아웃 상대경로·보안 헤더 4종 응답 존재·임의 slug 브랜드 404·홈 오렌지 CTA 1개·Finder/CategoryGrid는 §11 결정 후 배선 또는 삭제(데드 코드 잔존 금지).

---

## Phase 1: 미리보기 패널 재설계 (I7, P0 — 오너 피드백)

**오너 피드백**: ① 누르면 맨 위로 올라감(R19) ② 회색 음영 별로 ③ "미리보기"인데 기능이 약함.

**진단**: 현 구현은 원설계(UI_UX_DESIGN §5.9 "VS Code 에디터 패널")와 달리 **모달**이다 — `position:fixed` 패널 + 전면 딤(`.preview-backdrop` rgba(26,26,26,.3)) + 포커스 강탈 + 딤이 리스트 조작 차단. 내용은 `ToolDetail compact` 재탕 + 댓글.

**재설계 (디자인 결정 — 승인된 재량)**:

1. **비모달 도킹 패널**: 데스크톱(≥1024px)에서 backdrop **삭제**. 패널은 `tools-layout` 우측 sticky 칼럼(`top: 상단 오프셋`, `height: calc(100vh - 오프셋)`, 자체 스크롤). 리스트는 열림 시 폭 축소(기존 설계 유지) — **리스트 스크롤·클릭 계속 가능**, 다른 카드 클릭 시 패널 내용만 교체.
2. **선택 표시는 카드가 담당**: 딤 대신 선택 카드 beige `#F5F5F0` + `border-strong`(기존 `is-selected` 상태 활용). 패널은 `border-left 1px`만, 그림자 제거(도킹이므로).
3. **콘텐츠 재구성 (위→아래)**:
   - 헤더: 로고 48 + 이름 + status/type 배지 + 닫기(×). GitHub ★ 수치는 라벨 명시.
   - **Quick facts 그리드(2×3)**: Type · Status · License · Updated · Install risk · Source — 스캔 1초.
   - **Install**: 클라이언트 탭(Claude/Cursor/Generic/CLI) + 복사 — 기존 자산 유지.
   - Trust 요약 3줄(기존 TrustFacts).
   - 체인 로고 스트립(20px, 최대 8 + "+N").
   - Description(3줄 클램프 + "more" 토글).
   - 같은 카테고리 더 보기 링크(N1 완성 후 related 카드 3개로 승격).
   - 댓글: 개수 + 최신 1개 미리보기 + "댓글 모두 보기 →"(전체 페이지 앵커) — 현행 전체 댓글 폼 임베드는 제거(패널 비대 원인).
4. **하단 고정 액션 바**: `[Open full page]`(primary 오렌지 — 패널 내 유일) `[Add MCP]` `[Save]` `[Compare]` 아이콘+라벨. 미리보기→행동 전환 직결.
5. **키보드**: 리스트 포커스 상태에서 ↑/↓(또는 j/k)로 이전/다음 도구 미리보기 전환, Escape 닫기(유지). 포커스는 `preventScroll`.
6. **모바일 바텀시트는 모달 유지**(딤 포함 — 모바일에선 올바른 패턴). R17의 라벨/트랩 수리만 적용.
7. `role="dialog"` 제거 → `role="complementary"` + `aria-label="Tool preview"` (비모달이므로).

**수용 기준**: 딤 없음(데스크톱), 미리보기 열림 중 리스트 스크롤/타 카드 클릭 동작, 스크롤 위치 불변(R19), 패널 스크롤 독립, ↑↓ 전환 동작, `data-testid="preview-panel"` 보존, 375px 바텀시트 기존 동작.

---

## Phase 2: SEO/SSR 발견성 회복 (I1, P0) — 구 H2 대체

**문제**: 141개 도구 페이지 전부 고정 `<title>OnchainAI</title>`(`app/layout.tsx:18-21`), 본문 클라이언트 fetch — 크롤러에 빈 껍데기. `sitemap.xml` 404·robots 부재(실측).

**스펙**:
1. `app/tools/[slug]/page.tsx` 서버 컴포넌트화: 서버 fetch(`API_PROXY_TARGET` 재사용) → TanStack Query `HydrationBoundary`/props로 주입. 상호작용은 client 유지.
2. `generateMetadata`: `"{name} — {type} | OnchainAI"` + 설명 + OG/Twitter 카드 + canonical. 기본 OG 이미지(1200×630, 뉴트럴+로고) 1장 제작.
3. `app/sitemap.ts`(정적+slug+카테고리), `app/robots.ts`(admin/dashboard/toolkit disallow).
4. JSON-LD `SoftwareApplication` 상세 삽입.
5. 홈/목록 첫 페이지 프리렌더는 여력 시.

**수용 기준**: 상세 raw HTML에 이름/설명, `<title>` 도구별 상이, sitemap 200+slug, Lighthouse SEO ≥90, JSON-LD Rich Results 통과.

## Phase 3: 검색을 "모드"로 (I2, P1)

**문제(실측)**: 검색해도 히어로+캐러셀+프로모가 상단 그대로 — 결과 폴드 아래. 검색창 쿼리 에코 없음. 타이핑 피드백 없음.

**스펙**: ① `q` 존재 시 히어로/캐러셀/프로모 접고 결과 최상단(+"← Back to home") ② 검색창 `defaultValue=q` ③ 타이포어헤드: 디바운스 200ms, 목록 API `page_size=5`, 로고+이름+타입 드롭다운, ↑↓/Enter/Esc, `aria-activedescendant`, 모바일 전체화면 유지 ④ N2(⌘K)와 로직 공유. R14 스텁(`ToolFinder.tsx`) 흡수·정리.

**수용 기준**: 검색 직후 첫 결과 뷰포트 안(1280×800), 쿼리 에코, 키보드 전용 조작, 빈 결과 EmptyState 유지.

## Phase 4: 상세 페이지 정보 구조 (I3, P1)

**문제(실측)**: 800px 단일 칼럼 밋밋, 분류 배지가 죽은 텍스트, 17개 체인 플랫 나열, 관련 도구 없음 — 막다른 페이지.

**스펙**: ① 데스크톱 2칼럼(본문+사이드 About 그리드: GitHub ★/License/Source/Updated/Risk) ② function/asset_class/actor/체인 배지 → 필터 링크 칩 ③ 체인 8개 초과 "+N more" 접기 ④ R6·R7 반영 ⑤ 하단 Related tools 4카드(N1 소비, N1 전엔 섹션 비노출).

**수용 기준**: 상세→목록 필터 링크 ≥3종, 모바일 스크롤 정상, `data-testid` 보존.

## Phase 5: 비교 매트릭스 (I4, P1) — 구 G6 이행

**스펙**: ① 상단 속성 매트릭스(행: Type/Status/GitHub ★/License/Updated/Install risk/Chains 교집합 하이라이트/Pricing; 열: 도구 2~4) — 차이 셀 `neutral-surface` ② 열 "×" 제거 + "+ Add tool" 타이포어헤드 ③ URL `?tools=` 동기화(공유) ④ 설치 가이드는 아래 접이식 ⑤ 모바일 첫 열 sticky + 가로 스크롤.

**수용 기준**: 2~4개 URL 공유 재현, 차이 셀 구분, 375px 파손 없음.

## Phase 6: 모바일 밀도 + 레일 (I5·I6, P2)

- **I5**: 스티키 툴바 3행(≈280px)→1행 칩 스트립(정렬 드롭다운+타입 칩+카운트), 바텀시트 드래그 제스처(60%↔full, 원설계 §5.10 복원) + `aria-modal` + 포커스 트랩.
- **I6**: 레일 약어→lucide(Function=`layers`, Asset Class=`coins`, Actor=`users`, Type=`plug`, Status=`badge-check`, Pricing=`tag`, Install Risk=`shield`) + 툴팁, 브랜드 클리핑 수리.

**수용 기준**: 스티키 ≤96px, 드래그 확장/축소 + Tab 갇힘, 레일 클리핑 0·툴팁 표시·기존 펼침 동작 보존.

---

## Phase 7: Stack Blueprint — 모눈종이 설계도 캔버스 (N7, P1 신규 — 오너 제안)

**컨셉**: 로그인 유저가 **모눈종이 캔버스**에 툴 카드를 드래그로 배치하고 메모를 붙여 자기 스택/워크플로 설계도를 만든다. 에이전트 스택 구상 → 그대로 설치로 이어지는 보드.

### 7.1 라우트·진입

- `/blueprints` — 내 설계도 목록(카드 그리드: 제목·노드 수·수정일) + "New blueprint"(오렌지 primary, 페이지 유일).
- `/blueprints/{id}` — 에디터. 로그인 필수(비로그인 → LoginModal, 단 **로컬 드래프트 1장**은 localStorage로 체험 가능, 로그인 시 서버로 승격 저장).
- 진입점: 프로필 메뉴 "Blueprints" 항목, `/toolkit` 상단 "Plan on a blueprint →" 링크.

### 7.2 캔버스

- **모눈종이 배경**: CSS만 — 8px 보조선 + 40px 주선, 색은 beige 톤 조화(보조 `#EFEFE8`, 주선 `#E4E4DC` 수준, 시각 소음 최소). 배경 `#FFFFFF`.
- **팬**: 휠/트랙패드 스크롤 + 빈 공간 드래그. 줌은 v1 제외(§11 오픈).
- **스냅**: 8px 그리드 스냅(드래그 종료 시).
- 캔버스 크기: 논리 4000×4000 고정(v1), 뷰포트는 자유 팬.

### 7.3 노드 2종

- **툴 노드**: 콤팩트 카드(220×64 기준) — 로고 32 + 이름 + 타입 배지, 호버 시 우상단 제거(×). 클릭 = 선택(테두리 `border-strong` + beige), 더블클릭/링크 아이콘 = `/tools/{slug}` 새 탭. 동일 툴 중복 배치 허용.
- **메모 노드**: `neutral-surface #F5F5F0` 카드(기본 220px 폭, 높이 자동), 인라인 textarea 편집(플레이스홀더 "Add a note..."), 2000자 제한. 색상 변형 없음(뉴트럴 단일 — 디자인 불변식).

### 7.4 팔레트(좌측 도킹)

- 탭 2개: **Search** / **My Toolkit**(북마크 목록). Search는 Phase 3 타이포어헤드 컴포넌트 재사용 — 단 **Phase 7을 조기 착수해 Phase 3보다 먼저 도달하면** 기존 `searchTools` API로 단순 결과 리스트를 우선 구현하고 Phase 3 완성 시 교체(블로킹 금지).
- 추가 방식: 팔레트에서 캔버스로 드래그, 또는 클릭 시 뷰포트 중앙 배치.
- 툴바(캔버스 상단): "Add note", 제목 인라인 편집, 저장 상태 인디케이터("Saved · 2s ago"/"Saving..."), Delete(선택 노드), 목록으로 돌아가기.

### 7.5 저장·데이터

- **자동 저장**: 변경 디바운스 2s PUT, last-write-wins(v1), `aria-live`로 저장 상태 알림.
- **DB 마이그레이션** (sqlx, 이후 `sqlx prepare`). ⚠️ user FK 타입·참조 대상은 **기존 `bookmarks`/`comments` 테이블의 사용자 컬럼 패턴을 그대로 미러링**(`migrations/` 확인 후 작성 — 아래는 형태 예시):
  ```sql
  CREATE TABLE blueprints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,            -- 기존 bookmarks.user_id와 동일 타입/FK/CASCADE 규칙
    title TEXT NOT NULL DEFAULT 'Untitled blueprint',
    nodes JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
  );
  CREATE INDEX idx_blueprints_user ON blueprints(user_id, updated_at DESC);
  ```
  `nodes` 원소: `{id: string, kind: "tool"|"note", slug?: string, text?: string, x: int, y: int}`.
- **API v2** (Axum, 전부 인증 필수 + **소유자 검사는 서버사이드** — 핸들러에서 세션 user_id와 row user_id 대조):
  - `GET /api/v2/blueprints` (내 목록), `POST /api/v2/blueprints` (생성)
  - `GET/PUT/DELETE /api/v2/blueprints/{id}`
  - 입력 검증: 노드 ≤120/장, 설계도 ≤20/유저, 좌표 0..4000, text ≤2000자, slug 존재 검증은 렌더 시 관대 처리(삭제된 툴 = "removed tool" 고스트 카드).
- RLS: 기존 사용자 소유 테이블(bookmarks 등)에 RLS가 적용돼 있으면 동일 정책 추가 — [SECURITY.md](SECURITY.md) 패턴 준수. RLS 유무와 무관하게 서버사이드 소유자 검사는 필수.

### 7.6 구현 기술

- 드래그: `@dnd-kit/core`(a11y 내장) 또는 포인터 이벤트 직접 — Grok 재량, **번들 증가 ≤50KB gzip**. 외부 캔버스 라이브러리(react-flow 등)는 금지 아님이나 스코프 대비 과함 — 채택 시 근거 남길 것.
- 렌더: DOM 노드(absolute positioned) — SVG/캔버스 불필요(연결선 없음 v1).
- 접근성: 노드 탭 포커스, 화살표 8px 이동(Shift=40px), Delete 삭제, Enter로 메모 편집 진입, 이동/삭제 `aria-live` 공지.
- 모바일(<1024px): v1 **읽기 전용**(팬+노드 열람) + "Editing works best on desktop" 안내 배너.

### 7.7 디자인

- 오렌지는 "New blueprint"(목록) / 에디터에선 없음(저장 자동이라 primary 액션 부재 — 규칙 위배 없음).
- 노드 그림자 금지 — 1px 보더 + 드래그 중에만 `0 2px 8px rgba(0,0,0,0.06)`.
- 빈 캔버스 상태: 중앙 안내(lucide `compass` 아이콘 + "Drag tools from the left to start planning").

### 7.8 수용 기준

- 생성→드래그 배치→메모 작성→새로고침 후 위치·내용 복원(서버 저장 확인).
- 비로그인: 드래프트 1장 편집 가능, 로그인 후 서버 승격.
- 스냅/키보드 이동/삭제 동작, 콘솔 에러 0.
- `GET /api/v2/blueprints/{타인 id}` = 403/404(소유자 검사 실측).
- 한도 초과(21번째 장, 121번째 노드) 시 명확한 에러 카피.
- 375px 읽기 전용 렌더 정상.
- data-testid: `blueprint-list`, `blueprint-canvas`, `blueprint-node`, `blueprint-add-note`, `blueprint-save-state`.

---

## Phase 8: 백로그 신규 기능 (N1~N6, Phase 0~7 후)

권장 순서: **N6 → N1 → N3 → N2 → N5 → N4** (N6 계측이 이후 효과 측정 전제).

| ID | 기능 | 요약 | 수용 기준 핵심 |
|---|---|---|---|
| N1 | Related tools (P1) | `GET /api/v2/tools/{slug}/related?limit=4` — 동일 function(가중 3)+체인 교집합(1/체인)+동일 type(1), hidden 제외, 동점 GitHub ★순. 상세 하단 + 미리보기 패널(I7 승격) | 응답 ≤100ms, 0개면 비노출 |
| N2 | ⌘K 팔레트 (P2, 구 D5 승계) | I2 로직 공유, 도구/필터 점프 + "Copy install command". 모바일 비노출 | 키보드 전용 검색→이동 |
| N3 | What's new + RSS (P2, 구 G4 부분) | `/changelog` 주간 그룹 + 홈 "New this week: N tools →" 스트립 + `feed.xml` | RSS 유효성 통과 |
| N4 | 툴킷 공유 링크 (P2) | `POST /api/v2/toolkit/share` → 불변 스냅샷 `/toolkit/s/{share_id}`, 비로그인 열람, "Save all" 로그인 유도 | 스냅샷 불변, 삭제 가능 |
| N5 | 대시보드 v2 (P2) | 타입 분포 바·체인 top10·주간 신규 8주 — CSS 바(뉴트럴) | 쿼리 ≤3개 추가, LCP 유지 |
| N6 | 사용 계측 (P2) | Vercel Web Analytics(쿠키리스) + 이벤트 5종: `search_submit`/`tool_detail_view`(유입원)/`install_copy`/`compare_view`/`toolkit_save`(+`blueprint_create` 추가) | PII 0, 스크립트 ≤5KB |

---

## 9. 검증 (전 Phase 공통 DoD)

- `cd frontend && npm run lint`. 백엔드 변경 시 `cargo check --features ssr` + `cargo clippy --features ssr -- -W clippy::all` + `cargo fmt --check` + 마이그레이션 후 `sqlx prepare`.
- 스모크: 홈/목록/상세/비교/로그인/blueprints 데스크톱 1280 + 모바일 375 스크린샷 — 실제로 찍고 결과 보고(찍지 않았으면 찍지 않았다고 보고).
- 콘솔 에러 0(하이드레이션 포함). I 트랙 완료 시 axe 주요 페이지 위반 0.
- SEO(Phase 2 후): Lighthouse SEO ≥90, sitemap 200, 상세 raw HTML 콘텐츠 존재, 프로덕션 보안 헤더 curl 확인.
- 스크롤 위치 보존(R19/I7): 목록 중간에서 미리보기 열기→위치 불변 실측.
- `data-testid` 전수 보존 + Phase 7 신규 testid 추가.
- 푸시는 `[skip ci]`, CI/리뷰봇 수동 트리거 금지.

## 10. 기존 문서와의 관계 (중복 통합 맵)

| 기존 항목 | 처리 |
|---|---|
| PRODUCT_ENHANCEMENT_SPEC **H2**(SEO) | Phase 2로 대체 — "SSR 양호" 전제 무효(전면 CSR 회귀) |
| PRODUCT_ENHANCEMENT_SPEC **D7** SSR 문장 | 무효. 글리프/마이크로카피 지적 → R5 승계 |
| PRODUCT_ENHANCEMENT_SPEC **D5**(⌘K) | N2 승계(I2와 자산 공유) |
| PRODUCT_ENHANCEMENT_SPEC **G6**(비교 뷰) | Phase 5로 이행 완성 |
| PRODUCT_ENHANCEMENT_SPEC **G4**(구독/알림) | N3가 RSS만 선행, 나머지 유보 |
| PRODUCT_ENHANCEMENT_SPEC **G1**(컬렉션) | N4·N7과 별개(운영자 큐레이션). 공개 read-only 페이지 인프라는 공유 가능 |
| USER_FRIENDLY **Feature A**(Tool Finder) | 미이식 확인(R14) — quick-match는 Phase 3 흡수, 위저드는 §11 |
| USER_FRIENDLY **Feature B/E/F** | B→Phase 5, E→N4, F→Phase 3. 빈상태 이식 확인(실측) |
| UI_UX_DESIGN **§5.9**(미리보기 패널) | Phase 1(I7)이 원설계(비모달 도킹)로 복원+강화 — 완료 후 §5.9 갱신 필요 |
| UI_UX_DESIGN **§5.10**(바텀시트 드래그) | Phase 6 I5가 복원 |
| PRODUCT_ENHANCEMENT_SPEC **F2**(apex→www) | 해결 확인(실측 307) — 조치 불요 |
| PRODUCT_ENHANCEMENT_SPEC **D3**(접근성) | 유효하나 Leptos 참조 무효 — Next 기준 재감사(R17이 첫 항목) |
| PRODUCT_ENHANCEMENT_SPEC **C3**(개인 핸들) | About 해소 확인, TopNav 잔존 → R18 |
| SECURITY.md 웹 보안 헤더 절 | R13이 이행. "Vercel 프론트 헤더는 next.config 담당" 1줄 추가 |
| PRODUCT_ENHANCEMENT_SPEC D1(토큰)·D6(i18n)·C 잔여·E·F1 | 범위 밖 — 유효, 변경 없음 |

## 11. 오픈 결정 (Grok이 구현 중 만나면: 기본값 채택 + 결정 로그 남기고 진행)

### 결정 로그 (2026-07-03 실행)

| # | 결정 | 채택값 |
|---|---|---|
| 1 | Phase 2 ISR | `revalidate = 120` on tool detail page |
| 2 | 검색 모드 헤더 | 컴팩트 1줄 (`SearchModeHeader`) |
| 3 | Tool Finder 위저드 | 폐기 — `ToolFinder.tsx` 삭제 |
| 4 | 홈 카테고리 그리드 | **제거 확정** — `CategoryGrid` 삭제, 카테고리 진입은 사이드바 Function 필터 담당 |
| 5 | N7 줌 | v1 제외 |
| 6 | N7 공유 링크 | v2 유보 |
| 7–10 | 기타 | 명세 기본값 그대로 |

1. Phase 2 렌더 전략: ISR(`revalidate` 60~300s) vs 매 요청 SSR — **기본값 ISR 120s**.
2. Phase 3 검색 모드 히어로: 접기 vs 컴팩트 헤더 — **기본값 컴팩트 1줄 헤더**.
3. Tool Finder 가이드 위저드(R14): 부활 vs 폐기 — **기본값 폐기**(I2+N2로 충분, 데드 코드 삭제 + USER_FRIENDLY Feature A 종결 표기).
4. 홈 카테고리 그리드(R15): 복원 vs 제거 — **제거 확정**(오너 의도, 2026-06-26). 카테고리 진입은 사이드바 Function 필터·`/categories/:id`·`/tools?function=` 담당. `CategoryGrid.tsx` 삭제.
5. N7 줌: v1 제외 유지 vs 50~150% 추가 — **기본값 v1 제외**.
6. N7 공유(read-only 링크): v1 제외, N4 패턴으로 v2 — **기본값 v2 유보**.
7. N1 related 가중치 3:1:1 — 초기값, N6 데이터 후 조정.
8. N4 스냅샷 보존 기한 — **기본값 무기한**(삭제 가능하므로).
9. N6 도구: Vercel Web Analytics vs 자체 테이블 — **기본값 Vercel WA**.
10. 사이드바 레일 아이콘 세트(I6) — 명세대로, 시안 검토 1회 권장.

## 완료 조건

- Phase 0~6 전부 + Phase 7(Stack Blueprint) 동작 — §9 검증 전 항목 통과 증거(명령 출력·스크린샷) 첨부.
- Phase 8은 별도 사이클 가능(N6·N1 우선).
- 문서 후속: UI_UX_DESIGN §5.9 갱신, SECURITY.md 1줄, 폐기 결정 시 USER_FRIENDLY Feature A 표기 — 코드와 같은 PR에서.
