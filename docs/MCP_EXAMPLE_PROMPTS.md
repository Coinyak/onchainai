# OnchainAI MCP 예시 프롬프트

> Claude Code / Cursor에서 OnchainAI MCP 연결 후 입력할 수 있는 프롬프트 예시.
> 연결: `claude mcp add --transport http onchainai https://www.onchain-ai.xyz/mcp`

---

## 무료 툴 (결제 불필요)

### 1. search_tools — 도구 검색

```
OnchainAI에서 Base 체인 브릿지 도구를 찾아줘
```
```
USDC 결제 관련 MCP 서버가 있어?
```
```
Solana 지갑 SDK 중에서 검증된(verified) 도구를 보여줘
```

### 2. get_tool_detail — 도구 상세

```
lifi-sdk의 상세 정보를 알려줘 — 설치 위험도, 체인, 라이선스
```
```
goldrush-x402 툴의 x402 결제 정보와 엔드포인트 상태를 확인해줘
```

### 3. list_categories — 카테고리 탐색

```
OnchainAI에 어떤 카테고리가 있고 각각 몇 개의 도구가 있는지 알려줘
```

### 4. get_dashboard_snapshot — 전체 현황

```
OnchainAI 카탈로그 전체 현황을 요약해줘 — 총 도구 수, x402 도구, 검증된 도구
```

### 5. compare_tools — 도구 비교

```
lifi-sdk와 wormhole-typescript-sdk를 비교해줘 — trust, 체인, 설치 위험도
```
```
coingecko-mcp, chainlink-sdk, band-protocol-sdk 세 개를 비교해줘
```

### 6. get_install_guide — 설치 가이드

```
lifi-sdk를 Claude에 설치하는 방법을 알려줘
```
```
wormhole-typescript-sdk를 Cursor에 연결하는 설정을 보여줘
```

### 7. get_price_history — x402 가격 이력 (무료)

```
goldrush-x402의 최근 30일간 x402 프로브 이력을 보여줘 — 상태, 가격, 레이턴시
```
```
지난 7일간 goldrush-x402 엔드포인트가 살아있었는지 확인해줘
```

### 8. get_x402_trends — x402 생태계 트렌드 (무료)

```
전체 x402 도구의 최근 30일 트렌드를 보여줘 — live rate, 프로브 수, 최신 가격
```
```
x402 생태계에서 어떤 툴이 가장 안정적인지 알려줘
```

---

## 유료 툴 (공개 `/mcp` 프리미엄 — x402, HTTP 402)

> 기본 연결 URL `https://www.onchain-ai.xyz/mcp` 에서 **디스커버리 툴은 무료**다.
> 아래 4개만 공개 경로에서 과금된다 (Base USDC, Axis B / CDP; OKX fallback 가능).
> **OKX 마켓 전용 경로 `https://www.onchain-ai.xyz/mcp/okx` 는 별도 패키지** — 게이트 활성 시
> `tools/call` **전부** ~$0.1 USDT0 (X Layer). 코딩 에이전트는 기본 `/mcp` 를 쓴다.
>
> Claude Code / Cursor는 x402 결제를 완료할 수 없다.
> 지갑이 연결된 HTTP 클라이언트 또는 REST premium API로 호출하세요.
> MCP로 유료 툴을 시도하면 "Connection closed" 가 정상이다.

### 9. check_endpoint_health — 엔드포인트 생존 확인 (~$0.001 USDC)

```
goldrush-x402 엔드포인트가 지금 살아있는지 확인해줘 — 결제 직전 보험
```
```
x402 결제하기 전에 이 엔드포인트가 정말 응답하는지 프로브해줘
```

### 10. export_toolkit — 툴킷 내보내기 ($0.01 USDC)

```
bridge 카테고리의 상위 도구들을 JSON + 마크다운 설치 킷으로 내보내줘
```
```
lifi-sdk, wormhole-typescript-sdk, coingecko-mcp 세 개를 번들로 내보내줘
```

### 11. recommend_verified_tool — 검증된 추천 ($0.01 USDC)

```
"Base에서 USDC를 Ethereum으로 브릿지"하는 작업에 가장 신뢰할 수 있는 live x402 툴 하나만 추천해줘. 다른 후보들은 왜 탈락했는지도 알려줘.
```
```
"이더리움 가격 데이터를 x402로 가져오는" 작업에 검증된 live 엔드포인트를 하나 골라줘. 왜 그게 선택됐는지 설명도 함께.
```

### 12. gap_audit — 카탈로그 갭 분석 ($0.01 USDC)

```
"BTC를 Base로 브릿지해서 Morph에 예치하고 담보대출 받아서 루핑하는" 작업을 분해해서, OnchainAI 카탈로그에 각 단계별 도구가 있는지 확인해줘. 없으면 갭으로 표시해줘.
```
```
"USDC로 결제하고 영수증을 온체인에 기록하는" 워크플로우에 필요한 도구들이 카탈로그에 다 있는지 감사해줘
```

---

## 복합 워크플로우 예시

### 브릿지 도구 선택 + 설치

```
1. OnchainAI에서 Base 체인 브릿지 도구를 검색해줘
2. 상위 3개의 상세 정보를 비교해줘
3. 가장 안전한 걸 하나 골라서 Claude에 설치하는 방법을 알려줘
```

### x402 결제 전 안전 확인

```
1. goldrush-x402의 현재 x402 검증 플래그를 확인해줘 (무료 get_tool_detail)
2. 최근 프로브 이력을 봐줘 (무료 get_price_history)
3. 엔드포인트가 stale하면 실시간 프로브를 돌려줘 (유료 check_endpoint_health)
```

### 카탈로그 커버리지 감사

```
1. "SOL을 Jito에 스테이킹하고 수익을 USDC로 스왑하는" 작업을 분해해줘 (유료 gap_audit)
2. 각 서브골에 대해 카탈로그에 도구가 있는지 확인해줘
3. 갭이 있으면 수동으로 찾아야 할 항목을 정리해줘
```
