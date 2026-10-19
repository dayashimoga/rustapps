import { describe, it, expect } from 'vitest';
import type { BatchRenameRule, FileItem } from './types';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function computeRename(originalName: string, rule: BatchRenameRule, index = 0): string {
  const parts = originalName.split('.');
  const ext = parts.length > 1 ? '.' + parts.pop() : '';
  const stem = parts.join('.');

  switch (rule.type) {
    case 'PrefixSuffix':
      return `${rule.config.prefix}${stem}${rule.config.suffix}${ext}`;
    case 'Numbering': {
      const num = String(rule.config.start_number + index).padStart(rule.config.pad_digits, '0');
      return `${rule.config.prefix}${num}${ext}`;
    }
    case 'RegexReplace': {
      const re = new RegExp(rule.config.pattern, 'g');
      return originalName.replace(re, rule.config.replacement);
    }
    case 'ToLowerCase':
      return originalName.toLowerCase();
    case 'ToUpperCase':
      return originalName.toUpperCase();
  }
}

describe('AI File Explorer Frontend Utilities', () => {
  it('correctly formats byte sizes into human-readable strings', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1024)).toBe('1 KB');
    expect(formatBytes(1048576)).toBe('1 MB');
    expect(formatBytes(1073741824)).toBe('1 GB');
    expect(formatBytes(1073741824 * 2.5)).toBe('2.5 GB');
  });

  it('correctly simulates batch rename rules', () => {
    // Numbering rule
    const numRule: BatchRenameRule = {
      type: 'Numbering',
      config: { prefix: 'scan_', start_number: 1, pad_digits: 3 },
    };
    expect(computeRename('image.png', numRule, 0)).toBe('scan_001.png');
    expect(computeRename('image2.png', numRule, 5)).toBe('scan_006.png');

    // Prefix/Suffix rule
    const prefixSuffixRule: BatchRenameRule = {
      type: 'PrefixSuffix',
      config: { prefix: 'draft_', suffix: '_v2' },
    };
    expect(computeRename('spec.md', prefixSuffixRule)).toBe('draft_spec_v2.md');

    // Regex replace rule
    const regexRule: BatchRenameRule = {
      type: 'RegexReplace',
      config: { pattern: 'v1', replacement: 'final' },
    };
    expect(computeRename('report_v1.docx', regexRule)).toBe('report_final.docx');

    // Case conversions
    expect(computeRename('MyDocument.PDF', { type: 'ToLowerCase' })).toBe('mydocument.pdf');
    expect(computeRename('MyDocument.pdf', { type: 'ToUpperCase' })).toBe('MYDOCUMENT.PDF');
  });

  it('sorts and filters file items properly', () => {
    const files: FileItem[] = [
      { path: '/a/doc.txt', file_name: 'doc.txt', extension: 'txt', size_bytes: 500, is_dir: false, modified_timestamp_secs: 100, is_readonly: false },
      { path: '/a/src', file_name: 'src', extension: null, size_bytes: 0, is_dir: true, modified_timestamp_secs: 105, is_readonly: false },
      { path: '/a/app.rs', file_name: 'app.rs', extension: 'rs', size_bytes: 12000, is_dir: false, modified_timestamp_secs: 110, is_readonly: false },
    ];

    // Folders first, then alphabetical
    const sorted = [...files].sort((a, b) => {
      if (a.is_dir && !b.is_dir) return -1;
      if (!a.is_dir && b.is_dir) return 1;
      return a.file_name.localeCompare(b.file_name);
    });

    expect(sorted[0].is_dir).toBe(true);
    expect(sorted[0].file_name).toBe('src');
    expect(sorted[1].file_name).toBe('app.rs');
    expect(sorted[2].file_name).toBe('doc.txt');
  });
});
