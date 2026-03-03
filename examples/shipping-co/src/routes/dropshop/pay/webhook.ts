// ── PaymentsAndFulfillmentContext · PaymentReference ─────────────────────────
//
// Handles Stripe checkout sessions, webhook processing, and payment
// reconciliation for Drop fulfillment.

export interface PaymentReference {
  paymentReferenceId: string;
  stripeSession: string;
  amountMinor: number;
  currency: string;
  paymentStatus: string;
  paidAt?: Date;
  dropId: string;
  customerClaim: string;
}

export interface StripeWebhookEvent {
  id: string;
  type: string;
  data: Record<string, unknown>;
}

// ── Commands ─────────────────────────────────────────────────────────────────

export async function createCheckoutSession(
  dropId: string,
  claim: string,
): Promise<{ sessionUrl: string }> {
  // Create a Stripe Checkout Session for the given Drop.
  void dropId;
  void claim;
  return { sessionUrl: "https://checkout.stripe.com/placeholder" };
}

export async function processStripeWebhook(): Promise<void> {
  // Process an incoming Stripe webhook event.
  // Idempotent: duplicate event IDs are ignored.
}

export async function reconcilePaymentReference(
  paymentReferenceId: string,
): Promise<void> {
  // Reconcile a PaymentReference against Stripe's current state.
  void paymentReferenceId;
}

// ── Invariants ───────────────────────────────────────────────────────────────

const processedEventIds = new Set<string>();

/** PaidCheckoutCreatesPaymentReference: a paid checkout must create a PaymentReference. */
export function assertPaidCheckoutCreatesPaymentReference(
  paymentRef: PaymentReference | null,
  checkoutPaid: boolean,
): void {
  if (checkoutPaid && !paymentRef) {
    throw new Error("A paid checkout must create a PaymentReference");
  }
}

/** SuccessfulPaymentActivatesDropAndDispatchAddressee: paid Drops must be activated. */
export function assertSuccessfulPaymentActivatesDropAndDispatchAddressee(
  paymentRef: PaymentReference,
  dropActivated: boolean,
): void {
  if (paymentRef.paymentStatus === "paid" && !dropActivated) {
    throw new Error(
      "Successful payment must activate the Drop and dispatch the addressee notification",
    );
  }
}

/** WebhookEventsAreIdempotentPerStripeEventId: duplicate events must be no-ops. */
export function assertWebhookEventsAreIdempotentPerStripeEventId(
  event: StripeWebhookEvent,
): boolean {
  if (processedEventIds.has(event.id)) {
    return false; // Already processed — skip.
  }
  processedEventIds.add(event.id);
  return true;
}

/** CapturedPaymentsMustReferenceExistingDrop: payments must reference a real Drop. */
export function assertCapturedPaymentsMustReferenceExistingDrop(
  paymentRef: PaymentReference,
  dropExists: boolean,
): void {
  if (!dropExists) {
    throw new Error(
      `PaymentReference ${paymentRef.paymentReferenceId} references non-existent Drop ${paymentRef.dropId}`,
    );
  }
}
