/// Domain models for DevOps Secret Vault Mobile
///
/// Closely mirrors the Rust domain entities defined in `vault_core::models`
/// and serde representations emitted across the C-ABI FFI boundary.

import 'dart:convert';

/// Metadata summary of a stored secret
class SecretMetadata {
  final String id;
  final String title;
  final String? description;
  final String secretType;
  final List<String> tags;
  final DateTime createdAt;
  final DateTime updatedAt;
  final DateTime? expiresAt;
  final int version;
  final double entropyScore;
  final int accessCount;
  final DateTime? lastAccessedAt;

  const SecretMetadata({
    required this.id,
    required this.title,
    this.description,
    required this.secretType,
    required this.tags,
    required this.createdAt,
    required this.updatedAt,
    this.expiresAt,
    required this.version,
    required this.entropyScore,
    required this.accessCount,
    this.lastAccessedAt,
  });

  factory SecretMetadata.fromJson(Map<String, dynamic> json) {
    return SecretMetadata(
      id: json['id'] as String? ?? '',
      title: json['title'] as String? ?? 'Untitled',
      description: json['description'] as String?,
      secretType: json['secret_type'] as String? ?? 'generic',
      tags: (json['tags'] as List<dynamic>?)
              ?.map((e) => e.toString())
              .toList() ??
          [],
      createdAt: json['created_at'] != null
          ? DateTime.tryParse(json['created_at'] as String) ?? DateTime.now()
          : DateTime.now(),
      updatedAt: json['updated_at'] != null
          ? DateTime.tryParse(json['updated_at'] as String) ?? DateTime.now()
          : DateTime.now(),
      expiresAt: json['expires_at'] != null
          ? DateTime.tryParse(json['expires_at'] as String)
          : null,
      version: (json['version'] as num?)?.toInt() ?? 1,
      entropyScore: (json['entropy_score'] as num?)?.toDouble() ?? 0.0,
      accessCount: (json['access_count'] as num?)?.toInt() ?? 0,
      lastAccessedAt: json['last_accessed_at'] != null
          ? DateTime.tryParse(json['last_accessed_at'] as String)
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
        'id': id,
        'title': title,
        'description': description,
        'secret_type': secretType,
        'tags': tags,
        'created_at': createdAt.toIso8601String(),
        'updated_at': updatedAt.toIso8601String(),
        'expires_at': expiresAt?.toIso8601String(),
        'version': version,
        'entropy_score': entropyScore,
        'access_count': accessCount,
        'last_accessed_at': lastAccessedAt?.toIso8601String(),
      };
}

/// Fully decrypted secret payload and associated metadata
class DecryptedSecret {
  final SecretMetadata metadata;
  final Map<String, dynamic> payload;

  const DecryptedSecret({
    required this.metadata,
    required this.payload,
  });

  factory DecryptedSecret.fromJson(Map<String, dynamic> json) {
    final meta = SecretMetadata.fromJson(
        json['metadata'] as Map<String, dynamic>? ?? {});
    final p = json['payload'] as Map<String, dynamic>? ?? {};
    return DecryptedSecret(metadata: meta, payload: p);
  }

  String get payloadType => payload['type'] as String? ?? 'generic_password';
  Map<String, dynamic> get payloadData =>
      payload['data'] as Map<String, dynamic>? ?? {};
}

/// Real-time RFC 6238 TOTP token response
class TotpCodeResponse {
  final String secretId;
  final String code;
  final int secondsRemaining;
  final int periodSeconds;

  const TotpCodeResponse({
    required this.secretId,
    required this.code,
    required this.secondsRemaining,
    required this.periodSeconds,
  });

  factory TotpCodeResponse.fromJson(Map<String, dynamic> json) {
    return TotpCodeResponse(
      secretId: json['secret_id'] as String? ?? '',
      code: json['code'] as String? ?? '000000',
      secondsRemaining: (json['seconds_remaining'] as num?)?.toInt() ?? 0,
      periodSeconds: (json['period_seconds'] as num?)?.toInt() ?? 30,
    );
  }
}

/// 5-Dimensional Security Risk Health Report
class VaultHealthReport {
  final double overallScore;
  final String grade;
  final int totalSecrets;
  final Map<String, double> dimensionBreakdown;
  final List<SecurityAuditFinding> findings;

  const VaultHealthReport({
    required this.overallScore,
    required this.grade,
    required this.totalSecrets,
    required this.dimensionBreakdown,
    required this.findings,
  });

  factory VaultHealthReport.fromJson(Map<String, dynamic> json) {
    final rawBreakdown =
        json['dimension_breakdown'] as Map<String, dynamic>? ?? {};
    final breakdown = rawBreakdown.map((k, v) => MapEntry(k, (v as num).toDouble()));

    final rawFindings = json['findings'] as List<dynamic>? ?? [];
    final findingsList = rawFindings
        .map((f) => SecurityAuditFinding.fromJson(f as Map<String, dynamic>))
        .toList();

    return VaultHealthReport(
      overallScore: (json['overall_score'] as num?)?.toDouble() ?? 100.0,
      grade: json['grade'] as String? ?? 'A+',
      totalSecrets: (json['total_secrets'] as num?)?.toInt() ?? 0,
      dimensionBreakdown: breakdown,
      findings: findingsList,
    );
  }
}

/// Individual vulnerability finding identified during audit
class SecurityAuditFinding {
  final String findingType;
  final String severity; // low, medium, high, critical
  final String secretId;
  final String secretTitle;
  final String message;
  final String remediationAdvice;

  const SecurityAuditFinding({
    required this.findingType,
    required this.severity,
    required this.secretId,
    required this.secretTitle,
    required this.message,
    required this.remediationAdvice,
  });

  factory SecurityAuditFinding.fromJson(Map<String, dynamic> json) {
    return SecurityAuditFinding(
      findingType: json['finding_type'] as String? ?? 'General',
      severity: json['severity'] as String? ?? 'low',
      secretId: json['secret_id'] as String? ?? '',
      secretTitle: json['secret_title'] as String? ?? 'Unknown Secret',
      message: json['message'] as String? ?? '',
      remediationAdvice: json['remediation_advice'] as String? ?? '',
    );
  }
}

/// Authenticated backup restore / export summary
class BackupSummary {
  final int totalSecrets;
  final DateTime backupTimestamp;
  final String checksumSha256;

  const BackupSummary({
    required this.totalSecrets,
    required this.backupTimestamp,
    required this.checksumSha256,
  });

  factory BackupSummary.fromJson(Map<String, dynamic> json) {
    return BackupSummary(
      totalSecrets: (json['total_secrets'] as num?)?.toInt() ?? 0,
      backupTimestamp: json['backup_timestamp'] != null
          ? DateTime.tryParse(json['backup_timestamp'] as String) ??
              DateTime.now()
          : DateTime.now(),
      checksumSha256: json['checksum_sha256'] as String? ?? '',
    );
  }
}
