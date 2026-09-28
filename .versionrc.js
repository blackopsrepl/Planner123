/* Release configuration for commit-and-tag-version.

`commit-and-tag-version` owns `CHANGELOG.md` and every version surface listed
below; never edit them by hand. Run `make pre-release` on the tree to release,
then run the tool (see AGENT.md and skills/planner123/SKILL.md).

Version surfaces:
  - `Cargo.toml`  : `[package] version` (the crate version)
  - `Cargo.lock`  : the `planner123` package entry, so the lock stays in sync
  - `README.md`   : the version badge

Each surface declares an updater that reads the current version from a single
captured line and writes the next one back into that same line. Patterns are
anchored where the surrounding file allows it: `Cargo.toml` is matched in
multiline mode so `^version` is the package line, not a dependency constraint. */
const VERSION = '(\\d+\\.\\d+\\.\\d+)';

/** Build a commit-and-tag-version package/bump entry for one file. */
function surface(filename, pattern, template, flags = '') {
  const re = new RegExp(pattern, flags);
  return {
    filename,
    updater: {
      readVersion(contents) {
        const match = contents.match(re);
        return match ? match[1] : null;
      },
      writeVersion(contents, version) {
        return contents.replace(re, template.replace('{version}', version));
      },
    },
  };
}

const cargoToml = surface(
  'Cargo.toml',
  `^version = "${VERSION}"`,
  'version = "{version}"',
  'm',
);
const cargoLock = surface(
  'Cargo.lock',
  `name = "planner123"\\nversion = "${VERSION}"`,
  'name = "planner123"\nversion = "{version}"',
);
const readme = surface('README.md', `version-v${VERSION}`, 'version-v{version}');

module.exports = {
  tagPrefix: 'v',
  // Placeholder is `{{currentTag}}`, which this tool fills with the bare
  // version number (no tag prefix) — the form historic release commits used.
  releaseCommitMessageFormat: 'chore(release): {{currentTag}}',
  // Commit subjects keep the bare short hash the existing changelog uses.
  commitUrlFormat: '{{hash}}',
  compareUrlFormat:
    'https://github.com/blackopsrepl/Planner123/compare/{{previousTag}}...{{currentTag}}',
  packageFiles: [cargoToml, cargoLock, readme],
  bumpFiles: [cargoToml, cargoLock, readme],
};
