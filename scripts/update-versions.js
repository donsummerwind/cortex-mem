#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const glob = require('glob');

// Get project root directory (parent of scripts directory)
const PROJECT_ROOT = path.resolve(__dirname, '..');

// Read version from workspace Cargo.toml
function getWorkspaceVersion() {
	const workspaceCargoPath = path.join(PROJECT_ROOT, 'Cargo.toml');
	const content = fs.readFileSync(workspaceCargoPath, 'utf8');
	// Look for version in [workspace.package] section
	const match = content.match(/^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
	if (match) return match[1];
	// Fallback: look for workspace.version
	const fallbackMatch = content.match(/^version\s*=\s*"([^"]+)"/m);
	return fallbackMatch ? fallbackMatch[1] : null;
}

const VERSION = getWorkspaceVersion();
if (!VERSION) {
	console.error('Error: Could not find version in workspace Cargo.toml');
	process.exit(1);
}

const CARGO_TOML_PATTERN = '**/Cargo.toml';
const EXCLUDE_PATTERNS = ['**/target/**', '**/node_modules/**', '**/.git/**', '**/cortex-mem-insights/**'];

// ANSI color codes for terminal output
const colors = {
  reset: '\x1b[0m',
  bright: '\x1b[1m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m',
};

// Helper function to colorize output
function colorize(text, color) {
  return `${colors[color]}${text}${colors.reset}`;
}

// Function to find all Cargo.toml files
function findCargoTomlFiles() {
  console.log(colorize('Scanning for Cargo.toml files...', 'cyan'));

  try {
    const files = glob.sync(CARGO_TOML_PATTERN, {
      ignore: EXCLUDE_PATTERNS,
      cwd: PROJECT_ROOT,
      absolute: true
    });

    // Exclude the root Cargo.toml (workspace file)
    const filteredFiles = files.filter(f => f !== path.join(PROJECT_ROOT, 'Cargo.toml'));

    console.log(colorize(`Found ${filteredFiles.length} Cargo.toml files`, 'green'));
    return filteredFiles;
  } catch (error) {
    console.error(colorize('Error finding Cargo.toml files:', 'red'), error);
    process.exit(1);
  }
}

// Function to update version in a Cargo.toml file
function updateVersionInCargoToml(filePath) {
  try {
    const content = fs.readFileSync(filePath, 'utf8');
    let updated = false;

    // Update version.workspace = true (no change needed, it follows workspace)
    // Update standalone version = "x.x.x" in [package] section
    const newContent = content.replace(
      /^(\[package\][\s\S]*?)(^version\s*=\s*)"[^"]+"/m,
      (match, packageSection, versionPrefix) => {
        // Check if it's using version.workspace = true
        if (packageSection.includes('version.workspace = true')) {
          return match; // Already using workspace version, no change
        }
        updated = true;
        return packageSection + versionPrefix + '"' + VERSION + '"';
      }
    );

    if (updated) {
      fs.writeFileSync(filePath, newContent, 'utf8');
      console.log(colorize(`  Updated version in ${path.relative(PROJECT_ROOT, filePath)}`, 'green'));
      return true;
    } else {
      // Check if using workspace version
      if (content.includes('version.workspace = true')) {
        console.log(colorize(`  Using workspace version in ${path.relative(PROJECT_ROOT, filePath)}`, 'blue'));
      }
      return false;
    }
  } catch (error) {
    console.error(colorize(`Error processing ${filePath}:`, 'red'), error);
    return false;
  }
}

// Function to update internal dependencies
function updateInternalDependencies(filePath) {
  try {
    const content = fs.readFileSync(filePath, 'utf8');
    let updated = false;

    // List of internal crate names
    const internalCrates = [
      'cortex-mem-core',
      'cortex-mem-config',
      'cortex-mem-tools',
      'cortex-mem-rig',
      'cortex-mem-service',
      'cortex-mem-cli',
      'cortex-mem-mcp'
    ];

    let newContent = content;

    for (const crateName of internalCrates) {
      // Match various dependency declaration patterns:
      // 1. crate-name = { path = "...", version = "x.x.x" }
      // 2. crate-name = { version = "x.x.x", path = "..." }
      // 3. crate-name = { path = "..." } (no version, skip)
      const versionRegex = new RegExp(
        `(${crateName}\\s*=\\s*\\{[^}]*version\\s*=\\s*)"([^"]+)"`,
        'g'
      );
      
      if (versionRegex.test(newContent)) {
        newContent = newContent.replace(versionRegex, `$1"${VERSION}"`);
        updated = true;
      }
    }

    if (updated) {
      fs.writeFileSync(filePath, newContent, 'utf8');
      console.log(colorize(`  Updated internal dependencies in ${path.relative(PROJECT_ROOT, filePath)}`, 'blue'));
    }

    return updated;
  } catch (error) {
    console.error(colorize(`Error updating dependencies in ${filePath}:`, 'red'), error);
    return false;
  }
}

// Main function
function main() {
  console.log(colorize('='.repeat(50), 'cyan'));
  console.log(colorize('Cargo.toml Version Updater', 'bright'));
  console.log(colorize(`Updating all versions to ${VERSION}`, 'bright'));
  console.log(colorize('='.repeat(50), 'cyan'));

  const files = findCargoTomlFiles();
  let updatedFiles = 0;
  let skippedFiles = 0;
  let updatedDependencies = 0;

  // First pass: update package versions
  console.log(colorize('\nUpdating package versions...', 'cyan'));
  for (const file of files) {
    const result = updateVersionInCargoToml(file);
    if (result) {
      updatedFiles++;
    } else {
      skippedFiles++;
    }
  }

  // Second pass: update internal dependencies
  console.log(colorize('\nUpdating internal dependencies...', 'cyan'));
  for (const file of files) {
    if (updateInternalDependencies(file)) {
      updatedDependencies++;
    }
  }

  // Summary
  console.log(colorize('\n' + '='.repeat(50), 'cyan'));
  console.log(colorize('Update Summary:', 'bright'));
  console.log(`  ${colorize(updatedFiles.toString(), 'green')} package versions updated`);
  console.log(`  ${colorize(updatedDependencies.toString(), 'blue')} dependency references updated`);
  console.log(colorize('='.repeat(50), 'cyan'));

  if (updatedFiles > 0) {
    console.log(colorize('\nVersion update completed successfully!', 'green'));
    console.log(colorize('You may want to run "cargo check" to verify all changes.', 'yellow'));
  } else {
    console.log(colorize('\nNo files were updated.', 'yellow'));
  }
}

// Check if glob module is available
try {
  require.resolve('glob');
} catch (e) {
  console.error(colorize('Error: The "glob" package is required but not installed.', 'red'));
  console.error(colorize('Please install it with: npm install glob', 'yellow'));
  process.exit(1);
}

// Run the script
main();
