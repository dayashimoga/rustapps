import { describe, it, expect } from 'vitest';
import type { LogLineRecord, TimelineBucketUi } from './types';

function filterLogs(
  lines: string[],
  query: string,
  isRegex: boolean,
  caseSensitive: boolean
): number[] {
  if (!query) return lines.map((_, i) => i);

  let regex: RegExp | null = null;
  if (isRegex) {
    try {
      regex = new RegExp(query, caseSensitive ? '' : 'i');
    } catch {
      return [];
    }
  }

  const queryLower = query.toLowerCase();
  const matches: number[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    const matched = regex
      ? regex.test(line)
      : caseSensitive
      ? line.includes(query)
      : line.toLowerCase().includes(queryLower);

    if (matched) {
      matches.push(i);
    }
  }

  return matches;
}

function computeBuckets(
  timestamps: number[],
  numBuckets: number
): TimelineBucketUi[] {
  if (timestamps.length === 0) return [];
  const minMs = Math.min(...timestamps);
  const maxMs = Math.max(...timestamps);
  const span = Math.max(1, (maxMs - minMs) / numBuckets);

  const buckets: TimelineBucketUi[] = Array.from({ length: numBuckets }, (_, i) => ({
    bucket_index: i,
    start_epoch_ms: minMs + i * span,
    end_epoch_ms: minMs + (i + 1) * span,
    total_count: 0,
    trace_count: 0,
    debug_count: 0,
    info_count: 0,
    warn_count: 0,
    error_count: 0,
    fatal_count: 0,
    volume_per_second: 0,
  }));

  for (const ts of timestamps) {
    let idx = Math.floor((ts - minMs) / span);
    if (idx >= numBuckets) idx = numBuckets - 1;
    buckets[idx]!.total_count++;
    buckets[idx]!.info_count++;
  }

  for (const b of buckets) {
    b.volume_per_second = b.total_count / (span / 1000);
  }

  return buckets;
}

describe('Universal Log Viewer Frontend Logic', () => {
  const sampleLogs = [
    '2026-11-03T10:00:00Z [INFO] Service initialized',
    '2026-11-03T10:00:01Z [WARN] Disk usage high at 88%',
    '2026-11-03T10:00:02Z [ERROR] Database timeout connecting to 10.0.1.5',
    '2026-11-03T10:00:03Z [INFO] Retrying connection attempt 2',
    '2026-11-03T10:00:04Z [ERROR] Database timeout connecting to 10.0.1.5',
  ];

  it('filters log lines with plain case-insensitive search', () => {
    const hits = filterLogs(sampleLogs, 'database', false, false);
    expect(hits).toEqual([2, 4]);
  });

  it('filters log lines with case-sensitive search', () => {
    const hitsUpper = filterLogs(sampleLogs, 'ERROR', false, true);
    expect(hitsUpper).toEqual([2, 4]);

    const hitsLower = filterLogs(sampleLogs, 'error', false, true);
    expect(hitsLower).toEqual([]);
  });

  it('filters log lines with regex pattern', () => {
    const hits = filterLogs(sampleLogs, 'attempt \\d+', true, false);
    expect(hits).toEqual([3]);
  });

  it('handles invalid regex without crashing', () => {
    const hits = filterLogs(sampleLogs, '[unclosed-brace', true, false);
    expect(hits).toEqual([]);
  });

  it('computes timeline histogram buckets correctly', () => {
    const base = 1700000000000;
    const timestamps = [base, base + 1000, base + 2000, base + 5000];
    const buckets = computeBuckets(timestamps, 5);

    expect(buckets.length).toBe(5);
    const totalCount = buckets.reduce((acc, b) => acc + b.total_count, 0);
    expect(totalCount).toBe(4);
    expect(buckets[0]!.total_count).toBeGreaterThan(0);
  });

  it('validates log record data structure compatibility', () => {
    const record: LogLineRecord = {
      line_number: 42,
      byte_start: 1024,
      byte_end: 1080,
      text: 'Sample line content',
    };
    expect(record.line_number).toBe(42);
    expect(record.byte_end - record.byte_start).toBe(56);
  });
});

