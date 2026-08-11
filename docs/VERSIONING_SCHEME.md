# Omni versioning scheme

Omni project releases use the four-part scheme:

```text
stableRelease.majorRelease.minorRelease.patch
```

Current project release identity: **0.1.4.1**.

The components mean:

- `stableRelease`: increments for the first fully stable production line and later incompatible stable-era resets.
- `majorRelease`: increments for major pre-stable or stable capability milestones.
- `minorRelease`: increments for coherent feature wedges within the current major line.
- `patch`: increments for remediation, packaging, audit, compatibility, and other patch-only corrections that do not expand the claimed semantic surface.

Cargo package manifests remain SemVer-compatible because Cargo does not accept raw four-component versions. The Cargo crate version is therefore the compatibility base `0.1.4`, while each manifest carries `package.metadata.omni.project-version = "0.1.4.1"`. User-visible CLI identity, release manifests, audits, and qualification reports use the four-part project version.
