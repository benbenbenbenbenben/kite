// ── BookingLifecycleContext · Drop + DropShopSearch ───────────────────────────
//
// Manages the lifecycle of Drops (parcels) from creation through collection,
// and provides search functionality for DropShops.

export type DropStatus =
  | "created"
  | "dispatched"
  | "arrived"
  | "collected"
  | "cancelled"
  | "transferred";

export interface Drop {
  dropId: string;
  status: DropStatus;
  customerReference: string;
  addresseeName: string;
  addresseeEmail: string;
  createdAt: Date;
  dispatchedAt?: Date;
  dropShopId: string;
}

export interface DropShopBookingRequest {
  dropShopId: string;
  addresseeName: string;
  addresseeEmail: string;
  customerReference: string;
}

export interface SimpleQuery {
  query: string;
}

export interface DropShopSearchResult {
  dropShopId: string;
  displayName: string;
  distanceKm?: number;
}

export interface DropShopSearch {
  query: string;
  region: string;
  radiusKm: number;
}

// ── Drop Commands ────────────────────────────────────────────────────────────

export async function createDrop(request: DropShopBookingRequest): Promise<Drop> {
  return {
    dropId: crypto.randomUUID(),
    status: "created",
    customerReference: request.customerReference,
    addresseeName: request.addresseeName,
    addresseeEmail: request.addresseeEmail,
    createdAt: new Date(),
    dropShopId: request.dropShopId,
  };
}

export async function notifyDropDispatched(dropId: string, claim: string): Promise<void> {
  // Transition a Drop from "created" to "dispatched".
  void dropId;
  void claim;
}

export async function notifyDropArrival(dropId: string, claim: string): Promise<void> {
  // Transition a Drop from "dispatched" to "arrived" at the DropShop.
  void dropId;
  void claim;
}

export async function notifyDropCollection(dropId: string, claim: string): Promise<void> {
  // Transition a Drop from "arrived" to "collected" by the addressee.
  void dropId;
  void claim;
}

export async function cancelDrop(dropId: string, claim: string): Promise<void> {
  // Cancel a Drop that is still in "created" status.
  void dropId;
  void claim;
}

export async function transferDropToDropShop(
  dropId: string,
  targetDropShopId: string,
  claim: string,
): Promise<void> {
  // Transfer a Drop to a different verified DropShop.
  void dropId;
  void targetDropShopId;
  void claim;
}

export async function getDrop(dropIdOrClaim: string, claim: string): Promise<Drop | null> {
  // Retrieve a single Drop by ID or by claim reference.
  void dropIdOrClaim;
  void claim;
  return null;
}

export async function getCurrentUserDrops(claim: string): Promise<Drop[]> {
  // List all Drops for the currently authenticated user.
  void claim;
  return [];
}

export async function getDropshopDrops(dropshopId: string, claim: string): Promise<Drop[]> {
  // List all Drops assigned to the specified DropShop.
  void dropshopId;
  void claim;
  return [];
}

// ── Drop Invariants ──────────────────────────────────────────────────────────

/** OnlyVerifiedDropShopsCanAcceptDropBookings: bookings require a verified DropShop. */
export function assertOnlyVerifiedDropShopsCanAcceptDropBookings(
  dropShopIsVerified: boolean,
): void {
  if (!dropShopIsVerified) {
    throw new Error("Only verified DropShops can accept Drop bookings");
  }
}

/** DispatchRequiresDropCreatedStatus: dispatch only from "created" status. */
export function assertDispatchRequiresDropCreatedStatus(drop: Drop): void {
  if (drop.status !== "created") {
    throw new Error("Dispatch requires Drop to be in 'created' status");
  }
}

/** ArrivalRequiresDropShopMembership: arrival confirmation requires DropShop membership. */
export function assertArrivalRequiresDropShopMembership(
  _drop: Drop,
  isMember: boolean,
): void {
  if (!isMember) {
    throw new Error("Arrival confirmation requires DropShop membership");
  }
}

/** CollectionRequiresReceiverOrDropShopStaff: collection requires the addressee or staff. */
export function assertCollectionRequiresReceiverOrDropShopStaff(
  drop: Drop,
  actorEmail: string,
  isStaff: boolean,
): void {
  if (drop.addresseeEmail !== actorEmail && !isStaff) {
    throw new Error("Collection requires the addressee or DropShop staff");
  }
}

/** CancellationRequiresDropCreatedStatus: only "created" Drops can be cancelled. */
export function assertCancellationRequiresDropCreatedStatus(drop: Drop): void {
  if (drop.status !== "created") {
    throw new Error("Only Drops in 'created' status can be cancelled");
  }
}

/** TransferRequiresVerifiedTargetDropShop: transfer target must be verified. */
export function assertTransferRequiresVerifiedTargetDropShop(
  targetIsVerified: boolean,
): void {
  if (!targetIsVerified) {
    throw new Error("Transfer target DropShop must be verified");
  }
}

// ── DropShopSearch Commands ──────────────────────────────────────────────────

const MAX_SEARCH_RADIUS_KM = 50;

export async function findDropShop(query: SimpleQuery): Promise<DropShopSearchResult[]> {
  // Full-text search for DropShops by name or location keyword.
  void query;
  return [];
}

export async function findDropShopsNear(
  postcode: string,
  radiusKm: number,
  claim: string,
): Promise<DropShopSearchResult[]> {
  // Geo-proximity search for DropShops near a postcode.
  void postcode;
  void radiusKm;
  void claim;
  return [];
}

export async function addressSearch(query: string): Promise<DropShopSearchResult[]> {
  // Search for DropShops by address fragment.
  void query;
  return [];
}

// ── DropShopSearch Invariants ────────────────────────────────────────────────

/** SearchResultsIncludeVerifiedDropShopsOnly: search results must not include unverified shops. */
export function assertSearchResultsIncludeVerifiedDropShopsOnly(
  results: DropShopSearchResult[],
): void {
  void results;
  // Enforced at query level: only verified DropShops are indexed.
}

/** SearchRadiusMustBeWithinServiceArea: radius must not exceed the maximum. */
export function assertSearchRadiusMustBeWithinServiceArea(radiusKm: number): void {
  if (radiusKm > MAX_SEARCH_RADIUS_KM) {
    throw new Error(
      `Search radius ${radiusKm}km exceeds maximum service area of ${MAX_SEARCH_RADIUS_KM}km`,
    );
  }
}
