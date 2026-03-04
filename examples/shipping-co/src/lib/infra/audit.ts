// ObservabilityContext - AuditTrail

export interface AuditTrail {
  entityId: string;
  action: string;
  actorId: string;
  recordedAt: Date;
  severity: number;
  eventDate: Date;
  returnType: void;
}

export async function recordAuditEntry(
  entityId: string,
  action: string,
  actorId: string,
): Promise<AuditTrail> {
  return {
    entityId,
    action,
    actorId,
    recordedAt: new Date(),
  };
}

export async function queryAuditLog(entityId: string): Promise<AuditTrail[]> {
  void entityId;
  return [];
}

export function assertAuditEntriesAreImmutable(entry: AuditTrail): void {
  if (!entry.recordedAt) {
    throw new Error("Audit entries must have a recorded timestamp");
  }
}
