# Git synchronization status

Remote recorded in the retained repository metadata: `https://github.com/shreyashjagtap157/Omni.git`

The supplied `.git` metadata does not contain the Git object database needed to resolve the
recorded HEAD/parents in this environment. Do not create an unrelated orphan history or
force-push it.

To synchronize v0.1.4.1.1 safely from a real clone after applying the already-qualified v0.1.3
baseline:

```bash
git clone https://github.com/shreyashjagtap157/Omni.git
cd Omni
git checkout feature/omni-pipeline-completion   # or the intended target branch
# First reach the qualified v0.1.3 tree if the remote has not already done so.
git apply --index Omni-v0.1.3-to-v0.1.4.1.1-value-abi-collections.patch
git status
./scripts/qualify-release.sh
git commit -m "implement Omni v0.1.4.1.1 value ABI and collections foundation"
git push
```

If the real clone has diverged, apply without `--index`, resolve against real history, and
rerun the complete qualification gate before committing.
