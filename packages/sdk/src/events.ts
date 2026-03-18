/**
 * Typed event emitter for s1engine SDK.
 */
export class EventEmitter<T extends Record<string, unknown[]> = Record<string, unknown[]>> {
  private listeners = new Map<keyof T, Set<Function>>();

  /** Subscribe to an event. Returns an unsubscribe function. */
  on<K extends keyof T>(event: K, callback: (...args: T[K]) => void): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(callback);
    return () => this.off(event, callback);
  }

  /** Subscribe to an event for one invocation only. */
  once<K extends keyof T>(event: K, callback: (...args: T[K]) => void): () => void {
    const wrapper = (...args: T[K]) => {
      this.off(event, wrapper);
      callback(...args);
    };
    return this.on(event, wrapper);
  }

  /** Unsubscribe from an event. */
  off<K extends keyof T>(event: K, callback: Function): void {
    this.listeners.get(event)?.delete(callback);
  }

  /** Remove all listeners for an event, or all events if no event specified. */
  removeAllListeners(event?: keyof T): void {
    if (event) {
      this.listeners.delete(event);
    } else {
      this.listeners.clear();
    }
  }

  /** Emit an event to all subscribers. */
  protected emit<K extends keyof T>(event: K, ...args: T[K]): void {
    const callbacks = this.listeners.get(event);
    if (!callbacks) return;
    for (const cb of callbacks) {
      try {
        cb(...args);
      } catch (e) {
        console.error(`[s1engine] Error in ${String(event)} listener:`, e);
      }
    }
  }

  /** Get the number of listeners for an event. */
  listenerCount(event: keyof T): number {
    return this.listeners.get(event)?.size ?? 0;
  }
}
