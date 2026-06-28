# OnchainAI 자동 발견(Discovery) 전략 + 시드 후보

> 작성일: 2026-06-28 · 질문: "왜 내가 직접 툴을 찾아 알려줘야 하나? 시스템(AI)이 내가 모르는 것까지 찾게 할 수 없나?"
> 답: 가능하고, **이미 70%는 만들어져 있다.** 빠진 건 (1) **풍부한 소스 연결**, (2) **LLM 보강·갭분석·신규탐지 레이어**.

---

## 1. 진단 — 왜 수동으로 찾게 되는가

크롤러는 **실제로 돈다** ([src/lib.rs:349](src/lib.rs) `tokio::spawn` → [scheduler.rs](src/crawler/scheduler.rs), 5개 cron job). 문제는 **소스가 4개뿐**이고 전부 좁다:

| 현재 소스 | 긁는 곳 |
|---|---|
| cryptoskill | `cryptoskill.org/skills.json` |
| npm | `registry.npmjs.org` 검색 |
| web3mcp | web3-mcp-hub |
| github | GitHub 토픽 |

**안 긁는 곳(= 누락의 핵심)**: 공식 MCP Registry, Smithery(7k+), Glama(49k+), mcp.so, **awesome-mcp 큐레이션 리스트**. 단적으로, awesome-mcp의 `finance--crypto.md` 한 페이지에만 **크립토/금융 MCP 388개**가 있는데 이게 통째로 사각지대다. Base MCP([github.com/base/base-mcp](https://github.com/base/base-mcp))도 GitHub 토픽에 안 걸리면 안 들어온다.

→ **결론: 크롤러를 끈 게 아니라, 가장 알찬 소스들을 안 연결했다.** 이걸 메우면 수동 발견이 거의 사라진다.

---

## 2. 이미 갖춰진 것 (자산)

자동 발견 파이프라인의 절반 이상이 이미 있다:

1. **소스 크롤러 골격** — `SourceCrawler` 트레이트 + 스케줄러 ([sources/mod.rs](src/crawler/sources/mod.rs)).
2. **정규화** — `RawTool` → `Tool` ([normalizer.rs](src/crawler/normalizer.rs)).
3. **관련도 게이트** — `crypto_relevance_score`/`relevance_status` ([relevance.rs](src/crawler/relevance.rs)). **← 결정적.** 아래 §4 참조.
4. **중복 제거** — [deduper.rs](src/crawler/deduper.rs).
5. **운영자 리뷰 큐** — [review_persistence.rs](src/server/review_persistence.rs), [operator_harness.rs](src/server/operator_harness.rs).

빠진 것: **(A) 풍부한 소스, (B) LLM 보강, (C) 갭분석, (D) 신규탐지 에이전트.**

---

## 3. AI 자동 발견 파이프라인 (목표 설계)

```
[소스들] → 정규화 → 관련도 게이트 → 중복제거 → (NEW) LLM 보강/분류/품질점수 → 리뷰 큐 → 공개
                                                         ↑
                            (NEW) 갭분석 ──────────────┘   (NEW) 신규탐지 에이전트(주기)
```

**A. 소스 확장 (deterministic, 최우선)** — 각각 `SourceCrawler` 1개:
- 공식 MCP Registry API · Smithery API · Glama API · mcp.so
- awesome-* GitHub 마크다운 리스트 raw 파싱(awesome-mcp finance/crypto, awesome-web3)
- crates.io · PyPI · Docker Hub
- 효과: 1회 크롤로 수백 후보 유입. (스펙 B2 참조)

**B. LLM 보강 패스 (NEW)** — 키워드 분류기가 못 하는 걸 Claude가 처리:
- README/홈페이지 읽고 description 생성·정제, 3축(function/asset_class/actor)+type+chains 분류, 설치 위험도 추론.
- **시맨틱 중복 판별**(예: `uniswap-mcp` ↔ `uniswap-trader-mcp` 관계), 품질/주목도 점수.
- 최신·최강 Claude 모델 사용(이 제품 자체가 AI 앱이므로).

**C. 갭분석 — "내가 모르는 것"의 핵심 (NEW)**:
- 커버리지 매트릭스 계산: (function × chain × type) 중 **희소한 칸** 식별.
- 그 빈칸을 채울 타깃 질의를 LLM이 생성("X 체인의 Y 기능 MCP 찾아") → 검색형 크롤러에 피드백.
- 결과: 시스템이 *부족한 영역을 스스로 알아내고* 채우러 간다.

**D. 신규탐지 에이전트 (NEW, 주기 실행)**:
- "지난 실행 이후 등장한 주목할 크립토 MCP/CLI" LLM+웹 패스 → URL 실재 검증 → 리뷰 큐.

**E. 운영자 게이트(기존 유지)**: 공개 전 사람이 승인. 자동발견 ≠ 자동공개.

---

## 4. 품질 경고 — 원시 리스트는 ~40%가 노이즈

awesome-mcp 388개를 그대로 넣으면 안 된다. 실제로 섞여 있던 것들:
- 비크립토: 담배 데이터 검색, Magic:The Gathering 카드, 전기요금, 해외 은행(말레이시아/이스라엘/브라질 중앙은행) API, YNAB·Lunchmoney 가계부.
- 저품질·장난: "유머러스 주가 응답", coin-flip, "MON 잔액 조회" Monad 테스트넷 클론 **15개+** 거의 동일.

→ **OnchainAI의 관련도 게이트 + 중복제거가 정확히 이 필터**다(§2.3, §2.4). 자동발견의 핵심은 "많이 긁기"가 아니라 **"긁고 거르기"**이며, 그 인프라가 이미 있다는 게 강점이다. LLM 품질점수(§3.B)가 이 거르기를 한 단계 더 끌어올린다.

---

## 5. 지금 바로 넣을 큐레이션 시드 후보

388개에서 **실명·평판·크립토 적합성**으로 추린 고신호 셋. ⭐=공식/플래그십. (전체 388 출처: [awesome-mcp finance/crypto](https://github.com/TensorBlock/awesome-mcp-servers/blob/main/docs/finance--crypto.md))

**공식/플래그십**
- ⭐ `base/base-mcp` — Base 네트워크 + Coinbase API ([repo](https://github.com/base/base-mcp))
- ⭐ `coinbase/agentkit` (+ agentkit-MCP) — 에이전트 지갑 ([repo](https://github.com/coinbase/agentkit))
- ⭐ `solana-foundation/solana-dev-mcp` — Solana 개발(공식)
- ⭐ `nearai/near-mcp` — NEAR(공식) · `Bankless/onchain-mcp` — 온체인 데이터
- ⭐ `debridge-finance/debridge-mcp` · `reservoirprotocol/relay-mcp` · `li-fi/lifi-mcp` — 크로스체인
- ⭐ `coinpaprika/dexpaprika-mcp` · `CoinStatsHQ/coinstats-mcp` · `getAlby/lightning-tools-mcp-server`

**지갑/실행**: `armorwallet/armor-crypto-mcp` · `dcSpark/mcp-cryptowallet-{evm,solana}` · `magnetai/mcp-free-usdc-transfer`(Base USDC) · `zhangzhongnan928/mcp-coinbase-commerce` · `joaquim-verges/blockchain-mcp`(thirdweb)

**DeFi/DEX**: `kukapay/jupiter-mcp` · `kukapay/uniswap-trader-mcp` · `kukapay/uniswap-poolspy-mcp` · `dcSpark/mcp-server-defillama` · `Impa-Ventures/hyperliquid-mcp` · `arcadia-finance/mcp-server`

**마켓데이터/분석**: `crazyrabbitLTC/mcp-coingecko-server` · `shinzo-labs/coinmarketcap-mcp` · `kukapay/{crypto-news,crypto-sentiment,crypto-indicators,crypto-feargreed,funding-rates,etf-flow}-mcp` · `kukapay/thegraph-mcp` · `aaronjmars/web3-research-mcp`

**체인 RPC/익스플로러**: `mcpdotdirect/evm-mcp-server` · `sinco-lab/evm-mcp-server` · `milancermak/starknet-mcp` · `Tlazypanda/aptos-mcp-server` · `Genebson/stellar-mcp` · `Outblock/flow-mcp` · `kriuchkov/ton-mcp` · `haomingdev/etherscan-mcp`

**비트코인**: `AbdelStark/bitcoin-mcp` · `ordiscan/ordiscan-mcp` · `b-open-io/bsv-mcp`

**보안/트랜잭션 분석**: `mark3labs/phalcon-mcp`(BlockSec) · `kukapay/token-revoke-mcp`

**거래소(CEX)**: `doggybee/mcp-server-ccxt`(CCXT 다중) · `sammcj/bybit-mcp` · `zereight/bithumb-mcp`

**TradFi 크로스오버 (→ C4 카테고리 근거)**: `MardiantoS/alpaca-mcp-server`(주식+크립토) · `trayders/trayd-mcp`(Robinhood) · `ciri/ibkr-mcp-ts`(IBKR) · `financial-datasets/mcp-server` · `djsamseng/bloomberg-mcp`

**x402/에이전트 결제 (→ OnchainAI x402 축 근거)**: `FluxA-Agent-Payment/FluxA-AI-Wallet-MCP` · `agentc22/x402engine-mcp` · `hypeprinter007-stack/anchor-x402-mcp`

> 권장: 위를 시드로 직접 넣기보다 **§3.A 소스 크롤러(awesome-mcp + 공식 Registry)를 붙여 자동 유입**시키고, 관련도 게이트로 거르는 게 구조적. 단, `base/base-mcp`·`coinbase/agentkit`처럼 즉시 노출하고 싶은 플래그십은 시드로 먼저 박아도 됨.

---

## 6. Worked example — "내가 안 알려줬으면 Base MCP / BOB Gateway를 어떻게 찾았을까?"

핵심 원리 먼저: **AI는 이걸 기억(훈련데이터)으로 아는 게 아니다.** 니치하거나 신상 툴은 모델이 모를 수 있다. 발견은 **"툴이 스스로 등록하는 레지스트리를 긁고 → 관련도로 거르는"** 데서 나온다. AI의 역할은 *오라클*이 아니라 **보강·분류·갭탐지**다.

**BOB Gateway** → 이미 잡혔어야 한다.
- `@gobob/gateway-cli`는 **npm 패키지**다. OnchainAI엔 이미 **npm 크롤러**가 있고([npm.rs](src/crawler/sources/npm.rs)), 픽스처에도 `@gobob/gateway-cli`가 들어있다.
- 메커니즘: npm 레지스트리 검색을 crypto/web3/gateway/agent/mcp 키워드로 돌리면 자동 유입. README의 "어떤 디렉터리도 BOB을 안 갖고 있었다"는 출발점이 정확히 이 크롤러가 메우려던 케이스.
- 안 잡혔다면 원인은 **npm 검색 질의어가 좁거나** 관련도 게이트가 떨군 것 → 질의어 확장/게이트 튜닝 문제이지 구조 부재가 아님.

**Base MCP** → 레지스트리 소스만 붙이면 확실히 잡힌다.
- `base/base-mcp`는 (a) **공식 MCP Registry**에 게시, (b) **Smithery·Glama**에 등재, (c) **awesome-mcp** 리스트에 포함, (d) GitHub repo에 `mcp`/`model-context-protocol` 토픽.
- 현재 **GitHub 토픽 크롤러**가 올바른 토픽을 질의하면 (d)로 이미 걸릴 수도 있다(경계선). §3.A의 **MCP Registry / awesome-mcp 소스**를 붙이면 (a)(c)로 **확정적으로** 유입.

**일반화 — 모르는 걸 찾는 3겹 그물:**
1. **레지스트리 그물(deterministic)**: 툴은 어딘가(npm·crates·PyPI·MCP Registry·Smithery·GitHub 토픽·awesome 리스트)에 자기를 올린다. 그곳을 다 긁으면 "유명세"와 무관하게 들어온다. ← 커버리지의 본체.
2. **관련도+중복 그물**: 노이즈/중복 제거(기존 자산).
3. **신규/갭 그물(LLM, §3.C·D)**: 레지스트리에 아직 없는 신상은 주기적 웹+LLM 패스로, 부족한 (기능×체인) 칸은 갭분석이 타깃 질의를 만들어 메운다.

→ 결론: Base MCP·BOB Gateway 둘 다 **소스만 충분하면 사람 개입 없이** 들어왔어야 한다. 지금 수동이 된 건 그물 1번이 4칸(cryptoskill/npm/web3mcp/github)으로 좁기 때문(§1).
