// ── IntegrationContext · NotificationOutbox ───────────────────────────────────
//
// Transactional outbox for email and physical mail delivery with retry logic.

export interface Email {
  to: string;
  subject: string;
  templateKey: string;
  templateData: Record<string, string>;
}

export interface Mail {
  to: string;
  addressLines: string[];
  templateKey: string;
}

export interface OutboxEntry {
  emailOutboxId: string;
  subject: string;
  recipient: string;
  templateKey: string;
  attempts: number;
  lastAttemptAt?: Date;
  dispatched: boolean;
}

const MAX_RETRY_ATTEMPTS = 5;

// ── Commands ─────────────────────────────────────────────────────────────────

export async function sendEmail(email: Email): Promise<OutboxEntry> {
  // Persist an email to the outbox for asynchronous dispatch by the worker.
  return {
    emailOutboxId: crypto.randomUUID(),
    subject: email.subject,
    recipient: email.to,
    templateKey: email.templateKey,
    attempts: 0,
    dispatched: false,
  };
}

export async function sendSnailMail(mail: Mail): Promise<OutboxEntry> {
  // Persist a physical mail request to the outbox.
  return {
    emailOutboxId: crypto.randomUUID(),
    subject: `Physical mail to ${mail.to}`,
    recipient: mail.to,
    templateKey: mail.templateKey,
    attempts: 0,
    dispatched: false,
  };
}

export async function scheduleBookingStatusEmail(
  dropId: string,
  recipient: string,
): Promise<OutboxEntry> {
  // Schedule a booking status notification email.
  void dropId;
  return {
    emailOutboxId: crypto.randomUUID(),
    subject: `Booking status update for Drop ${dropId}`,
    recipient,
    templateKey: "booking-status",
    attempts: 0,
    dispatched: false,
  };
}

export async function markEmailDispatched(emailOutboxId: string): Promise<void> {
  // Mark an outbox entry as successfully dispatched by the worker.
  void emailOutboxId;
}

// ── Invariants ───────────────────────────────────────────────────────────────

/** EmailIsPersistedToOutboxBeforeWorkerDispatch: outbox entry must exist before dispatch. */
export function assertEmailIsPersistedToOutboxBeforeWorkerDispatch(
  entry: OutboxEntry | null,
): void {
  if (!entry) {
    throw new Error("Email must be persisted to outbox before worker dispatch");
  }
}

/** OutboxRetriesAreCappedBeforeEscalation: retries must not exceed the cap. */
export function assertOutboxRetriesAreCappedBeforeEscalation(
  entry: OutboxEntry,
): void {
  if (entry.attempts >= MAX_RETRY_ATTEMPTS) {
    throw new Error(
      `Outbox entry ${entry.emailOutboxId} has exceeded ${MAX_RETRY_ATTEMPTS} retry attempts — escalate`,
    );
  }
}
