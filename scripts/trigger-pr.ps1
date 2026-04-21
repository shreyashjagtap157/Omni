param()

$branch = "ci/trigger-llvm-backend-" + (Get-Date -Format "yyyyMMdd-HHmmss")
Write-Output "Branch: $branch"

$st = git status --porcelain
if ($st -ne "") {
    git add -A
    if (-not (git config --get user.name 2>$null)) { git config user.name "Omni CI Bot" }
    if (-not (git config --get user.email 2>$null)) { git config user.email "ci-bot@example.com" }
    git commit -m "ci: add LLVM backend workflow and helper scripts (auto)"
} else {
    Write-Output "No changes to commit"
}

# Create and switch to new branch
git checkout -b $branch

# Check remote
$remote = $null
try {
    $remote = git remote get-url origin 2>$null
} catch {}

if (-not $remote) {
    Write-Output "No remote 'origin' configured; cannot push. Created local branch: $branch"
    exit 2
}

Write-Output ("Pushing branch {0} to {1}" -f $branch, $remote)

git push -u origin $branch
if ($LASTEXITCODE -ne 0) {
    Write-Output "git push failed with code $LASTEXITCODE"
    exit $LASTEXITCODE
}

if (Get-Command gh -ErrorAction SilentlyContinue) {
    Write-Output "gh found; creating PR..."
    gh pr create --fill --title "CI: Run LLVM backend tests" --body "Auto-trigger LLVM backend CI workflow" | Write-Output
} else {
    Write-Output "gh CLI not found; please create a PR on GitHub to trigger the workflow (or install gh and re-run this script)."
}
