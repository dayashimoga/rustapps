import { describe, it, expect } from 'vitest';
import type { SecretMetadata, SecretPayload } from './types';

// Password strength calculation test
function computeStrength(pwd: string) {
  const length = pwd.length;
  if (length === 0) {
    return { entropy_bits: 0, rating: 'very_weak' };
  }

  const has_lowercase = /[a-z]/.test(pwd);
  const has_uppercase = /[A-Z]/.test(pwd);
  const has_digits = /[0-9]/.test(pwd);
  const has_symbols = /[^a-zA-Z0-9]/.test(pwd);

  let pool = 0;
  if (has_lowercase) pool += 26;
  if (has_uppercase) pool += 26;
  if (has_digits) pool += 10;
  if (has_symbols) pool += 33;

  const entropy_bits = pool > 0 ? length * Math.log2(pool) : 0;
  let rating = 'very_weak';
  if (entropy_bits >= 80 && length >= 16) rating = 'very_strong';
  else if (entropy_bits >= 60 && length >= 12) rating = 'strong';
  else if (entropy_bits >= 45 && length >= 8) rating = 'fair';
  else if (entropy_bits >= 28) rating = 'weak';

  return { entropy_bits, rating };
}

function getPrimarySecretValue(payload: SecretPayload): string {
  switch (payload.type) {
    case 'password': return payload.data.password;
    case 'ssh_key': return payload.data.private_key;
    case 'certificate': return payload.data.private_key;
    case 'api_key': return payload.data.api_key;
    case 'database_credential': return payload.data.password;
    case 'aws_credential': return payload.data.secret_access_key;
    case 'azure_credential': return payload.data.client_secret;
    case 'terraform_secret': return payload.data.tf_vars ?? '';
    case 'totp': return payload.data.secret_base32;
  }
}

describe('DevOps Secret Vault Frontend Utilities', () => {
  it('correctly calculates password entropy and rating', () => {
    const weak = computeStrength('123456');
    expect(weak.rating).toBe('very_weak');

    const fair = computeStrength('AdminPass1');
    expect(fair.rating).toBe('fair');

    const strong = computeStrength('C0rrect-H0rse-Battery-Staple!#2026');
    expect(strong.rating).toBe('very_strong');
    expect(strong.entropy_bits).toBeGreaterThan(80);
  });

  it('extracts primary secret values across all payload variants', () => {
    const pwd: SecretPayload = { type: 'password', data: { password: 'my-pass-123' } };
    expect(getPrimarySecretValue(pwd)).toBe('my-pass-123');

    const ssh: SecretPayload = {
      type: 'ssh_key',
      data: { private_key: '-----BEGIN OPENSSH PRIVATE KEY-----', public_key: null, passphrase: null },
    };
    expect(getPrimarySecretValue(ssh)).toContain('OPENSSH PRIVATE KEY');

    const cert: SecretPayload = {
      type: 'certificate',
      data: { private_key: '-----BEGIN RSA PRIVATE KEY-----', certificate_pem: 'cert', chain_pem: null },
    };
    expect(getPrimarySecretValue(cert)).toContain('RSA PRIVATE KEY');

    const api: SecretPayload = {
      type: 'api_key',
      data: { api_key: 'api-token-999', api_secret: null, endpoint_url: null },
    };
    expect(getPrimarySecretValue(api)).toBe('api-token-999');

    const db: SecretPayload = {
      type: 'database_credential',
      data: {
        engine: 'postgresql',
        host: 'localhost',
        port: 5432,
        database: 'db',
        username: 'usr',
        password: 'db-pass-456',
        connection_string: null,
      },
    };
    expect(getPrimarySecretValue(db)).toBe('db-pass-456');

    const aws: SecretPayload = {
      type: 'aws_credential',
      data: {
        access_key_id: 'AKIAIOSFODNN7EXAMPLE',
        secret_access_key: 'wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY',
        region: 'us-east-1',
        session_token: null,
      },
    };
    expect(getPrimarySecretValue(aws)).toBe('wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY');

    const az: SecretPayload = {
      type: 'azure_credential',
      data: {
        subscription_id: 'sub-001',
        tenant_id: 'tenant-123',
        client_id: 'app-456',
        client_secret: 'azure-secret-value-789',
      },
    };
    expect(getPrimarySecretValue(az)).toBe('azure-secret-value-789');

    const tf: SecretPayload = {
      type: 'terraform_secret',
      data: {
        tf_vars: 'db_password = "supersecretpassword"\napi_key = "tf-api-key"',
      },
    };
    expect(getPrimarySecretValue(tf)).toContain('supersecretpassword');

    const totp: SecretPayload = {
      type: 'totp',
      data: {
        secret_base32: 'JBSWY3DPEHPK3PXP',
        issuer: 'GitHub',
        account_name: 'devops-admin',
        algorithm: 'SHA1',
        digits: 6,
        period_seconds: 30,
      },
    };
    expect(getPrimarySecretValue(totp)).toBe('JBSWY3DPEHPK3PXP');
  });

  it('filters secrets by type and search query correctly', () => {
    const items: SecretMetadata[] = [
      {
        id: '1',
        title: 'AWS Production Access',
        secret_type: 'api_key',
        tags: ['aws', 'prod'],
        username: 'admin',
        url: null,
        notes: null,
        version: 1,
        created_at: 100,
        updated_at: 100,
      },
      {
        id: '2',
        title: 'Internal DB Cluster',
        secret_type: 'database_credential',
        tags: ['db', 'prod'],
        username: 'postgres',
        url: null,
        notes: null,
        version: 1,
        created_at: 100,
        updated_at: 100,
      },
    ];

    const apiOnly = items.filter(s => s.secret_type === 'api_key');
    expect(apiOnly).toHaveLength(1);
    expect(apiOnly[0]!.id).toBe('1');

    const searchHits = items.filter(s => s.title.toLowerCase().includes('database') || s.tags.includes('db'));
    expect(searchHits).toHaveLength(1);
    expect(searchHits[0]!.id).toBe('2');
  });
});
