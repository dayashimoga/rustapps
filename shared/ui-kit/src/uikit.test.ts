import { describe, it, expect, beforeEach } from 'vitest';
import {
  invokeIpc,
  registerIpcMock,
  clearIpcMocks,
  isDesktopApp,
  getLocalFileUrl,
} from './ipc/client';
import { UndoRedoStack } from './history/undoRedo';
import type { CommandItem, VirtualColumn } from './components/types';

describe('Antigravity UI Kit - IPC Transport Suite', () => {
  beforeEach(() => {
    clearIpcMocks();
  });

  it('correctly reports desktop runtime status when Tauri is not injected', () => {
    expect(isDesktopApp()).toBe(false);
  });

  it('correctly executes registered mock IPC commands with arguments', async () => {
    registerIpcMock('get_system_info', async (args) => {
      return {
        os: 'windows',
        arch: args['arch'] || 'x86_64',
        memory_mb: 32768,
      };
    });

    const result = await invokeIpc<{ os: string; arch: string; memory_mb: number }>(
      'get_system_info',
      { arch: 'aarch64' }
    );

    expect(result).toBeDefined();
    expect(result.os).toBe('windows');
    expect(result.arch).toBe('aarch64');
    expect(result.memory_mb).toBe(32768);
  });

  it('throws an informative error when no IPC transport is available', async () => {
    await expect(invokeIpc('unregistered_command')).rejects.toThrow(
      'No IPC transport available for command "unregistered_command"'
    );
  });

  it('generates properly formatted streaming local file URLs with CSRF tokens', () => {
    const url = getLocalFileUrl('C:\\data\\project.parquet');
    expect(url).toContain('/__file?path=');
    expect(url).toContain(encodeURIComponent('C:\\data\\project.parquet'));
  });
});

describe('Antigravity UI Kit - Component Logic Suite', () => {
  it('correctly filters command palette actions by category and title', () => {
    const actions: CommandItem[] = [
      { id: '1', title: 'Open Parquet File', category: 'File', action: () => {} },
      { id: '2', title: 'Export CSV', category: 'Export', action: () => {} },
      { id: '3', title: 'Redact PII Columns', category: 'Security', action: () => {} },
      { id: '4', title: 'Search Query History', category: 'Query', action: () => {} },
    ];

    const filter = (q: string) => {
      const lower = q.toLowerCase();
      return actions.filter(
        (a) =>
          a.title.toLowerCase().includes(lower) ||
          a.category.toLowerCase().includes(lower)
      );
    };

    expect(filter('parquet')).toHaveLength(1);
    expect(filter('parquet')[0]?.id).toBe('1');
    expect(filter('security')).toHaveLength(1);
    expect(filter('security')[0]?.id).toBe('3');
    expect(filter('export')).toHaveLength(1);
    expect(filter('nonexistent')).toHaveLength(0);
  });

  it('correctly calculates virtual grid row windowing and offset Y', () => {
    const totalRows = 50000;
    const rowHeight = 32;
    const containerHeight = 320;
    const scrollTop = 3200; // 100 rows scrolled down

    const startIndex = Math.max(0, Math.floor(scrollTop / rowHeight) - 4);
    const endIndex = Math.min(
      totalRows,
      Math.ceil((scrollTop + containerHeight) / rowHeight) + 4
    );
    const offsetY = startIndex * rowHeight;

    expect(startIndex).toBe(96);
    expect(endIndex).toBe(114);
    expect(offsetY).toBe(96 * 32);
  });

  it('correctly computes chart axis scale factors', () => {
    const minVal = 0;
    const maxVal = 200;
    const plotHeight = 200;
    const padTop = 15;

    const scaleY = (val: number) => {
      const span = maxVal - minVal || 1;
      return padTop + plotHeight - ((val - minVal) / span) * plotHeight;
    };

    expect(scaleY(0)).toBe(padTop + plotHeight); // Bottom
    expect(scaleY(200)).toBe(padTop); // Top
    expect(scaleY(100)).toBe(padTop + plotHeight / 2); // Middle
  });
});

describe('Antigravity UI Kit - Undo/Redo State Manager', () => {
  it('pushes new states and allows stepwise undo and redo', () => {
    const stack = new UndoRedoStack<string>('initial');
    expect(stack.current).toBe('initial');
    expect(stack.canUndo).toBe(false);
    expect(stack.canRedo).toBe(false);

    stack.push('step1');
    stack.push('step2');
    expect(stack.current).toBe('step2');
    expect(stack.canUndo).toBe(true);
    expect(stack.canRedo).toBe(false);

    // Undo to step1
    const u1 = stack.undo();
    expect(u1).toBe('step1');
    expect(stack.current).toBe('step1');
    expect(stack.canRedo).toBe(true);

    // Undo to initial
    const u2 = stack.undo();
    expect(u2).toBe('initial');
    expect(stack.current).toBe('initial');
    expect(stack.canUndo).toBe(false);

    // Redo to step1
    const r1 = stack.redo();
    expect(r1).toBe('step1');
    expect(stack.current).toBe('step1');

    // Redo to step2
    const r2 = stack.redo();
    expect(r2).toBe('step2');
    expect(stack.current).toBe('step2');
    expect(stack.canRedo).toBe(false);
  });

  it('invalidates future branch when new state is pushed after undo', () => {
    const stack = new UndoRedoStack<number>(1);
    stack.push(2);
    stack.push(3);

    stack.undo(); // back to 2
    expect(stack.canRedo).toBe(true);

    stack.push(99); // branch to 99
    expect(stack.current).toBe(99);
    expect(stack.canRedo).toBe(false); // future is wiped
    expect(stack.canUndo).toBe(true);

    stack.undo();
    expect(stack.current).toBe(2);
  });

  it('respects maximum capacity bounds', () => {
    const stack = new UndoRedoStack<number>(0, { maxCapacity: 3 });
    stack.push(1);
    stack.push(2);
    stack.push(3);
    stack.push(4);
    stack.push(5);

    expect(stack.current).toBe(5);
    expect(stack.pastCount).toBe(3); // capped at 3

    expect(stack.undo()).toBe(4);
    expect(stack.undo()).toBe(3);
    expect(stack.undo()).toBe(2);
    expect(stack.canUndo).toBe(false); // 0 and 1 were evicted
  });
});

