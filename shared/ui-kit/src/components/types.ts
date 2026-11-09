/**
 * Component Interface Contracts for Antigravity UI Kit
 */

export interface CommandItem {
  id: string;
  title: string;
  category: string;
  shortcut?: string;
  description?: string;
  badge?: string;
  action: () => void | Promise<void>;
}

export interface VirtualColumn<T = any> {
  id: string;
  header: string;
  width?: number; // In px, or flex if omitted
  minWidth?: number;
  maxWidth?: number;
  align?: 'left' | 'center' | 'right';
  isMonospace?: boolean;
  sortable?: boolean;
  render?: (row: T, index: number) => string;
}

export type ChartType = 'line' | 'bar' | 'scatter' | 'sparkline';

export interface ChartSeries {
  id: string;
  name: string;
  color?: string;
  points: { x: number; y: number; label?: string }[];
}

export interface ToastMessage {
  id: string;
  title: string;
  description?: string;
  durationMs?: number;
  undoAction?: () => void | Promise<void>;
  undoLabel?: string;
  type?: 'info' | 'success' | 'warning' | 'error';
}

