#!/usr/bin/env bash
# Make a dist-generated Homebrew formula install the shell completions and man pages that ship in
# the release archive (completions/ and man/, see scripts/release-assets.sh).
#
#   scripts/homebrew-formula-extras.sh Formula/fl.rb
#
# dist 0.33's formula template only installs binaries and copies every other file into pkgshare,
# where neither the shells nor man look. This inserts the installs before the template's leftover
# handling, so the files land in Homebrew's completion and man directories instead. The release
# workflow's publish-homebrew-formula job runs it before committing the formula to the tap (a hand
# edit to re-apply after `dist generate`; see CONTRIBUTING.md). Running it twice is a no-op; it
# fails if the template's anchor line is gone, so a dist upgrade cannot silently drop the installs.
set -euo pipefail

formula="${1:?usage: $0 FORMULA.rb}"
anchor='    # Homebrew will automatically install these, so we don'"'"'t need to do that'

if grep -q 'bash_completion.install' "$formula"; then
  echo "$formula already installs completions"
  exit 0
fi
if ! grep -qF "$anchor" "$formula"; then
  echo "error: $formula: anchor line not found; has dist's formula template changed?" >&2
  exit 1
fi

ANCHOR="$anchor" perl -0pi -e '
  my $extra = join "", map { $_ eq "" ? "\n" : "    $_\n" } (
    "# Shell completions and man pages from the archive (added by fideslang-tools'"'"'",
    "# scripts/homebrew-formula-extras.sh).",
    "bash_completion.install \"completions/fl.bash\" => \"fl\"",
    "zsh_completion.install \"completions/_fl\"",
    "fish_completion.install \"completions/fl.fish\"",
    "man1.install Dir[\"man/*.1\"]",
    "rm_r %w[completions man]",
    "",
  );
  s/^\Q$ENV{ANCHOR}\E$/$extra$ENV{ANCHOR}/m or die "anchor not replaced\n";
' "$formula"
echo "$formula now installs completions and man pages"
