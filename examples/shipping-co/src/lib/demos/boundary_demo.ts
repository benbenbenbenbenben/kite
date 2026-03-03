// Boundary demo: references IntentDemoContext (which is forbidden)
import { IntentDemo } from "./intent_demo";

export interface BoundaryDemo {
  id: string;
}

// This type alias references IntentDemoContext via a name that contains it
type IntentDemoContext = IntentDemo;

export function crossContextCall(): IntentDemoContext {
  return { id: "cross-context-violation" };
}
