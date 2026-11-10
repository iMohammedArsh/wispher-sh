# Hams project recovery — 2026-10-21

## Location

This recovered copy is in `/Users/mohammedarsh/Developer/wispher-sh`, outside the Documents/iCloud path.

## What was found

- The former workspace `/Users/mohammedarsh/Documents/projects/wispher-sh` no longer exists.
- The surviving folder `/Users/mohammedarsh/Documents/Documents - mohammed’s MacBook Pro/wispher-sh` contained approximately 14 GB of build/dependency artifacts, the speech model and server, and one Git object pack. It had no app source files, Git references, or Git configuration.
- That entire surviving folder was copied here using macOS clone-copy before recovery. The original has not been moved or deleted.
- A checksum-based rsync dry-run comparison completed successfully: no source files were missing or differed in the copy. It listed only destination-only files added by the Git recovery. The comparison used dry-run mode; it did not delete anything.
- The Git pack was indexed successfully and its two dangling commit histories were given recovery branches.

## Recovered history

- Checked-out branch: `recovery/icloud-2026-10-21`
- Newest recoverable tip: `a8323313de269cd8e151a0d389f5a8ab8114bbde` (2026-10-01, “feat: add Gemini 3.5 transcription pipeline”). Tracked source files were restored from this commit.
- Other recovered tip: branch `recovery/overlay-lab`, commit `84ccdaa417cfd6b3d4b41dec4743c4b7a8e17f8f`.
- Original branch names and remote configuration were missing. These recovery branch names are new, not evidence of the original names.
- `git fsck --full` passed after restoring the branches.

## Incomplete recovery — do not delete the original yet

The later development launcher, shared overlay, sidebar grouping, and other uncommitted changes discussed in this chat were not present in the recovered Git archive. This is an older recovered source version, not a verified restoration of the latest app.

No app was launched and no build was performed. The copied dependency folder is incomplete; do not assume the recovered app is ready to run. Existing compiled binaries may be newer than the restored source and are not build verification.

The accessible Documents, Desktop, Downloads and Developer folders were checked for similarly named project folders. A similarly named folder in Movies contains videos, not source, and was left untouched. macOS denied access to the user's Trash; it must be checked manually in Finder. iCloud Recently Deleted and any backups should also be checked before cleanup.

If the missing workspace is recovered, compare and merge it into this copy while preserving its `.git`, untracked files, and local edits. Do not overwrite either copy blindly. Only remove a duplicate after confirming all unique files and history have been preserved. No project duplicate has been deleted during this recovery.
