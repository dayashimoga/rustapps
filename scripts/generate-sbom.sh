#!/usr/bin/env bash
# SBOM Generation Script (ADR-0030)
# Generates compliant CycloneDX 1.5 JSON Software Bill of Materials (SBOM)
set -euo pipefail

APP_NAME="${1:-}"
OUTPUT_DIR="${2:-target/sbom}"

echo "=========================================================="
echo "  Generating CycloneDX Software Bill of Materials (SBOM)  "
echo "=========================================================="

mkdir -p "${OUTPUT_DIR}"
if [ -n "${APP_NAME}" ]; then
    OUTPUT_FILE="${OUTPUT_DIR}/${APP_NAME}-sbom.json"
else
    OUTPUT_FILE="${OUTPUT_DIR}/sbom-cyclonedx.json"
fi

if command -v cargo-cyclonedx &>/dev/null || cargo cyclonedx --version &>/dev/null; then
    if [ -n "${APP_NAME}" ]; then
        cargo cyclonedx --package "${APP_NAME}" --format json --output-pattern "${OUTPUT_FILE}"
    else
        cargo cyclonedx --format json --output-pattern "${OUTPUT_FILE}"
    fi
    echo "SBOM generated via cargo-cyclonedx at ${OUTPUT_FILE}"
else
    echo "cargo-cyclonedx not available; generating compliant offline CycloneDX 1.5 SBOM..."
    if command -v python3 &>/dev/null; then
        python3 -c "
import json, uuid, datetime

components = []
current_name = ''
with open('Cargo.lock', 'r', encoding='utf-8') as f:
    for line in f:
        line = line.strip()
        if line.startswith('name = '):
            current_name = line.split('\"')[1]
        elif line.startswith('version = ') and current_name:
            version = line.split('\"')[1]
            components.append({
                'type': 'library',
                'name': current_name,
                'version': version,
                'purl': f'pkg:cargo/{current_name}@{version}',
                'bom-ref': f'pkg:cargo/{current_name}@{version}'
            })
            current_name = ''

target_name = '${APP_NAME}' if '${APP_NAME}' else 'rustapps-workspace'
now = datetime.datetime.now(datetime.timezone.utc).isoformat()
bom = {
    'bomFormat': 'CycloneDX',
    'specVersion': '1.5',
    'serialNumber': f'urn:uuid:{uuid.uuid4()}',
    'version': 1,
    'metadata': {
        'timestamp': now,
        'tools': [{'vendor': 'Offline Engineering', 'name': 'cargo-sbom-generator', 'version': '1.5.0'}],
        'component': {'type': 'application', 'name': target_name, 'version': '0.1.0'}
    },
    'components': components
}
with open('${OUTPUT_FILE}', 'w', encoding='utf-8') as f:
    json.dump(bom, f, indent=2)
print(f'Offline CycloneDX 1.5 SBOM generated with {len(components)} components at ${OUTPUT_FILE}')
"
    elif command -v node &>/dev/null; then
        node -e "
const fs = require('fs');
const lock = fs.readFileSync('Cargo.lock', 'utf8').split('\n');
let currentName = '';
const components = [];
for (const line of lock) {
  const trimmed = line.trim();
  if (trimmed.startsWith('name = ')) {
    currentName = trimmed.split('\"')[1];
  } else if (trimmed.startsWith('version = ') && currentName) {
    const version = trimmed.split('\"')[1];
    components.push({
      type: 'library',
      name: currentName,
      version: version,
      purl: \`pkg:cargo/\${currentName}@\${version}\`,
      'bom-ref': \`pkg:cargo/\${currentName}@\${version}\`
    });
    currentName = '';
  }
}
const targetName = '${APP_NAME}' || 'rustapps-workspace';
const bom = {
  bomFormat: 'CycloneDX',
  specVersion: '1.5',
  serialNumber: \`urn:uuid:\${require('crypto').randomUUID()}\`,
  version: 1,
  metadata: {
    timestamp: new Date().toISOString(),
    tools: [{ vendor: 'Offline Engineering', name: 'cargo-sbom-generator', version: '1.5.0' }],
    component: { type: 'application', name: targetName, version: '0.1.0' }
  },
  components
};
fs.writeFileSync('${OUTPUT_FILE}', JSON.stringify(bom, null, 2), 'utf8');
console.log(\`Offline CycloneDX 1.5 SBOM generated with \${components.length} components at ${OUTPUT_FILE}\`);
"
    fi
fi
