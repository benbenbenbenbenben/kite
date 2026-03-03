// ── IdentityAccessContext · KnownUserSession ─────────────────────────────────
//
// Manages passwordless authentication via PASETO claims.

export interface KnownUserSession {
  knownUserId: string;
  email: string;
  sessionIssuedAt: Date;
  sessionExpiresAt: Date;
  primaryDropShopId: string;
}

export interface PasetoClaim {
  subject: string;
  issuedAt: Date;
  expiresAt: Date;
  revoked: boolean;
}

// ── Commands ─────────────────────────────────────────────────────────────────

export async function requestSession(email: string): Promise<PasetoClaim> {
  // Issue a PASETO magic-link claim for the given email.
  // The claim is sent via email and verified on return.
  const now = new Date();
  return {
    subject: email,
    issuedAt: now,
    expiresAt: new Date(now.getTime() + 15 * 60_000),
    revoked: false,
  };
}

export async function verifyClaim(claim: string): Promise<KnownUserSession | null> {
  // Decode and verify a PASETO claim string.
  // Returns the authenticated session if valid, null otherwise.
  void claim;
  return null;
}

export async function getUserDetails(claim: string): Promise<KnownUserSession | null> {
  // Retrieve full session details for an authenticated claim.
  void claim;
  return null;
}

export async function refreshSession(claim: string): Promise<PasetoClaim | null> {
  // Refresh an existing session, issuing a new claim with an extended expiry.
  // Fails if the original claim is expired or revoked.
  void claim;
  return null;
}

export async function revokeSession(claim: string): Promise<void> {
  // Revoke a session claim, preventing further use for authentication.
  void claim;
}

// ── Invariants ───────────────────────────────────────────────────────────────

/** SessionClaimsResolveSubject: every verified claim must resolve to a known user. */
export function assertSessionClaimsResolveSubject(session: KnownUserSession): void {
  if (!session.knownUserId) {
    throw new Error("Session claim does not resolve to a known user subject");
  }
}

/** MembershipChecksUseAuthenticatedSubject: membership lookups must use the authenticated subject. */
export function assertMembershipChecksUseAuthenticatedSubject(session: KnownUserSession): void {
  if (!session.knownUserId || !session.primaryDropShopId) {
    throw new Error("Membership check requires an authenticated subject with a primary DropShop");
  }
}

/** ExpiredClaimsCannotAuthorizeMutations: expired claims must be rejected for write operations. */
export function assertExpiredClaimsCannotAuthorizeMutations(claim: PasetoClaim): void {
  if (claim.expiresAt < new Date()) {
    throw new Error("Expired claims cannot authorize mutations");
  }
}

/** RevokedClaimsCannotBeRefreshed: revoked claims must not be refreshable. */
export function assertRevokedClaimsCannotBeRefreshed(claim: PasetoClaim): void {
  if (claim.revoked) {
    throw new Error("Revoked claims cannot be refreshed");
  }
}
