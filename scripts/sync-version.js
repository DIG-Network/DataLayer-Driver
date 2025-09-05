#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

// Parse command line arguments
const args = process.argv.slice(2);
if (args.length === 0) {
  console.error('Usage: node sync-version.js <version> or node sync-version.js patch|minor|major');
  process.exit(1);
}

// Files to update
const files = [
  { path: 'Cargo.toml', type: 'toml' },
  { path: 'napi/package.json', type: 'json' },
  { path: 'napi/npm/darwin-arm64/package.json', type: 'json' },
  { path: 'napi/npm/darwin-x64/package.json', type: 'json' },
  { path: 'napi/npm/linux-arm64-gnu/package.json', type: 'json' },
  { path: 'napi/npm/linux-x64-gnu/package.json', type: 'json' },
  { path: 'napi/npm/win32-x64-msvc/package.json', type: 'json' },
];

// Find project root by looking for Cargo.toml in parent directories
function findProjectRoot() {
  let dir = __dirname;
  while (dir !== path.dirname(dir)) {
    if (fs.existsSync(path.join(dir, 'Cargo.toml')) && 
        fs.existsSync(path.join(dir, 'napi'))) {
      return dir;
    }
    dir = path.dirname(dir);
  }
  throw new Error('Could not find project root');
}

const projectRoot = findProjectRoot();

// Function to read current version from package.json
function getCurrentVersion() {
  const packageJson = JSON.parse(fs.readFileSync(path.join(projectRoot, 'napi/package.json'), 'utf8'));
  return packageJson.version;
}

// Function to bump version based on type
function bumpVersion(currentVersion, bumpType) {
  const parts = currentVersion.split('.').map(Number);
  
  switch (bumpType) {
    case 'patch':
      parts[2]++;
      break;
    case 'minor':
      parts[1]++;
      parts[2] = 0;
      break;
    case 'major':
      parts[0]++;
      parts[1] = 0;
      parts[2] = 0;
      break;
    default:
      // If it's not a bump type, assume it's a version string
      return bumpType;
  }
  
  return parts.join('.');
}

// Function to update version in a file
function updateVersion(filePath, fileType, newVersion) {
  const fullPath = path.join(projectRoot, filePath);
  
  if (!fs.existsSync(fullPath)) {
    console.warn(`Warning: ${filePath} does not exist, skipping...`);
    return;
  }
  
  if (fileType === 'json') {
    // Update JSON files
    const content = JSON.parse(fs.readFileSync(fullPath, 'utf8'));
    content.version = newVersion;
    fs.writeFileSync(fullPath, JSON.stringify(content, null, 2) + '\n');
  } else if (fileType === 'toml') {
    // Update TOML file
    let content = fs.readFileSync(fullPath, 'utf8');
    content = content.replace(
      /^version = ".*"$/m,
      `version = "${newVersion}"`
    );
    fs.writeFileSync(fullPath, content);
  }
}

// Main execution
try {
  const currentVersion = getCurrentVersion();
  console.log(`Current version: ${currentVersion}`);
  
  const newVersion = bumpVersion(currentVersion, args[0]);
  console.log(`New version: ${newVersion}`);
  
  // Validate version format
  if (!/^\d+\.\d+\.\d+$/.test(newVersion)) {
    console.error('Invalid version format. Expected format: X.Y.Z');
    process.exit(1);
  }
  
  // Update all files
  console.log('\nUpdating versions...');
  for (const file of files) {
    console.log(`  ✓ ${file.path}`);
    updateVersion(file.path, file.type, newVersion);
  }
  
  console.log('\n✅ All versions synchronized to', newVersion);
  console.log('\nNext steps:');
  console.log('1. Review the changes: git diff');
  console.log('2. Commit the changes: git add -A && git commit -m "v' + newVersion + '"');
  console.log('3. Create a tag: git tag v' + newVersion);
  console.log('4. Push changes and tag: git push && git push --tags');
  
} catch (error) {
  console.error('Error:', error.message);
  process.exit(1);
}
