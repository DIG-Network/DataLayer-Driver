# Version Management Scripts

This directory contains scripts to manage version synchronization across all package files in the DataLayer-Driver project.

## Files that are synchronized

- `Cargo.toml` - Main Rust crate version
- `napi/package.json` - Main npm package version
- `napi/npm/darwin-arm64/package.json` - macOS ARM64 package
- `napi/npm/darwin-x64/package.json` - macOS x64 package
- `napi/npm/linux-arm64-gnu/package.json` - Linux ARM64 package
- `napi/npm/linux-x64-gnu/package.json` - Linux x64 package
- `napi/npm/win32-x64-msvc/package.json` - Windows x64 package

## Available Commands

Run these commands from the `napi` directory:

### Check Version Synchronization

Check if all files have the same version:

```bash
npm run check:versions
```

### Bump Version

Bump the patch version (e.g., 0.1.37 → 0.1.38):

```bash
npm run bump:patch
```

Bump the minor version (e.g., 0.1.37 → 0.2.0):

```bash
npm run bump:minor
```

Bump the major version (e.g., 0.1.37 → 1.0.0):

```bash
npm run bump:major
```

Set a specific version:

```bash
npm run bump:version 0.2.0
```

## Release Process

1. **Bump the version:**
   ```bash
   npm run bump:patch  # or minor/major
   ```

2. **Review the changes:**
   ```bash
   git diff
   ```

3. **Commit the version bump:**
   ```bash
   git add -A
   git commit -m "v0.1.38"  # Replace with your version
   ```

4. **Create a version tag:**
   ```bash
   git tag v0.1.38  # Replace with your version
   ```

5. **Push changes and tag:**
   ```bash
   git push
   git push --tags
   ```

The CI/CD pipeline will automatically:
- Build all platform binaries
- Publish to npm (when a version tag is pushed)
- Publish to crates.io (when a version tag is pushed)

## Notes

- Always ensure versions are synchronized before creating a release
- The scripts can be run from any directory within the project
- Version format must be `X.Y.Z` (semantic versioning)
