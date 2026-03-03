// ── DropShopOnboardingContext · DropShop ──────────────────────────────────────
//
// Manages DropShop registration, verification, profile updates, and membership.

export interface DropShopRegistration {
  displayName: string;
  legalName: string;
  contactEmail: string;
}

export interface DropShopVerification {
  dropShopId: string;
  verificationCode: string;
}

export interface DropShop {
  dropShopId: string;
  displayName: string;
  legalName: string;
  contactEmail: string;
  isVerified: boolean;
  admins: string[];
  users: string[];
  verificationCode?: string;
  verificationExpiresAt?: Date;
}

// ── Commands ─────────────────────────────────────────────────────────────────

export async function registerDropShop(registration: DropShopRegistration): Promise<DropShop> {
  const verificationCode = Math.random().toString(36).slice(2, 8).toUpperCase();
  return {
    dropShopId: crypto.randomUUID(),
    displayName: registration.displayName,
    legalName: registration.legalName,
    contactEmail: registration.contactEmail,
    isVerified: false,
    admins: [],
    users: [],
    verificationCode,
    verificationExpiresAt: new Date(Date.now() + 24 * 60 * 60_000),
  };
}

export async function verifyDropShopRegistration(
  verification: DropShopVerification,
): Promise<void> {
  // Verify the DropShop registration using the provided verification code.
  void verification;
}

export async function resendDropShopVerification(dropShopId: string): Promise<void> {
  // Regenerate and resend the verification code for a DropShop.
  void dropShopId;
}

export async function updateDropShopProfile(
  dropShopId: string,
  displayName: string,
  contactEmail: string,
): Promise<void> {
  // Update the display name and contact email for a verified DropShop.
  void dropShopId;
  void displayName;
  void contactEmail;
}

export async function addAdminToDropShop(
  newAdminId: string,
  dropShopId: string,
): Promise<void> {
  // Add a new administrator to the DropShop.
  void newAdminId;
  void dropShopId;
}

export async function removeAdminFromDropShop(
  adminId: string,
  dropShopId: string,
): Promise<void> {
  // Remove an administrator from the DropShop.
  void adminId;
  void dropShopId;
}

export async function addUserToDropShop(
  newUserId: string,
  dropShopId: string,
): Promise<void> {
  // Add a regular user to the DropShop.
  void newUserId;
  void dropShopId;
}

export async function removeUserFromDropShop(
  userId: string,
  dropShopId: string,
): Promise<void> {
  // Remove a user from the DropShop.
  void userId;
  void dropShopId;
}

// ── Invariants ───────────────────────────────────────────────────────────────

/** VerificationTokenMustBeUnverified: only unverified DropShops can be verified. */
export function assertVerificationTokenMustBeUnverified(shop: DropShop): void {
  if (shop.isVerified) {
    throw new Error("DropShop is already verified");
  }
}

/** OnlyExistingDropShopAdminsCanMutateMembership: membership mutations require admin status. */
export function assertOnlyExistingDropShopAdminsCanMutateMembership(
  shop: DropShop,
  actorId: string,
): void {
  if (!shop.admins.includes(actorId)) {
    throw new Error("Only existing admins can mutate DropShop membership");
  }
}

/** VerificationCodeExpiresBeforeConfirmation: codes must not be expired. */
export function assertVerificationCodeExpiresBeforeConfirmation(shop: DropShop): void {
  if (shop.verificationExpiresAt && shop.verificationExpiresAt < new Date()) {
    throw new Error("Verification code has expired");
  }
}

/** VerifiedDropShopsMustRetainAtLeastOneAdmin: cannot remove the last admin. */
export function assertVerifiedDropShopsMustRetainAtLeastOneAdmin(shop: DropShop): void {
  if (shop.isVerified && shop.admins.length <= 1) {
    throw new Error("Verified DropShops must retain at least one administrator");
  }
}
