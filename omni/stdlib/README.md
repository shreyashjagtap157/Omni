# Omni-source standard library status (v0.1.2 remediation)

There is **no active `.omni` standard-library implementation in the qualified v0.1.2 native baseline**.

The historical prototype sources were moved to `docs/archive/stdlib-prototypes/` because they contained
partial bodies and bootstrap-only shims that were easy to mistake for implemented Edition-1 library
semantics. The currently compiled helper crate is `crates/omni-stdlib`; it is Rust bootstrap infrastructure,
not the normative Omni standard library.

A real Omni-source core/standard library will return to this location only when its APIs are implemented
end-to-end through the qualified native compiler and covered by conformance tests.
