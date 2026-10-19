import 'package:flutter/material.dart';
import '../models/secret.dart';
import '../services/vault_service.dart';
import '../services/biometric_service.dart';
import 'secret_detail_screen.dart';
import 'totp_screen.dart';
import 'health_screen.dart';
import 'backup_screen.dart';
import 'unlock_screen.dart';

class VaultHomeScreen extends StatefulWidget {
  final VaultService vaultService;
  final BiometricService biometricService;

  const VaultHomeScreen({
    super.key,
    required this.vaultService,
    required this.biometricService,
  });

  @override
  State<VaultHomeScreen> createState() => _VaultHomeScreenState();
}

class _VaultHomeScreenState extends State<VaultHomeScreen> {
  final TextEditingController _searchController = TextEditingController();
  List<SecretMetadata> _secrets = [];
  bool _isLoading = true;
  String _selectedCategory = 'all';
  int _currentNavIndex = 0;

  final List<String> _categories = [
    'all',
    'aws_credential',
    'azure_credential',
    'database_credential',
    'totp',
    'tls_certificate',
    'ssh_key',
  ];

  @override
  void initState() {
    super.initState();
    _loadSecrets();
  }

  Future<void> _loadSecrets([String query = '*']) async {
    setState(() => _isLoading = true);
    try {
      final results = await widget.vaultService.searchSecrets(query);
      setState(() {
        _secrets = results;
        _isLoading = false;
      });
    } catch (e) {
      setState(() => _isLoading = false);
    }
  }

  List<SecretMetadata> get _filteredSecrets {
    if (_selectedCategory == 'all') return _secrets;
    return _secrets.where((s) => s.secretType == _selectedCategory).toList();
  }

  Future<void> _handleLock() async {
    await widget.vaultService.lock();
    if (mounted) {
      Navigator.of(context).pushReplacement(
        MaterialPageRoute(
          builder: (_) => UnlockScreen(
            vaultService: widget.vaultService,
            biometricService: widget.biometricService,
          ),
        ),
      );
    }
  }

  IconData _getIconForType(String type) {
    switch (type) {
      case 'aws_credential':
      case 'azure_credential':
      case 'gcp_service_account':
        return Icons.cloud_queue;
      case 'database_credential':
        return Icons.dns_outlined;
      case 'totp':
        return Icons.pin_outlined;
      case 'tls_certificate':
        return Icons.verified_user_outlined;
      case 'ssh_key':
        return Icons.terminal_outlined;
      default:
        return Icons.vpn_key_outlined;
    }
  }

  Color _getEntropyColor(double entropy) {
    if (entropy >= 80) return const Color(0xFF00E676);
    if (entropy >= 50) return const Color(0xFFFFB300);
    return const Color(0xFFFF5252);
  }

  void _showAddSecretDialog() {
    final titleController = TextEditingController();
    final keyController = TextEditingController();
    final valueController = TextEditingController();
    String selectedType = 'generic_password';

    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: const Color(0xFF131D31),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (ctx) {
        return StatefulBuilder(
          builder: (context, setModalState) {
            return Padding(
              padding: EdgeInsets.only(
                left: 20,
                right: 20,
                top: 20,
                bottom: MediaQuery.of(context).viewInsets.bottom + 20,
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      const Text(
                        'Store New Secret',
                        style: TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.bold,
                          color: Colors.white,
                        ),
                      ),
                      IconButton(
                        icon: const Icon(Icons.close, color: Colors.white60),
                        onPressed: () => Navigator.pop(ctx),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  DropdownButtonFormField<String>(
                    value: selectedType,
                    dropdownColor: const Color(0xFF1E293B),
                    style: const TextStyle(color: Colors.white),
                    decoration: InputDecoration(
                      labelText: 'Secret Category',
                      labelStyle: const TextStyle(color: Colors.white70),
                      filled: true,
                      fillColor: const Color(0xFF0F172A),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(10),
                      ),
                    ),
                    items: const [
                      DropdownMenuItem(
                          value: 'generic_password',
                          child: Text('Generic Password')),
                      DropdownMenuItem(
                          value: 'aws_credential', child: Text('AWS Credential')),
                      DropdownMenuItem(
                          value: 'database_credential',
                          child: Text('Database Credential')),
                      DropdownMenuItem(
                          value: 'totp', child: Text('2FA / TOTP Authenticator')),
                    ],
                    onChanged: (val) {
                      if (val != null) setModalState(() => selectedType = val);
                    },
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: titleController,
                    style: const TextStyle(color: Colors.white),
                    decoration: InputDecoration(
                      labelText: 'Title / Identifier',
                      labelStyle: const TextStyle(color: Colors.white70),
                      filled: true,
                      fillColor: const Color(0xFF0F172A),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(10),
                      ),
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: keyController,
                    style: const TextStyle(color: Colors.white),
                    decoration: InputDecoration(
                      labelText: selectedType == 'totp'
                          ? 'Account / User'
                          : 'Username / Key ID',
                      labelStyle: const TextStyle(color: Colors.white70),
                      filled: true,
                      fillColor: const Color(0xFF0F172A),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(10),
                      ),
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: valueController,
                    obscureText: true,
                    style: const TextStyle(color: Colors.white),
                    decoration: InputDecoration(
                      labelText: selectedType == 'totp'
                          ? 'Base32 Secret'
                          : 'Secret Value / Password',
                      labelStyle: const TextStyle(color: Colors.white70),
                      filled: true,
                      fillColor: const Color(0xFF0F172A),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(10),
                      ),
                    ),
                  ),
                  const SizedBox(height: 20),
                  SizedBox(
                    width: double.infinity,
                    height: 48,
                    child: ElevatedButton(
                      onPressed: () async {
                        final title = titleController.text.trim();
                        final k = keyController.text.trim();
                        final v = valueController.text.trim();
                        if (title.isEmpty || v.isEmpty) return;

                        Map<String, dynamic> data = {};
                        if (selectedType == 'totp') {
                          data = {
                            'secret_base32': v,
                            'account_name': k.isNotEmpty ? k : 'account',
                          };
                        } else if (selectedType == 'aws_credential') {
                          data = {
                            'access_key_id': k,
                            'secret_access_key': v,
                            'region': 'us-east-1',
                          };
                        } else {
                          data = {
                            'username': k,
                            'password': v,
                          };
                        }

                        Navigator.pop(ctx);
                        await widget.vaultService.createSecret(
                          title: title,
                          secretType: selectedType,
                          data: data,
                          tags: ['mobile', selectedType],
                        );
                        _loadSecrets();
                      },
                      style: ElevatedButton.styleFrom(
                        backgroundColor: const Color(0xFF00E5FF),
                        foregroundColor: const Color(0xFF0A0E1A),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(10),
                        ),
                      ),
                      child: const Text('Save Secret to Vault',
                          style: TextStyle(fontWeight: FontWeight.bold)),
                    ),
                  ),
                ],
              ),
            );
          },
        );
      },
    );
  }

  Widget _buildBody() {
    switch (_currentNavIndex) {
      case 1:
        return TotpScreen(vaultService: widget.vaultService);
      case 2:
        return HealthScreen(vaultService: widget.vaultService);
      case 3:
        return BackupScreen(vaultService: widget.vaultService);
      default:
        return _buildSecretsList();
    }
  }

  Widget _buildSecretsList() {
    return Column(
      children: [
        // Omnibar Search
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          child: Container(
            decoration: BoxDecoration(
              color: const Color(0xFF131D31),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: Colors.white.withOpacity(0.08)),
            ),
            child: TextField(
              controller: _searchController,
              style: const TextStyle(color: Colors.white),
              onChanged: (val) => _loadSecrets(val.isEmpty ? '*' : val),
              decoration: InputDecoration(
                hintText: 'Search credentials, tags, services...',
                hintStyle: TextStyle(color: Colors.white.withOpacity(0.4)),
                prefixIcon:
                    const Icon(Icons.search, color: Color(0xFF00E5FF), size: 20),
                suffixIcon: _searchController.text.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.clear, color: Colors.white54, size: 18),
                        onPressed: () {
                          _searchController.clear();
                          _loadSecrets('*');
                        },
                      )
                    : null,
                border: InputBorder.none,
                contentPadding: const EdgeInsets.symmetric(vertical: 14),
              ),
            ),
          ),
        ),

        // Filter chips horizontal scroll
        SizedBox(
          height: 38,
          child: ListView.separated(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            scrollDirection: Axis.horizontal,
            itemCount: _categories.length,
            separatorBuilder: (_, __) => const SizedBox(width: 8),
            itemBuilder: (context, index) {
              final cat = _categories[index];
              final isSelected = _selectedCategory == cat;
              return ChoiceChip(
                label: Text(
                  cat.replaceAll('_', ' ').toUpperCase(),
                  style: TextStyle(
                    fontSize: 11,
                    fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
                    color: isSelected ? const Color(0xFF0A0E1A) : Colors.white70,
                  ),
                ),
                selected: isSelected,
                selectedColor: const Color(0xFF00E5FF),
                backgroundColor: const Color(0xFF131D31),
                onSelected: (selected) {
                  setState(() => _selectedCategory = cat);
                },
              );
            },
          ),
        ),
        const SizedBox(height: 8),

        // List View
        Expanded(
          child: _isLoading
              ? const Center(
                  child: CircularProgressIndicator(color: Color(0xFF00E5FF)))
              : _filteredSecrets.isEmpty
                  ? Center(
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Icon(Icons.inventory_2_outlined,
                              size: 48, color: Colors.white.withOpacity(0.3)),
                          const SizedBox(height: 12),
                          Text(
                            'No secrets found',
                            style: TextStyle(
                              color: Colors.white.withOpacity(0.5),
                              fontSize: 15,
                            ),
                          ),
                        ],
                      ),
                    )
                  : ListView.separated(
                      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                      itemCount: _filteredSecrets.length,
                      separatorBuilder: (_, __) => const SizedBox(height: 10),
                      itemBuilder: (context, index) {
                        final secret = _filteredSecrets[index];
                        final entropyColor =
                            _getEntropyColor(secret.entropyScore);

                        return Material(
                          color: const Color(0xFF131D31),
                          borderRadius: BorderRadius.circular(12),
                          clipBehavior: Clip.antiAlias,
                          child: InkWell(
                            onTap: () {
                              Navigator.of(context).push(
                                MaterialPageRoute(
                                  builder: (_) => SecretDetailScreen(
                                    vaultService: widget.vaultService,
                                    secretId: secret.id,
                                  ),
                                ),
                              );
                            },
                            child: Container(
                              padding: const EdgeInsets.all(14),
                              decoration: BoxDecoration(
                                border: Border.all(
                                  color: Colors.white.withOpacity(0.06),
                                ),
                                borderRadius: BorderRadius.circular(12),
                              ),
                              child: Row(
                                children: [
                                  Container(
                                    padding: const EdgeInsets.all(10),
                                    decoration: BoxDecoration(
                                      color: const Color(0xFF00E5FF).withOpacity(0.1),
                                      borderRadius: BorderRadius.circular(10),
                                    ),
                                    child: Icon(
                                      _getIconForType(secret.secretType),
                                      color: const Color(0xFF00E5FF),
                                      size: 22,
                                    ),
                                  ),
                                  const SizedBox(width: 14),
                                  Expanded(
                                    child: Column(
                                      crossAxisAlignment:
                                          CrossAxisAlignment.start,
                                      children: [
                                        Text(
                                          secret.title,
                                          style: const TextStyle(
                                            color: Colors.white,
                                            fontWeight: FontWeight.w600,
                                            fontSize: 15,
                                          ),
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                        ),
                                        const SizedBox(height: 4),
                                        Row(
                                          children: [
                                            Text(
                                              secret.secretType
                                                  .replaceAll('_', ' '),
                                              style: TextStyle(
                                                color: Colors.white
                                                    .withOpacity(0.5),
                                                fontSize: 12,
                                              ),
                                            ),
                                            const SizedBox(width: 8),
                                            if (secret.tags.isNotEmpty)
                                              Container(
                                                padding:
                                                    const EdgeInsets.symmetric(
                                                  horizontal: 6,
                                                  vertical: 1,
                                                ),
                                                decoration: BoxDecoration(
                                                  color: Colors.white
                                                      .withOpacity(0.08),
                                                  borderRadius:
                                                      BorderRadius.circular(4),
                                                ),
                                                child: Text(
                                                  secret.tags.first,
                                                  style: const TextStyle(
                                                    color: Colors.white70,
                                                    fontSize: 10,
                                                  ),
                                                ),
                                              ),
                                          ],
                                        ),
                                      ],
                                    ),
                                  ),
                                  // Entropy indicator
                                  Column(
                                    crossAxisAlignment: CrossAxisAlignment.end,
                                    children: [
                                      Text(
                                        '${secret.entropyScore.toInt()}%',
                                        style: TextStyle(
                                          color: entropyColor,
                                          fontWeight: FontWeight.bold,
                                          fontSize: 12,
                                        ),
                                      ),
                                      Text(
                                        'entropy',
                                        style: TextStyle(
                                          color: Colors.white.withOpacity(0.4),
                                          fontSize: 10,
                                        ),
                                      ),
                                    ],
                                  ),
                                ],
                              ),
                            ),
                          ),
                        );
                      },
                    ),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0A0E1A),
      appBar: AppBar(
        backgroundColor: const Color(0xFF0F172A),
        elevation: 0,
        title: Row(
          children: [
            const Icon(Icons.shield_outlined,
                color: Color(0xFF00E5FF), size: 20),
            const SizedBox(width: 8),
            const Text(
              'DevOps Vault',
              style: TextStyle(
                color: Colors.white,
                fontWeight: FontWeight.bold,
                fontSize: 18,
              ),
            ),
            const SizedBox(width: 8),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
              decoration: BoxDecoration(
                color: const Color(0xFF00E676).withOpacity(0.15),
                borderRadius: BorderRadius.circular(4),
              ),
              child: const Text(
                'AIR-GAPPED',
                style: TextStyle(
                  color: Color(0xFF00E676),
                  fontSize: 10,
                  fontWeight: FontWeight.bold,
                ),
              ),
            ),
          ],
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.lock_outline, color: Colors.white70),
            tooltip: 'Lock Vault',
            onPressed: _handleLock,
          ),
        ],
      ),
      body: _buildBody(),
      floatingActionButton: _currentNavIndex == 0
          ? FloatingActionButton(
              onPressed: _showAddSecretDialog,
              backgroundColor: const Color(0xFF00E5FF),
              child: const Icon(Icons.add, color: Color(0xFF0A0E1A)),
            )
          : null,
      bottomNavigationBar: BottomNavigationBar(
        currentIndex: _currentNavIndex,
        onTap: (index) => setState(() => _currentNavIndex = index),
        backgroundColor: const Color(0xFF0F172A),
        selectedItemColor: const Color(0xFF00E5FF),
        unselectedItemColor: Colors.white54,
        type: BottomNavigationBarType.fixed,
        items: const [
          BottomNavigationBarItem(
            icon: Icon(Icons.vpn_key_outlined),
            activeIcon: Icon(Icons.vpn_key),
            label: 'Secrets',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.pin_outlined),
            activeIcon: Icon(Icons.pin),
            label: '2FA TOTP',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.health_and_safety_outlined),
            activeIcon: Icon(Icons.health_and_safety),
            label: 'Health',
          ),
          BottomNavigationBarItem(
            icon: Icon(Icons.cloud_download_outlined),
            activeIcon: Icon(Icons.cloud_download),
            label: 'Backup',
          ),
        ],
      ),
    );
  }
}
