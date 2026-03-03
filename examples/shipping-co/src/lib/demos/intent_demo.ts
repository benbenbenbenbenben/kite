// Intent demo: getProfile is a read-looking symbol bound to a write command

export interface IntentDemo {
  id: string;
}

export function getProfile(): Record<string, string> {
  return {};
}
