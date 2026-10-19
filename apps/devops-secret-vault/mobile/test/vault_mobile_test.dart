import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import '../lib/models/secret.dart';
import '../lib/services/vault_service.dart';
import '../lib/services/biometric_service.dart';
import '../lib/screens/unlock_screen.dart';
import '../lib/screens/vault_home_screen.dart';
import '../lib/screens/totp_screen.dart';
import '../lib/screens/health_screen.dart';
import '../lib/screens/backup_screen.dart';

void main() {
  group('Vault Mobile Models Unit Tests', () {
    test('SecretMetadata serialization and deserialization', () {
      final json = {
        'id': 'sec-12345',
        'title': 'Production AWS Root',
        'description': 'Root access keys for us-east-1',
        'secret_type': 'aws_credential',
        'tags': ['aws', 'production', 'root'],
        'created_at': '2026-10-19T10:00:00.000Z',
        'updated_at': '2026-10-19T10:00:00.000Z',
        'version': 1,
        'entropy_score': 94.5,
        'access_count': 3,
      };

      final meta = SecretMetadata.fromJson(json);
      expect(meta.id, 'sec-12345');
      expect(meta.title, 'Production AWS Root');
      expect(meta.secretType, 'aws_credential');
      expect(meta.tags.length, 3);
      expect(meta.entropyScore, 94.5);
      expect(meta.version, 1);
      expect(meta.accessCount, 3);

      final outJson = meta.toJson();
      expect(outJson['id'], 'sec-12345');
      expect(outJson['secret_type'], 'aws_credential');
    });

    test('DecryptedSecret payload parsing', () {
      final json = {
        'metadata': {
          'id': 'sec-db',
          'title': 'PostgreSQL Master',
          'secret_type': 'database_credential',
          'tags': ['postgres'],
          'created_at': '2026-10-19T10:00:00.000Z',
          'updated_at': '2026-10-19T10:00:00.000Z',
          'version': 1,
          'entropy_score': 88.0,
          'access_count': 0,
        },
        'payload': {
          'type': 'database_credential',
          'data': {
            'engine': 'postgres',
            'host': '127.0.0.1',
            'port': 5432,
            'username': 'pgadmin',
            'password': 'StrongPassword123!',
          },
        },
      };

      final secret = DecryptedSecret.fromJson(json);
      expect(secret.metadata.title, 'PostgreSQL Master');
      expect(secret.payloadType, 'database_credential');
      expect(secret.payloadData['engine'], 'postgres');
      expect(secret.payloadData['port'], 5432);
    });

    test('TotpCodeResponse deserialization', () {
      final json = {
        'secret_id': 'sec-totp-1',
        'code': '729104',
        'seconds_remaining': 24,
        'period_seconds': 30,
      };

      final totp = TotpCodeResponse.fromJson(json);
      expect(totp.secretId, 'sec-totp-1');
      expect(totp.code, '729104');
      expect(totp.secondsRemaining, 24);
      expect(totp.periodSeconds, 30);
    });

    test('VaultHealthReport deserialization', () {
      final json = {
        'overall_score': 91.2,
        'grade': 'A',
        'total_secrets': 12,
        'dimension_breakdown': {
          'entropy': 95.0,
          'uniqueness': 90.0,
          'rotation': 85.0,
        },
        'findings': [
          {
            'finding_type': 'WeakPassword',
            'severity': 'medium',
            'secret_id': 'sec-1',
            'secret_title': 'Legacy Staging Key',
            'message': 'Password entropy is below 70%',
            'remediation_advice': 'Regenerate using a 24-character random string',
          }
        ],
      };

      final report = VaultHealthReport.fromJson(json);
      expect(report.overallScore, 91.2);
      expect(report.grade, 'A');
      expect(report.totalSecrets, 12);
      expect(report.dimensionBreakdown['entropy'], 95.0);
      expect(report.findings.length, 1);
      expect(report.findings.first.severity, 'medium');
    });

    test('BackupSummary deserialization', () {
      final json = {
        'total_secrets': 8,
        'backup_timestamp': '2026-10-19T10:00:00.000Z',
        'checksum_sha256': 'a1b2c3d4e5f67890',
      };

      final summary = BackupSummary.fromJson(json);
      expect(summary.totalSecrets, 8);
      expect(summary.checksumSha256, 'a1b2c3d4e5f67890');
    });
  });

  group('VaultService Lifecycle Unit Tests (Mock Mode)', () {
    late VaultService vaultService;

    setUp(() async {
      vaultService = VaultService(mockMode: true);
      await vaultService.open(':memory:');
    });

    test('initialization, unlock, and lock lifecycle', () async {
      expect(await vaultService.isLocked(), isTrue);

      await vaultService.initialize('MasterPass123!');
      expect(await vaultService.isInitialized(), isTrue);
      expect(await vaultService.isLocked(), isFalse);

      await vaultService.lock();
      expect(await vaultService.isLocked(), isTrue);

      final unlocked = await vaultService.unlock('MasterPass123!');
      expect(unlocked, isTrue);
      expect(await vaultService.isLocked(), isFalse);

      final wrongUnlock = await vaultService.unlock('WrongPassword');
      expect(wrongUnlock, isFalse);
    });

    test('secret creation, search, and retrieval roundtrip', () async {
      await vaultService.initialize('MasterPass123!');

      final created = await vaultService.createSecret(
        title: 'AWS Production Deployer',
        secretType: 'aws_credential',
        data: {
          'access_key_id': 'AKIAIOSFODNN7EXAMPLE',
          'secret_access_key': 'wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY',
          'region': 'us-east-1',
        },
        tags: ['aws', 'deploy'],
      );

      expect(created.title, 'AWS Production Deployer');
      expect(created.secretType, 'aws_credential');
      expect(created.tags, contains('aws'));

      final results = await vaultService.searchSecrets('deployer');
      expect(results.length, 1);
      expect(results.first.id, created.id);

      final retrieved = await vaultService.getSecret(created.id);
      expect(retrieved.metadata.title, 'AWS Production Deployer');
      expect(retrieved.payloadData['access_key_id'], 'AKIAIOSFODNN7EXAMPLE');
    });

    test('totp generation and health report query', () async {
      await vaultService.initialize('MasterPass123!');

      final totpSecret = await vaultService.createSecret(
        title: 'GitLab 2FA',
        secretType: 'totp',
        data: {
          'secret_base32': 'JBSWY3DPEHPK3PXP',
          'account_name': 'alice',
        },
      );

      final codeResp = await vaultService.generateTotp(totpSecret.id);
      expect(codeResp.code.length, 6);
      expect(codeResp.periodSeconds, 30);
      expect(codeResp.secondsRemaining, inInclusiveRange(1, 30));

      final health = await vaultService.getHealthReport();
      expect(health.overallScore, greaterThan(80.0));
      expect(health.dimensionBreakdown.containsKey('entropy'), isTrue);
    });

    test('backup export and import flow', () async {
      await vaultService.initialize('MasterPass123!');
      await vaultService.createSecret(
        title: 'Test Key',
        secretType: 'generic_password',
        data: {'password': 'Secret'},
      );

      final export = await vaultService.exportBackup(
          'BackupPass!', '/path/to/backup.vaultbackup');
      expect(export.totalSecrets, greaterThanOrEqualTo(1));
      expect(export.checksumSha256, isNotEmpty);

      final import = await vaultService.importBackup(
          'BackupPass!', '/path/to/backup.vaultbackup');
      expect(import.totalSecrets, greaterThan(0));
    });
  });

  group('BiometricService Unit Tests', () {
    test('mock biometric authentication success', () async {
      final bioService = BiometricService(mockMode: true);
      expect(await bioService.isBiometricsSupported(), isTrue);

      final types = await bioService.getAvailableBiometrics();
      expect(types, isNotEmpty);

      final authenticated = await bioService.authenticate();
      expect(authenticated, isTrue);
    });
  });

  group('Vault Mobile Widget Tests', () {
    testWidgets('UnlockScreen renders master password input',
        (WidgetTester tester) async {
      final vaultService = VaultService(mockMode: true);
      final bioService = BiometricService(mockMode: true);

      await tester.pumpWidget(MaterialApp(
        home: UnlockScreen(
          vaultService: vaultService,
          biometricService: bioService,
        ),
      ));

      await tester.pumpAndSettle();

      expect(find.text('DevOps Secret Vault'), findsOneWidget);
      expect(find.byType(TextField), findsWidgets);
      expect(find.byType(ElevatedButton), findsOneWidget);
    });

    testWidgets('VaultHomeScreen renders omnibar and navigation',
        (WidgetTester tester) async {
      final vaultService = VaultService(mockMode: true);
      final bioService = BiometricService(mockMode: true);
      await vaultService.open(':memory:');
      await vaultService.initialize('TestPass!');

      await tester.pumpWidget(MaterialApp(
        home: VaultHomeScreen(
          vaultService: vaultService,
          biometricService: bioService,
        ),
      ));

      await tester.pumpAndSettle();

      expect(find.text('DevOps Vault'), findsOneWidget);
      expect(find.text('AIR-GAPPED'), findsOneWidget);
      expect(find.byType(BottomNavigationBar), findsOneWidget);
      expect(find.text('Secrets'), findsOneWidget);
      expect(find.text('2FA TOTP'), findsOneWidget);
      expect(find.text('Health'), findsOneWidget);
      expect(find.text('Backup'), findsOneWidget);
    });

    testWidgets('HealthScreen renders score meter and dimensions',
        (WidgetTester tester) async {
      final vaultService = VaultService(mockMode: true);
      await vaultService.open(':memory:');
      await vaultService.initialize('TestPass!');

      await tester.pumpWidget(MaterialApp(
        home: HealthScreen(vaultService: vaultService),
      ));

      await tester.pumpAndSettle();

      expect(find.text('Security Posture'), findsOneWidget);
      expect(find.text('5-DIMENSIONAL RISK METRICS'), findsOneWidget);
      expect(find.text('AUDIT FINDINGS & REMEDIATION'), findsOneWidget);
    });

    testWidgets('BackupScreen renders export and import actions',
        (WidgetTester tester) async {
      final vaultService = VaultService(mockMode: true);
      await vaultService.open(':memory:');
      await vaultService.initialize('TestPass!');

      await tester.pumpWidget(MaterialApp(
        home: BackupScreen(vaultService: vaultService),
      ));

      await tester.pumpAndSettle();

      expect(find.text('Export Encrypted Backup'), findsOneWidget);
      expect(find.text('Restore from Backup'), findsOneWidget);
      expect(find.text('Generate Encrypted Backup'), findsOneWidget);
      expect(find.text('Restore Secrets to Vault'), findsOneWidget);
    });
  });
}
