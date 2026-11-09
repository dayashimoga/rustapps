/**
 * Antigravity UI Kit
 * "Precision Carbon & Obsidian Glass" Design System & Desktop IPC Bridge
 */

// Re-export IPC Client & Types
export * from './ipc/types';
export * from './ipc/client';

// Re-export Component Interfaces
export * from './components/types';

// Re-export Svelte Components
export { default as AgCommandPalette } from './components/AgCommandPalette.svelte';
export { default as AgLicenseModal } from './components/AgLicenseModal.svelte';
export { default as AgVirtualGrid } from './components/AgVirtualGrid.svelte';
export { default as AgSplitPane } from './components/AgSplitPane.svelte';
export { default as AgChartCanvas } from './components/AgChartCanvas.svelte';
export { default as AgUndoToast } from './components/AgUndoToast.svelte';

