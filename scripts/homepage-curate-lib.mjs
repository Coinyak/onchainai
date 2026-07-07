import { loadFirstPartyOrgs } from "./vendor-orgs-lib.mjs";

export const FIRST_PARTY_ORGS = loadFirstPartyOrgs();

/** GitHub org login → brand + docs (lowercase keys). */
export const ORG_BRANDS = {
  aave: { homepage: "https://aave.com", docs: "https://docs.aave.com" },
  "alchemyplatform": {
    homepage: "https://www.alchemy.com",
    docs: "https://docs.alchemy.com",
  },
  "anza-xyz": {
    homepage: "https://www.solanakit.com",
    docs: "https://www.solanakit.com/docs",
  },
  base: { homepage: "https://base.org", docs: "https://docs.base.org" },
  "base-org": { homepage: "https://base.org", docs: "https://docs.base.org" },
  "bnb-chain": {
    homepage: "https://www.bnbchain.org",
    docs: "https://docs.bnbchain.org",
  },
  "bob-collective": {
    homepage: "https://gobob.xyz",
    docs: "https://docs.gobob.xyz",
  },
  blockrunai: { homepage: "https://blockrun.ai", docs: null },
  circlefin: {
    homepage: "https://www.circle.com",
    docs: "https://developers.circle.com",
  },
  "crcl-main": {
    homepage: "https://www.circle.com",
    docs: "https://developers.circle.com",
  },
  coinbase: {
    homepage: "https://www.coinbase.com/developer-platform",
    docs: "https://docs.cdp.coinbase.com",
  },
  consensys: {
    homepage: "https://consensys.io",
    docs: "https://docs.metamask.io",
  },
  crossmint: {
    homepage: "https://crossmint.com",
    docs: "https://docs.crossmint.com",
  },
  ethereum: {
    homepage: "https://ethereum.org",
    docs: "https://ethereum.org/developers/docs",
  },
  farcasterxyz: {
    homepage: "https://www.farcaster.xyz",
    docs: "https://docs.farcaster.xyz",
  },
  "goat-sdk": {
    homepage: "https://crossmint.com/goat",
    docs: "https://docs.crossmint.com/goat",
  },
  graphprotocol: {
    homepage: "https://thegraph.com",
    docs: "https://thegraph.com/docs",
  },
  "input-output-hk": {
    homepage: "https://cardano.org",
    docs: "https://docs.cardano.org",
  },
  "layerzero-labs": {
    homepage: "https://layerzero.network",
    docs: "https://docs.layerzero.network",
  },
  metamask: {
    homepage: "https://metamask.io",
    docs: "https://docs.metamask.io",
  },
  "modelcontextprotocol": {
    homepage: "https://modelcontextprotocol.io",
    docs: "https://modelcontextprotocol.io/docs",
  },
  neynarhq: { homepage: "https://neynar.com", docs: "https://docs.neynar.com" },
  neynarxyz: { homepage: "https://neynar.com", docs: "https://docs.neynar.com" },
  openai: { homepage: "https://openai.com", docs: "https://platform.openai.com/docs" },
  projectopensea: {
    homepage: "https://opensea.io",
    docs: "https://docs.opensea.io",
  },
  quicknode: {
    homepage: "https://www.quicknode.com",
    docs: "https://www.quicknode.com/docs",
  },
  "reown-com": { homepage: "https://reown.com", docs: "https://docs.reown.com" },
  "safe-global": {
    homepage: "https://safe.global",
    docs: "https://docs.safe.global",
  },
  smartcontractkit: {
    homepage: "https://chain.link",
    docs: "https://docs.chain.link",
  },
  "solana-foundation": {
    homepage: "https://solana.com",
    docs: "https://solana.com/docs",
  },
  "solana-labs": {
    homepage: "https://solana.com",
    docs: "https://solana.com/docs",
  },
  "thirdweb-dev": {
    homepage: "https://thirdweb.com",
    docs: "https://portal.thirdweb.com",
  },
  tinyhumansai: { homepage: "https://tiny.place", docs: null },
  "ton-blockchain": { homepage: "https://ton.org", docs: "https://docs.ton.org" },
  tronprotocol: {
    homepage: "https://tron.network",
    docs: "https://developers.tron.network",
  },
  uniswap: {
    homepage: "https://uniswap.org",
    docs: "https://docs.uniswap.org",
  },
  walletconnect: {
    homepage: "https://walletconnect.com",
    docs: "https://docs.reown.com",
  },
  wevm: { homepage: "https://viem.sh", docs: "https://viem.sh/docs" },
  "wormhole-foundation": {
    homepage: "https://wormhole.com",
    docs: "https://docs.wormhole.com",
  },
  "x402-foundation": { homepage: "https://x402.org", docs: "https://x402.org" },
  github: { homepage: "https://github.com", docs: "https://docs.github.com" },
  discord: {
    homepage: "https://discord.com/developers",
    docs: "https://discord.com/developers/docs",
  },
  grammyjs: { homepage: "https://grammy.dev", docs: "https://grammy.dev/guide" },
  anthropics: {
    homepage: "https://www.anthropic.com",
    docs: "https://docs.anthropic.com",
  },
  "google-gemini": {
    homepage: "https://ai.google.dev",
    docs: "https://ai.google.dev/gemini-api/docs",
  },
};

/** npm scope (no @) → org key in ORG_BRANDS */
export const NPM_SCOPE_ORG = {
  coinbase: "coinbase",
  "circle-fin": "circlefin",
  "base-org": "base",
  metamask: "metamask",
  "safe-global": "safe-global",
  "thirdweb-dev": "thirdweb-dev",
  "layerzerolabs": "layerzero-labs",
  wormholefoundation: "wormhole-foundation",
  reown: "reown-com",
  walletconnect: "walletconnect",
  solana: "solana-foundation",
  anza: "anza-xyz",
};

/** slug → explicit homepage/docs override */
export const SLUG_OVERRIDES = {
  bnbagent: {
    homepage: "https://www.bnbchain.org/en/bnb-agent-studio",
    docs: "https://docs.bnbchain.org/developer-kit/bnbagent-sdk/",
  },
  "bnbagent-studio": {
    homepage: "https://www.bnbchain.org/en/bnb-agent-studio",
    docs: "https://docs.bnbchain.org/developer-kit/bnbchain-studio/",
  },
  wagmi: { homepage: "https://wagmi.sh", docs: "https://wagmi.sh/react/getting-started" },
  "wagmi-core": { homepage: "https://wagmi.sh", docs: "https://wagmi.sh/react/getting-started" },
  "wagmi-connectors": {
    homepage: "https://wagmi.sh",
    docs: "https://wagmi.sh/react/getting-started",
  },
  viem: { homepage: "https://viem.sh", docs: "https://viem.sh/docs/getting-started" },
  abitype: { homepage: "https://abitype.dev", docs: "https://abitype.dev" },
  onchainkit: {
    homepage: "https://onchainkit.xyz",
    docs: "https://docs.base.org/onchainkit/getting-started",
  },
  agentkit: {
    homepage: "https://docs.cdp.coinbase.com/agentkit/welcome",
    docs: "https://docs.cdp.coinbase.com/agentkit/welcome",
  },
  "circle-agent-stack": {
    homepage: "https://agents.circle.com",
    docs: "https://developers.circle.com",
  },
  "solana-mcp-official": {
    homepage: "https://solana.com",
    docs: "https://solana.com/docs",
  },
  "solana-dev-skill": {
    homepage: "https://solana.com",
    docs: "https://solana.com/docs",
  },
  okx: {
    homepage: "https://web3.okx.com",
    docs: "https://web3.okx.com/build/dev-docs",
  },
  "okx-onchainos-skills": {
    homepage: "https://web3.okx.com",
    docs: "https://web3.okx.com/build/dev-docs",
  },
  "github-mcp-server": {
    homepage: "https://github.com/github/github-mcp-server",
    docs: "https://github.com/github/github-mcp-server#readme",
  },
  tinyagents: { homepage: "https://tiny.place", docs: "https://tiny.place" },
  openhuman: { homepage: "https://tiny.place", docs: "https://tiny.place" },
  "tiny-place": { homepage: "https://tiny.place", docs: "https://tiny.place" },
  x402: { homepage: "https://x402.org", docs: "https://x402.org" },
  "x-api-mcp": {
    homepage: "https://developer.x.com",
    docs: "https://developer.x.com/en/docs/x-api",
  },
  "x-api-typescript-sdk": {
    homepage: "https://developer.x.com",
    docs: "https://developer.x.com/en/docs/x-api",
  },
  "solana-developers-helpers": {
    homepage: "https://solana.com",
    docs: "https://solana.com/docs",
  },
};

export function parseGithubRepo(url) {
  if (!url) return null;
  const m = String(url).match(/github\.com\/([^/]+)\/([^/#?]+)/i);
  if (!m) return null;
  return { org: m[1], repo: m[2].replace(/\.git$/, "") };
}

export function isGithubUrl(url) {
  return !!url && /github\.com/i.test(url);
}

export function npmScope(pkg) {
  if (!pkg || !pkg.startsWith("@") || !pkg.includes("/")) return null;
  return pkg.slice(1).split("/")[0].toLowerCase();
}

export function docsFromHomepage(homepage) {
  if (!homepage) return null;
  try {
    const u = new URL(homepage);
    const host = u.hostname.replace(/^www\./, "");
    if (host.startsWith("docs.")) return homepage;
  } catch {
    return null;
  }
  return null;
}

/** Infer brand homepage/docs from slug prefixes (cryptoskill / clawhub skills). */
export function inferBrandFromSlug(slug) {
  const rules = [
    [/^okx-/, { homepage: "https://web3.okx.com", docs: "https://web3.okx.com/build/dev-docs" }],
    [/^binance-/, { homepage: "https://www.binance.com", docs: "https://developers.binance.com" }],
    [/^circle-official-/, { homepage: "https://developers.circle.com", docs: "https://developers.circle.com" }],
    [/^moonpay-official-/, { homepage: "https://www.moonpay.com", docs: "https://dev.moonpay.com" }],
    [/^base-official-/, { homepage: "https://base.org", docs: "https://docs.base.org" }],
    [/^uniswap-official-/, { homepage: "https://uniswap.org", docs: "https://docs.uniswap.org" }],
    [/^rocketpool-official-/, { homepage: "https://rocketpool.net", docs: "https://docs.rocketpool.net" }],
    [/^nethermind-official-/, { homepage: "https://www.nethermind.io", docs: "https://docs.nethermind.io" }],
    [/^defillama-official-/, { homepage: "https://defillama.com", docs: "https://docs.llama.fi" }],
    [/^dune-official-/, { homepage: "https://dune.com", docs: "https://docs.dune.com" }],
    [/^alchemy/, { homepage: "https://www.alchemy.com", docs: "https://docs.alchemy.com" }],
    [/^moralis/, { homepage: "https://moralis.io", docs: "https://docs.moralis.io" }],
    [/^heurist-official-/, { homepage: "https://heurist.ai", docs: "https://docs.heurist.ai" }],
    [/^bitget-official-/, { homepage: "https://www.bitget.com", docs: "https://www.bitget.com/api-doc" }],
    [/^privy-/, { homepage: "https://privy.io", docs: "https://docs.privy.io" }],
    [/^octav-official-/, { homepage: "https://octav.fi", docs: "https://docs.octav.fi" }],
    [/^zapper/, { homepage: "https://zapper.xyz", docs: "https://protocol.zapper.xyz/docs" }],
    [/^coinmarketcap-/, { homepage: "https://coinmarketcap.com", docs: "https://coinmarketcap.com/api/documentation/v1/" }],
    [/^celo-official-/, { homepage: "https://celo.org", docs: "https://docs.celo.org" }],
    [/^bnb-official-/, { homepage: "https://www.bnbchain.org", docs: "https://docs.bnbchain.org" }],
    [/^aave-/, { homepage: "https://aave.com", docs: "https://docs.aave.com" }],
    [/^wormhole-mcp$/, { homepage: "https://wormhole.com", docs: "https://docs.wormhole.com" }],
    [/^quicknode-mcp$/, { homepage: "https://www.quicknode.com", docs: "https://www.quicknode.com/docs" }],
    [/^solana-mcp-server$/, { homepage: "https://solana.com", docs: "https://solana.com/docs" }],
    [/^sui-mcp-server$/, { homepage: "https://sui.io", docs: "https://docs.sui.io" }],
    [/^evm-mcp$/, { homepage: "https://github.com/mcpdotdirect/evm-mcp-server", docs: null }],
  ];
  for (const [re, brand] of rules) {
    if (re.test(slug)) return brand;
  }
  return null;
}

export function resolvePatch(tool) {
  const slug = tool.slug;
  const override = SLUG_OVERRIDES[slug];
  const slugBrand = inferBrandFromSlug(slug);
  const gh = parseGithubRepo(tool.repo_url) || parseGithubRepo(tool.homepage);
  const orgKey = gh?.org?.toLowerCase() ?? null;
  const brand = orgKey ? ORG_BRANDS[orgKey] : null;
  const scope = npmScope(tool.npm_package);
  const scopeOrg = scope ? NPM_SCOPE_ORG[scope] ?? scope : null;
  const scopeBrand = scopeOrg ? ORG_BRANDS[scopeOrg] : null;

  let homepage = tool.homepage;
  let docs = null;

  if (override) {
    homepage = override.homepage ?? homepage;
    docs = override.docs ?? docs;
  } else if (slugBrand) {
    if (!homepage || isGithubUrl(homepage)) homepage = slugBrand.homepage;
    docs = docs ?? slugBrand.docs;
  } else if (!homepage && !tool.repo_url && tool.source_url) {
    homepage = tool.source_url;
    docs = tool.source_url;
  }

  if (slug.includes("wagmi") && !override) {
    homepage = "https://wagmi.sh";
    docs = "https://wagmi.sh/react/getting-started";
  } else if (slug === "viem" || slug.startsWith("viem-")) {
    homepage = homepage && !isGithubUrl(homepage) ? homepage : "https://viem.sh";
    docs = docs ?? "https://viem.sh/docs/getting-started";
  } else if (slug.includes("abitype")) {
    homepage = homepage && !isGithubUrl(homepage) ? homepage : "https://abitype.dev";
    docs = docs ?? "https://abitype.dev";
  }

  if (isGithubUrl(homepage) && brand?.homepage) {
    homepage = brand.homepage;
  } else if (isGithubUrl(homepage) && scopeBrand?.homepage) {
    homepage = scopeBrand.homepage;
  }

  if (!docs) docs = brand?.docs ?? scopeBrand?.docs ?? null;
  if (!docs) docs = docsFromHomepage(tool.homepage);
  if (!docs && homepage) docs = docsFromHomepage(homepage);

  // Circle agent surfaces
  if (orgKey === "circlefin" && gh?.repo?.includes("skill")) {
    homepage = homepage && !isGithubUrl(homepage) ? homepage : "https://agents.circle.com";
    docs = docs ?? "https://developers.circle.com";
  }

  const links = [];
  if (docs && docs !== homepage) {
    links.push({ label: "Documentation", url: docs });
  } else if (docs && docs === homepage) {
    links.push({ label: "Documentation", url: docs });
  }
  if (
    brand?.homepage &&
    homepage &&
    homepage !== brand.homepage &&
    !links.some((l) => l.url === brand.homepage)
  ) {
    links.push({ label: "Product", url: brand.homepage });
  }

  const changedHomepage =
    homepage && homepage !== tool.homepage ? homepage : null;
  const needsDocsLink = links.length > 0;

  if (!changedHomepage && !needsDocsLink) return null;

  return {
    slug,
    homepage: changedHomepage,
    links,
    reason: [
      override ? "slug-override" : null,
      brand ? `org:${orgKey}` : null,
      scopeBrand ? `npm-scope:${scope}` : null,
    ]
      .filter(Boolean)
      .join(", "),
  };
}