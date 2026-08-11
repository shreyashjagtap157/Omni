# Archived unqualified backend implementations

These sources are historical experiments preserved for future re-qualification. They are **not** active Omni
execution backends. They were removed from the live v0.1.2 code path because some operations did not preserve
Edition-1 checked arithmetic or pointer semantics. Future backend work should start from the normative LIR
contract and differential tests, not silently reactivate these snapshots.
