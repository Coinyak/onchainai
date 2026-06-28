# 구현 검증 매트릭스 (Codex 자가 검증용)

> 목적: 코딩 에이전트(Codex 등)가 [PRODUCT_ENHANCEMENT_SPEC.md](PRODUCT_ENHANCEMENT_SPEC.md) 항목을 구현한 뒤 **"제대로 됐는지"를 기계적으로 확인**하는 장치.
> 실행기: [`scripts/spec-verify.sh`](../scripts/spec-verify.sh). 이 표는 사람이 읽는 정본, 스크립트는 자동 실행분.

## 사용법 — 2개 모드

```bash
./scripts/spec-verify.sh            # report: 전체 현황, 항상 exit 0 (대시보드, 빨강 정상)
./scripts/spec-verify.sh full       # report + cargo test/clippy/fmt
./scripts/spec-verify.sh C3 J1 D7   # gate: 이 ID들만 반드시 PASS, 하나라도 FAIL이면 exit 1
./scripts/spec-verify.sh report C3  # ID 지정해도 report로(exit 0)
PROD_URL=https://staging.example ./scripts/spec-verify.sh
```

- **report 모드**(기본, ID 없을 때): 전체를 보여주되 **항상 exit 0**. 미구현 FAIL이 많아도 정상.
- **gate 모드**(ID 지정 시 자동): **선택한 ID만** 판정, FAIL 있으면 **exit 1**. ← Codex가 "내가 맡은 ID"를 green으로 만들 때 쓰는 모드.

규칙 (Codex에게):
1. 맡은 항목을 구현하면 **`./scripts/spec-verify.sh <그 ID들>`이 exit 0**(전부 PASS)이어야 한다.
2. 그다음 `./scripts/spec-verify.sh full`(report)로 **새로 빨개진 게 없는지**(회귀) 확인.
3. **절대 수(예: "7 PASS")로 판단하지 말 것** — PASS/SKIP 수는 네트워크·`full` 여부로 달라진다(예: 오프라인은 curl이 SKIP). 기준은 **"내가 맡은 ID가 PASS인가"** 뿐.
4. `MANUAL`은 자동화 불가(GitHub 설정·브라우저·시각 QA) — 사람이 확인.
5. `cargo` 게이트는 코드 변경의 필수 통과(단 OpenSSL 등 시스템 라이브러리 부재 환경에선 SKIP으로 표기됨).

상태 표기: ✅구현완료(검증됨) · 🔴미구현/회귀(레드가 정상) · ⏸보류.

---

## 0. 항상 통과해야 하는 품질 게이트 (auto)

| ID | 검사 | PASS 조건 |
|---|---|---|
| Q1 | `cargo check --features ssr` | exit 0 |
| Q2 | `cargo test --features ssr` | exit 0 (`full`) |
| Q3 | `cargo clippy --features ssr -- -W clippy::all` | 경고 0 (`full`) |
| Q4 | `cargo fmt --check` | exit 0 (`full`) |

---

## 1. 완료 항목 — 회귀 가드 (auto)

| ID | 항목 | 검사 | PASS 조건 |
|---|---|---|---|
| A1 | rmcp 제거 | `grep rmcp Cargo.toml Cargo.lock` | 0건 |
| E1 | stray 제거 | `git ls-files \| grep -E 'playwright-.*yaml\|railway-config-pull'` | 0건 + `.gitignore`에 패턴 존재 |

---

## 2. Top 5 트랙 검증

### 트랙 1 — 인증·canonical (F1·F2·H1)
| ID | 검사 | PASS 조건 | 유형 |
|---|---|---|---|
| F1-app | `curl -s $PROD_URL/auth/github`의 Location | `redirect_uri=https%3A%2F%2Fwww.onchain-ai.xyz%2Fauth%2Fcallback` 포함 | auto (이미 PASS) |
| F1-gh | GitHub OAuth 앱에 www 콜백 등록 + 실제 로그인 왕복 | 로그인 성공, redirect_uri 오류 없음 | **MANUAL** |
| H1 | `curl -sI $PROD_URL_APEX/` (apex) | 301/308 → `Location: https://www.onchain-ai.xyz/...` | auto (현재 🔴 404) |
| F2 | 지갑(SIWX) 로그인 + Safari 세션 유지 | 재현 실패 0 | **MANUAL** |
| C3 | About 카피·핸들 정리 | `app.rs`에 `MVP does not include self-service`·`hoyeon4315-cpu` **0건** | auto |

### 트랙 2 — 발견 커버리지 (B1·B2)
| ID | 검사 | PASS 조건 | 유형 |
|---|---|---|---|
| B1 | 트레이트 해킹 제거 + 레지스트리 | `_SourceCrawlerSealed` 0건 AND `mod.rs`/scheduler에 소스 레지스트리(`Vec<Box<dyn SourceCrawler>>` 등) 존재 | auto |
| B2 | 신규 소스 ≥1 | `src/crawler/sources/*.rs` 모듈 수 > 4 (예: `mcp_registry.rs`/`awesome_mcp.rs` 추가) | auto |
| B2-fix | 신규 소스 픽스처 테스트 | `cargo test` 내 신규 소스 파싱 테스트 통과 | auto(full) |

### 트랙 3 — 검색 품질 Tier0 (A4=C1)
| ID | 검사 | PASS 조건 | 유형 |
|---|---|---|---|
| A4-desc | MCP `search_tools` description 보강 | `tools/list`의 `search_tools.description` 길이 > 60자 + 예시 포함 | auto(curl) |
| A4-params | 정렬/페이지네이션 파라미터 | `search_tools.inputSchema`에 `sort`·`limit`·`cursor` 존재 | auto(curl) |
| A4-rank | 별 수 단독 정렬 탈피 | `queries.rs`의 검색 정렬이 `ORDER BY stars` 단독이 아님(가중/관련도) | auto(grep) |

### 트랙 4 — 채택 동선 (D4·J1·J2)
| ID | 검사 | PASS 조건 | 유형 |
|---|---|---|---|
| D4 | "에이전트에 연결" CTA | 홈/About에 Claude·Cursor config 복사 UI 존재(`mcpServers` 문자열 노출) | auto(grep/curl) |
| J1 | OnchainAI Skill | `skills/onchainai-crypto-tools/SKILL.md` 존재 + frontmatter `name`·`description` | auto |
| J1-rule | Skill 안전 규칙 | SKILL.md에 `critical`·`x402` 단어 포함(위험/결제 규칙 명시) | auto |
| J2 | Plugin 번들 | `.claude-plugin/plugin.json` + `.mcp.json` + `commands/*.md` 존재 | auto |
| J2-mcp | 플러그인 MCP 연결 | `.mcp.json`에 `www.onchain-ai.xyz/mcp` 포함 | auto |

### 트랙 5 — 신뢰 카드 (C2=D7=I1)
| ID | 검사 | PASS 조건 | 유형 |
|---|---|---|---|
| D7-icon | 글리프→SVG | `tool_card.rs`에 `☆` 글리프 0건(북마크 SVG화) | auto |
| D7-trust | 카드 신뢰신호 | `tool_card.rs`가 `install_risk_level` 참조(위험 시각화) | auto |
| I1 | 완성도 점수 | 툴 상세/카드에 메타 완성도 표기 로직 존재 | auto(grep) / MANUAL 시각확인 |

---

## 3. Next 묶음 (구현 시 검증)

| ID | 검사 | PASS 조건 | 유형 |
|---|---|---|---|
| A2 | MCP 툴 ≥8 | `tools/list` 길이 ≥ 8 (compare/export/changes 추가) | auto(curl) |
| C4-tradfi | TradFi asset_class | `normalizer.rs`에 `tradfi` 분기 + 분류 테스트 통과 | auto |
| C4-ws | WebSocket type | `normalizer.rs`에 `websocket` 분기 + 테스트 | auto |
| I4 | 에이전트 호환 필터 | `?agent=` 필터 파라미터 처리 존재 | auto(grep) |
| H2 | SEO | `/sitemap.xml`·`/robots.txt` 200 + 툴 상세 OG 메타 | auto(curl) |
| H3 | 크롤러 관측성 | admin crawler 페이지에 소스별 last-success/error 노출 | MANUAL |
| E2 | functions 분할 | `src/server/**/*.rs` 중 800 LOC 초과 0 | auto |
| E3 | 패닉 감사 | 비테스트 런타임 모듈 `unwrap()/expect(` 0(또는 주석 정당화) | auto(grep, 소프트) |
| D3 | 접근성 | 주요 페이지 axe 위반 0 | MANUAL |

---

## 4. 보류 (검증 비활성)

K1~K3(수익화), A4 Tier2 임베딩, D5/D6, G4~G6, I2/I3, J3 등은 **보류** — `scripts/spec-verify.sh`에서 `SKIP`로 표기, 착수 시 표에 검사 추가.

---

## 5. 해석

- 지금 다수 항목이 🔴 **FAIL이 정상**이다 — 아직 미구현. 구현하며 하나씩 GREEN으로.
- `MANUAL`은 스크립트가 "사람이 확인" 안내만 출력. GitHub 설정·브라우저·시각 QA가 그 대상.
- 회귀 방지: PR/커밋 전 `./scripts/spec-verify.sh full` 그린(또는 새 FAIL 0) 확인.
