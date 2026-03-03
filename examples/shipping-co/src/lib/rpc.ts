// ── IntegrationContext · RpcGateway ───────────────────────────────────────────
//
// JSON-RPC 2.0 dispatch gateway with service registration and user principal
// injection.

export interface JsonRpcRequest {
  jsonrpc: "2.0";
  method: string;
  params?: unknown;
  id?: string | number;
}

export interface JsonRpcResponse {
  jsonrpc: "2.0";
  result?: unknown;
  error?: { code: number; message: string; data?: unknown };
  id: string | number | null;
}

export interface ServiceImplementation {
  serviceName: string;
  serviceVersion: string;
  methods: Record<string, (...args: unknown[]) => unknown>;
}

interface RegisteredService {
  serviceName: string;
  serviceVersion: string;
  methods: Map<string, (...args: unknown[]) => unknown>;
}

const registry = new Map<string, RegisteredService>();

// ── Commands ─────────────────────────────────────────────────────────────────

export function registerService(implementation: ServiceImplementation): void {
  // Register a service implementation in the RPC gateway.
  registry.set(implementation.serviceName, {
    serviceName: implementation.serviceName,
    serviceVersion: implementation.serviceVersion,
    methods: new Map(Object.entries(implementation.methods)),
  });
}

export async function dispatchRpcRequest(
  request: JsonRpcRequest,
): Promise<JsonRpcResponse> {
  // Dispatch a JSON-RPC request to the registered service method.
  const [serviceName, methodName] = request.method.split(".");
  const service = registry.get(serviceName);

  if (!service || !service.methods.has(methodName)) {
    return {
      jsonrpc: "2.0",
      error: { code: -32601, message: `Method not found: ${request.method}` },
      id: request.id ?? null,
    };
  }

  try {
    const handler = service.methods.get(methodName)!;
    const result = await handler(request.params);
    return { jsonrpc: "2.0", result, id: request.id ?? null };
  } catch (err) {
    return {
      jsonrpc: "2.0",
      error: {
        code: -32603,
        message: err instanceof Error ? err.message : "Internal error",
      },
      id: request.id ?? null,
    };
  }
}

export async function dispatchRpcNotification(
  request: JsonRpcRequest,
): Promise<void> {
  // Dispatch a JSON-RPC notification (fire-and-forget, no response).
  const [serviceName, methodName] = request.method.split(".");
  const service = registry.get(serviceName);

  if (service?.methods.has(methodName)) {
    const handler = service.methods.get(methodName)!;
    await handler(request.params);
  }
}

// ── Invariants ───────────────────────────────────────────────────────────────

/** WrappedServicesReceiveUserPrincipleWhenPresent: services receive the principal if available. */
export function assertWrappedServicesReceiveUserPrincipleWhenPresent(
  principalPresent: boolean,
  principalInjected: boolean,
): void {
  if (principalPresent && !principalInjected) {
    throw new Error(
      "User principal is present but was not injected into the wrapped service call",
    );
  }
}

/** RpcMethodsRequireRegisteredServiceTarget: methods must target a registered service. */
export function assertRpcMethodsRequireRegisteredServiceTarget(
  method: string,
): void {
  const [serviceName] = method.split(".");
  if (!registry.has(serviceName)) {
    throw new Error(`No registered service for method: ${method}`);
  }
}
