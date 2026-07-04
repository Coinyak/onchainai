# OnchainAI 고도화 스펙 (Product Enhancement Spec)

> 작성일: 2026-06-28 · 대상: 기능/MCP/플러그인 확장, UI 유저 친화화, 시스템 견고화·불순물 제거·코드 최적화
> 범위: 현재 코드베이스(`src/` 29.4k LOC, 서버펑션 59개, 크롤러 소스 4종, MCP 툴 5종, 마이그레이션 22개) 진단 → 우선순위 기반 개선 스펙.

이 문서는 "무엇을 왜 어떻게"를 한 번에 본다. 각 항목은 **근거(파일:라인)** → **목표** → **작업** → **수용 기준** 순서로 적었다. **우선순위의 단일 출처는 바로 아래 "북극성 & 실행 우선순위"** 다(섹션 A~K는 상세 정의, §3은 의존성 순서). 검증 장치는 [docs/VERIFICATION.md](docs/VERIFICATION.md) + `scripts/spec-verify.sh`.

> 2026-06-29 재정렬: 23개 P1을 **Top 5 트랙**으로 압축, 중복 통합, 수익화(K) 보류.

---

## ★ 북극성 & 실행 우선순위

**북극성**: 크립토 에이전트가 *필요한 툴을 신뢰할 수 있게 발견·설치*하는 1순위 디렉터리.
**차별화(wedge)**: 규모(mcp.so 19k·Glama 49k)로 싸우지 않는다 → **크립토 특화 × 큐레이션 × 신뢰/설치안전**. 경쟁사가 안 하는 관련도 게이트·`install_risk`·x402 검증·운영자 리뷰가 무기.
**현실 직시**: 3일 된 솔로 제품. P1은 23개가 아니라 **5개**다.

### 지금 (Top 5 트랙)
1. **인증·도메인 정상화** (F1·F2·H1) — 로그인/지갑이 깨지면 나머지가 다 무의미. 사용자 차단 해제(대부분 설정 변경).
2. **발견 커버리지** (B1 레지스트리화 + B2 소스 1~2개: 공식 MCP Registry·awesome-mcp) — "수동으로 찾는" 문제의 구조적 해결.
3. **채택 동선** (D4 + J1 Skill → 이후 J2 Plugin) — MCP를 "붙이고 능동적으로 쓰게". *D4=A5=J1/J2는 한 트랙.*
4. **검색 품질 Tier 0** (A4 Tier0 = C1) — 툴 description·결과 랭킹. 임베딩 아님.
5. **신뢰 카드** (C2 = D7 = I1 통합) — 카드에서 `install_risk`·x402검증·완성도를 한눈에. 차별화의 시각화.

### 다음 (Next)
A2 MCP 툴 확장 · C4 TradFi/WebSocket · I4 에이전트 호환필터 · G1 컬렉션 · G2 헬스배지 · G3 클레임 · H2 SEO · H3 크롤러 관측성 · E2 functions 분할 · E3 패닉감사 · D3 접근성

### 나중 (Later)
A4 Tier2 임베딩 · 관측성·헬스(A3+H3+E5 통합) · B2 잔여 소스 · B3 견고성 · D1 토큰 · D2 분해 · D5 ⌘K · D6 i18n · E4 의존성 · E6 dead_code · G4~G6 · H4·H5 · I2 트렌딩 · I3 평점 · J3 export

### 보류 (Deferred)
**K(수익화) 전체 — 트래픽·사용량 확보 후.** 3일 된 제품은 사용자가 먼저다. 어트리뷰션 인프라(K1)는 이미 있으니 *끄지 않고 데이터만 쌓되*, 노출/프리미엄 게이트(K2·K3)는 수요 생긴 뒤.

### 중복 통합 맵 (같은 작업 — 한 번만 구현)
| 통합 트랙 | 합쳐진 항목 |
|---|---|
| 검색 품질 | A4(Tier0/1) + C1 |
| 신뢰 카드 | C2 + D7 + I1 |
| 채택/배포 | D4 + A5 + J1 + J2 |
| 관측성 | A3 + H3 + E5 |
| canonical | F2 + H1 (동일 apex→www) |

> **표기 규칙**: 각 항목 본문의 인라인 `(P0/P1/P2)`는 *원래 심각도(참고용)*다. **현재 실행 우선순위의 정본은 위 "지금/다음/나중/보류"** 다. 충돌 시 위가 이긴다.

---

## 0. 현재 상태 스냅샷

| 영역 | 현황 | 신호 |
|---|---|---|
| 아키텍처 | Rust 단일 바이너리: Leptos SSR + Axum + sqlx + tokio-cron | 견고 |
| 테스트 | `#[test]`/`#[tokio::test]` 약 399개 | 양호 |
| 기술부채 마커 | `TODO/FIXME/HACK` **0건** | 양호 |
| MCP 서버 | 손수 짠 JSON-RPC, 툴 5종, HTTP POST 단일 | 개선 여지 |
| 크롤러 소스 | cryptoskill, github, npm, web3mcp (4종, 하드코딩 등록) | 확장 여지 |
| 서버펑션 | **`functions.rs` 단일 파일 4843 LOC에 59개 집중** | 분할 필요 |
| 테마 | 하드코딩 hex 400+개(`color-scheme: light only`) | 유지보수 부채 (다크모드는 범위 외) |
| 리포 위생 | ~~루트에 `.playwright-*.yaml` 17개 + `.railway-config-pull-5114`~~ | ✅ 제거됨 (2026-06-28) |
| 의존성 | ~~`rmcp`(server, transport-sse) 미사용~~ | ✅ 제거됨 (2026-06-28) |
| MCP 검색 | 풀텍스트(tsvector) + stars 정렬, **임베딩/시맨틱 없음** | A4 참조 (단계적) |
| 분류 축 | function(FK, 14) × asset_class(crypto/rwa/derivatives/stablecoins) × type(mcp/cli/sdk/api/x402/skill) | TradFi·WebSocket 추가 → C4 |

---

## 1. 진단 (Findings)

### F1. `rmcp` 의존성이 죽어 있다
- 근거: `Cargo.toml`의 `ssr` 피처가 `rmcp = { features=["server","transport-sse"] }`를 끌어오지만 `grep -rn "rmcp" src` → **0건**. MCP는 [src/server/mcp.rs](src/server/mcp.rs)에서 raw JSON-RPC로 직접 구현.
- 영향: 빌드 시간·바이너리·`Cargo.lock` 트리에 불필요한 무게. 표준 MCP 기능(SSE 스트리밍, resources/prompts, 페이지네이션)도 못 받음.

### F2. MCP 서버가 프로토콜 표면이 얕다
- 근거: [src/server/mcp.rs:91-101](src/server/mcp.rs) — `initialize`/`tools/list`/`tools/call`만. `protocolVersion`이 `"2024-11-05"` 하드코딩, `capabilities`는 `{ "tools": {} }`만. SSE 없음, `resources`/`prompts` 없음, `tools/list` 페이지네이션 없음.
- 근거: `search_tools`는 `ORDER BY stars DESC LIMIT 50` 고정([src/server/mcp.rs:286](src/server/mcp.rs)), 커서/오프셋·정렬 옵션·관련도 랭킹 없음.
- 영향: 에이전트가 "내 툴킷 내보내기 / 툴 비교 / 다음 페이지 / 변경 구독"을 못함. 발견 품질이 별 수에만 의존.

### F3. 크롤러 소스가 정적으로 하드코딩됨 + 트레이트가 dead-code 취급
- 근거: [src/crawler/sources/mod.rs:46-66](src/crawler/sources/mod.rs) — `SourceCrawler` 트레이트에 `#[allow(dead_code)]`와 `_SourceCrawlerSealed`라는 워닝 억제 해킹. 소스는 `cryptoskill/github/npm/web3mcp` 4개로 모듈 고정.
- 누락 소스: Smithery, mcp.so, 공식 MCP Registry(modelcontextprotocol/registry), PyPI, crates.io, Docker Hub, Awesome-MCP 류. README가 "fragmented across CryptoSkill, Smithery, npm..."라고 적었지만 Smithery 크롤러는 없음.
- 영향: 커버리지 정체. 새 소스 추가 시 트레이트 마찰(워닝 해킹)이 진입장벽.

### F4. 테마가 하드코딩 hex 컬러로 도배됨 (유지보수 부채)
- 근거: `src/**.rs`에 hex 리터럴 400+개 — `#E5E5E5`(136), `#6B6B6B`(106), `#C0392B`(46), `#E76F00`(15)… [src/app.rs:163](src/app.rs)는 `color-scheme: light only`, CSS 변수/토큰 사용 0건.
- 영향: (1) 브랜드 컬러 한 번 바꾸려면 수백 곳을 손으로 수정. (2) WCAG 대비를 코드 차원에서 보장하기 어려움. (3) 색이 미묘하게 어긋나도(예: 거의 같은 회색 여러 종) 잡기 힘듦.
- 참고: **다크모드는 제품 범위에서 제외**(2026-06-28 결정). 이 항목은 라이트 테마 한정 유지보수성 개선이며, 우선순위는 낮음(선택).

### F5. `functions.rs` 4843 LOC 단일 파일에 server fn 59개
- 근거: [src/server/functions.rs](src/server/functions.rs) — tools/dashboard/admin/comments/featured/referral/crawler/users가 한 파일에 혼재.
- 영향: 컴파일 단위 비대(증분 빌드 느림), 리뷰·소유권·충돌 위험, 도메인 경계 흐려짐.

### F6. 리포에 스크래치 아티팩트 커밋됨 (불순물)
- 근거: 루트에 `.playwright-{add,clean,del,dns,dns2,dns3,dns4,dns-new,after-cname,edit-www,external,form,forward,fwd,fwd2,portfolio,reload}.yaml` ≈17개(각 6~30KB, DNS/폼 자동화 녹화) + `.railway-config-pull-5114/` 디렉터리. `git ls-files`에 추적됨.
- 영향: 리포 노이즈, 비밀/내부 DNS 노출 위험, 신규 기여자 혼란. `AGENTS.md` "Never commit … stray …" 정신에 위배.

### F7. 문서·기능 드리프트
- 근거: [src/app.rs:206-207](src/app.rs) About 페이지: "MVP does not include self-service registration yet" — 그러나 `/submit` 라우트와 `SubmitPage`가 실재([src/app.rs:177](src/app.rs), [src/pages/submit.rs](src/pages/submit.rs)). 또한 GitHub 이슈 URL이 개인 핸들 `Coinyak` 하드코딩.
- 영향: 신뢰도 저하, 깨진 안내.

### F8. 견고성 신호: 핫패스 `expect`/`unwrap` 점검 필요
- 근거: `.expect(`가 [src/server/review_persistence.rs](src/server/review_persistence.rs) 27건, [src/app.rs](src/app.rs) 9건 등. 다수는 테스트지만 런타임 경로 혼입 여부 감사 필요.
- 영향: 패닉 = 요청 단위 500 또는 (위치에 따라) 워커 중단 가능.

---

## 2. 개선 스펙

### A. MCP 서버 고도화

**A1. `rmcp` 제거 — ✅ 완료 (2026-06-28)**
- 현행 raw JSON-RPC가 잘 동작하고 테스트가 탄탄(`mcp.rs` 단위테스트 다수)하므로 미사용 `rmcp`(server, transport-sse)를 제거.
- 실행: `Cargo.toml` `ssr` 피처에서 `dep:rmcp` + 의존성 라인 삭제 → `cargo check --features ssr` **exit 0** 확인.
- 결과: `grep rmcp Cargo.toml` 0건, 컴파일 무영향(코드에서 미사용이었음).

**A2. MCP 툴 표면 확장 (P1)**
- 신규 툴:
  - `compare_tools(slugs: [string])` — 2~5개 툴을 trust/x402/chains/stars 축으로 비교 표 반환.
  - `export_toolkit(slugs|category)` — 에이전트가 한 번에 설치 가능한 설치 묶음(JSON + markdown). 기존 [build_toolkit_payload](src/server/functions.rs:1205) 재사용.
  - `get_changes(since: ISO8601)` — 신규/갱신 툴 델타(폴링 기반 "구독" 대용).
- `search_tools` 개선: `limit`/`cursor`/`sort`(`stars|trust|recent|relevance`) 파라미터 추가, 응답에 `next_cursor`·`total`. 현재 50 고정 제거.
- `tools/list`에 표준 `nextCursor` 페이지네이션 골격.
- 수용 기준: `tools/list` 길이 테스트 갱신, 각 신규 툴 단위테스트, 커서 왕복 테스트.

**A3. MCP 관측성 + 버전 협상 (P2)**
- `initialize`에서 클라이언트 `protocolVersion` 에코/협상(하드코딩 제거).
- 호출 카운트/지연/에러율을 `referral_events`와 별개의 `mcp_usage` 집계로 기록(이미 IP 레이트리밋 존재 — [src/server/rate_limit.rs](src/server/rate_limit.rs) 옆에 메트릭 추가).
- 수용 기준: `/healthz` 또는 admin 대시보드에 MCP 호출 메트릭 노출.

**A4. 에이전트의 "인식"과 검색 품질 — 임베딩이 필요한가? (핵심 질문)**

> 질문: 에이전트가 OnchainAI MCP를 *효율적으로 쓰고 인식*하게 하는 로직이 있나? 임베딩이 필요한가?
> 현황: **없음.** 검색은 Postgres 풀텍스트(`to_tsvector`/`plainto_tsquery`, 'english') + `ORDER BY stars`([src/server/queries.rs:79-88](src/server/queries.rs)). 임베딩·시맨틱·오타허용·동의어 전부 없음. MCP 툴 description은 한 줄 짜리("Search crypto MCP/CLI/SDK/API tools").

3단계로 본다. **임베딩은 필수가 아니라 마지막 단계**다 — 더 싸고 효과 큰 게 앞에 있다.

- **Tier 0 — 툴 description·결과 구조 (가장 큰 ROI, 인프라 0).** 에이전트가 이 툴을 *부를지 말지*는 LLM이 `description`+`inputSchema`만 보고 정한다. 지금 description이 빈약해서 호출 정확도를 깎는다. 예시 포함 설명("능력으로 크립토 툴 찾기 — 예: 'bridge USDC to Base', 'Uniswap MCP 서버'"), 파라미터 설명, 그리고 결과 상위 1~2개가 거의 정답이 되도록 랭킹. 에이전트는 보통 첫 결과를 집으므로 **Top-1 정확도**가 곧 체감 품질.
- **Tier 1 — 어휘 검색 + 랭킹 개선 (저비용, 고효과).** 동의어/별칭 확장(DEX↔swap, perp↔derivatives, KMS↔wallet), `pg_trgm` 오타 허용, 별 수 단독 정렬을 **합성 점수(관련도×trust×최근성×stars)**로 교체.
- **Tier 2 — 하이브리드 시맨틱(임베딩).** MCP의 청중은 *자연어로 질의하는 에이전트*("내 에이전트가 API 호출 비용을 내게 해줘")라서 어휘검색이 못 잡는 의도 매칭에 임베딩이 진짜 값을 한다. Supabase가 **pgvector 지원** → 임베딩 컬럼 + HNSW 인덱스 + 크롤 시 재임베딩, 질의 시 FTS와 벡터를 **RRF(reciprocal rank fusion)**로 합치는 하이브리드가 정석. 툴 수가 수백~수천 규모면 비용·지연 모두 작음.
- **판정**: 오늘 당장은 **불필요**. 순서는 Tier 0(설명/랭킹) → Tier 1(어휘/오타/동의어) → Tier 2(임베딩). Tier 0/1만으로 체감 품질의 대부분을 얻고, 그 다음 임베딩.

**A5. MCP "발견" = 외부 레지스트리 등재 (배포)**
- 에이전트가 *OnchainAI MCP 자체*를 찾게 하려면, 우리 MCP를 공식 MCP Registry·Smithery·Glama·mcp.so·PulseMCP에 **등재**해야 한다(아래 B2 소스들이 곧 우리의 배포 채널이기도 함).
- 수용 기준: 최소 공식 Registry + Smithery에 OnchainAI MCP 엔트리 등록.

### B. 플러그인/크롤러 소스 확장

**B1. 소스 레지스트리화 — 트레이트 마찰 제거 (P1)**
- `SourceCrawler` 등록을 `Vec<Box<dyn SourceCrawler>>` 레지스트리로 모으고 스케줄러가 순회. `#[allow(dead_code)]`/`_SourceCrawlerSealed` 해킹 삭제([src/crawler/sources/mod.rs:46-66](src/crawler/sources/mod.rs)).
- 소스별 메타(활성/주기/마지막 성공)를 `sources` 테이블과 admin Crawler 페이지에 일원화([src/pages/admin/crawler.rs](src/pages/admin/crawler.rs) 이미 존재).
- 수용 기준: 새 소스 추가가 "파일 1개 + 레지스트리 1줄"로 끝나고 워닝 0.

**B2. 신규 소스 추가 — 조사 결과 (2026-06-28 웹 리서치 기반)**

현재 소스 4종(cryptoskill, github, npm, web3mcp) 외 추가 후보. 각 소스는 기존 normalizer/relevance/deduper 파이프라인 재사용([src/crawler/normalizer.rs](src/crawler/normalizer.rs), [relevance.rs](src/crawler/relevance.rs), [deduper.rs](src/crawler/deduper.rs)). 크롤 후 `relevance` 게이트가 비크립토를 걸러내므로 범용 레지스트리도 안전.

*그룹 1 — MCP 레지스트리 (API 있음, 최우선)*
| 소스 | 규모/특징 | 수집 방식 |
|---|---|---|
| 공식 MCP Registry (`registry.modelcontextprotocol.io`) | 표준 스펙, 앱스토어 격 | REST API |
| Smithery (`smithery.ai`) | 7,000+ 서버 + **skills**, "MCP의 Docker Hub", CLI/API | Registry API |
| Glama (`glama.ai/mcp/servers`) | 49,000+ 서버 최대 규모 | API |
| mcp.so / PulseMCP / servemcp.com | 커뮤니티 제출형 | API/스크랩 |
| awesome-mcp-servers `finance--crypto.md` (TensorBlock) | 크립토 큐레이션 마크다운 | GitHub raw |

*그룹 2 — web3 개발자 툴 디렉터리/패키지*
| 소스 | 규모/특징 |
|---|---|
| The Grid (`thegrid.id`) | dev-tooling 458개 회사 |
| Alchemy dapps (`alchemy.com/dapps`) | web3 dev tool 401개 |
| awesome-web3 (ahmet/awesome-web3) | 큐레이션 GitHub 리스트 |
| crates.io / PyPI / Docker Hub | 패키지 레지스트리(크립토 SDK·이미지) |

*그룹 3 — 시드로 우선 등재할 만한 개별 툴 (조사 중 포착)*
- **Orbis API Marketplace** — 20,200+ API, **x402 USDC(Base/Solana) 마이크로페이먼트** → OnchainAI x402 축과 직결.
- **Lambda Finance** — 멀티에셋 MCP(크립토+주식+매크로) 198 tools → **TradFi 크로스오버** 증거(아래 C4).
- **CoinGecko MCP** — 200+ 체인·8M+ 토큰 가격/마켓.
- **thirdweb / Alchemy** — CLI + MCP + skills 풀스택.
- **AIXBT CLI / web3cli** — 터미널 크립토 인텔리전스.
- **Foundry / Hardhat / Viem / Ethers.js** — dev-tool 표준(누락 시 보강).

- 우선순위: ① 공식 MCP Registry ② Smithery(skills 포함) ③ awesome-*crypto* GitHub 리스트 ④ Glama/mcp.so ⑤ crates.io/PyPI/Docker Hub.
- 수용 기준: 소스별 픽스처 기반 파싱 테스트 + dedup 충돌 테스트.
- **근본 원인 확인(2026-06-28)**: 크롤러는 실제로 돌지만([lib.rs:349](src/lib.rs)) 소스가 4개(cryptoskill/npm/web3mcp/github-topics)뿐이라 **공식 Registry·Smithery·awesome-mcp 리스트가 사각지대**. awesome-mcp `finance--crypto.md` 한 곳에만 크립토 MCP **388개** 존재. 이게 "수동으로 찾게 되는" 직접 원인.
- 상세 설계·큐레이션 시드 후보(Base MCP·Coinbase AgentKit 등 ~50개)·AI 발견 파이프라인: **[docs/TOOL_DISCOVERY.md](docs/TOOL_DISCOVERY.md)** 참조.
- 출처: [공식 MCP Registry](https://github.com/modelcontextprotocol/registry) · [Smithery CLI](https://github.com/smithery-ai/cli) · [Glama](https://glama.ai/mcp/servers) · [awesome-mcp finance/crypto](https://github.com/TensorBlock/awesome-mcp-servers/blob/main/docs/finance--crypto.md) · [The Grid dev tooling](https://thegrid.id/discovery/productType/developer-tooling) · [Alchemy dapps](https://www.alchemy.com/dapps/top/developer-tools) · [awesome-web3](https://github.com/ahmet/awesome-web3)

**B3. 크롤 견고성 (P2)**
- ETag/Last-Modified 조건부 요청, 소스별 백오프, 부분 실패 격리(한 소스 실패가 다른 소스 결과를 막지 않음 — 이미 orchestrator 지향). 이미 30s 타임아웃/UA 존재([src/crawler/sources/mod.rs:21-39](src/crawler/sources/mod.rs)).
- 수용 기준: 소스 1개 강제 실패 주입 시 나머지 정상 반영되는 통합테스트.

### C. 기능 고도화

**C1. 발견(Discovery) 품질 (P1)**
- 검색 랭킹을 별 수 단독 → `trust_score`·최근성·관련도 가중 합성으로. 동의어/별칭(예: "wallet" ↔ "kms")·오타 허용(trigram) 검토.
- 카테고리/체인/타입 다축 필터 URL 상태 보존(이미 [src/filter_query.rs](src/filter_query.rs) 존재 — 활용 확대).

**C2. 신뢰·안전 신호 강화 (P1)**
- 설치 위험도(`install_risk_level`)·x402 검증 배지·official/claimed 상태를 카드/상세/ MCP 응답에서 **일관 표기**(이미 [src/components/tool_trust_facts.rs](src/components/tool_trust_facts.rs), [trust_verification.rs](src/trust_verification.rs) 존재).
- `critical` 위험 차단 정책을 MCP·웹·문서에서 동일 카피로 통일.

**C3. 셀프서비스 제출 플로우 마감 (P0, 작음)**
- About 페이지 카피를 실제 `/submit` 기능에 맞게 수정, 개인 GitHub 핸들 하드코딩 제거([src/app.rs:206-216](src/app.rs)).
- 수용 기준: About → Submit 동선이 깨지지 않고, 조직 핸들/설정값으로 치환.

**C4. TradFi 자산군 + WebSocket 타입 추가 (P1) — 사용자 요청**

분류 체계 확인 결과 둘 다 **기존 브라우징 축에 그대로** 들어간다. `function`만 `categories` 테이블 FK이고, `asset_class`/`type`은 `tools`의 자유 텍스트 컬럼이라 **DB 마이그레이션이 필요 없다**(분류기 키워드 + UI 라벨/패싯만 추가).

- **TradFi → 새 `asset_class` 값 `tradfi`** (기존 `crypto`/`rwa`/`derivatives`/`stablecoins`에 추가).
  - 근거: [classify_asset_class](src/crawler/normalizer.rs:219)가 텍스트로 자산군 분류, `asset_class`는 이미 필터 축([src/filter_query.rs:145](src/filter_query.rs), [tools_browser.rs:69](src/components/tools_browser.rs)).
  - 작업: `classify_asset_class`에 `tradfi` 분기 추가(키워드: equities, stock, tokenized stock, brokerage, etf, forex, treasury, bond, securities, RWA-of-equity 등) — **`rwa`보다 먼저** 평가할지 경계 정의 주의. 표시 라벨/패싯 옵션에 "TradFi" 노출.
  - 시드 근거: Lambda Finance(크립토+주식+매크로), Finnhub(주식/포렉스/크립토), Alpaca(크립토+주식 브로커리지), Kaiko(기관급) — 실제 TradFi×크립토 크로스오버 존재.
- **WebSocket → 새 `type` 값 `websocket`** (기존 `mcp`/`cli`/`sdk`/`api`/`x402`/`skill`에 추가).
  - 근거: `type`은 이미 대시보드 패싯([DASHBOARD_TYPE_COUNTS_SQL](src/server/queries.rs:90)). WebSocket은 전송/인터페이스 타입.
  - 작업: 타입 분류기에 `websocket` 분기(키워드: websocket, `ws://`, `wss://`, streaming, real-time feed, subscription, orderbook stream, mempool stream). 라벨 "WebSocket / Streaming".
  - 시드 근거: Bitquery(GraphQL subscription·mempool 스트림), mempool.space WS, Coinbase/Binance WS, Finnhub WS.
- 검증 포인트: 패싯 옵션이 **데이터 파생인지 하드코딩 리스트인지** 확인 — 하드코딩이면 sidebar/filter 옵션에 두 값 추가 필요.
- 수용 기준: 새 키워드의 분류 단위테스트(예: "tokenized Apple stock"→tradfi, "binance orderbook websocket"→websocket), `/tools?asset_class=tradfi`·`/tools?type=websocket` 필터 동작, 대시보드 패싯 노출.

### D. UI/UX 유저 친화화

**D1. 디자인 토큰 정리 — 라이트 한정, 유지보수 개선 (P2, 선택)**
- ⚠️ **다크모드는 범위 외**(2026-06-28 결정). 이 항목은 다크모드를 만들지 않고, 흩어진 hex를 한 곳에서 관리하기 위한 유지보수 작업이다. 안 해도 제품 기능에는 영향 없음.
- CSS custom properties 토큰 도입(`--color-bg`, `--color-fg`, `--color-muted`, `--color-border`, `--color-accent`, `--color-danger`, `--color-success`). 현재 hex(`#E5E5E5`→border, `#6B6B6B`→muted, `#E76F00`→accent, `#C0392B`→danger …)를 토큰으로 매핑하되 **값은 현행 라이트 컬러 그대로**(시각 변화 0).
- 단계: ① 토큰 정의 + 라이트값 = 현행값 ② hex→토큰 기계적 치환(고빈도부터: 보더/뮤트/액센트). 다크 팔레트·토글은 만들지 않음.
- 수용 기준: 시각 스냅샷 회귀 없음, `ui-change-gate.sh` 그린. 색 변경 시 토큰 1곳만 수정으로 충분.

**D2. 핵심 컴포넌트 분해 (P2)**
- `tools_browser.rs` 1142 LOC, `sidebar.rs` 736 LOC를 하위 컴포넌트로 분해(필터바/결과그리드/페이저). `ui-component-patterns` 스킬 기준 적용.
- 수용 기준: 동작 동일, `data-testid` 보존, 컴포넌트 단위 가독성↑.

**D3. 접근성·반응형·로딩 상태 (P1)**
- 스켈레톤/빈상태/에러상태 컴포넌트 이미 존재([skeleton.rs](src/components/skeleton.rs), [empty_state.rs](src/components/empty_state.rs), [error_state.rs](src/components/error_state.rs)) — 모든 비동기 패널에 일관 적용 확인.
- 키보드 포커스/aria-label 감사(`web-accessibility` 스킬), 모바일 bottom-sheet 동선 점검([bottom_sheet.rs](src/components/bottom_sheet.rs)).
- 수용 기준: axe 위반 0(주요 페이지), 모바일 360px 레이아웃 깨짐 없음.

**D4. 첫 방문 온보딩 + "내 에이전트에 연결" 동선 (P1, 유저 친화 최대 레버)**
- 처음 온 사람이 "이게 뭐고 어떻게 쓰지?"를 3초에 이해하도록: 홈 히어로에 1줄 가치제안 + "에이전트에 OnchainAI MCP 연결" CTA(Claude/Cursor 탭별 복사용 config — 이미 [copy_button.rs](src/components/copy_button.rs)/install guide 자산 존재).
- 툴 상세에 "이 툴을 [Claude]/[Cursor]에 추가" 탭(플랫폼별 1-클릭 복사).
- 수용 기준: 신규 방문자가 로그인 없이 "연결 방법"에 1클릭 도달, 카피 동작.

**D5. ⌘K 커맨드 팔레트 / 즉시 검색 (P2)**
- 어디서든 ⌘K로 툴·카테고리·체인 즉시 검색·점프(에이전트 친화 + 파워유저). 기존 [search_bar.rs](src/components/search_bar.rs) 로직 재사용.
- 수용 기준: 키보드만으로 검색→상세 이동, 모바일에선 비노출/대체.

**D6. 한국어 i18n (P2, 타깃 사용자 적합)**
- 운영자·1차 사용자층이 한국어([OPERATOR_GUIDE.md](docs/OPERATOR_GUIDE.md)도 한국어). UI 문자열을 i18n 키로 추출하고 ko/en 토글. SSR이라 `Accept-Language`/쿠키 기반 초기 언어 선택 가능.
- 수용 기준: 핵심 페이지(홈/목록/상세/로그인) ko/en 전환, 하드코딩 문자열 0.

**D7. UI 폴리시 디테일 — 코드 실측 발견 (P1, 저비용)**
- **아이콘 일관성 위반**: DESIGN.md는 "이모지 금지, Lucide SVG"인데 [tool_card.rs:190,237](src/components/tool_card.rs)가 별점 `★`·북마크 `☆/★`를 **텍스트 글리프**로 사용. 게다가 별점 `★`과 북마크 `★`이 **같은 글리프 → 의미 충돌**. → 별점은 star 아이콘, 북마크는 bookmark 아이콘(SVG)으로 분리.
- **마이크로카피**: `"comments "{count}`("comments 3" 어색), `"No description."`, 누락 시간 `"—"`. → "3 comments"·아이콘+숫자, 설명 없을 때 더 나은 placeholder.
- **디자인 토큰 미사용**: DESIGN.md frontmatter에 토큰 정의가 있는데 코드는 하드코딩 hex 400+개(D1). → 토큰화 시 DESIGN.md 값과 1:1.
- **배지 과적재**: status + type + (claimed | verified-install) 배지가 모바일에서 난잡 가능 → 개수/우선순위 제한.
- **카드 신뢰 신호 약함**: `install_risk_level`·x402 검증을 카드에서 색/배지로 더 강하게(C2/I1 연계) — 차별화 축인데 현재 카드에선 약함.
- 수용 기준: 글리프→SVG 교체, 마이크로카피 정리, 모바일 배지 줄바꿈 없음, `ui-change-gate.sh` 그린.
- 참고: SSR은 정상 확인됨(홈 raw HTML 228KB, 카드 56개 서버 렌더) — SEO 토대는 양호, H2는 sitemap/OG/JSON-LD 보강만.

### E. 견고화 · 불순물 제거 · 최적화

**E1. 리포 위생 — stray 아티팩트 제거 — ✅ 완료 (2026-06-28)**
- `.playwright-*.yaml` 17개 + `.railway-config-pull-5114/` 추적 해제(`git rm`). 스크립트/CI 참조 0건·비밀정보 0건 사전 확인.
- 결과: `git ls-files | grep -E "playwright-|railway-config"` **0건**. (남은 작업) `.gitignore`에 `'.playwright-*.yaml'`, `'.railway-config-*'` 추가는 재발 방지용으로 다음 커밋에 포함.

**E2. `functions.rs` 도메인 분할 (P1)**
- `server/functions/{tools,dashboard,admin,comments,featured,referral,crawler,users}.rs`로 분할하고 `functions/mod.rs`에서 re-export(외부 호출부 무변경).
- 수용 기준: 각 모듈 <800 LOC, `#[server]` 매크로 라우팅 동일, 전체 테스트 그린, 증분 빌드 단축 측정.

**E3. 패닉 안전 감사 (P1)**
- 런타임 경로의 `unwrap()/expect()`를 `?`/`ServerFnError` 변환으로. 정적 단언(컴파일타임/테스트)만 `expect` 잔존 허용.
- 수용 기준: 비테스트 모듈 핫패스 `unwrap/expect` 0(또는 주석으로 불변식 증명), clippy 그린.

**E4. 의존성·빌드 다이어트 (P2)**
- `rmcp` 제거(A1)와 함께 `cargo-udeps`/`cargo machete`로 미사용 크레이트 정리. WASM 번들 크기 회귀 가드(`verify-wasm-bundle.sh` 존재).
- 수용 기준: 미사용 의존성 0 보고, WASM 번들 크기 동일/감소.

**E5. 관측성·헬스 (P2)**
- `/healthz`(DB ping 포함) + 구조적 로깅 레벨 정리(`tracing` 이미 존재). admin 대시보드에 크롤/큐/ MCP 지표.
- 수용 기준: 헬스 엔드포인트 200/503 정확, 배포 후 `post-deploy-verify.sh`와 연동.

**E6. dead_code 감사 — 크롤러 소스 정리 (P2)**
- 근거(2026-06-28 감사): `#[allow(dead_code)]` **30개**가 크롤러 소스에 집중 — github.rs 10, cryptoskill.rs 5, normalizer.rs 5, npm.rs 3, sources/mod.rs 2(트레이트 해킹), crawler/mod.rs 2, deduper.rs 1, siwx.rs 1, auth/routes.rs 1.
- 크롤러는 가동 중이므로 "꺼짐"은 아니고, **소스 내부의 미사용 헬퍼/필드/파생 구조체**가 워닝 억제로 가려진 상태. B1 레지스트리화와 함께 진짜 죽은 코드는 삭제, 살릴 건 실제 사용처에 연결.
- **미사용 의존성 정밀 감사는 `cargo machete`/`cargo-udeps` 필요**(grep 휴리스틱은 매크로/전이 의존성 때문에 신뢰 불가 — 이번 라운드 휴리스틱은 오탐으로 폐기). `rmcp`는 이미 제거됨(A1).
- 수용 기준: `#[allow(dead_code)]` 잔존은 "왜 필요한지" 주석 1줄로 정당화되거나 삭제, `cargo machete` 미사용 의존성 0.

### F. 인증 신뢰성 — GitHub OAuth / 지갑 로그인 (P0 프로덕션 버그)

> 증상(사용자 보고 2026-06-28): 지갑 연결·GitHub 로그인 오류가 잦음. GitHub에서 *"Be careful! The redirect_uri is not associated with this application."* 발생.

**F1. GitHub OAuth `redirect_uri` 불일치 (P0)**
- **메커니즘(코드 확인)**: 직접 OAuth 방식(Supabase-hosted 아님). `redirect_uri`는 [auth/routes.rs:42-55](src/auth/routes.rs) `callback_url()`이 생성 →
  - `GITHUB_REDIRECT_URI`가 있으면 그 값, 없으면 prod에서 **`https://{SIWX_DOMAIN}/auth/callback`** ([routes.rs:54](src/auth/routes.rs)).
  - `SIWX_DOMAIN`은 필수 env ([config.rs:91](src/config.rs)). 테스트가 canonical을 **`www.onchain-ai.xyz`**로 못박음 ([routes.rs:344](src/auth/routes.rs), [session.rs:228](src/auth/session.rs)).
- **라이브 실측(2026-06-28, `/auth/github` 프로빙)** — 앱 쪽은 **정상**으로 확인:
  - `https://www.onchain-ai.xyz/auth/github` → 307 → `https://github.com/login/oauth/authorize?client_id=Ov23liL5w1POM26Ywlto&redirect_uri=https%3A%2F%2Fwww.onchain-ai.xyz%2Fauth%2Fcallback&scope=read:user&...`
  - 즉 앱은 **올바른 www redirect_uri**를 전송(= `SIWX_DOMAIN` 정상). client_id는 공개값.
  - `https://onchain-ai.xyz/auth/github` (apex) → **404** (apex가 앱을 서빙하지 않음, apex→www 리다이렉트 없음).
- **확정 진단**: 앱이 `https://www.onchain-ai.xyz/auth/callback`을 보내는데 오류가 난다면 → **GitHub OAuth App(client_id `Ov23liL5w1POM26Ywlto`)에 그 콜백이 등록돼 있지 않다.** (apex `onchain-ai.xyz/...` 또는 옛 `*.up.railway.app` 콜백이 등록돼 있을 개연성). GitHub는 호스트 정확 일치 요구(www≠apex).
- **수정 (코드/ENV 변경 불필요 — ENV는 이미 올바름; GitHub 앱 설정만)**:
  1. github.com/settings/developers → Client ID `Ov23liL5w1POM26Ywlto` 앱 열기.
  2. **Authorization callback URL = `https://www.onchain-ai.xyz/auth/callback`** (정확히, 추가로 localhost dev 콜백은 별도 dev 앱에).
  3. (권장) apex `onchain-ai.xyz` → `www` 301 리다이렉트 추가(F2) → apex 404·혼선 제거.
- **남은 미지(사용자/오너만 확인 가능)**: GitHub 앱의 현재 등록 콜백 목록(비공개 설정 화면). 위 1~2로 www를 등록하면 해결.
- **수용 기준**: `https://www.onchain-ai.xyz`에서 GitHub 로그인 왕복 성공, redirect_uri 오류 0.

**F2. 지갑(SIWX)·세션 쿠키 안정화 — 동일 root-cause (P0)**
- SIWX 콜백·쿠키도 `SIWX_DOMAIN` 기반([routes.rs:99,232](src/auth/routes.rs), `cookie_secure_for_domain`/`local_dev_host`). 쿠키는 `HttpOnly; Secure(prod); SameSite=Lax`.
- 실측상 apex(`onchain-ai.xyz`)는 **404**라 사용자가 apex로 들어오면 앱을 아예 못 만남. www 진입 후에도 쿠키가 `SIWX_DOMAIN`(www)에 묶이므로, 과거 커밋("Safari auth hydration", "session hint cookie")이 가리키는 세션 불안정과 겹침.
- **수정**: **apex → www 301 리다이렉트**를 DNS/프록시(또는 앱 엣지)에서 강제해 **단일 canonical origin** 보장. 그러면 OAuth redirect_uri·SIWX·쿠키가 한 도메인으로 정렬됨.
- **수용 기준**: apex 접속이 www로 301, 지갑 로그인 재현 실패율 0, Safari 포함 세션 유지.
- 참고: 보안 규칙은 `docs/SECURITY.md` 준수(시크릿 노출 금지). 이 항목은 **설정 정렬**이지 새 코드가 아님.

### G. 추가 기능 제안 (Product)

**G1. 큐레이션 컬렉션 / 스타터킷 (P1)**
- "Base 에이전트 스타터킷", "DeFi 데이터 스택", "x402 결제 툴셋" 같은 **묶음**을 운영자가 만들고 공유(URL). 에이전트는 컬렉션을 한 번에 `export_toolkit`(A2)으로 설치.
- 기존 featured 카드 인프라([featured.rs](src/pages/admin/featured.rs)) 확장. 수용 기준: 컬렉션 CRUD + 공개 페이지 + MCP 노출.

**G2. 툴 헬스/라이브니스 신호 (P1)**
- repo archived/deprecated, 마지막 커밋 경과, npm deprecated, MCP 엔드포인트 응답성 등을 주기 점검해 **"활성/정체/위험" 배지**. 죽은 툴을 자동 강등(별 수만으로는 신선도 못 잡음).
- star sync 인프라([github.rs](src/crawler/sources/github.rs) `sync_stars`) 옆에 health sync 추가. 수용 기준: 배지 표기 + 정체 툴 랭킹 하향.

**G3. 소유권 클레임 + 공식 배지 흐름 (P1)**
- 메인테이너가 자기 툴을 **claim**(GitHub 소유 검증)하면 설명/링크/x402 메타 수정권 + "Official/Claimed" 배지. `claim_state` 컬럼·[claim_status_timeline.rs](src/components/claim_status_timeline.rs) 이미 존재 → 흐름 마감.
- 수용 기준: 클레임 신청→검증→승인 동선, 클레임 후 셀프 수정.

**G4. 구독/알림 (P2)**
- "카테고리 X에 새 툴" / "내 북마크 툴 업데이트"를 RSS·웹훅·이메일로. 에이전트는 A2 `get_changes` 폴링, 사람은 구독.
- 수용 기준: 카테고리 RSS 피드 + 북마크 변경 알림.

**G5. 공개 카탈로그 API + README 배지 (P2)**
- MCP·server fn 외 **공개 REST/JSON** 엔드포인트(`/api/tools.json` 등 읽기 전용, 레이트리밋)로 외부 통합 허용. + "Listed on OnchainAI" SVG 배지로 역링크/유입.
- 수용 기준: 문서화된 읽기 API + 배지 마크다운 스니펫.

**G6. 비교 뷰 UI (P2)**
- A2 `compare_tools`의 웹 버전 — 2~5개 툴을 trust/chains/pricing/health 축으로 나란히. 수용 기준: 비교 URL 공유 가능.

### H. 운영 측면 (Operational)

**H1. canonical 도메인 정렬 (P0, F와 연계)**
- apex `onchain-ai.xyz` 404·OAuth/쿠키 혼선 → **apex→www 301** 강제(DNS/프록시). 단일 origin으로 인증·SEO·쿠키 일원화. 수용 기준: apex 301, 라이브 스모크([smoke-test.sh](scripts/smoke-test.sh)) 통과.

**H2. SEO + AI 발견성 (P1, 성장 레버)**
- `sitemap.xml`·`robots.txt`, 툴별 OpenGraph/Twitter 카드, **JSON-LD 구조화 데이터**(SoftwareApplication). 디렉터리는 검색·AI 크롤러 유입이 생명 — 현재 SSR이라 토대는 있음.
- 수용 기준: 사이트맵 생성, 툴 상세 OG 미리보기, 리치결과 검증 통과.

**H3. 크롤러 관측성 (P1)**
- 소스별 last-success / 에러율 / 신규·갱신 카운트를 `sources` 테이블 + admin Crawler 페이지([crawler.rs](src/pages/admin/crawler.rs))에 노출. "자동 발견이 실제로 도는가"를 한눈에(현재는 깜깜이 — 사용자가 수동 발견하게 된 한 원인).
- 수용 기준: 소스별 상태 카드 + 수동 트리거 결과 표시.

**H4. 신뢰성·보호 (P1→P2)**
- 레이트리밋을 auth/submit/comment까지 확장(브루트포스·스팸; MCP IP 리밋은 이미 존재 [rate_limit.rs](src/server/rate_limit.rs)). 에러 모니터링/구조적 로그 알림 + 외부 업타임/상태 페이지.
- 수용 기준: 남용 시나리오 차단 테스트, 5xx 알림 경로.

**H5. 데이터 안전·시크릿 (P2)**
- DB 백업/복구 점검, 마이그레이션 롤백 전략, 시크릿 로테이션 정책(특히 인증 디버깅 중 노출 가능성). `docs/SECURITY.md`와 연결.
- 수용 기준: 백업 복구 리허설 1회, 시크릿 로테이션 런북.

### J. 배포 패키징 — MCP / Skill / Plugin 3계층 (사용자 제안)

> 핵심: 셋은 경쟁이 아니라 **계층**이다. MCP=런타임 백본(있음), Skill=노하우(없음), Plugin=원클릭 배포 래퍼(없음). 게다가 OnchainAI는 *디렉터리*라 skill/plugin을 **모으고·되고·내보내는** 재귀 구조가 가능.

**J1. OnchainAI Skill — SKILL.md (P1, 저비용·고효과)**
- MCP만 연결하면 에이전트는 "언제 쓰고 결과를 어떻게 해석할지"를 모름. Skill이 그 격차를 메워 **수동 API → 능동 능력**으로 전환.
- 내용: 트리거("온체인 작업이 필요한데 적합 툴이 없을 때 OnchainAI 검색"), `search_tools`/`get_install_guide` 호출법, 결과 해석 규칙(`install_risk_level`·x402 검증·official/claimed 우선), **critical 위험 차단** 규칙, 설치 안내.
- 형식: `SKILL.md`(name/description frontmatter) + 예시. Claude 앱·Claude Code·API 모두에서 동작.
- 수용 기준: 관련 상황에서 스킬이 트리거되어 OnchainAI를 호출, 위험/검증 규칙 준수.

**J2. OnchainAI Plugin — 원클릭 배포 (P1→P2)**
- 번들: `.mcp.json`(MCP 서버 `npx mcp-remote www.onchain-ai.xyz/mcp` 자동 연결) + J1 Skill + slash command(`/find-tool` 또는 `/onchain-tool`).
- Claude Code 플러그인 마켓플레이스 등재 → A5(외부 레지스트리 등재)와 함께 **채택 퍼널**. config 복붙 제거.
- 수용 기준: 플러그인 설치 시 MCP 자동 연결 + Skill 로드 + 커맨드 동작.

**J3. 발견한 툴을 Skill/Plugin으로 export (P2, 차별화)**
- 큐레이션 컬렉션(G1)을 **설치 가능한 플러그인 매니페스트로 생성** — "Base 에이전트 스타터킷" 한 방 설치.
- 카탈로그가 skill/plugin 자체도 1급 시민으로 취급(`type=skill` 이미 있음, B2에 Smithery skills 포함). type 패싯에 `plugin` 추가 검토(C4 WebSocket 추가와 동일 메커니즘, 마이그레이션 불필요).
- 수용 기준: 컬렉션 → 플러그인/스킬 번들 생성기, `type=skill|plugin` 필터.

**관계**: J1/J2는 D4(온보딩 "에이전트 연결")·A5(등재)의 *구현 수단*이자 상위 배포 전략. MCP 단독 배포보다 채택률↑.
상세 구현 스펙(SKILL.md 초안·plugin.json·.mcp.json·슬래시 커맨드): **[docs/SKILL_PLUGIN_SPEC.md](docs/SKILL_PLUGIN_SPEC.md)**.

### K. x402 수익화 방안 (Monetization) — ⏸ 보류 (트래픽 확보 후)

> ⏸ **우선순위 보류(2026-06-29)**: 3일 된 제품은 **사용자·사용량이 먼저**다. 수익화는 트래픽이 붙은 뒤 착수. 단 어트리뷰션 인프라(K1)는 이미 있으니 *끄지 말고 데이터만 축적*. K2/K3(프리미엄·유료노출)는 수요 신호가 생긴 뒤. 아래는 그때를 위한 설계 보존.
> ⚠️ **하드룰 준수** (AGENTS.md / [X402_REFERRAL_SPEC.md](docs/X402_REFERRAL_SPEC.md)): OnchainAI는 **커스터디·facilitator·게이트웨이·자금이동을 하지 않는다.** 아래는 전부 (a) 어트리뷰션/레퍼럴이거나 (b) OnchainAI **자신의 서비스 대가를 x402로 수취**하는 것 — 제3자 자금을 한시도 보관/중계하지 않음.

**K0. 두 개의 합법 수익 축**
- **축 A (발견 어트리뷰션)**: 에이전트가 OnchainAI 경유로 유료 x402 툴을 찾아 호출 → builder_code/payout split(협조형) 또는 어트리뷰션 로그(데이터형). 결제 경로에 안 낌.
- **축 B (자기 서비스 판매)**: OnchainAI가 자기 기능을 x402로 *판매* — OnchainAI가 payee, 제3자 자금 안 만짐.

**K1. 발견 어트리뷰션 (축 A — 이미 ~70% 구현)**
- 인프라 존재: `referral_events`, `x402_builder_code`/`referral_bps`/`referral_payout_address`, [mcp.rs](src/server/mcp.rs)의 install_guide 어트리뷰션 기록·레퍼럴 메타 노출.
- 핵심 전략: **규모 = 협상력.** 전환/설치가이드 호출을 모아 툴 측과 revenue-share 협상(track1 협조 split, track2 데이터만). 상세는 [X402_REFERRAL_SPEC.md](docs/X402_REFERRAL_SPEC.md).
- 고도화: MCP·Skill·웹 경유 호출의 어트리뷰션을 일관 기록 + 유저 투명 고지.

**K2. x402-게이트 프리미엄 MCP/API (축 B — 신규, on-brand) ⭐**
- OnchainAI MCP 자신이 x402 툴이 됨(도그푸딩): `search_tools`/`get_tool_detail`는 무료, **`export_toolkit`·`compare_tools`·대량 export·고급 랭킹**은 **호출당 x402 마이크로페이먼트**.
- 의의: x402를 *설명*하는 디렉터리가 x402로 *작동* → 가장 강력한 신뢰 데모이자 자기 정합성. 하드룰 위배 아님(자기 서비스 대가 수취).
- 수용 기준: 무료/유료 툴 분리, 프리미엄 호출 시 402 핸드셰이크 E2E 1건.

**K3. 유료 노출/검증 티어 (축 B)**
- Featured 배치(G1)·우선 검증·"verified" 배지 신청을 **x402로 결제**(메이커가 OnchainAI에 USDC). 반드시 **"Sponsored" 명시 라벨**(규제/신뢰).
- **가드레일**: 돈 내도 **public quality gate 우회 불가** — critical 위험·비관련 툴은 결제해도 노출 안 됨. 신뢰가 상품의 본질이라 절대 양보 금지.

**K4. 공통 가드레일**
- 레퍼럴 ON 툴은 검증 플래그(payment/endpoint/price_verified)를 **trust signal로 강조** — 미검증 유료 툴 권유는 법적 노출.
- 수수료는 유저에게 **투명 고지**, `payout_address`(OnchainAI 공개주소)만 노출하고 제3자 정산데이터는 sanitize.
- **영구 범위 밖**: facilitator 프록시/커스터디/자금 보관.

---

### L. 경쟁 벤치마크 & 차별화 (참조)

> 2026-06-28 리서치: MCP 레지스트리(Smithery/Glama/mcp.so/MCP Market/공식), 모델 허브(Hugging Face), 익스텐션 스토어(VS Code Marketplace/Raycast), 패키지(npm/crates), 크립토(DefiLlama/DappRadar).

### 차별화 명제 (Wedge)
대형 디렉터리는 **규모로 경쟁하지만 품질 신호가 비었다**: mcp.so(19k)·Glama(49k)는 **평점·리뷰·사용통계·검증·보안스캔이 없음**. → **OnchainAI는 규모로 싸우지 말고 "크립토 특화 × 큐레이션 × 신뢰/설치안전"으로 이긴다.** 이미 가진 무기: 관련도 게이트, `install_risk_level`, x402 검증, 운영자 리뷰 — **경쟁사가 안 하는 것들**이다. 이걸 전면에 내세운다.

### 사이트별 벤치마크 → OnchainAI 적용

| 사이트 | 잘하는 것 | OnchainAI 적용 / 공략 |
|---|---|---|
| 공식 MCP Registry | 메타데이터 정본 + REST API | **소스로 소비**(B2) + **자기 등재**(A5) |
| Smithery | 발견→실행 <1분, CLI+호스팅, OAuth 모달 | **설치 UX 속도**(D4 "에이전트 연결" 1-클릭) |
| MCP Market | 23+ 카테고리 시각 탐색, featured/official 지정 | featured 이미 있음 → **official 배지**(G3) 강화 |
| mcp.so / Glama | 최대 규모 | **약점 공략**: 평점·리뷰·사용통계 없음 → 우리가 제공 |
| Hugging Face | **모델 카드**(구조화 메타), 태그/파이프라인 필터, **"Use this model" 스니펫**, 트렌딩, 컬렉션, likes | **툴 카드**(아래 신규 I1), 리치 태그, D4, **트렌딩**(I2), G1 컬렉션 |
| VS Code Marketplace | install count/rating/updated **정렬**, **verified publisher**(도메인+6개월), 리뷰·Q&A, 서명/검증 | **랭킹 정렬 옵션**(C1 확장), **검증 배지 기준**(G3), 리뷰·Q&A(I3), 설치 provenance |
| Raycast Store | **OS 호환만 표시**, 전량 오픈소스+커뮤니티 리뷰, 스크린샷/명령/저자 | **에이전트 호환 필터**(I4 "Claude/Cursor와 동작"), 리치 상세 |
| npm / crates | weekly downloads, deprecated 플래그, dependents | **헬스/신선도**(G2: deprecated·last-commit·다운로드) |
| DefiLlama | 투명성·방법론 공개·audited 플래그·고밀도 데이터 | **신뢰=투명성**: 검증 근거를 그대로 노출(C2) |

### 벤치마크로 새로 발굴된 항목 (신규 섹션 I)

**I1. 리치 "툴 카드" + 완성도 점수 (P1)** — HF 모델 카드 모델.
- 툴 상세를 구조화 카드로: 무엇을/어떤 체인/설치/위험/검증/예시 호출 + **메타 완성도 %**. 완성도 높을수록 featured·검색 상위(유지보수자가 채우게 만드는 인센티브 루프).
- 수용 기준: 완성도 점수 표기, 빈 필드 안내, 완성도 높은 툴 랭킹 가점.

**I2. 트렌딩 / 시간창 인기 (P2)** — HF 트렌딩.
- 최근 N일 조회·북마크·설치가이드 호출 급증을 "Trending"으로. 별 수(누적)와 별개 신호. 기존 `referral_events`/북마크 재사용.
- 수용 기준: 트렌딩 섹션 + 시간창 랭킹.

**I3. 평점·리뷰 + 메인테이너 Q&A (P2)** — VS Code 신뢰 신호.
- 기존 댓글([comments_section.rs](src/components/comments_section.rs))을 구조화 평점(1~5)+리뷰로 확장, 메인테이너 응답(클레임 G3 연계)으로 Q&A. mcp.so가 못 하는 바로 그 신호.
- 수용 기준: 평점 집계 표기, 메인테이너 답변 배지.

**I4. 에이전트 호환 필터 (P1)** — Raycast OS-호환 모델.
- "내 에이전트(Claude/Cursor/일반)에서 동작"으로 필터 — 사용자가 *자기 환경에서 되는 것만* 보게. type=mcp + install guide 플랫폼 메타 재사용.
- 수용 기준: `?agent=claude` 필터, 상세에 호환 배지.

**I5. (엔터프라이즈, 범위 밖 메모)** — TrueFoundry식 RBAC/audit/버전롤백/관측성은 향후 B2B 티어 후보로만 기록(현재 범위 아님).

---

## 3. 실행 순서 (의존성)

우선순위 출처는 상단 **"북극성 & 실행 우선순위"** 하나다(중복 방지). 여기는 *의존성 순서*만:

1. **인증·canonical (F1·F2·H1)** 먼저 — 막혀 있으면 사용자가 아무것도 못 함. 거의 설정 변경이라 빠름.
2. **발견 커버리지 (B1→B2)** — B1(레지스트리화)이 B2(신규 소스) 추가를 쉽게 만든다. B1 먼저.
3. **검색 품질 Tier0 (A4=C1)** 과 **신뢰 카드 (C2=D7=I1)** — 유입된 툴을 잘 보여주는 쌍. 병렬 가능.
4. **채택 동선 (D4→J1→J2)** — D4(연결 UI)·J1(Skill) 후 J2(Plugin)가 둘을 번들.
5. 그다음 **Next** 묶음, 마지막 **Later/보류(K)**.

> ✅ 완료: E1(stray 제거) · A1(rmcp 제거). 각 항목 검증은 [VERIFICATION.md](docs/VERIFICATION.md) + `scripts/spec-verify.sh`.

---

## 4. 측정 지표 (Definition of Done)

### 4.1 기술 게이트 (구현 완료 판정)
- **품질**: `cargo test --features ssr` 그린, `cargo clippy -- -W clippy::all` 무경고, `cargo fmt --check` 통과.
- **UI**: `./scripts/ui-change-gate.sh` 그린, 라이트 테마 WCAG AA 대비, 주요 페이지 axe 위반 0. (다크모드 제외)
- **위생/구조**: 추적된 stray 0, 미사용 의존성 0(`cargo machete`), 서버 도메인 모듈 800 LOC 초과 0.
- **인증**: GitHub 로그인 왕복 성공, redirect_uri 오류 0, apex→www 301, 지갑/세션 재현 실패 0.
- **항목별 수용 기준 → 기계 검증**: [VERIFICATION.md](docs/VERIFICATION.md) 매트릭스 + `scripts/spec-verify.sh` 그린.

### 4.2 제품 KPI (북극성 측정 — *현재 계측 없음, H3/분석 필요*)
- **공개 승인 툴 수** (성장) — 목표: 분기별 우상향.
- **자동 유입 비율** = 크롤러 자동 등재 ÷ 전체 신규 (수동 시드 의존도↓ — "수동으로 찾는" 문제 해소 지표).
- **주간 MCP 호출 수** + 호출당 결과 채택률 (검색 품질).
- **검색→상세/설치가이드 전환율** (발견 효과).
- **외부 유입** (레지스트리 등재·README 배지 역링크) — 채택 트랙 성과.
- ⚠️ 이 KPI들은 측정 장치가 없음 → **H3(크롤러 관측성)+간단 분석**이 KPI의 선행 작업.

## 5. 리스크 & 가드레일

- **UI/auth/routing 변경 절대 규칙**: `dev-watch.sh`로 반복, UI 게이트로 마감. `cargo build`로 끝내지 않는다(`AGENTS.md` Hard Rules).
- **스키마 변경 시**: 마이그레이션 + `sqlx prepare` 필수.
- **x402 범위 한정**: 귀속/신뢰 메타데이터만 — custody/facilitator/split 필드 금지.
- **토큰 마이그레이션(D1, 진행 시)**: 라이트값을 현행 hex와 1:1로 매핑해 시각 회귀 0을 유지. 다크 팔레트는 만들지 않는다(범위 외).
- **불순물 제거 전 비밀 노출 점검**: `.playwright-*`/railway 설정에 민감정보가 있었다면 제거만으로 부족 — 회전 필요.
