/**
 * Client-Side Undo/Redo History Manager
 *
 * Implements bounded history stack with branching history invalidation
 * and keyboard shortcut bindings (Ctrl+Z / Ctrl+Y / Cmd+Shift+Z).
 */

export interface UndoRedoOptions {
  maxCapacity?: number;
}

export class UndoRedoStack<T> {
  private past: T[] = [];
  private present: T;
  private future: T[] = [];
  private readonly maxCapacity: number;

  constructor(initialState: T, options: UndoRedoOptions = {}) {
    this.present = initialState;
    this.maxCapacity = options.maxCapacity ?? 100;
  }

  /**
   * Current active state
   */
  get current(): T {
    return this.present;
  }

  /**
   * Push a new state onto the history stack, clearing any redos.
   */
  push(nextState: T): void {
    this.past.push(this.present);
    if (this.past.length > this.maxCapacity) {
      this.past.shift();
    }
    this.present = nextState;
    this.future = [];
  }

  /**
   * Move one step backward in history.
   * Returns the restored state, or undefined if at the beginning.
   */
  undo(): T | undefined {
    if (!this.canUndo) {
      return undefined;
    }
    const previous = this.past.pop()!;
    this.future.unshift(this.present);
    this.present = previous;
    return this.present;
  }

  /**
   * Move one step forward in history.
   * Returns the restored state, or undefined if at the latest state.
   */
  redo(): T | undefined {
    if (!this.canRedo) {
      return undefined;
    }
    const next = this.future.shift()!;
    this.past.push(this.present);
    this.present = next;
    return this.present;
  }

  get canUndo(): boolean {
    return this.past.length > 0;
  }

  get canRedo(): boolean {
    return this.future.length > 0;
  }

  get pastCount(): number {
    return this.past.length;
  }

  get futureCount(): number {
    return this.future.length;
  }

  /**
   * Reset stack with a fresh initial state.
   */
  reset(initialState: T): void {
    this.past = [];
    this.present = initialState;
    this.future = [];
  }

  /**
   * Peek at the previous state without mutating history.
   */
  peekUndo(): T | undefined {
    return this.past.length > 0 ? this.past[this.past.length - 1] : undefined;
  }

  /**
   * Peek at the next state without mutating history.
   */
  peekRedo(): T | undefined {
    return this.future.length > 0 ? this.future[0] : undefined;
  }
}

/**
 * Attaches Ctrl+Z (Undo) and Ctrl+Y / Cmd+Shift+Z (Redo) window event listeners.
 * Returns an unbind cleanup function.
 */
export function setupUndoRedoKeyboard(options: {
  onUndo: () => void;
  onRedo: () => void;
  target?: Window | HTMLElement;
}): () => void {
  const target = options.target || (typeof window !== 'undefined' ? window : null);
  if (!target) return () => {};

  const handler = (e: Event) => {
    const keyEvent = e as KeyboardEvent;
    const isMeta = keyEvent.metaKey || keyEvent.ctrlKey;
    if (!isMeta) return;

    // Check if user is typing in an active text input or textarea
    const activeEl = document.activeElement;
    if (
      activeEl &&
      (activeEl.tagName === 'INPUT' || activeEl.tagName === 'TEXTAREA' || (activeEl as HTMLElement).isContentEditable)
    ) {
      return;
    }

    if (keyEvent.key.toLowerCase() === 'z') {
      if (keyEvent.shiftKey) {
        // Redo (Cmd+Shift+Z)
        keyEvent.preventDefault();
        options.onRedo();
      } else {
        // Undo (Ctrl+Z / Cmd+Z)
        keyEvent.preventDefault();
        options.onUndo();
      }
    } else if (keyEvent.key.toLowerCase() === 'y') {
      // Redo (Ctrl+Y)
      keyEvent.preventDefault();
      options.onRedo();
    }
  };

  target.addEventListener('keydown', handler);
  return () => {
    target.removeEventListener('keydown', handler);
  };
}
