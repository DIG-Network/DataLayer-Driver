#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

// Files to check
const files = [
  { path: 'Cargo.toml', type: 'toml', name: 'Main Cargo.toml' },
  { path: 'napi/package.json', type: 'json', name: 'Main package.json' },
  { path: 'napi/npm/darwin-arm64/package.json', type: 'json', name: 'Darwin ARM64' },
  { path: 'napi/npm/darwin-x64/package.json', type: 'json', name: 'Darwin x64' },
  { path: 'napi/npm/linux-arm64-gnu/package.json', type: 'json', name: 'Linux ARM64' },
  { path: 'napi/npm/linux-x64-gnu/package.json', type: 'json', name: 'Linux x64' },
  { path: 'napi/npm/win32-x64-msvc/package.json', type: 'json', name: 'Windows x64' },
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

// Function to get version from a file
function getVersion(filePath, fileType) {
  const fullPath = path.join(projectRoot, filePath);
  
  if (!fs.existsSync(fullPath)) {
    return null;
  }
  
  if (fileType === 'json') {
    const content = JSON.parse(fs.readFileSync(fullPath, 'utf8'));
    return content.version;
  } else if (fileType === 'toml') {
    const content = fs.readFileSync(fullPath, 'utf8');
    const match = content.match(/^version = "(.*)"/m);
    return match ? match[1] : null;
  }
  
  return null;
}

// Main execution
console.log('Checking versions across all files...\n');

const versions = {};
let allSynced = true;
let mainVersion = null;

for (const file of files) {
  const version = getVersion(file.path, file.type);
  
  if (version === null) {
    console.log(`❌ ${file.name.padEnd(20)} - File not found`);
    allSynced = false;
    continue;
  }
  
  if (mainVersion === null) {
    mainVersion = version;
  }
  
  const status = version === mainVersion ? '✅' : '❌';
  console.log(`${status} ${file.name.padEnd(20)} - ${version}`);
  
  if (version !== mainVersion) {
    allSynced = false;
  }
  
  versions[version] = (versions[version] || 0) + 1;
}

console.log('\n' + '='.repeat(50));

if (allSynced) {
  console.log('✅ All versions are synchronized:', mainVersion);
} else {
  console.log('❌ Version mismatch detected!');
  console.log('\nVersion distribution:');
  for (const [version, count] of Object.entries(versions)) {
    console.log(`  ${version}: ${count} file(s)`);
  }
  console.log('\nRun "npm run bump:version <version>" to synchronize all versions');
  process.exit(1);
}
