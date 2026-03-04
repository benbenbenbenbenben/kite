// Arity demo: processOrder has 3 params but kite declares only 1

export interface ArityDemo {
  id: string;
}

export function processOrder(orderId: string, priority: number, express: boolean): void {
  void orderId;
  void priority;
  void express;
}
