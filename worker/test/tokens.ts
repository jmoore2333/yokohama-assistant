/**
 * **Access assertions for the cells** (spec §5.2's identity cells): an RSA key
 * pair made with WebCrypto, its public half as a JWK under a `kid`, and tokens
 * signed with it — so identity is tested with no network and no backdoor. The
 * sign-in arms' cells (the auth cycle, Task 3; `src/signin.unit.test.ts`) add
 * a P-256 pair, since an OpenID Connect issuer may sign ES256 instead.
 *
 * Nothing here is node's or workerd's alone — WebCrypto, `btoa` and
 * `TextEncoder` only — so the node cells (`src/identity.unit.test.ts`) and the
 * pool's (`test/worker.test.ts`) both import it.
 */
import type { JwkSet } from "../src/identity";

/** base64url with no padding, as a JWT carries each of its three parts. */
export function b64url(data: Uint8Array | string): string {
  const bytes = typeof data === "string" ? new TextEncoder().encode(data) : data;
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** One key pair: the public half as the JWK a key set lists, and a signer. */
export interface Signer {
  kid: string;
  /** The public key as a JWK, under `kid`. */
  jwk: JsonWebKey & { kid: string };
  /** A set holding this key alone. */
  set(): JwkSet;
  /**
   * A token over `claims`, signed with this key. The header is
   * `{"alg":<the pair's alg>,"kid":<kid>,"typ":"JWT"}` — `RS256` for
   * `signer`, `ES256` for `ecSigner` — unless `header` replaces it, which is
   * how a cell forges another key's `kid` or `alg: "none"`.
   */
  sign(claims: Record<string, unknown>, header?: Record<string, unknown>): Promise<string>;
}

const RSA = { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" } as const;

/** A fresh 2048-bit RS256 key pair under `kid`. */
export async function signer(kid: string): Promise<Signer> {
  const pair = (await crypto.subtle.generateKey(
    { ...RSA, modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]) },
    true,
    ["sign", "verify"],
  )) as CryptoKeyPair;
  return signerOf(pair, kid, "RS256", RSA.name);
}

/** A fresh P-256 ES256 key pair under `kid`. WebCrypto's ECDSA signature is
 *  already the raw `r || s` (64 bytes) a JWS carries, so it is encoded as is. */
export async function ecSigner(kid: string): Promise<Signer> {
  const pair = (await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
    "sign",
    "verify",
  ])) as CryptoKeyPair;
  return signerOf(pair, kid, "ES256", { name: "ECDSA", hash: "SHA-256" });
}

/** The `Signer` over `pair`: its public half under `kid`, and tokens whose
 *  default header names `alg`, signed by WebCrypto's `signWith`. */
async function signerOf(
  pair: CryptoKeyPair,
  kid: string,
  alg: string,
  signWith: Parameters<SubtleCrypto["sign"]>[0],
): Promise<Signer> {
  const exported = (await crypto.subtle.exportKey("jwk", pair.publicKey)) as JsonWebKey;
  const jwk = { ...exported, kid };
  return {
    kid,
    jwk,
    set: () => ({ keys: [jwk] }),
    async sign(claims, header = { alg, kid, typ: "JWT" }) {
      const signed = `${b64url(JSON.stringify(header))}.${b64url(JSON.stringify(claims))}`;
      const signature = await crypto.subtle.sign(signWith, pair.privateKey, new TextEncoder().encode(signed));
      return `${signed}.${b64url(new Uint8Array(signature))}`;
    },
  };
}

/** The claims Access puts in a PERSON's application token, valid for ten
 *  minutes from `now` (seconds). */
export function personClaims(email: string, c: { teamDomain: string; aud: string; now: number }): Record<string, unknown> {
  return {
    aud: [c.aud],
    email,
    exp: c.now + 600,
    iat: c.now,
    nbf: c.now,
    iss: `https://${c.teamDomain}`,
    type: "app",
    sub: "00000000-0000-4000-8000-000000000000",
  };
}

/** The claims Access puts in a Service Auth credential's token: the client id
 *  as `common_name`, and no `email` (spec §0.3 row 1). */
export function credentialClaims(clientId: string, c: { teamDomain: string; aud: string; now: number }): Record<string, unknown> {
  return {
    aud: [c.aud],
    common_name: clientId,
    exp: c.now + 600,
    iat: c.now,
    nbf: c.now,
    iss: `https://${c.teamDomain}`,
    type: "app",
    sub: "",
  };
}
