/** EIP-1193 provider (MetaMask and compatible wallets). */
export interface Eip1193Provider {
  request: (args: { method: string; params?: unknown[] }) => Promise<unknown>;
}

declare global {
  interface Window {
    ethereum?: Eip1193Provider;
  }
}

export class SiwxError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "SiwxError";
  }
}

function provider(): Eip1193Provider {
  if (typeof window === "undefined" || !window.ethereum) {
    throw new SiwxError(
      "No wallet found. Install MetaMask or another Web3 wallet, then try again.",
    );
  }
  return window.ethereum;
}

async function postJson<T>(url: string, body: unknown): Promise<T> {
  const res = await fetch(url, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw new SiwxError("Wallet sign-in failed. Try again.");
  }
  return res.json() as Promise<T>;
}

/**
 * SIWX wallet sign-in: challenge → personal_sign → verify.
 * Sets session cookies on success; returns server redirect path.
 */
export async function connectWalletSiwx(apiBase = ""): Promise<{ redirect: string }> {
  const eth = provider();
  const accounts = (await eth.request({ method: "eth_requestAccounts" })) as string[];
  const address = accounts[0]?.trim();
  if (!address) {
    throw new SiwxError("No wallet account selected.");
  }

  const chainIdHex = (await eth.request({ method: "eth_chainId" })) as string;
  const chainIdNum = Number.parseInt(chainIdHex, 16);
  if (!Number.isInteger(chainIdNum) || chainIdNum < 0) {
    throw new SiwxError("Wallet returned an invalid chain ID.");
  }
  const chainId = chainIdNum.toString();

  const challenge = await postJson<{ nonce: string; message: string }>(
    `${apiBase}/auth/siwx/challenge`,
    { wallet_address: address, chain_id: chainId },
  );

  const signature = (await eth.request({
    method: "personal_sign",
    params: [challenge.message, address],
  })) as string;

  const result = await postJson<{ ok: boolean; redirect: string }>(
    `${apiBase}/auth/siwx/verify`,
    { nonce: challenge.nonce, signature },
  );

  return { redirect: result.redirect || "/" };
}