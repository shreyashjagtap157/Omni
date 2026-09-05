# Git Synchronization & Automated Versioning Status

Remote recorded in the retained repository metadata: `https://github.com/shreyashjagtap157/Omni.git`

## Automated Four-Part Versioning (`stable.major.minor.patch`)

The repository includes automated versioning and synchronization tooling:

1. **Pre-Commit Automatic Bumping**:
   - The git hook at `.githooks/pre-commit` (running `scripts/auto-version-hook.py`) automatically detects when code or implementation changes are staged.
   - It automatically increments the 4th component (`x.y.z.w -> x.y.z.(w+1)`) and re-stages all modified manifests, constants, and release files into the current commit without manual intervention.

2. **Milestone Promotion Tooling**:
   - `scripts/bump-version.py` advances `patch`, `minor`, `major`, or `stable` releases in lockstep across Cargo manifests, compiler constants, and qualification manifests.

3. **Remote & GitHub Synchronization**:
   - Run PowerShell: `.\scripts\sync-github.ps1 -Type patch -Remote origin`
   - Run Bash: `./scripts/sync-github.sh patch`
   - To sync to GitHub:
     ```bash
     git remote add github https://github.com/shreyashjagtap157/Omni.git
     .\scripts\sync-github.ps1 -Type minor -Remote github
     ```
   The script verifies all source gates, creates a release tag `v<x.y.z.w>`, and pushes commits and tags to the target remote.
