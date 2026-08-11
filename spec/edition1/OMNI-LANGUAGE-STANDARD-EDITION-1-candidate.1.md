# Omni Programming Language Standard

## Edition 1 — Complete Candidate 1.0.0-candidate.1

> This consolidated view is generated from the modular normative suite. Module files and machine-readable artifacts remain authoritative as described by OMNI-STD-ROOT.


---

# OMNI-STD-ROOT: Standard Suite Root

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | None |
| Date | `2026-08-04` |

## 1. Scope

Architecture, authority, terminology ownership, compatibility, governance, conformance claims, release classes.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Product definition

Omni Edition 1 is a statically typed, memory-safe-by-default, capability-secure, effect-aware, multi-paradigm language compiled normally to native machine code. Its default memory discipline is affine ownership with borrowing and regions. Explicit standard types provide reference counting and managed domains. Raw memory, foreign interfaces, device access, and inline assembly exist only behind audited unsafe obligations.

The suite defines four independently claimable products:

1. **Core Language**: source through dynamic semantics, memory, FFI, security, core library, and conformance.
2. **Platform**: target identity, ABI, runtime, object/link/startup/debug supplements.
3. **Distribution**: package, build, update, reproducibility, diagnostics, formatter, and documentation.
4. **Optional Profiles**: managed, accelerated, persistent, distributed, verified, deterministic, hardened, realtime, and constant-time facilities.

A product claim SHALL name the exact release manifest and every enabled profile.

## 4. Normative authority

Within a published release, domain-specific artifacts are co-normative and SHALL agree:

- source bytes and tokenization: `OMNI-SOURCE` and `OMNI-LEX`;
- parse trees: `OMNI-GRAMMAR` and `grammar/omni-edition1.ebnf`;
- static judgments: prose rules plus `models/static-semantics.yaml`;
- dynamic behavior: prose rules plus the abstract-machine transition definitions;
- memory behavior: prose axioms plus `models/memory-model.yaml`;
- ABI and wire formats: prose plus exact schemas and golden vectors.

Tests witness rules but do not invent semantics. When two normative artifacts disagree, no conforming interpretation exists until an erratum resolves the defect.

## 5. Required invariants

| Rule | Requirement |
|---|---|
| `ROOT-0001` | The normative Omni Edition 1 language definition consists of every module listed as `required` by the signed release manifest plus the exact data files and schemas named there. |
| `ROOT-0002` | No implementation, reference interpreter, compiler, test, example, or prior draft may override normative language text or formal rules; conflicts are specification defects that block publication. |
| `ROOT-0003` | Syntax is jointly governed by OMNI-SOURCE, OMNI-LEX, and OMNI-GRAMMAR. Static semantics are jointly governed by OMNI-NAMES, OMNI-TYPES, OMNI-NUM, OMNI-OWN, and OMNI-EFFECTS. Dynamic semantics are jointly governed by OMNI-MACHINE, OMNI-EVAL, OMNI-ERROR, OMNI-CONC, and OMNI-MEM. |
| `ROOT-0004` | A release SHALL contain no unresolved normative placeholder, no unclassified observable behavior, and no rule whose required conformance evidence is absent from the release manifest. |
| `ROOT-0005` | Safe well-typed Edition 1 programs SHALL NOT have undefined behavior. Every permitted implementation choice is classified as implementation-defined, unspecified, conditionally supported, or resource-dependent. |
| `ROOT-0006` | Native ahead-of-time compilation is the default execution route. A bytecode VM, interpreter, JIT, or REPL is an optional implementation technique and SHALL preserve Edition 1 observations. |
| `ROOT-0007` | Core language semantics are independent of optimization level, debug instrumentation, or build mode. |
| `ROOT-0008` | Extensions SHALL be namespaced, disabled by strict-conformance mode, fingerprinted into artifacts, and prohibited from changing the meaning of accepted Edition 1 programs. |
| `ROOT-0009` | A conforming implementation SHALL publish its supported targets, profiles, limits, external ABI references, implementation-defined choices, and known deviations in machine-readable form. |
| `ROOT-0010` | Edition, ABI, wire-schema, profile, Unicode-data, toolchain, and package versions are independent axes and SHALL NOT be conflated. |
| `ROOT-0011` | The specification text and machine-readable registries SHALL be published as immutable content-addressed artifacts with a signed manifest. |
| `ROOT-0012` | Security or soundness errata may narrow accepted programs but SHALL preserve already-specified safe observations whenever technically possible; compatibility impact SHALL be published. |

## 6. Edition compatibility

An Edition 1 source file is parsed and checked only under Edition 1 rules unless its package manifest selects a later edition. Later editions MAY accept an Edition 1 program with the same observations, or reject it only through an explicit migration boundary. They MUST NOT silently reinterpret the same token sequence under an Edition 1 manifest.

## 7. Completeness criterion

This candidate is definition-complete because every construct admitted by the grammar has a name-resolution rule, typing rule, ownership/effect rule, dynamic rule, fault behavior, and conformance classification. Optional facilities are complete within their named profiles. Ratification and implementation certification are separate processes and are not implied by definition completeness.

## 8. Intellectual property and contribution baseline

The specification text is intended for publication under a permissive specification license permitting implementation, quotation, translation, and derivative conformance material. Source examples and schemas are intended for a permissive software/documentation license. A production standards organization SHALL adopt an explicit royalty-free patent policy, contributor agreement, disclosure duty, appeal procedure, and security-embargo process before accepting external normative contributions.


---

# OMNI-TERMS: Vocabulary and Behavioral Taxonomy

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-STD-ROOT` |
| Date | `2026-08-04` |

## 1. Scope

Normative terms; required, implementation-defined, unspecified, invalid execution, erroneous, conditionally supported, observable behavior.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Normative vocabulary

| Term | Definition |
|---|---|
| **required behavior** | The single behavior or set of behaviors mandated by a normative rule. |
| **implementation-defined behavior** | A permitted choice selected by an implementation, documented before translation, queryable by tools, and stable for an artifact fingerprint. |
| **unspecified behavior** | One behavior from a finite set explicitly listed by the standard; the implementation need not document which member occurs on a particular execution. |
| **conditionally supported** | A feature or limit that an implementation may omit, but whose presence, absence, limits, and semantics must be declared. |
| **erroneous program** | A source program that violates a diagnosable rule. Translation SHALL fail before a release artifact is emitted. |
| **invalid unsafe execution** | A dynamic execution that reaches an unsafe operation while its stated precondition is false. Requirements after that event are withdrawn only for that execution as defined by OMNI-UNSAFE. |
| **resource failure** | Failure to obtain finite resources such as memory, stack, handles, storage, quota, or time. It produces the defined failure policy and never grants permission for memory unsafety. |
| **environmental failure** | A failure reported by the host, device, network, filesystem, process, or foreign service through a declared result/fault channel. |
| **unsupported program** | A well-formed program requiring a target, profile, limit, or feature not claimed by the implementation; rejection SHALL identify the unmet requirement. |
| **observable behavior** | An event included in the active observation set: returned values, emitted I/O, capability interactions, volatile/device operations, atomic synchronization, persistent commits, faults, termination, and explicitly exposed traces. |
| **undefined behavior** | A phrase prohibited for safe Edition 1 behavior. It may appear only when describing a foreign standard or as an informal synonym immediately corrected to `invalid unsafe execution`. |
| **artifact** | A content-addressed executable, library, object, package, schema, or image produced by a declared build action. |
| **execution** | One instantiation of an artifact with a target environment, capabilities, inputs, scheduler, and resource policy. |
| **place** | A typed storage location or projection that may be read, written, borrowed, moved, or addressed. |
| **value** | A valid inhabitant of a type, independent of any particular storage location. |
| **capability** | An unforgeable typed authority value issued by a trusted provider and attenuable but not widenable by ordinary code. |
| **effect** | A statically tracked class of observable action that a computation may perform. |
| **profile** | A named, versioned set of additional facilities and restrictions that does not redefine core syntax or core expression meaning. |

## 4. Behavioral classification rules

| Rule | Requirement |
|---|---|
| `TERM-0001` | Every normative clause that admits more than one observable result SHALL classify the choice using exactly one term from this document. |
| `TERM-0002` | Implementation-defined choices SHALL appear in the implementation manifest and artifact fingerprint when they can affect linking, layout, execution, persistence, or diagnostics. |
| `TERM-0003` | Unspecified choices SHALL be finite and explicitly enumerated; unconstrained behavior is prohibited. |
| `TERM-0004` | Quality-of-implementation latitude MAY affect performance, diagnostic wording beyond required fields, code layout, and other non-observable details. |
| `TERM-0005` | An erroneous program SHALL not produce a conforming release artifact, although tools MAY construct recovery trees for editing. |
| `TERM-0006` | A resource failure SHALL be reported through the allocator/API result, artifact fault policy, or target-mandated abort path named by the applicable rule. |
| `TERM-0007` | An external hardware failure does not make prior safe behavior invalid; target supplements SHALL classify its delivered signal, trap, abort, or environmental result. |
| `TERM-0008` | Timing, power, electromagnetic leakage, cache state, and speculative microarchitectural state are not ordinary core observations; constant-time and hardened profiles add explicit security observations. |

## 5. Prohibited ambiguous terms

Normative documents SHALL NOT use “undefined”, “platform dependent”, “compiler dependent”, “normally”, “usually”, “reasonable”, “as needed”, or “best effort” to describe semantics unless the clause immediately maps the phrase to a defined category and exact obligation.


---

# OMNI-RULES: Rule and Registry Infrastructure

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-STD-ROOT`, `OMNI-TERMS` |
| Date | `2026-08-04` |

## 1. Scope

Stable rule IDs, status lifecycle, hashes, dependency edges, test/proof links, reserved extension ranges.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Rule identifiers

A rule identifier has the form `DOMAIN-NNNN`, where `DOMAIN` is registered and `NNNN` is a four-digit monotonically allocated number. Published identifiers are never reused. Removed rules remain tombstones with their replacement or reason.

| Rule | Requirement |
|---|---|
| `RULE-0001` | Core domains reserve identifiers 0001 through 7999; profile standards use 8000 through 8999; vendor extensions use a reverse-domain namespace and cannot use unqualified Omni identifiers. |
| `RULE-0002` | Each rule registry record SHALL contain ID, title, normative text hash, module version, applicability, dependencies, diagnostic codes, tests, proofs/models, compatibility class, and lifecycle state. |
| `RULE-0003` | Lifecycle states are proposed, candidate, ratified, deprecated, superseded, withdrawn, and erratum-corrected. |
| `RULE-0004` | A semantic change to a ratified rule SHALL allocate a new revision record and preserve the prior text hash. |
| `RULE-0005` | Rule dependencies SHALL be acyclic after collapsing explicitly declared mutually recursive semantic clusters. |
| `RULE-0006` | Every required diagnostic and conformance test SHALL reference at least one rule ID; every ratified P0 rule SHALL reference at least one test or mechanically checked model obligation. |
| `RULE-0007` | Extension names use `x.<reverse-domain>.<name>` for attributes/effects/profiles and `X-<reverse-domain>-NNNN` for rule IDs. |
| `RULE-0008` | Collision resolution is by registered namespace ownership; display-name similarity never grants identity. |

## 4. Registry schema

The normative schema is `schemas/rule-registry.schema.json`. JSON objects are canonicalized using lexicographically sorted UTF-8 property names, no insignificant whitespace, and normalized LF before hashing.


---

# OMNI-SOURCE: Source Representation

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-TERMS` |
| Date | `2026-08-04` |

## 1. Scope

UTF-8, Unicode data versions, normalization, line endings, spans, bidi/invisible controls, source hashing.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Source unit

A source unit is a tuple `(edition, package-id, module-path, normalized-bytes, provenance)`. Source decoding occurs before lexical analysis. Ill-formed source never enters macro expansion or parsing.

| Rule | Requirement |
|---|---|
| `SRC-0001` | Source files SHALL be well-formed UTF-8. An optional UTF-8 BOM is accepted only at byte offset zero and is excluded from semantic hashing. |
| `SRC-0002` | Line endings CRLF and CR are semantically normalized to LF. Tools SHALL preserve an original-byte mapping for diagnostics and edits. |
| `SRC-0003` | Identifier equality uses Unicode NFC after tokenization. String and character data are never silently normalized. |
| `SRC-0004` | Edition 1 pins Unicode 17.0.0, UAX #15, UAX #31, UAX #9, UAX #24, UAX #44, and UTS #39 Revision 32 data through the release reference manifest and SHA-256 digests. |
| `SRC-0005` | Outside comments and literals, bidi control characters, noncharacters, unassigned code points, variation selectors, and default-ignorable format characters are source errors. |
| `SRC-0006` | Inside comments, bidi controls and invisible format characters require an escaped visible annotation generated by the formatter; strict mode rejects unannotated occurrences. |
| `SRC-0007` | Inside string and character literals, all Unicode scalar values except prohibited unescaped controls are data. Escape processing is defined by OMNI-LEX. |
| `SRC-0008` | Source spans are half-open byte ranges in normalized UTF-8 plus a source-file identity. Display columns are informative views. |
| `SRC-0009` | Semantic source identity is SHA-256 over edition tag, normalized UTF-8 bytes, package/module identity, generated-source recipe identity, and declared source attributes. |
| `SRC-0010` | Filesystem case, symlink spelling, timestamps, inode numbers, and enumeration order do not participate in module identity. |
| `SRC-0011` | Generated source SHALL record generator artifact digest, declared inputs, output path identity, and source map. |

## 4. Identifier security profile

Identifiers use `XID_Start` and `XID_Continue` from the pinned UCD, with `_` additionally permitted at the start and continuation. Join controls are excluded in Edition 1. Public identifiers SHALL satisfy the UTS #39 Highly Restrictive profile. Private identifiers that fail it are accepted only with an explicit `#[allow(mixed_script_identifier)]` attribute and remain subject to confusable collision checks.

Two identifiers in the same namespace whose NFKC casefolded skeletons are equal are a compile error unless both are ASCII and byte-distinct solely by case in a case-sensitive namespace, in which case strict mode still diagnoses them. Keywords are ASCII and compared byte-for-byte.

## 5. Hashing and archives

The candidate pins exact Unicode versions and revisions. A ratified mirrored publication SHALL additionally record SHA-256 digests for every normative Unicode data file; a compiler using different data SHALL reject an Edition 1 strict-conformance claim rather than silently retokenize source.


---

# OMNI-LEX: Lexical Grammar

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-SOURCE` |
| Date | `2026-08-04` |

## 1. Scope

Tokens, keywords, identifiers, literals, comments, punctuation, lexical errors, maximal munch.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Token classes

Edition 1 tokens are identifiers, keywords, literals, punctuation, delimiters, comments, and end-of-file. Comments and whitespace are retained in a lossless concrete syntax tree but are absent from the semantic token stream.

### 3.1 Keywords

`Never Self Sized addrspace as async await bare_metal bf16 bool break byte cap catch char const continue dec128 dec32 dec64 defer distributed dyn effect else enum ensure extern f128 f16 f32 f64 false fn for hosted i128 i16 i32 i64 i8 if impl in is isolate isize let loop macro managed match module move mut not opaque override package panic persistent pub pure ref relation require return script self static str struct super thread_local trait true try type typeof u128 u16 u32 u64 u8 unsafe use usize verified where while yield`

### 3.2 Punctuation

Single tokens: `(` `)` `[` `]` `{` `}` `,` `;` `:` `.` `@` `#` `?` `$` `_`.

Operators and compound tokens: `::` `->` `=>` `:-` `..` `..=` `...` `!` `!=` `=` `==` `<` `<=` `>` `>=` `+` `-` `*` `/` `%` `&` `|` `^` `~` `&&` `||` `<<` `>>` `+=` `-=` `*=` `/=` `%=` `&=` `|=` `^=` `<<=` `>>=` `?.` `??`.

Reserved for future editions: `<-` `<=>` `**` `??=` `|>` and any maximal punctuation sequence not listed above.

| Rule | Requirement |
|---|---|
| `LEX-0001` | Tokenization uses maximal munch except that `>>` in a type generic-argument context is two `>` tokens; the parser supplies this single context-sensitive split without changing source offsets. |
| `LEX-0002` | Whitespace separates tokens but is otherwise insignificant. Newlines never terminate statements. |
| `LEX-0003` | Line comments begin with `//` and end before LF. Block comments `/* ... */` nest. Unterminated comments are lexical errors. |
| `LEX-0004` | Documentation comments are `///`, `//!`, `/** ... */`, and `/*! ... */` and are converted to `doc` attributes before item parsing. |
| `LEX-0005` | Raw identifiers use `r#identifier`; the underlying identifier may equal a keyword but must otherwise satisfy identifier rules. |
| `LEX-0006` | Integer separators `_` are allowed only between digits of the same radix. Leading, trailing, adjacent-to-prefix, adjacent-to-suffix, or doubled separators are errors. |
| `LEX-0007` | Escape sequences are `\0`, `\t`, `\n`, `\r`, `\"`, `\'`, `\\`, `\xNN`, and `\u{H...}` with one to six hexadecimal digits naming a Unicode scalar. |
| `LEX-0008` | Character literals contain exactly one Unicode scalar after escape processing. Byte literals contain exactly one value 0 through 255. |
| `LEX-0009` | Raw strings use `r"..."` or `r#*"..."#*` with matching hash count from zero through 255. No escapes are processed. |
| `LEX-0010` | Interpolated strings use `f"text ${ expression } text"`; braces in text are escaped as `{{` and `}}`. Interpolation is tokenized recursively with balanced delimiters. |
| `LEX-0011` | Unknown punctuation is a lexical error unless introduced by an enabled namespaced extension. |

## 4. Literal grammar

Integer literals are binary (`0b`), octal (`0o`), decimal, or hexadecimal (`0x`) digit sequences with an optional fixed primitive suffix. Decimal floating literals require a decimal point or exponent; hexadecimal floating literals require a `p` exponent. Supported suffixes are `i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f16 bf16 f32 f64 f128 dec32 dec64 dec128` subject to profile support.

String prefixes are `b` for bytes, `r` for raw, `f` for interpolation, and `br`/`rb` for raw bytes. Interpolation and raw mode cannot be combined in Edition 1.

## 5. Lexical errors

A malformed token consumes the shortest prefix that proves malformed while permitting deterministic recovery. Recovery tokens are tooling-only and SHALL NOT appear in a release AST.


---

# OMNI-GRAMMAR: Syntactic Grammar

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-LEX` |
| Date | `2026-08-04` |

## 1. Scope

Complete grammar, precedence, associativity, disambiguation, parse errors, grammar versioning.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Grammar notation

`{ X }` means zero or more repetitions, `[ X ]` means optional, `( A | B )` means choice, and quoted strings are lexical terminals. `A - B` means the subset of `A` excluding forms classified as `B`; the distributed grammar replaces this convenience with generated productions.

| Rule | Requirement |
|---|---|
| `GRAM-0001` | The normative grammar is `grammar/omni-edition1.ebnf`; prose in this document resolves only notation and explicitly stated context restrictions. |
| `GRAM-0002` | A block final expression lacks a semicolon and yields the block value. A non-block expression used as a statement requires `;`. A block-form expression statement may omit `;`. |
| `GRAM-0003` | Newlines are not grammar terminals and cannot change a parse. |
| `GRAM-0004` | Assignment operators are right-associative. All other binary operators are left-associative except comparisons, which do not chain. |
| `GRAM-0005` | Postfix operators bind tighter than unary operators; unary operators bind tighter than multiplicative operators; the complete precedence is encoded by grammar nonterminals. |
| `GRAM-0006` | Expression generic arguments after a path or method use the explicit `::` introducer where needed to avoid comparison ambiguity. |
| `GRAM-0007` | Error productions MAY exist in an IDE parser but SHALL be tagged recovery-only and SHALL NOT be accepted in release translation. |
| `GRAM-0008` | Feature-gated grammar exists only in a named edition or profile namespace and is included in the source and artifact fingerprint. |

## 4. Semicolon and block rules

The grammar admits no automatic semicolon insertion. The parser never examines line breaks to decide whether a statement ended. `return`, `break`, and `continue` are expressions; when used as statements they follow the same semicolon rule.

## 5. Precedence, from lowest to highest

1. assignment;
2. ranges;
3. logical OR;
4. logical AND;
5. null coalescing;
6. one comparison;
7. bitwise OR, XOR, AND;
8. shifts;
9. additive;
10. multiplicative;
11. casts;
12. unary;
13. postfix;
14. primary.

## 6. Parse determinism

A conforming parser SHALL produce the canonical concrete tree defined by the grammar. Ambiguity is a specification defect. The release corpus includes every pair of adjacent token classes, nested generic/shift cases, macro token trees, and semicolon-sensitive token sequence.


---

# OMNI-ATTR: Attributes and Annotations

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-GRAMMAR`, `OMNI-NAMES` |
| Date | `2026-08-04` |

## 1. Scope

Attribute syntax, namespaces, retention, duplication, target gating, unknown attributes, semantic phases.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Built-in attributes

Edition 1 defines `#[repr(...)]`, `#[inline(...)]`, `#[cold]`, `#[must_use]`, `#[deprecated(...)]`, `#[target_feature(...)]`, `#[cfg(...)]`, `#[derive(...)]`, `#[no_mangle]`, `#[export_name(...)]`, `#[link_name(...)]`, `#[panic_policy(...)]`, `#[numeric(...)]`, `#[allow(...)]`, `#[warn(...)]`, `#[deny(...)]`, `#[forbid(...)]`, `#[test]`, `#[bench]`, and `#[doc(...)]`.

| Rule | Requirement |
|---|---|
| `ATTR-0001` | Attributes are resolved in the attribute namespace after tokenization and outer macro expansion but before the semantic phase named by the attribute definition. |
| `ATTR-0002` | Unknown unqualified attributes are errors. Unknown namespaced attributes are errors in strict mode and may be retained as inert metadata only when their namespace policy allows it. |
| `ATTR-0003` | An attribute definition declares targets, multiplicity, argument grammar, retention (`source`, `HIR`, `MIR`, `object`, or `runtime`), and whether it affects semantics or only diagnostics/tooling. |
| `ATTR-0004` | Duplicate non-repeatable attributes are errors. Repeatable attributes preserve source order unless their definition declares set semantics. |
| `ATTR-0005` | Attributes cannot widen capabilities, suppress type/ownership safety, or create unchecked behavior except through the standard `unsafe` mechanisms. |
| `ATTR-0006` | Target/profile-gating attributes remove an item before name resolution only when the condition depends solely on manifest-declared target/profile facts. |
| `ATTR-0007` | Built-in semantic attributes are reserved under `omni.*`; user and vendor attributes require a registered namespace. |

## 4. Attribute phase order

1. lexical conversion of documentation comments;
2. inner source/module attributes;
3. declarative token macros;
4. target/profile item filtering;
5. import and name resolution;
6. derive and typed macros;
7. static semantics;
8. lowering/optimization hints;
9. object/runtime metadata retention.


---

# OMNI-MACHINE: Abstract Machine and Translation Model

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-SOURCE`, `OMNI-GRAMMAR`, `OMNI-TERMS` |
| Date | `2026-08-04` |

## 1. Scope

Translation phases, program/environment model, observations, termination, resource failure, host/target distinction.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Abstract machine state

The machine state is `A = (P, Σ, T, C, X, R)`:

- `P`: immutable program and type metadata;
- `Σ`: allocations, object lifetimes, values, and persistent buffers;
- `T`: tasks/threads with control stacks and cleanup stacks;
- `C`: capabilities and provider state;
- `X`: ordered external input/event stream;
- `R`: finite resource budgets and artifact policies.

A transition `A --event--> A′` either emits no observation (`τ`) or one typed observation. A terminating machine produces `exit(status)`, `panic(payload)`, `isolate(task,payload)`, `abort(reason)`, or `target_trap(code)` as defined by the active artifact and target policy.

| Rule | Requirement |
|---|---|
| `MACH-0001` | A program is the closed package graph, selected target and profiles, linked artifacts, manifest capabilities, and entry point. |
| `MACH-0002` | Translation proceeds through source decoding, lexing, parsing, macro expansion, name resolution, static checking, semantic lowering, target lowering, object emission, linking, loading, runtime-component initialization, and entry invocation. |
| `MACH-0003` | Each translation phase consumes only declared inputs and emits content-addressed outputs plus diagnostics; no phase may observe undeclared time, randomness, environment, filesystem state, or network. |
| `MACH-0004` | An execution consists of an initial abstract store, task set, capability set, external-event stream, scheduler choices, and resource policy. |
| `MACH-0005` | Core observations are ordered I/O/capability events, volatile/device events, synchronization events explicitly exposed by APIs, persistent commit outcomes, returned exit status, faults, panic/isolation, and termination. |
| `MACH-0006` | Internal allocation addresses, stack layout, register contents, padding bytes, optimizer choices, and elapsed time are not core observations. |
| `MACH-0007` | Nontermination is a permitted outcome when the program has an infinite transition sequence and no stronger progress/realtime contract applies. |
| `MACH-0008` | Resource exhaustion follows OMNI-ERROR and cannot invalidate prior safe observations or permit memory/type violations. |
| `MACH-0009` | External asynchronous events enter only at target/profile-defined delivery points and are represented as typed events, cancellation, signals, interrupts, or device completions. |
| `MACH-0010` | A hosted entry point is `fn main(args: Args, caps: AppCaps) -> Exit ! ε`; a freestanding target supplement defines reset/entry signatures; script mode synthesizes a package and uses hosted semantics. |
| `MACH-0011` | Normal process termination flushes only resources whose APIs promise flush-on-close; power loss and forced termination do not run ordinary destructors. |

## 4. Host and target separation

Compile-time execution runs in a deterministic host-independent semantic machine. Target constants such as pointer width and endianness are explicit inputs. Host pointer size, locale, floating environment, path separator, and process environment cannot leak into target semantics.

## 5. Full expressions and sequence points

A full expression is an initializer, condition, return/break value, expression statement, match guard, call argument, aggregate element/field initializer, or interpolation expression at the grammar boundary named by OMNI-EVAL. Full-expression completion is a cleanup boundary for non-extended temporaries.


---

# OMNI-NAMES: Names, Modules, Linkage, and Initialization

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-GRAMMAR`, `OMNI-MACHINE` |
| Date | `2026-08-04` |

## 1. Scope

Scopes, imports, visibility, package/module identity, linkage, initialization, symbol identity, coherence anchors.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Scope model

Scopes are lexical and nested. Item declarations are visible throughout their module after target/profile filtering, except macro declarations whose visibility follows their declared expansion phase. Local bindings become visible after their initializer completes, preventing self-reference by accident.

| Rule | Requirement |
|---|---|
| `NAME-0001` | Module identity is `(package-source-id, package-name, package-version, feature-instance, module-path)` and never a raw filesystem path. |
| `NAME-0002` | A source file declares one module or one explicitly permitted fragment. Fragment merge order is the lexicographic order of normalized source identities and SHALL NOT affect semantic resolution. |
| `NAME-0003` | Separate namespaces exist for modules/types, values, traits, macros, lifetimes, labels, effects, capabilities, and attributes. |
| `NAME-0004` | Within a namespace, a declaration may not duplicate another declaration with the same normalized identifier unless it is an explicitly mergeable module fragment or trait implementation. |
| `NAME-0005` | Imports are explicit, non-transitive, and resolved independent of source/file enumeration order. Wildcard imports that create ambiguity are errors. |
| `NAME-0006` | Lexical shadowing is permitted for local values and lifetimes; items, generic parameters, labels, effects, and capabilities cannot be silently shadowed in the same declaration header. |
| `NAME-0007` | Name lookup order is local binding, generic parameter, current item members, explicit imports, module items, then prelude. Ambiguity at one level is an error and lower levels are not searched. |
| `NAME-0008` | Free functions do not overload by parameter type. Method and operator polymorphism are trait-based and coherence-checked. |
| `NAME-0009` | Import cycles are allowed only when all cycle edges are type/macro-signature-only and no value initialization depends cyclically. The initialization graph must be acyclic. |
| `NAME-0010` | `const` values are evaluated at translation. Immutable `static` values use constant initialization or an explicit lazy cell. Mutable static access is unsafe unless mediated by a safe synchronization type. |
| `NAME-0011` | Thread-local initialization occurs on first odr-use per thread, and failure is cached as the declared panic/error policy. |
| `NAME-0012` | Public symbol identity includes package identity, module path, item name, generic signature, ABI version, and representation/effect fingerprint. |
| `NAME-0013` | Trait coherence is global across the resolved package graph: an implementation is legal only if the trait or nominal self type is local to the defining package, except sealed delegated extension points. |
| `NAME-0014` | Dynamic loading cannot introduce a trait implementation that would overlap an implementation visible when the consuming artifact was linked. |

## 4. Initialization algorithm

1. evaluate all compile-time constants;
2. construct immutable constant-initialized statics;
3. register lazy and thread-local cells without executing user bodies;
4. initialize selected runtime components in dependency order;
5. invoke the entry point.

Top-level arbitrary executable initialization is prohibited. Cross-module initialization cycles therefore cannot hide in linker order.

## 5. Method resolution

For receiver type `R`, method resolution constructs a finite autoderef chain using built-in references and in-scope `Deref` implementations. At each level it considers inherent methods, then explicitly imported trait methods, applying at most one autoref. Exactly one candidate must remain after generic constraints. Return type alone cannot select a method.


---

# OMNI-TYPES: Static Type System

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-NAMES`, `OMNI-MACHINE` |
| Date | `2026-08-04` |

## 1. Scope

Types, inference, generics, traits, subtyping/coercions, variance, object validity, unsized and zero-sized types.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Static judgment


- `Γ` is the lexical typing and name environment.
- `Ω` is the ownership and initialization state.
- `Ε` is the available effect/capability environment.
- `Σ` is the abstract store, including allocation identities and object lifetimes.
- `Μ` is the concurrent memory event graph.
- `Γ; Ω; Ε ⊢ e : T ! ε ⇒ Ω′` means expression `e` has type `T`, may perform effect row `ε`, and transforms ownership state `Ω` to `Ω′`.
- `⟨e, Σ, κ⟩ → ⟨e′, Σ′, κ′⟩` is one dynamic evaluation step.
- `hb` denotes happens-before; `sw` denotes synchronizes-with; `mo` denotes per-atomic modification order.


A declaration is accepted only when name resolution is unique, type/effect constraints terminate with one solution, all ownership paths are valid, every potentially executed operation is authorized by its effect/capability context, and all required refinements are proved or checked.

| Rule | Requirement |
|---|---|
| `TYPE-0001` | Type identity is nominal for structs, enums, traits, opaque types, capabilities, effects, and aliases declared `distinct`; ordinary `type` aliases are transparent. |
| `TYPE-0002` | Tuples, function types, references, raw pointers, arrays, slices, and generic instantiations are structurally equal when all constituents and qualifiers are equal. |
| `TYPE-0003` | Recursive nominal types require an indirection, unsized tail, managed reference, or opaque boundary; infinitely sized value types are erroneous. |
| `TYPE-0004` | Every safe value satisfies its type validity invariant. Invalid scalar bit patterns cannot be constructed or observed in safe code. |
| `TYPE-0005` | `bool` has values `false` and `true`; `char` is any Unicode scalar; references are non-null, aligned, live, and provenance-valid; enums have a declared live variant. |
| `TYPE-0006` | Zero-sized types occupy no logical bytes but have distinct ownership/drop events. Arrays of zero-sized types retain length and iteration count. |
| `TYPE-0007` | `Never` is uninhabited and coerces to any type. Diverging expressions have type `Never`. |
| `TYPE-0008` | Unsized types are `[T]`, `str`, `dyn Trait`, and declared extern types. They may occur only behind a pointer/reference/owner or as the final field of a declared dynamically sized aggregate. |
| `TYPE-0009` | Public functions, public fields, FFI declarations, persisted schemas, capabilities, and exported constants require explicit types and effects. |
| `TYPE-0010` | Inference is local to a function/item body and must have a unique principal solution after defaults. Ambiguity is a diagnostic, not arbitrary selection. |
| `TYPE-0011` | Generic parameter kinds are type, lifetime, const value, effect row, and capability type. Defaults may reference earlier parameters only. |
| `TYPE-0012` | Constraint solving normalizes aliases and associated types, applies coherence-selected implementations, and terminates under the edition solver restrictions. |
| `TYPE-0013` | Trait objects contain only object-safe traits: no methods requiring `Self: Sized`, no generic methods, and associated types/consts required by calls must be fixed. |
| `TYPE-0014` | Downcast identity uses a 128-bit stable type fingerprint only for types opting into runtime identity; ordinary private type identity is not serialized. |
| `TYPE-0015` | Specialization exists only inside a sealed specialization family with a strict partial order where each pair of applicable implementations has one unique greatest element. |
| `TYPE-0016` | Implicit coercions are limited to lifetime shortening, `&mut T` to `&T`, reborrow, array reference to slice reference, concrete reference/owner to object-safe trait object, function item to function pointer, noncapturing closure to compatible function pointer, unsizing, and `Never` to any type. |
| `TYPE-0017` | No implicit numeric conversion, allocation, cloning, reference-count change, dynamic boxing, blocking, or capability acquisition occurs as a coercion. |
| `TYPE-0018` | Refinements are conjunctions of linear integer bounds, equalities, finite-set membership, length/alignment/unit predicates, and sealed protocol-state predicates. Undischarged obligations require explicit runtime checks or proof parameters. |
| `TYPE-0019` | Units and dimensions normalize to a rational scale and integer exponent vector. Unit metadata is erased from native ABI only after conversion is explicit and overflow-checked. |
| `TYPE-0020` | `Dynamic` is an explicit boxed tagged value with runtime type identity, checked extraction, `dynamic` effect for reflective operations, and no implicit conversion to static types. |

## 4. Variance

- `&'a T` is covariant in `'a` and in `T` when `T` contains no interior-mutability exposure through that reference.
- `&'a mut T`, `Own<T>`, `Pinned<T>`, and mutable capability types are invariant in `T`.
- raw pointers are invariant and have no safe subtyping.
- arrays and immutable containers inherit element covariance only for shared-borrow views, never for mutable/owning forms.
- function parameters are contravariant, returns covariant, and effect rows covariant by subset (a purer function substitutes for a more effectful allowance).

## 5. Traits and associated items

Trait laws are normative documentation/proof obligations but are not assumed by the optimizer unless attached as proved contracts. Marker traits affecting safety (`Copy`, `Send`, `Sync`, `Unpin`, `Capability`) are compiler-known, sealed, or unsafe to implement with explicit obligations.

## 6. Refinement fallback

The compiler uses the solver fragment version pinned in the release manifest. A failed proof is not proof of falsity. The programmer may add a checked contract, provide a proof term, or rewrite the program; the compiler may not guess.


---

# OMNI-NUM: Numeric Semantics

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-TYPES`, `OMNI-MACHINE` |
| Date | `2026-08-04` |

## 1. Scope

Integers, bit-precise types, floating modes, decimal, NaNs, rounding, exceptions, conversions, reproducibility.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Integer and bit-precise model

For `uint<N>`, values are integers in `[0, 2^N-1]`. For `int<N>`, values are integers in `[-2^(N-1), 2^(N-1)-1]`. The abstract value is mathematical; the native representation is fixed by this document and the target endian/layout supplement.

| Rule | Requirement |
|---|---|
| `NUM-0001` | Fixed-width signed integers use two’s-complement with no padding bits. Unsigned integers use pure binary representation. |
| `NUM-0002` | Edition 1 implementations SHALL support bit-precise `int<N>` and `uint<N>` for every `1 <= N <= 4096`; larger widths are conditionally supported and declared. |
| `NUM-0003` | `usize` and `isize` match the target default data-pointer address width but do not imply every address-space pointer has that representation. |
| `NUM-0004` | Integer literals are arbitrary-precision until constrained; unconstrained integer literals default to `i64`. Nonrepresentable literals are compile errors. |
| `NUM-0005` | Default integer arithmetic is checked in every build mode. Overflow, invalid shift counts, division by zero, and signed-minimum divided by minus one raise `ArithmeticFault`. |
| `NUM-0006` | Explicit arithmetic families are `checked`, `wrapping`, `saturating`, `widening`, `carrying/borrowing`, and proof-qualified `exact`. |
| `NUM-0007` | Shifts accept a nonnegative integer count strictly less than the left operand width. Wrapping shift APIs reduce the count modulo width explicitly. |
| `NUM-0008` | Integer division truncates toward zero; remainder has the dividend sign and satisfies `a = (a/b)*b + a%b` when defined. |
| `NUM-0009` | No implicit conversion occurs between distinct concrete numeric types. `as?` is checked and returns `Option`; `as!` traps on failure; named wrapping/truncating/bitcast APIs state alternate semantics. |
| `NUM-0010` | Bitcast requires equal bit width and a destination representation for which the produced bits are valid, or returns a raw byte/MaybeUninit form requiring validation. |
| `NUM-0011` | Floating functions inherit a lexical numeric policy: `strict` by default, `reproducible`, `contract`, or `fast`. |
| `NUM-0012` | `strict` uses IEEE roundTiesToEven for ordinary operations, preserves signed zero and infinities, forbids reassociation, and quiets signaling NaNs. NaN payload selection is implementation-defined and declared. |
| `NUM-0013` | `reproducible` canonicalizes NaNs, fixes subnormal handling to gradual underflow, fixes operation decomposition, and guarantees bit-identical results for the supported operation set across conforming targets. |
| `NUM-0014` | `contract` permits fused multiply-add only at source-marked contraction sites. `fast` permits only the exact relaxations listed in its annotation and never changes memory/type/authority safety. |
| `NUM-0015` | `f16`, `bf16`, `f32`, and `f64` have their named storage formats. Evaluation occurs in the declared type except explicit widening operations. `f128` is conditionally supported. |
| `NUM-0016` | Floating comparisons follow IEEE ordered/unordered semantics; total ordering is provided by a named `total_cmp` operation. |
| `NUM-0017` | Decimal types are available in the decimal profile with IEEE decimal32/64/128 interchange semantics, explicit decimal context, and no implicit binary-decimal conversion. |
| `NUM-0018` | Compile-time and runtime numeric behavior are identical under the same policy and target feature set. |

## 4. Floating environment

Edition 1 does not expose a mutable ambient processor rounding mode to ordinary arithmetic. Alternate rounding is an explicit operation or decimal/binary context capability. Implementations SHALL save/restore or avoid incompatible host floating state at foreign boundaries.

## 5. Constant-time interaction

Numeric operations whose target latency depends on secret operands are rejected in the constant-time profile unless the target supplement certifies the instruction sequence or a verified constant-time library implementation is selected.


---

# OMNI-OWN: Ownership, Lifetimes, Regions, and Destruction

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-TYPES`, `OMNI-MACHINE` |
| Date | `2026-08-04` |

## 1. Scope

Moves, borrows, reborrows, regions, pinning, partial initialization/moves, drops, interior mutability.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Ownership judgment


- `Γ` is the lexical typing and name environment.
- `Ω` is the ownership and initialization state.
- `Ε` is the available effect/capability environment.
- `Σ` is the abstract store, including allocation identities and object lifetimes.
- `Μ` is the concurrent memory event graph.
- `Γ; Ω; Ε ⊢ e : T ! ε ⇒ Ω′` means expression `e` has type `T`, may perform effect row `ε`, and transforms ownership state `Ω` to `Ω′`.
- `⟨e, Σ, κ⟩ → ⟨e′, Σ′, κ′⟩` is one dynamic evaluation step.
- `hb` denotes happens-before; `sw` denotes synchronizes-with; `mo` denotes per-atomic modification order.


The ownership state maps each place projection to initialization status and active loans. Control-flow joins are legal only when all incoming states admit a single conservative state in which no potentially moved value is treated as initialized and no expired loan is treated as live.

| Rule | Requirement |
|---|---|
| `OWN-0001` | Every non-`Copy` value is affine: it may be moved at most once and destroyed at most once. Unused owned values are destroyed at their cleanup boundary. |
| `OWN-0002` | A place is initialized, partially initialized, moved, or uninitialized. Reads and drops require the relevant portion to be initialized. |
| `OWN-0003` | A move transfers value ownership and marks the source projection uninitialized. Moving through a shared borrow is prohibited. |
| `OWN-0004` | `Copy` may be implemented only for types with no destructor and only `Copy` fields. Copying duplicates the value without invalidating the source. |
| `OWN-0005` | At any time a memory location has either any number of usable shared borrows or one usable mutable borrow, unless access is mediated by `UnsafeCell` and a safe synchronization contract. |
| `OWN-0006` | Reborrowing creates a child loan whose permissions and lifetime are no greater than the parent; conflicting use of the parent is suspended while the child is live. |
| `OWN-0007` | Borrow lifetimes are inferred from actual use and control flow. Lifetime elision applies only to the listed function-signature patterns and never guesses among multiple input lifetimes. |
| `OWN-0008` | Higher-ranked lifetime bounds quantify explicitly or through the `for<...>` form; escaping a locally bound lifetime is erroneous. |
| `OWN-0009` | Compiler-generated mutable receiver autoref uses a two-phase loan: reservation before later argument evaluation and activation immediately before call entry. No other borrow is two-phase unless explicitly specified. |
| `OWN-0010` | Partial moves are legal for aggregates without an unconditional whole-value destructor, or where compiler-generated drop flags identify each remaining field. User `Drop` types cannot be partially moved through safe syntax. |
| `OWN-0011` | Locals drop in reverse successful-initialization order. Fields and active variant payloads drop in reverse declaration order. Arrays drop from highest initialized index to lowest. |
| `OWN-0012` | Closure capture fields are ordered by first source occurrence of the captured root binding, then by projection order; they drop in reverse field order. |
| `OWN-0013` | Temporaries drop in reverse creation order at the end of the full expression unless a grammar-defined binding extends them. Borrow diagnostics and MIR expose any hidden binding. |
| `OWN-0014` | When returning, the return value is fully evaluated and moved into the return slot before local cleanup; cleanup then runs and the initialized return slot transfers to the caller. |
| `OWN-0015` | A failed constructor drops every successfully initialized field in reverse initialization order and never drops uninitialized fields. |
| `OWN-0016` | Pinning guarantees that the pinned value will not move or have its storage reused until its pinned destructor completes. Projection is safe only through a pin-projection contract. |
| `OWN-0017` | Self-referential initialization requires a pinned construction API that prevents observation before all internal references are established. |
| `OWN-0018` | `UnsafeCell<T>` is the sole primitive for legal interior mutation through shared references. Safe wrappers must establish synchronization or thread confinement. |
| `OWN-0019` | Regions own allocations collectively. Region references cannot outlive the region; unique regions may move between tasks; frozen regions may be shared immutably. |
| `OWN-0020` | Bulk region reclamation does not run element destructors unless the region was created as a finalizing region, which records and runs drops in reverse registration order. |
| `OWN-0021` | `Shared<T>` uses atomic reference counts with release on decrement and acquire fence on the transition to zero. `LocalShared<T>` is thread-confined and non-atomic. |
| `OWN-0022` | Weak upgrade races are resolved atomically: success obtains a strong count while the object is live; failure returns `None`. Cycles are not collected unless placed in a managed domain or explicit cycle collector. |

## 4. Lifetime elision

A single input reference lifetime is assigned to every elided output reference lifetime. For methods, an elided output lifetime is assigned the receiver lifetime. Any other elided output lifetime is an error. Body lifetime inference does not alter public signature identity.

## 5. Destructors

`Drop.drop(&mut self)` runs exactly once for a fully initialized owned value on ordinary scope exit and supported unwind. It cannot move fields through safe syntax. Destructor panic behavior is defined by OMNI-ERROR. Process abort, power loss, target reset, and invalid unsafe execution do not promise destructor execution.

## 6. Arena and managed interactions

A managed reference cannot point into a shorter native region unless the managed object holds an owning region handle. Native pointers into movable GC objects cannot cross a safepoint; pin, handle, or copy is required.


---

# OMNI-EFFECTS: Effects, Capabilities, and Authority

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-TYPES`, `OMNI-OWN` |
| Date | `2026-08-04` |

## 1. Scope

Effect rows, inference, subtyping, handlers, resumptions, capability issuance/delegation/revocation, determinism.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Effect typing


- `Γ` is the lexical typing and name environment.
- `Ω` is the ownership and initialization state.
- `Ε` is the available effect/capability environment.
- `Σ` is the abstract store, including allocation identities and object lifetimes.
- `Μ` is the concurrent memory event graph.
- `Γ; Ω; Ε ⊢ e : T ! ε ⇒ Ω′` means expression `e` has type `T`, may perform effect row `ε`, and transforms ownership state `Ω` to `Ω′`.
- `⟨e, Σ, κ⟩ → ⟨e′, Σ′, κ′⟩` is one dynamic evaluation step.
- `hb` denotes happens-before; `sw` denotes synchronizes-with; `mo` denotes per-atomic modification order.


Effect rows are written `!{fs.read, alloc | E}`. The empty row is pure. A caller must provide both static permission for the effect and any capability argument required by the callee.

| Rule | Requirement |
|---|---|
| `EFF-0001` | An effect row is a finite canonical set of effect terms plus at most one row variable. Duplicate terms normalize to one term. |
| `EFF-0002` | Effect row equality is equality after alias expansion, parameter normalization, and canonical sorting. |
| `EFF-0003` | A function with effect row `ε1` substitutes where `ε2` is allowed when `ε1` is a subset of `ε2` after constraint solving. |
| `EFF-0004` | Private functions may infer effects. Public functions and trait methods declare an explicit upper bound; adding an externally visible effect is a breaking API change unless already polymorphic. |
| `EFF-0005` | Effect masking is permitted only by a handler that discharges the effect and whose own residual effects are included in the result row. |
| `EFF-0006` | Effects are generalized only for syntactic value bindings whose captured values satisfy ownership constraints; effectful computations are not implicitly generalized. |
| `EFF-0007` | Built-in effects include allocation, panic, cancellation, synchronization, blocking, async suspension, I/O families, time, randomness, environment, dynamic reflection, foreign calls, persistence, devices, accelerators, nondeterminism, and unsafe families. |
| `EFF-0008` | An effect declaration states observations and handler protocol; it does not itself grant authority over a resource. |
| `EFF-0009` | A capability is a sealed nominal value created only by a provider trusted by the selected profile/host. Integer, byte, reflection, cloning, deserialization, and FFI operations cannot forge it. |
| `EFF-0010` | Capabilities may be attenuated to a strict subset of rights, scope, quota, duration, address/path range, protocol, or operation. Ordinary code cannot widen them. |
| `EFF-0011` | Delegation follows ownership: move transfers authority, borrow lends authority for the borrow lifetime, and explicit sub-capability creation delegates narrowed authority. |
| `EFF-0012` | Revocation is provider-defined and races are resolved at the authorized operation: the provider atomically accepts under the prior epoch or rejects as revoked. A check does not guarantee future use. |
| `EFF-0013` | Capability equality, hashing, serialization, and cross-process transfer are absent unless a provider-defined trait supplies an authenticated representation and import validation. |
| `EFF-0014` | Audit events disclose capability identity and operation metadata only to the extent declared by the audit capability; secret payloads are not automatically logged. |
| `EFF-0015` | Handlers are one-shot by default. A continuation can resume at most once and is affine. |
| `EFF-0016` | A multi-shot handler requires the effect declaration to be `multishot`, every captured continuation value to be clonable or persistent, and the `alloc` effect unless statically eliminated. |
| `EFF-0017` | Handler invocation preserves ownership, cancellation, and capability restrictions. Resumption cannot outlive captured borrows or detach child tasks. |
| `EFF-0018` | Deterministic code excludes unrecorded nondeterministic effects. Providers may satisfy such effects with explicit replay streams whose identity is recorded. |

## 4. Capability authenticity boundaries

Across a process, plugin, FFI, or distributed boundary, a capability is transferred only through a profile-defined authenticated handle exchange. Receiving bytes that resemble a handle does not create authority. Imports validate issuer, audience, rights, freshness, revocation epoch, and target resource binding.

## 5. Standard capability families

Edition 1 standard profiles define narrowed capabilities for files/directories, sockets/endpoints, process creation, console, monotonic and wall clocks, secure and pseudo randomness, environment variables, devices/MMIO/DMA, persistence transactions, accelerators, dynamic loading, reflection, audit, supervisors, and build inputs.


---

# OMNI-EVAL: Dynamic Evaluation Semantics

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-MACHINE`, `OMNI-TYPES`, `OMNI-OWN`, `OMNI-EFFECTS` |
| Date | `2026-08-04` |

## 1. Scope

Expression and statement operational semantics, places/values, calls, patterns, loops, closures, temporaries.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Dynamic judgment


- `Γ` is the lexical typing and name environment.
- `Ω` is the ownership and initialization state.
- `Ε` is the available effect/capability environment.
- `Σ` is the abstract store, including allocation identities and object lifetimes.
- `Μ` is the concurrent memory event graph.
- `Γ; Ω; Ε ⊢ e : T ! ε ⇒ Ω′` means expression `e` has type `T`, may perform effect row `ε`, and transforms ownership state `Ω` to `Ω′`.
- `⟨e, Σ, κ⟩ → ⟨e′, Σ′, κ′⟩` is one dynamic evaluation step.
- `hb` denotes happens-before; `sw` denotes synchronizes-with; `mo` denotes per-atomic modification order.


The evaluation relation is deterministic for a single task given the same store, provider responses, and explicit nondeterministic choices. Concurrent interleavings are constrained by OMNI-CONC and OMNI-MEM.

| Rule | Requirement |
|---|---|
| `EVAL-0001` | Operands, arguments, receiver expressions, aggregate fields, array elements, interpolation expressions, match guards, and chained postfix operations evaluate left-to-right. |
| `EVAL-0002` | `&&` and `||` evaluate the right operand only when required. `??` evaluates the right operand only when the left optional/result form is absent as defined by its trait. |
| `EVAL-0003` | A function call evaluates the callee, receiver if any, and arguments left-to-right; creates parameter bindings left-to-right; then activates any reserved receiver loan and enters the body. |
| `EVAL-0004` | Named arguments are reordered to parameter positions only after their source expressions have evaluated in source order. |
| `EVAL-0005` | An assignment first evaluates the left expression to a place without reading its old value, then evaluates the right expression, drops the old initialized destination value, and stores the new value. |
| `EVAL-0006` | A compound assignment evaluates the left place exactly once, reserves the required mutable access, evaluates the right operand, performs the trait operation, and writes/commits according to that trait contract. |
| `EVAL-0007` | A cast evaluates its operand once. Checked casts return `Option`; trapping casts panic with `ConversionFault`; bit casts follow OMNI-NUM and OMNI-UNSAFE validity rules. |
| `EVAL-0008` | Field and index access evaluate the base before the selector/index. Bounds are checked before producing a reference or reading/writing the element. |
| `EVAL-0009` | A block executes statements in source order. Its final un-terminated expression is the block value; otherwise its value is unit. |
| `EVAL-0010` | An `if` evaluates its condition once and then exactly one branch. Conditions require `bool`. |
| `EVAL-0011` | A `match` evaluates the scrutinee once into a temporary place, tests structural alternatives in source order semantics, evaluates a guard only after its pattern binds successfully, and commits moves only for the selected arm. |
| `EVAL-0012` | Or-pattern alternatives must bind the same names with the same types and binding modes. Failed alternatives leave the scrutinee unchanged. |
| `EVAL-0013` | A `loop` yields the value of a matching labeled `break`; all reachable breaks for a value-producing loop must coerce to one type. |
| `EVAL-0014` | `while` evaluates its condition before each iteration. `for` invokes `IntoIterator.into_iter` once and repeatedly calls `next` in source-defined sequence. |
| `EVAL-0015` | `continue` runs cleanup for scopes exited within the current iteration and begins the next iteration. `break` and `return` evaluate their value before cleanup. |
| `EVAL-0016` | `defer` registers a cleanup after its registration expression succeeds. Cleanups run in reverse registration order on normal transfer and supported unwind. |
| `EVAL-0017` | `async defer` may suspend only in an async scope and runs during asynchronous scope cleanup subject to cancellation masking bounds. |
| `EVAL-0018` | A closure capture mode is the least powerful mode required by its body: shared borrow, mutable borrow, or move. `move` forces capture by value. Capture expressions evaluate when the closure is created. |
| `EVAL-0019` | Closure parameters bind at invocation. A closure value is callable according to whether captures permit repeated shared calls, repeated mutable calls, or a single consuming call. |
| `EVAL-0020` | An async block constructs a lazy future and does not execute its body until polled. Captures occur at construction; body locals initialize on first poll as reached. |
| `EVAL-0021` | The postfix `?` performs the standard `Try` branch operation, returning the residual through the enclosing compatible function/try block after cleanup. |
| `EVAL-0022` | Panic, cancellation, and faults interrupt ordinary evaluation only at the exact operations defined by OMNI-ERROR, OMNI-CONC, or target/profile rules. |

## 4. Temporary scopes

A temporary normally lives until the end of the smallest enclosing full expression. A temporary borrowed by a grammar-defined `let` binding may be materialized as a hidden binding lasting to the end of the lexical scope when the binding pattern directly stores that borrow. No other lifetime extension occurs. The compiler SHALL display hidden bindings in expanded MIR and diagnostics.

## 5. Place evaluation

Place expressions include locals, statics, dereferences, field projections, indexing projections, and compiler-validated downcasts. Evaluating a place may fault for null/invalid raw pointers, bounds, alignment, capability, or device access only when the specific projection operation requires it.

## 6. Method calls

Method lookup follows OMNI-NAMES. Operator syntax maps to sealed or imported operator traits. Desugaring preserves evaluation order, borrow reservation/activation, effects, and diagnostics.


---

# OMNI-ERROR: Errors, Faults, Panic, Unwind, Cancellation

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-EVAL`, `OMNI-OWN`, `OMNI-EFFECTS` |
| Date | `2026-08-04` |

## 1. Scope

Result/Option, fault taxonomy, cleanup, double-fault behavior, OOM, stack exhaustion, cancellation.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Fault taxonomy

Defined fault categories are `ArithmeticFault`, `BoundsFault`, `ConversionFault`, `ContractFault`, `CapabilityFault`, `StackFault`, `AlignmentFault`, `InvalidValueFault`, `TargetFeatureFault`, `CancellationFault` (for adaptation only), and profile-specific device/persistence faults. Faulting operations either return a typed result by API contract or initiate panic/isolation/abort by artifact policy.

| Rule | Requirement |
|---|---|
| `ERR-0001` | Expected failure is represented by `Result<T,E>`; absence by `Option<T>`. Unchecked exceptions are not part of ordinary APIs. |
| `ERR-0002` | A panic payload is a typed `PanicInfo` containing stable fault category, optional static message ID, source location, cause chain, and profile-controlled backtrace token. |
| `ERR-0003` | Catchability is explicit: `catch panic` requires the `unwind` effect and catches only Omni panics within an unwind-enabled artifact, not aborts, stack faults, hardware traps, or foreign exceptions unless adapted. |
| `ERR-0004` | Artifact panic policies are `abort`, `unwind`, or `isolate`. The policy and unwind ABI are fingerprinted. |
| `ERR-0005` | On unwind, initialized locals and registered defers are cleaned in reverse order subject to destructor rules. A destructor that panics while another panic/unwind is active causes immediate artifact-policy abort or isolate termination; the second panic is recorded. |
| `ERR-0006` | Destructors are not permitted to return errors. Fallible cleanup is an explicit method called before destruction. |
| `ERR-0007` | Unwind-safe types are those whose invariants remain valid if a protected operation unwinds. Safe catch APIs require `UnwindSafe` or an explicit assertion wrapper. |
| `ERR-0008` | Poisoning of locks/cells is a library contract, not automatic language behavior. Standard mutexes mark poison when a panic exits a locked critical section and allow explicit recovery. |
| `ERR-0009` | Primitive allocation is fallible and returns `Result<..., AllocError>`. Infallible wrappers invoke the artifact OOM policy only after requested cleanup/retry hooks return failure. |
| `ERR-0010` | OOM policy is `abort`, `panic`, or `isolate`; panic/isolate OOM handling uses an emergency reserve and cannot attempt unbounded allocation. If reserve is unavailable, abort is permitted. |
| `ERR-0011` | Stack exhaustion produces `StackFault` only on targets with reliable guard/detection and a safe emergency stack; otherwise the target supplement mandates immediate abort. Recovery cannot resume the exhausted frame. |
| `ERR-0012` | Cancellation is a distinct control signal. It is observed only at declared cancellation points unless a target/profile explicitly defines asynchronous cancellation for unsafe compartments. |
| `ERR-0013` | When success/error/panic/cancellation race, the first event atomically committed by the operation protocol wins; later events become suppressed causes or are ignored according to that protocol. |
| `ERR-0014` | A completed successful result cannot be replaced by cancellation after its completion commit. Cancellation requested before commit may win at the next cancellation point. |
| `ERR-0015` | Cleanup runs under a bounded cancellation mask. A cleanup that exceeds its profile limit triggers the scope policy and may escalate to isolate/abort in realtime profiles. |
| `ERR-0016` | `isolate` terminates the current task/actor compartment, cancels children, runs only isolation-safe cleanups, revokes compartment capabilities where supported, and reports to the supervisor. |
| `ERR-0017` | Panic never crosses an ABI boundary lacking an explicit compatible panic protocol; the boundary converts, traps, isolates, or aborts as declared. |
| `ERR-0018` | Backtrace addresses are not stable semantic observations; stable frame identities are symbol/source IDs exposed only when the debug profile is enabled. |

## 4. Cleanup precedence

For any control transfer, the transfer value/cause is first committed to a hidden slot, then cleanups execute. Cleanup panic supersedes ordinary success/error/cancellation and chains the prior cause. A second panic during active unwind invokes the double-panic rule. Resource failure during cleanup is handled by the cleanup API’s declared result or escalates through the artifact policy.

## 5. Contracts

Failed runtime `require`, `ensure`, invariant, or resource contracts produce `ContractFault` unless the contract explicitly returns a typed validation error. Statically proved contracts emit no runtime check. Trusted assumptions are allowed only at unsafe/FFI boundaries and create named obligations.


---

# OMNI-CONST: Compile-Time Evaluation, Macros, and Reflection

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-GRAMMAR`, `OMNI-EVAL`, `OMNI-ERROR` |
| Date | `2026-08-04` |

## 1. Scope

Const evaluator, macro phases/hygiene, reflection, generated code, determinism, quotas, diagnostics.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Const eligibility

A function is const-callable when declared `const fn`, all reached operations are const-supported, its effect row is empty except permitted compile-time allocation/panic, and every called function is const-callable. Termination is enforced by a deterministic fuel and recursion-depth budget, not assumed.

| Rule | Requirement |
|---|---|
| `CONST-0001` | Const evaluation uses the same value, arithmetic, ownership, pattern, panic, and pure call semantics as runtime, with deterministic quotas and no undeclared external effects. |
| `CONST-0002` | Const evaluation may allocate immutable interpreter objects. Their addresses are abstract and cannot be converted to stable integers or compared across independent evaluations. |
| `CONST-0003` | A const pointer may refer only to a live const allocation or static object permitted by its type. Relocation records preserve provenance into the emitted artifact. |
| `CONST-0004` | Const panic rejects the containing constant/item with a required diagnostic; it does not emit a runtime panic unless the source explicitly requests deferred checking. |
| `CONST-0005` | Macro phases are: lexical token macros, item/declaration macros, target/profile filtering, import resolution, typed derive macros, and expression/type macros at their grammar positions. |
| `CONST-0006` | Macro input and output are token trees or typed semantic objects according to the declared phase. Textual preprocessing and arbitrary source concatenation are prohibited. |
| `CONST-0007` | Hygiene assigns syntax-context marks to introduced identifiers. Introduced names resolve at definition context; passed-through names retain call-site context. |
| `CONST-0008` | Deliberate capture uses explicit `capture(callsite, name)` or `capture(defsite, name)` APIs and is visible in expansion output. |
| `CONST-0009` | Generated identifiers carry source span, macro invocation, definition, expansion index, and stable generation key. |
| `CONST-0010` | Macro expansion is deterministic, cycle-checked, and limited by declared token, depth, time-instruction, and memory quotas. Exceeding a quota is a compile-time diagnostic. |
| `CONST-0011` | Compile-time file/resource access requires a declared build capability and content digest. Directory enumeration is sorted and captured as an input. |
| `CONST-0012` | Compile-time network, wall clock, ambient environment, process creation, and unseeded randomness are forbidden in reproducible mode. |
| `CONST-0013` | Reflection sees only declarations visible at the reflection site plus explicitly exported metadata. Private members cannot be reflected across package boundaries without a capability/attribute. |
| `CONST-0014` | Runtime reflection metadata is linked only for types/items reachable from an explicit `reflect` root or dynamic boundary. |
| `CONST-0015` | Compile-time and runtime type identities use the same canonical signature fingerprint where runtime identity is opted in. |

## 4. Macro declaration contract

A macro declares accepted fragment kinds, output fragment kind, phase, determinism class, required build capabilities, resource limits, and edition. Expansion output is parsed/validated under the same edition and cannot bypass static semantics.

## 5. Reflection contract

Compile-time reflection returns immutable descriptors, not mutable compiler internals. Descriptor schemas are versioned. Layout is visible only for an explicit `repr`, target-specific query, or after layout finalization; querying layout makes the target/layout input part of the build key.


---

# OMNI-UNSAFE: Unsafe Semantics and Obligations

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-EVAL`, `OMNI-OWN`, `OMNI-MEM` |
| Date | `2026-08-04` |

## 1. Scope

Unsafe operations, obligation language, invalid-execution scope, raw memory, MMIO, DMA, inline assembly.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Unsafe obligation model

An obligation is `O = (id, operation, precondition, scope, assumptions, containment)`. The compiler emits obligation IDs into MIR and optional object metadata. Safe wrapper documentation SHALL list every obligation it discharges and the invariant used.

| Rule | Requirement |
|---|---|
| `UNSAFE-0001` | `unsafe` permits performing operations with programmer-proved preconditions; it does not disable ordinary typing, ownership, effect, capability, initialization, or control-flow checks. |
| `UNSAFE-0002` | Unsafe operations are legal only inside an `unsafe` block/function or through an unsafe trait implementation, and their unsafe effect is visible in MIR and audit reports. |
| `UNSAFE-0003` | Calling an unsafe function requires an unsafe context even when the caller can prove its preconditions; safe wrappers discharge and document the proof. |
| `UNSAFE-0004` | Each unsafe operation has a named obligation record with precondition, protected invariant, permitted optimizer assumptions, and failure containment class. |
| `UNSAFE-0005` | If an unsafe precondition is false at event `e`, that execution becomes invalid from `e` onward. Observations before `e` remain required. Other executions and paths not reaching `e` retain full semantics. |
| `UNSAFE-0006` | An optimizer may assume an unsafe precondition only on paths that reach the operation and only while the facts/objects on which it depends remain live. Assumption provenance SHALL point to the obligation. |
| `UNSAFE-0007` | Invalid unsafe execution does not authorize compile-time compromise, cross-process effects, or modification of immutable artifacts; such outcomes are outside the language model and addressed by hardening. |
| `UNSAFE-0008` | Raw allocation requires `(size, alignment, address-space, allocator-family)` and returns a provenance-bearing allocation handle or failure. Zero-size allocations use a non-dereferenceable aligned sentinel or a declared unique object. |
| `UNSAFE-0009` | Deallocation requires the original live allocation identity, compatible allocator family, exact or accepted layout, no live derived references, and no outstanding device/foreign ownership. |
| `UNSAFE-0010` | Raw dereference requires live allocation provenance, in-bounds access, alignment unless using unaligned operations, valid address space, permissions, and a valid value for typed reads. |
| `UNSAFE-0011` | Inline assembly declares target, feature predicates, inputs, outputs, late outputs, clobbered registers, flags, memory regions, stack behavior, unwind behavior, control-flow successors, and volatility. |
| `UNSAFE-0012` | Assembly marked `pure`/`nomem`/`readonly` is verified syntactically where possible and remains an unsafe promise. Omitted memory clobbers cannot be inferred. |
| `UNSAFE-0013` | Assembly may branch only to declared labels, may unwind only with an explicit compatible unwind contract, and may not silently alter stack pointer, capability registers, TLS, or reserved runtime registers. |
| `UNSAFE-0014` | MMIO values use volatile device operations at declared widths and endianness. Ordinary references to MMIO storage are prohibited. |
| `UNSAFE-0015` | DMA registration pins or otherwise stabilizes buffers, establishes device-visible address mapping, records direction/coherence, and transfers a typed DMA lease. CPU access while leased follows the lease contract. |
| `UNSAFE-0016` | Device reset/revocation invalidates DMA leases atomically from the provider perspective; cleanup must tolerate completion/reset races without reusing storage early. |

## 4. Unchecked and checked unsafe modes

`unsafe.checked` retains runtime validation/sanitizers where available and converts violations to a defined trap before memory corruption. `unsafe.unchecked` permits omission of those checks using the obligation assumptions. Hardened and high-assurance profiles may prohibit unchecked unsafe or require compartment containment.

## 5. Transmutation

Typed transmutation is allowed only when source and destination have equal size, compatible alignment/storage, every destination validity invariant is proved, provenance/ownership is preserved, and no private layout is assumed without explicit `repr`. Otherwise conversion proceeds through bytes plus validation.


---

# OMNI-CONC: Concurrency and Async Semantics

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-EVAL`, `OMNI-ERROR`, `OMNI-OWN`, `OMNI-EFFECTS` |
| Date | `2026-08-04` |

## 1. Scope

Threads, tasks, scopes, actors, channels, blocking, progress, fairness, TLS, signals/interrupt interaction.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Structured task state

A child state is `created`, `runnable`, `waiting`, `completed(value)`, `failed(error)`, `panicked(info)`, or `cancelled`. Terminal transition is atomic. A scope maintains spawn order and completion order as separate sequences.

| Rule | Requirement |
|---|---|
| `CONC-0001` | Native threads, scoped tasks, async tasks, actors, and interrupts are distinct execution agents with profile-defined capabilities and memory participation. |
| `CONC-0002` | Thread/task creation publishes all moved arguments and initialized captured state to the child start; child completion synchronizes with successful join. |
| `CONC-0003` | A structured task scope cannot complete until every child is joined, cancelled and joined, or transferred to an authorized long-lived supervisor. |
| `CONC-0004` | Standard scope policies are `all`, `cancel_on_error`, `first_success`, `race`, `supervise`, and `quorum(k)` with the exact aggregation rules below. |
| `CONC-0005` | `all` waits for all children and returns ordered results by spawn order. `cancel_on_error` commits the first error/panic by completion order, requests cancellation of unfinished siblings, joins them, and returns the committed cause with suppressed causes. |
| `CONC-0006` | `first_success` returns the first committed success, cancels and joins others; if none succeeds, it aggregates terminal causes by completion order. `race` returns the first committed terminal outcome. |
| `CONC-0007` | `quorum(k)` commits once `k` successes exist, returns them in completion order, and cancels/joins remaining children; impossibility to reach `k` returns aggregated failure. |
| `CONC-0008` | Detach requires a supervisor/service capability and moves all captures. Detached work becomes a child of that supervisor, never an orphan. |
| `CONC-0009` | Bounded channels have capacity `N`; send commits when an element enters the buffer or is handed directly to a receiver. Receive commits when ownership of one element transfers to the receiver. |
| `CONC-0010` | Closing a channel prevents new successful sends; buffered elements remain receivable. After the buffer empties, receive returns `Closed`. Concurrent close/send is decided by the operation’s atomic commit order. |
| `CONC-0011` | Channel ordering is FIFO per sender and globally FIFO by send commit order for a single channel. Fairness is not guaranteed unless the channel type/profile states it. |
| `CONC-0012` | Cancellation before a channel operation commits leaves no element transferred. Cancellation after commit returns/records the committed result and cannot roll it back. |
| `CONC-0013` | Standard mutex lock acquisition may be unfair and may wake spuriously only through condition-variable waits, not lock success. Fair mutexes are separate types. |
| `CONC-0014` | Condition-variable wait atomically releases the mutex and blocks, may wake spuriously, then reacquires before returning. Callers must test predicates in a loop. |
| `CONC-0015` | Semaphores commit permit decrement/increment atomically; cancellation before acquire commit consumes no permit. |
| `CONC-0016` | Standard mutexes poison after panic exits a held critical section. Poison is advisory and recoverable through an explicit API; it is not memory unsafety. |
| `CONC-0017` | An executor SHALL not poll one task concurrently with itself, SHALL tolerate duplicate/coalesced wakes, and SHALL not poll after terminal completion. |
| `CONC-0018` | Polling a future is non-reentrant unless the future explicitly implements a reentrant protocol. A wake during poll schedules a later poll and does not recursively enter the task. |
| `CONC-0019` | Blocking operations in an async context require the `block` effect and a provider that routes them to a blocking resource or rejects them. |
| `CONC-0020` | Actor mailboxes preserve message commit order per sender; cross-sender order is by mailbox commit. A restart never rolls back external effects already committed. |
| `CONC-0021` | Region transfer in a message commits ownership atomically with enqueue. On enqueue failure ownership remains with the sender; on committed delivery failure the supervisor policy owns disposal/retry. |
| `CONC-0022` | Thread-local values initialize per native thread, are inaccessible from tasks migrated between threads unless task-local storage is used, and drop at orderly thread exit only. |
| `CONC-0023` | Signals/interrupts may access only async-signal/interrupt-safe operations declared by the target/profile. Ordinary locks, allocation, and destructors are prohibited unless explicitly supported. |

## 4. Progress classifications

APIs declare one of: blocking, obstruction-free, lock-free, wait-free, bounded-wait, or realtime-bounded. Absence of a declaration means only safety and eventual response under provider progress; fairness is not implied.

## 5. Async lowering

An async body lowers to a pinned state machine containing live-across-suspension locals and cleanup states. Every suspension point checks borrow/pin validity. Dropping the future begins cancellation cleanup and cannot silently detach child scopes.

## 6. Deterministic concurrency

The deterministic profile permits disjoint parallel mutation, deterministic reductions, explicitly ordered channels, recorded arbitration, and replayed external events. General scheduler-dependent shared-memory outcomes require the `nondeterministic` effect and are rejected otherwise.


---

# OMNI-MEM: Memory, Atomics, and Provenance Model

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-MACHINE`, `OMNI-OWN`, `OMNI-CONC` |
| Date | `2026-08-04` |

## 1. Scope

Allocation objects, validity, provenance, atomics, happens-before, races, tearing, volatile/device/persistent memory.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Event model

Memory events are reads, writes, read-modify-writes, fences, lock/unlock, spawn/start, completion/join, channel transfer, cancellation publication, volatile/device operations, and persistent flush/commit. Relations include sequenced-before (`sb`), synchronizes-with (`sw`), happens-before (`hb = (sb ∪ sw)+`), modification order (`mo`), reads-from (`rf`), and from-read (`fr`).

| Rule | Requirement |
|---|---|
| `MEM-0001` | An allocation has a unique allocation identity, address space, size, alignment, lifetime interval, permissions, and storage bytes. Reuse creates a new identity even at the same address. |
| `MEM-0002` | Object lifetime begins only after storage is allocated and initialization establishes a valid value; it ends before destruction releases/reuses storage. `MaybeUninit` storage has allocation lifetime but no `T` object lifetime until initialized. |
| `MEM-0003` | A pointer carries `(allocation-id or exposed-origin, address-space, offset/address, bounds, permissions, provenance-state)` abstractly; target representations may encode less but must preserve semantics. |
| `MEM-0004` | Derived pointers retain provenance and may narrow bounds/permissions. Pointer arithmetic is valid within the allocation and may form one-past; one-past cannot be dereferenced. |
| `MEM-0005` | Pointer subtraction/order is defined only for pointers into the same live allocation (including one-past) and compatible element type/address space. Equality has a named address-only form and a provenance-aware identity form. |
| `MEM-0006` | `expose_addr` emits an integer address and marks provenance as exposed. `from_exposed_addr` may recover dereference authority only if a live exposed allocation in the address space contains the address and policy permits recovery; ambiguity is an unsafe error unless an allocation handle is supplied. |
| `MEM-0007` | Integer casts that do not use exposure/recovery APIs do not create dereferenceable provenance. |
| `MEM-0008` | A data race occurs when two non-atomic conflicting memory events from different agents are not ordered by happens-before, at least one writes, and the accesses overlap in storage. Volatile does not make an access atomic. |
| `MEM-0009` | Unsafe, foreign, signal, interrupt, and device agents participate in race analysis according to their declared event contracts. A violated synchronization contract creates invalid unsafe execution. |
| `MEM-0010` | Concurrent mixed-size overlapping accesses are races unless every access is atomic and the target supplement defines the combination. Atomic tearing is forbidden for supported atomic widths/alignment. |
| `MEM-0011` | Each atomic object has one modification order. Reads select a write allowed by the ordering axioms; compare-exchange has separate success/failure ordering and failure cannot be release or acquire-release. |
| `MEM-0012` | Release stores and successful release operations synchronize with acquire reads/RMWs that read from their release sequence. Fences synchronize only through the specified atomic communication pattern. |
| `MEM-0013` | Sequentially consistent operations participate in one total order consistent with happens-before and modification order. |
| `MEM-0014` | A data-race-free safe program using only SC atomics has sequentially consistent abstract behavior. Relaxed atomics expose only outcomes permitted by this model. |
| `MEM-0015` | Atomic supported widths, alignments, and lock-free status are target-manifest facts. Unsupported widths use a conforming library lock or are rejected by a lock-free requirement. |
| `MEM-0016` | Ordinary shared immutable memory is safe after publication. Shared mutable memory requires atomics or synchronization abstractions. |
| `MEM-0017` | Volatile/device accesses occur exactly once at their abstract operation, are not merged, invented, or reordered across declared device barriers, and may produce target/device faults. |
| `MEM-0018` | Persistent durability order is separate from coherence. Stores become durable only after profile-defined flush and drain/commit operations; crash observations include only durability-committed records. |
| `MEM-0019` | Address spaces are nominal. Pointers from distinct address spaces do not convert implicitly and may have different width, provenance, and accessibility. |
| `MEM-0020` | Capability-pointer targets preserve bounds/permissions/tag validity; copying raw bytes does not necessarily copy a valid capability tag. Tagged-memory behavior is declared by target supplement. |
| `MEM-0021` | Safe code cannot observe uninitialized bytes, padding, invalid discriminants, dangling provenance, or IR poison. Raw byte views require validation/unsafe contracts. |

## 4. Atomic order semantics

- `relaxed`: atomicity and modification order only;
- `acquire`: prevents later ordinary/atomic events from moving before and imports writes from the release sequence read;
- `release`: publishes prior events to an acquiring reader of its release sequence;
- `acq_rel`: both for successful read-modify-write;
- `seq_cst`: acq_rel/acquire/release as appropriate plus membership in the SC total order.

Consume ordering is not part of Edition 1.

## 5. DRF-SC scope

The DRF-SC guarantee covers safe ordinary memory and synchronization primitives whose unsafe internals meet this model. It excludes explicit relaxed outcomes, foreign/device contracts that declare weaker ordering, invalid unsafe executions, and persistent durability observations.

## 6. Hardware mapping

Target supplements SHALL provide litmus-tested mappings to x86-TSO, AArch64, RVWMO, and other claimed models. A backend may strengthen ordering but may not weaken it.


---

# OMNI-FFI: Foreign Function and Memory Interfaces

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-TYPES`, `OMNI-OWN`, `OMNI-ERROR`, `OMNI-UNSAFE`, `OMNI-MEM` |
| Date | `2026-08-04` |

## 1. Scope

Foreign values, calls, callbacks, ownership, validation, varargs, exceptions, longjmp, TLS, foreign threads.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Boundary principle

The FFI boundary is a validation and ownership transition, not a magical exemption. Raw foreign representation may be received into raw/`MaybeUninit` forms; constructing a safe Omni value requires validation before any safe operation observes it.

| Rule | Requirement |
|---|---|
| `FFI-0001` | An FFI declaration names an ABI string and exact foreign type representation. Unknown ABI strings are errors in strict mode. |
| `FFI-0002` | Foreign scalar values entering safe Omni types are validated: `bool` is 0/1, `char` is a Unicode scalar, references satisfy non-null/alignment/liveness/provenance, and enum tags are valid. |
| `FFI-0003` | Nullable foreign pointers map to raw/optional pointer representations, never directly to non-null safe references without validation. |
| `FFI-0004` | Foreign aggregates use explicit `repr(C)`, `repr(omni_v1)`, or schema representations. Padding bytes are not read for equality/hash/serialization and may be uninitialized only behind raw storage types. |
| `FFI-0005` | Varargs are permitted only for target ABIs with a supplement and require each variadic argument to use the ABI-promoted foreign type explicitly. Safe generic values cannot be passed directly. |
| `FFI-0006` | Callbacks expose an ABI function pointer plus context handle and lifetime/ownership protocol. The foreign side may call only during the declared interval unless it owns a transferred callback object. |
| `FFI-0007` | A foreign thread calling Omni shall attach through the runtime ABI before touching Omni TLS, managed domains, panic, async, or stack-map facilities, and detach after all Omni frames/resources are gone. |
| `FFI-0008` | Foreign exceptions, SEH, Objective-C exceptions, and `longjmp` shall not cross Omni frames unless a target supplement defines a compatible adapter. Default behavior is boundary trap/abort or conversion by wrapper. |
| `FFI-0009` | `setjmp`/`longjmp` across live Omni owned values is prohibited because it skips destruction; wrappers may isolate the jump within a foreign frame and convert the result. |
| `FFI-0010` | POSIX signals and hardware exceptions use target-defined adapters and cannot call arbitrary Omni code. |
| `FFI-0011` | Panic cannot cross a foreign boundary without an explicit panic ABI. Callback wrappers catch/convert/isolate/abort according to the declaration. |
| `FFI-0012` | Ownership transfer annotations are `borrowed`, `borrowed_mut`, `consumed`, `returned_owned`, `shared_retain`, or custom handle protocol. Every boundary value has one annotation. |
| `FFI-0013` | Memory allocated by one allocator family may be freed by another only through an explicit compatibility contract. Stable buffers carry a deallocator callback/allocator handle when ownership crosses modules. |
| `FFI-0014` | Foreign memory imported as a safe slice/reference requires validated length, alignment, lifetime token, mutability/exclusivity, address-space accessibility, and provenance handle. |
| `FFI-0015` | Managed objects cross FFI only by pinned reference, stable handle, or copy. A raw pointer into movable storage cannot survive a safepoint. |
| `FFI-0016` | FFI calls carry the `foreign` effect plus every declared external effect. Foreign purity/noexcept/read-only attributes are unsafe promises subject to validation/audit. |

## 4. ABI declaration example

```omni
extern "C" {
    #[ffi(ownership="borrowed", null="forbidden")]
    fn write(fd: c_int, data: *const byte, len: usize) -> isize !{foreign, io};
}
```

## 5. Foreign callback lifetime

A callback token is affine. Revocation waits for or prevents new entries according to its synchronization protocol. Destroying the token before the foreign side relinquishes callback authority is an unsafe contract violation.


---

# OMNI-IR: Normative Semantic IRs

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `toolchain-required` |
| Dependencies | `OMNI-EVAL`, `OMNI-ERROR`, `OMNI-MEM` |
| Date | `2026-08-04` |

## 1. Scope

HIR/TIR/OIR/MIR/LIR schemas, verifiers, semantics, serialization, source/proof provenance.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Authority and levels

| Rule | Requirement |
|---|---|
| `IR-0001` | Source-language semantics are primary. Normative IRs are executable elaborations that SHALL be proven or differentially cross-checked against source rules and cannot redefine them. |
| `IR-0002` | Each IR has a versioned schema, verifier, type/effect/ownership invariants, source mapping, and feature fingerprint. |
| `IR-0003` | HIR contains resolved syntax and desugared constructs while preserving generics and source ownership notation. |
| `IR-0004` | TIR contains fully explicit types, coercions, effects, capabilities, contracts, numeric policy, trait selections, and dynamic boundaries. |
| `IR-0005` | OIR contains moves, copies, borrows, loans, regions, initialization state, drop flags, pinning, suspension liveness, and unsafe obligations. |
| `IR-0006` | MIR is typed SSA/CFG with explicit memory objects, checks/fault edges, cleanup/unwind/cancellation edges, calls, effects, and provenance operations. |
| `IR-0007` | Domain IRs preserve affine loops, vectors, tensors, sparse layouts, async machines, storage transactions, protocols, and cryptographic contracts until validated lowering. |
| `IR-0008` | LIR contains explicit layout, address spaces, atomics, fences, ABI calls, stack maps, exception tables, and runtime hooks but remains target-neutral. |
| `IR-0009` | Machine IR contains target instructions, registers, scheduling dependencies, stack frames, relocations, feature predicates, and debug/unwind mapping. |
| `IR-0010` | No IR value originating from safe source may become observable `undef` or poison. Internal invalid states are verifier errors or explicit assumptions with provenance and freeze/validation before observation. |
| `IR-0011` | Uninitialized storage is represented by a storage state, not an arbitrary typed value. Loads require initialized bytes/value validity or an unsafe raw operation. |
| `IR-0012` | IR serialization uses canonical CBOR-like schema rules defined in `schemas/ir-module.schema.json`, content hashes, and forward-compatible unknown-field rejection for normative fields. |
| `IR-0013` | Every source-to-IR elaboration rule records the source rule IDs it implements. The reference evaluator executes MIR only after verification. |
| `IR-0014` | Optimizer passes consume and emit verified IR; verifier failure is a compiler defect and no artifact is emitted. |

## 4. MIR core operations

MIR defines constants, aggregates, projections, checked arithmetic, comparisons, casts, allocation/deallocation, lifetime start/end, load/store, borrow/reborrow/end-loan, move/copy/drop, call/invoke, branch/switch, panic/fault, spawn/join/channel, atomic/fence, volatile/device, capability invoke, and return/abort. Every operation has operand types, effects, preconditions, normal successors, and exceptional successors.

## 5. MIR reference execution

The reference engine interprets verified MIR with the OMNI-MACHINE store and OMNI-MEM event graph. It is intentionally unoptimized and deterministic given explicit external/scheduler inputs. Source-vs-MIR differential cases are normative conformance evidence.

## 6. Preservation obligations

For each lowering `L`, accepted source state `s`, and generated IR `i=L(s)`, every permitted IR observation is a permitted source observation, and every required source termination/fault is represented. Aggressive lowerings may refine nondeterminism but cannot introduce disallowed observations.


---

# OMNI-OPT: Optimization Legality and Validation

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `toolchain-required` |
| Dependencies | `OMNI-IR`, `OMNI-MACHINE`, `OMNI-MEM` |
| Date | `2026-08-04` |

## 1. Scope

Permitted transformations, observable equivalence, assumptions, translation validation, proof certificates.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Observational refinement

For original program `P` and transformed program `P′`, legality requires `Obs(P′, I) ⊆ Obs(P, I)` for every conforming input/environment `I`, and equality where the source requires a unique observation. Refinement may choose among explicitly unspecified outcomes but cannot choose an outcome the source forbids.

| Rule | Requirement |
|---|---|
| `OPT-0001` | An optimization is legal only when the transformed artifact refines or equals the source permitted observation set under the selected profiles and target facts. |
| `OPT-0002` | Potential observations include panic/fault occurrence and ordering, allocation failure when allocation is semantically observable, destructor effects, I/O/capability operations, atomics/synchronization, volatile/device events, persistent commits, cancellation points, and numeric results under policy. |
| `OPT-0003` | Allocations may be removed when identity/address/OOM/destructor/trace effects are unobservable by contract. The optimizer cannot assume allocation always succeeds unless a proof or infallible policy permits it. |
| `OPT-0004` | Checks may be removed only when proved redundant from types, refinements, dominating checks, target guarantees, or explicit unsafe obligations. |
| `OPT-0005` | Reassociation, contraction, vectorization, and reduction reordering obey the lexical numeric policy and determinism profile. |
| `OPT-0006` | Concurrency transformations preserve happens-before, atomic modification order constraints, data-race freedom, cancellation/suspension points declared observable, and progress contracts. |
| `OPT-0007` | Assumptions carry origin rule/obligation, operands, scope, dominance, object lifetime, invalidation events, and proof status. Stale assumptions are verifier errors. |
| `OPT-0008` | Profile data affects profitability, layout, and dispatch but never semantic legality. Its source/IR hash, collection target, counters, merge algorithm, and age are recorded. |
| `OPT-0009` | Autotuning/ML-guided choices use a deterministic pinned model/seed in reproducible mode, have a deterministic fallback, and submit selected schedules to the same legality validator. |
| `OPT-0010` | Translation-validation classes are `proved`, `validated-complete`, `validated-bounded`, `differential-only`, and `unvalidated`. |
| `OPT-0011` | Verified/high-assurance builds reject `unvalidated` transformations and may restrict bounded validators to proved domains. Balanced builds report coverage and use conservative fallback when validation is unsupported or times out. |
| `OPT-0012` | Validation failure rejects the transformed result and reruns a conservative pipeline; repeated mismatch is a compiler defect and blocks release qualification. |
| `OPT-0013` | Post-link optimization preserves symbols, unwind/debug semantics required by profile, feature dispatch, control-flow integrity, and relocation/loader contracts. |

## 4. Optimization reports

The compiler emits machine-readable remarks for inlining, devirtualization, check elimination/retention, allocation/copy/retain/drop, vectorization, loop transforms, dispatch, code layout, profile use, and validation status. Remarks cite source and rule IDs.

## 5. No release-mode semantic changes

Optimization level cannot change overflow, bounds, aliasing, panic, race, cancellation, floating policy, or unsafe preconditions. Separate source/profile annotations choose alternate semantics.


---

# OMNI-TARGET: Target Identity and Feature Registry

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `platform-required` |
| Dependencies | `OMNI-MACHINE`, `OMNI-RULES` |
| Date | `2026-08-04` |

## 1. Scope

Target triples, data models, endianness, address spaces, ISA/OS features, deployment policy, multiversioning.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Target triple grammar

```text
arch        = x86_64 | aarch64 | riscv64 | registered-name
vendor      = pc | apple | unknown | registered-name
os          = linux | windows | macos | none | registered-name
environment = gnu | musl | msvc | elf | registered-name
abi         = omni_v1 | c | platform-name
data-model  = lp64 | llp64 | ilp32 | cap128 | registered-name
```

| Rule | Requirement |
|---|---|
| `TARGET-0001` | A canonical target identity is `arch-vendor-os-environment-abi[data-model]+feature-set`, with lowercase registered components and sorted feature names. |
| `TARGET-0002` | Aliases resolve through the signed target registry to exactly one canonical identity and cannot depend on the host machine. |
| `TARGET-0003` | Target manifests declare byte order, scalar widths/alignments, pointer widths per address space, maximum object size, stack alignment, atomic widths/lock-freedom, vector model, object format, ABI supplement, unwind/debug formats, and feature detection method. |
| `TARGET-0004` | Baseline feature sets are explicit. A distribution artifact cannot execute an instruction outside its baseline except within a correctly guarded multiversioned body. |
| `TARGET-0005` | Runtime feature detection is trusted only through the OS/firmware mechanism named by the target manifest; unprivileged self-reporting may further narrow but not widen authorized features. |
| `TARGET-0006` | Feature dispatch chooses a body whose required feature set is a subset of detected and deployment-policy-authorized features. Selection is cached with thread-safe initialization. |
| `TARGET-0007` | Target-specific semantics are limited to implementation-defined data layout, ABI/platform facilities, conditional instructions, and classified hardware faults. Core expression meaning remains unchanged. |
| `TARGET-0008` | Unsupported target features produce translation rejection or `TargetFeatureFault` before entering the body; illegal instruction traps are not an acceptable dispatch mechanism in safe hosted code. |
| `TARGET-0009` | Address spaces are registered with width, representation, accessibility, coherence, atomicity, provenance recovery, and conversion rules. |

## 4. Edition 1 reference targets

The suite includes target manifests for x86-64 Linux ELF SysV, x86-64 Windows PE/COFF, AArch64 Linux ELF, AArch64 macOS Mach-O, RV64GC Linux ELF, and freestanding AArch64/RV64 reference images. A target claim is conforming only with its exact supplement and test results.


---

# OMNI-ABI: Common Stable Omni ABI

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `platform-required` |
| Dependencies | `OMNI-TYPES`, `OMNI-OWN`, `OMNI-ERROR`, `OMNI-FFI`, `OMNI-TARGET` |
| Date | `2026-08-04` |

## 1. Scope

Stable data/call ABI, mangling, visibility, version negotiation, ownership, trait objects, async boundary.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Stable data and call ABI

| Rule | Requirement |
|---|---|
| `ABI-0001` | The stable ABI name is `omni_v1`. ABI use requires explicit `extern "omni_v1"` or `#[repr(omni_v1)]`; default native layout/calling is not stable. |
| `ABI-0002` | `omni_v1` supports LP64 little-endian, LLP64 little-endian, and registered capability variants. Endianness is part of ABI identity; cross-endian calls require serialization. |
| `ABI-0003` | Fixed integers use their exact width/alignment. `bool` is one byte 0 or 1. `char` is `u32` containing a Unicode scalar. `usize/isize` follow the data model. |
| `ABI-0004` | Aggregates use declaration order with deterministic padding and target-supplement alignment. Padding is not part of equality/hash/wire data and is zeroed only when the ABI contract requests hardened zero-padding. |
| `ABI-0005` | Enums use a stable explicit tag of the smallest declared tag type plus a union payload aligned to the maximum variant; niche optimization is not used unless `repr(nullable)` explicitly selects the standardized nullable representation. |
| `ABI-0006` | `Option<T>` and `Result<T,E>` use stable tagged layouts by default. Nullable pointer-like Option uses null only under `repr(nullable)` and when `T` has exactly one standardized invalid null representation. |
| `ABI-0007` | A slice/`str` ABI value is `{data: nonnull pointer, len: usize}`; zero length may use the registered aligned dangling sentinel. `str` bytes are valid UTF-8. |
| `ABI-0008` | An owned cross-boundary buffer is `{data, len, capacity, allocator_handle, drop_fn}`. Ordinary `String`/`Vec` private layouts are not ABI-stable unless wrapped by this descriptor. |
| `ABI-0009` | A trait object is `{data, vtable}`. The vtable begins with version, type fingerprint, size, alignment, drop function, and method count followed by methods in trait declaration order including inherited linearization. |
| `ABI-0010` | A closure object is `{env, invoke_fn, drop_fn, clone_fn?}` with call capability encoded by which functions are non-null. |
| `ABI-0011` | A dynamic value is `{type_fingerprint, data, descriptor}` where the descriptor supplies validation, drop, clone, equality/hash if supported, and reflection schema. |
| `ABI-0012` | An async ABI value is a pinned future object `{state, poll_fn, cancel_fn, drop_fn, descriptor}`. Poll uses an explicit context/waker ABI and returns `pending`, `ready`, or `panicked/failed` according to signature. |
| `ABI-0013` | Symbol mangling is `_O1` followed by length-prefixed UTF-8 NFC package/module/item components, kind/signature codes, ABI/profile fingerprint, and a 128-bit BLAKE3-derived collision suffix encoded base32. |
| `ABI-0014` | Demangling is deterministic. A linker detects full canonical-signature collisions even if the suffix collides and rejects the artifact. |
| `ABI-0015` | ABI evolution is additive within `omni_v1` only for reserved fields/vtable tails negotiated by size/version. Breaking layout/call changes require `omni_v2`. |
| `ABI-0016` | Symbol versioning and interface fingerprints allow loaders to reject incompatible libraries before calling code. |
| `ABI-0017` | Panic, ownership, effects, capabilities, target features, numeric policy, and profile requirements crossing an ABI boundary are part of the exported signature descriptor. |

## 4. Calling convention

The common ABI defines logical argument classification. Target supplements map classes `integer`, `float`, `vector`, `aggregate-register`, `indirect`, `capability`, and `sret` to platform registers/stack. Callee/caller-saved state, stack alignment, red zone, unwind records, TLS, and varargs are supplement-specific.

## 5. Vtable compatibility

A consumer requires a vtable major version and minimum byte size. New optional methods append to the tail and have feature bits/default adapters. Reordering or changing existing entries is breaking.

## 6. Async boundary

Cancellation is a request through `cancel_fn`; it does not free the state. The owner continues polling cleanup or invokes `drop_fn` only when the descriptor allows immediate cancellation drop. Wakers are ref-counted ABI objects with wake, wake-by-ref, clone, and drop functions.


---

# OMNI-ABI-*: Target and OS ABI Supplements

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `claim-dependent` |
| Dependencies | `OMNI-ABI`, `OMNI-TARGET`, `OMNI-IR` |
| Date | `2026-08-04` |

## 1. Scope

Calling convention, object format, relocation, TLS, unwind, debug mapping, startup, linker, platform C ABI.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Supplement contract

| Rule | Requirement |
|---|---|
| `ABISUP-0001` | Each supplement pins an external platform ABI version, object format, relocation set, unwind format, debug format, loader behavior, and required OS feature contracts. |
| `ABISUP-0002` | Startup defines process/reset entry, initial stack/register state, TLS setup, capability import, runtime construction, `main` invocation, exit, and abnormal shutdown. |
| `ABISUP-0003` | Orderly shutdown runs process-scope defers and runtime finalizers in reverse dependency order; it does not promise cleanup after `_exit`, abort, reset, power loss, or invalid execution. |
| `ABISUP-0004` | Object/link rules define section names/flags, COMDAT/grouping, weak/common symbols, visibility, interposition policy, LTO container identity, and deterministic resolution. |
| `ABISUP-0005` | Link resolution is independent of archive/member/filesystem enumeration order after canonical symbol ordering. Duplicate strong definitions are errors. |
| `ABISUP-0006` | Unwind tables describe every unwind-enabled frame, including assembly. Crossing a frame without compatible unwind metadata is prohibited. |

## 4. Included supplements

- `OMNI-ABI-X86_64-SYSV-ELF-1`
- `OMNI-ABI-X86_64-WINDOWS-PECOFF-1`
- `OMNI-ABI-AARCH64-SYSV-ELF-1`
- `OMNI-ABI-AARCH64-APPLE-MACHO-1`
- `OMNI-ABI-RV64-SYSV-ELF-1`
- `OMNI-ABI-AARCH64-FREESTANDING-1`
- `OMNI-ABI-RV64-FREESTANDING-1`

Exact machine-readable supplements are in `targets/` and `abi/`.


---

# OMNI-RUNTIME: Runtime Component Contracts

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `profile-required` |
| Dependencies | `OMNI-ERROR`, `OMNI-CONC`, `OMNI-MEM`, `OMNI-ABI` |
| Date | `2026-08-04` |

## 1. Scope

Startup, allocation, panic, unwinding, schedulers, GC domains, metadata, shutdown, observability.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Component ABI

| Rule | Requirement |
|---|---|
| `RT-0001` | Runtime support is a graph of explicitly selected components; unused allocator, unwinder, executor, GC, reflection, I/O, tracing, or accelerator components are not initialized or linked unless required. |
| `RT-0002` | Each component declares ABI version, dependencies, initialization/finalization, thread/task hooks, capabilities, effects, memory ownership, fault policy, and target/profile compatibility. |
| `RT-0003` | Component discovery occurs at link time or through an explicit signed plugin manifest. Ambient dynamic discovery and classpath-style scanning are prohibited. |
| `RT-0004` | Absence of an optional component is reported at translation/link time when statically required or as a typed `Unsupported` result when explicitly dynamically queried. |
| `RT-0005` | Runtime initialization is deterministic dependency-topological order with lexical tie-break by component identity; cycles are errors. |
| `RT-0006` | GC domains are explicit objects with collector policy, heap, roots, safepoints, barriers, weak/finalizer queues, and cross-domain rules. |
| `RT-0007` | Precise stack maps and typed roots are required. Conservative pointer guessing is not conforming for safe managed references. |
| `RT-0008` | A safepoint may occur only at operations marked in MIR. No raw pointer into movable managed storage may remain live across a safepoint. |
| `RT-0009` | Write/read barriers are explicit in IR and collector ABI. Missing a required barrier is a runtime/compiler defect. |
| `RT-0010` | Pinning removes an object from movement or uses a stable indirection according to collector policy; pin duration and fragmentation impact are inspectable. |
| `RT-0011` | Cross-domain managed references are prohibited unless represented by an owning bridge handle whose collector protocol traces both domains without cycles of uncoordinated collection. |
| `RT-0012` | Weak references do not keep objects alive. Upgrade atomically participates in liveness. Finalizers are unordered except dependency-safe constraints, may be delayed indefinitely, and cannot resurrect into ordinary safe ownership unless the profile explicitly permits one-shot resurrection. |
| `RT-0013` | Foreign calls pin/copy/handle managed references and publish roots through the runtime ABI. Foreign-attached threads register stack maps/handles. |

## 4. Core component set

`start`, `panic`, optional `unwind`, allocator providers, thread/TLS, parking/synchronization, async reactor/executor, managed domains, reflection, stack maps, sanitizers/hardening, I/O/persistence/device providers, and observability providers are separate components.

## 5. Finalization warning

Managed finalizers are not deterministic resource management. Files, locks, capabilities, transactions, and device leases require explicit lexical/async cleanup even inside a managed domain.


---

# OMNI-LIB-CORE: Freestanding Core Library

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-TYPES`, `OMNI-NUM`, `OMNI-OWN`, `OMNI-EFFECTS`, `OMNI-EVAL` |
| Date | `2026-08-04` |

## 1. Scope

Exact APIs and semantic contracts for core traits, values, slices, iterators, atomics, intrinsics.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Canonical API inventory

The machine-readable normative inventory is `library/core-api.yaml`. It is the exhaustive Edition 1 freestanding API set. Implementations MAY provide additional namespaced libraries but cannot add members to standard modules in strict mode.

| Rule | Requirement |
|---|---|
| `LIBCORE-0001` | The freestanding core library is available on every conforming Core Language target and performs no allocation, I/O, blocking, thread creation, reflection metadata loading, or hidden capability access unless its signature declares it. |
| `LIBCORE-0002` | Every public API record declares type/effect signature, ownership, panic/faults, allocation, blocking, determinism, thread safety, complexity, target/profile availability, and unsafe obligations. |
| `LIBCORE-0003` | Core modules are `core.bool`, `core.option`, `core.result`, `core.cmp`, `core.hash`, `core.clone`, `core.marker`, `core.mem`, `core.ptr`, `core.slice`, `core.str`, `core.array`, `core.iter`, `core.ops`, `core.num`, `core.atomic`, `core.task`, `core.contract`, and `core.intrinsics`. |
| `LIBCORE-0004` | `Option` and `Result` combinators evaluate callbacks at most once, left-to-right, and forward callback effects polymorphically. |
| `LIBCORE-0005` | Iterator `next` consumes one logical step. `size_hint` is a lower/optional upper bound, not a safety promise. `ExactSizeIterator` and `TrustedLen` carry stronger obligations, with `TrustedLen` unsafe to implement. |
| `LIBCORE-0006` | Slice indexing is bounds-checked; unchecked indexing is unsafe. Mutable iteration yields nonoverlapping elements according to the iterator contract. |
| `LIBCORE-0007` | UTF-8 string APIs distinguish bytes, Unicode scalars, and grapheme/text algorithms. Core supplies bytes/scalars only; grapheme/normalization/case/locale data belongs to text profile libraries. |
| `LIBCORE-0008` | Hashing is trait-based. No core hash algorithm is stable for persistence unless explicitly named. Default hashers in hosted collections use per-process secret seeding and are not deterministic. |
| `LIBCORE-0009` | Atomic APIs expose exact memory ordering and target lock-free queries; invalid compare-exchange failure ordering is a compile-time error when constant and runtime error otherwise. |
| `LIBCORE-0010` | Raw memory functions require validity, nonoverlap where specified, alignment, provenance, initialization, and allocator obligations stated by each function. |
| `LIBCORE-0011` | Intrinsic APIs are compiler-versioned and not stable public source contracts unless promoted into another core module. |

## 4. Complexity notation

Complexity is stated in abstract operations: comparisons, hashes, moves/copies, allocations, atomic operations, and element visits. Amortized bounds name the potential/resource assumptions. A target may have different constant factors but not asymptotically worse behavior for a conforming implementation.

## 5. Panic and allocation visibility

An API documented `no_panic` cannot panic for valid inputs except process-level target failure. An API documented `no_alloc` cannot invoke any allocator even if allocation would later be optimized away.


---

# OMNI-LIB-*: Profile Library Specifications

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `claim-dependent` |
| Dependencies | `OMNI-LIB-CORE`, `OMNI-PROFILES`, `OMNI-RUNTIME` |
| Date | `2026-08-04` |

## 1. Scope

Exact APIs, complexity, effects, allocation, blocking, panic, determinism, security and target availability.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Standard profile library families

- `alloc`: owned collections, allocators, arenas, RC;
- `system`: process, filesystem, networking, time, entropy, dynamic libraries;
- `async`: tasks, executors, reactors, async synchronization and I/O;
- `managed`: managed domains and collection policies;
- `accelerated`: vectors, tensors, kernels, devices;
- `persistent`: schemas, transactions, durable storage and direct I/O;
- `distributed`: RPC, protocols, discovery, resilience;
- `verified`: proofs, ghost state, contracts and verified collections;
- `text`: Unicode text algorithms and locale adapters.

| Rule | Requirement |
|---|---|
| `LIBP-0001` | Profile libraries are versioned separately but use the same type/effect/ownership semantics as core. |
| `LIBP-0002` | Every collection declares iterator/reference invalidation for insertion, removal, reserve, shrink, move, swap, and concurrent mutation. |
| `LIBP-0003` | `Vec` invalidates element references when capacity changes and preserves them for mutations that do not reallocate or remove/move the referenced element; all structural mutation still requires exclusive access. |
| `LIBP-0004` | Hash maps do not promise iteration order unless using an ordered-map type. Rehash invalidates internal iterators/references according to the API; keys/values remain owned by the map. |
| `LIBP-0005` | Default hosted hash maps use a cryptographically strong keyed hash or equivalent DoS-resistant strategy with explicit randomness capability at construction. Deterministic maps use a named stable algorithm and warn against hostile keys. |
| `LIBP-0006` | Text libraries pin Unicode data independently of source-edition Unicode data and expose the data version at runtime/compile time. |
| `LIBP-0007` | Filesystem APIs operate through directory/file capabilities, use platform-native raw path forms plus explicit Unicode views, and never assume paths are valid UTF-8. |
| `LIBP-0008` | Network APIs expose partial I/O, timeouts/cancellation, address-family capability, and protocol state; a write success means accepted by the local provider, not remote delivery. |
| `LIBP-0009` | Async I/O buffers remain owned/pinned by the operation until completion/cancellation commit and cleanup returns them. |
| `LIBP-0010` | Cryptographic APIs are provider/version explicit, avoid generic “encrypt/hash” defaults, expose constant-time claims by profile, and separate key capabilities from bytes. |
| `LIBP-0011` | Persistent and distributed libraries expose partial failure and do not promise transparent rollback of external effects. |

## 4. API completeness

Each profile release contains a canonical API inventory and conformance corpus. A distribution may omit an optional profile but cannot provide a partial standard module while claiming that profile.


---

# OMNI-PROFILES: Profile Definitions and Composition

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `claim-dependent` |
| Dependencies | `OMNI-STD-ROOT`, `OMNI-TARGET`, `OMNI-RUNTIME` |
| Date | `2026-08-04` |

## 1. Scope

Execution/runtime/assurance/numeric profiles, composition algebra, conflicts, fingerprints, required specs.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Composition table

| Profile | Requires | Conflicts/forbids | Meaning |
|---|---|---|---|
| `core` | none | none | Freestanding language and core library. |
| `alloc` | core | none | Allocator interfaces and owned collections. |
| `system` | core | none | Hosted OS providers through capabilities. |
| `async` | core | none | Structured tasks and async state machines. |
| `managed` | alloc | realtime.strict_gc_free | Explicit tracing-GC domains. |
| `accelerated` | core | none | SIMD/tensor/device kernels. |
| `persistent` | alloc | none | Canonical schemas and durable transactions. |
| `distributed` | system, async | none | Networked protocols with partial failure. |
| `verified` | core | none | Proof terms and verified contracts. |
| `deterministic` | core | ambient_nondeterminism | Recorded or excluded nondeterminism. |
| `hardened` | core | none | Defense-in-depth code generation/runtime. |
| `constant_time` | hardened | secret_dependent_observations | Secret-independent control/memory timing contract. |
| `realtime` | core | unbounded_blocking, unbounded_gc | Bounded allocation, blocking, and scheduling. |
| `high_assurance` | verified, hardened, deterministic | unchecked_unsafe, unvalidated_opt | Tight trusted base and validation. |

| Rule | Requirement |
|---|---|
| `PROF-0001` | Profile composition is set union followed by transitive requirement closure and conflict checking. A conflict makes the composition invalid rather than choosing one profile silently. |
| `PROF-0002` | A profile may add APIs, effects, capabilities, restrictions, runtime components, and conformance tests but may not redefine core syntax, type validity, evaluation order, or safe memory semantics. |
| `PROF-0003` | The artifact profile fingerprint includes profile IDs/versions, options, runtime policies, Unicode/text data, panic/OOM policy, numeric default, target features, and ABI supplements. |
| `PROF-0004` | Libraries/artifacts may link only when required profiles are present and every shared semantic/profile option is compatible or adapted through an explicit boundary. |
| `PROF-0005` | Core-required profiles for a Core Language claim are `core` and `safe`. All others are optional claim-dependent suites. |
| `PROF-0006` | `managed`, `accelerated`, `persistent`, `distributed`, and `verified` ratify independently and do not block Core Language 1.0. |
| `PROF-0007` | Restriction profiles compose monotonically: enabling deterministic, realtime, constant-time, hardened, or high-assurance can reject programs but cannot add authority or weaken safety. |

## 4. Compatibility

A function/library signature may be profile-polymorphic over facilities it does not inspect. Profile-specific behavior must be represented in types/effects/capabilities or artifact metadata; it cannot be selected by hidden global mode.


---

# OMNI-MANIFEST: Package and Artifact Manifest Schemas

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `distribution-required` |
| Dependencies | `OMNI-RULES`, `OMNI-TARGET` |
| Date | `2026-08-04` |

## 1. Scope

Package manifest, lockfile, build manifest, artifact metadata, extension rules, canonical serialization.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Canonical schemas

Normative JSON schemas are `schemas/omni-manifest.schema.json` and `schemas/omni-lock.schema.json`. TOML is converted to the schema data model before validation; integer ranges and strings are checked without lossy conversion.

| Rule | Requirement |
|---|---|
| `MANIFEST-0001` | The source manifest is `omni.toml`, restricted to TOML 1.0 syntax with UTF-8, unique keys, no duplicate tables, and no datetime values in normative identity fields. |
| `MANIFEST-0002` | The lockfile is canonical JSON named `omni.lock`; object keys are sorted, numbers are integers, strings are NFC UTF-8, and no insignificant whitespace participates in canonical bytes. |
| `MANIFEST-0003` | A manifest declares package identity, edition, source roots, targets, profiles, features, dependencies, capabilities requested, build actions, exported artifacts, license/provenance, and security policy. |
| `MANIFEST-0004` | Unknown fields in the standard namespace are errors. Namespaced extension tables are retained and included in identity only when marked semantic. |
| `MANIFEST-0005` | Edition/profile/target selection is explicit and cannot be inferred from installed compiler defaults for release builds. |
| `MANIFEST-0006` | A lock record identifies every dependency by source identity, exact version/revision, content digest, feature instance, dependency edges, license/provenance metadata, and yanked/advisory state at resolution time. |
| `MANIFEST-0007` | Manifest and lock schemas are versioned independently. A tool that cannot interpret a semantic field SHALL reject rather than ignore it. |

## 4. Capability requests

A dependency may declare required capability interfaces but receives no authority during build or execution unless the root artifact/host explicitly delegates a scoped capability. Transitive requests are visible in the build and deployment report.


---

# OMNI-PKG: Package Identity and Resolution

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `distribution-required` |
| Dependencies | `OMNI-MANIFEST`, `OMNI-NAMES` |
| Date | `2026-08-04` |

## 1. Scope

Package identity, version constraints, solver semantics, features, source identity, yanks, replacement, vendoring.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Resolution model

Resolution is a deterministic constraint problem over package identities and feature instances. The normative result is the unique maximal solution under the ordering defined by this document; if no solution exists, the resolver emits a deterministic incompatibility proof.

| Rule | Requirement |
|---|---|
| `PKG-0001` | A package source identity is one of registry, signed archive, Git commit, path workspace, or vendored content, each with the exact canonical fields below. |
| `PKG-0002` | Registry identity is `(registry-root-key-id, namespace, package-name, version, content-digest)`. Git identity includes normalized remote identity, full commit hash, subdirectory, and tree digest. Path identity is workspace-relative canonical path plus content digest. |
| `PKG-0003` | Versions use `MAJOR.MINOR.PATCH[-prerelease][+metadata]`. Compatibility operators are exact `=`, compatible `^`, patch-compatible `~`, comparisons, comma intersection, and `||` union. |
| `PKG-0004` | `^1.2.3` means `>=1.2.3,<2.0.0`; `^0.2.3` means `>=0.2.3,<0.3.0`; `^0.0.3` means exactly the 0.0 patch line. Prereleases are selected only by explicit prerelease constraints. |
| `PKG-0005` | Resolution chooses the highest non-yanked version satisfying all constraints for each package instance; ties use lexicographic source identity and then digest. Conflict explanations use deterministic lexicographic decision order. |
| `PKG-0006` | Different major versions and semantically incompatible feature instances may coexist as distinct package identities. The resolver never globally unifies features merely because names match. |
| `PKG-0007` | Features are namespaced and additive within one package instance. A feature that changes public semantics/ABI creates a distinct feature instance included in package identity and cannot be unified with a conflicting instance. |
| `PKG-0008` | Target-conditional dependencies are evaluated from manifest target facts only. Resolution records all target branches required by requested multi-target builds. |
| `PKG-0009` | Content digests cover normalized archive entries, permissions, symlink targets, and bytes; timestamps/owner IDs are excluded or canonicalized. |
| `PKG-0010` | Yanked versions remain usable only when already locked or explicitly allowed by policy. Security-revoked content is rejected even when locked unless an emergency waiver is recorded. |

## 4. Source forms

- registry: signed metadata plus content digest;
- archive: immutable URL/locator, digest, signature policy;
- Git: full commit and verified tree digest, never a floating branch in a release lock;
- path/workspace: local development only unless vendored into release inputs;
- vendored: content tree embedded under the root release source with original provenance.


---

# OMNI-BUILD: Hermetic Build Action Semantics

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `distribution-required` |
| Dependencies | `OMNI-MANIFEST`, `OMNI-PKG`, `OMNI-CONST` |
| Date | `2026-08-04` |

## 1. Scope

Build graph, declared inputs/outputs, environment, sandbox, caching, remote execution, generated code.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Action record

```text
Action = {
  action_schema, tool_digest, argv, virtual_workdir,
  input_tree_digest, environment_map, target_manifest_digest,
  profile_fingerprint, capabilities, limits, expected_outputs
}
```

| Rule | Requirement |
|---|---|
| `BUILD-0001` | A build action is a pure declared function from content-addressed inputs and fixed parameters to content-addressed outputs plus structured diagnostics. |
| `BUILD-0002` | Inputs include source trees, dependency artifacts, tools, target/profile manifests, environment variables explicitly named, build capabilities, and resource limits. |
| `BUILD-0003` | Undeclared files, environment, clock, timezone, locale, randomness, network, process state, home directory, registry state, and filesystem enumeration cannot be observed. |
| `BUILD-0004` | Build network access is denied by default. A fetch action is separate, content-addressed, policy-authorized, and produces immutable inputs for later offline build actions. |
| `BUILD-0005` | Paths inside the sandbox are canonical virtual paths. Host paths do not enter object/debug/generated bytes except through normalized source remapping records. |
| `BUILD-0006` | Generated source identity includes generator artifact digest, command/schema version, exact inputs, semantic extension settings, and output path identity. |
| `BUILD-0007` | Incremental cache keys include every semantically relevant query input and compiler/spec data version. Missing an input is a correctness defect. |
| `BUILD-0008` | Remote cache/execution results are accepted only with authenticated action digest, worker/toolchain attestation, output digests, and policy-compatible isolation. |
| `BUILD-0009` | Build action outputs are written atomically and verified before publication. Partial output is never treated as cache hit. |
| `BUILD-0010` | Parallel action completion order cannot affect merged outputs; merge order is canonical by declared key. |
| `BUILD-0011` | Resource-limit failure is a structured build failure and not permission to emit a partial release artifact. |

## 4. Build scripts

Arbitrary host shell scripts are not a package primitive. Build extensions are Omni/Wasm-like sandboxed tools with declared schemas. Native tools may run only in a stronger isolated provider and are fingerprinted as trusted build inputs.


---

# OMNI-REGISTRY: Registry and Publication Protocol

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `distribution-required` |
| Dependencies | `OMNI-PKG`, `OMNI-SECURITY` |
| Date | `2026-08-04` |

## 1. Scope

Namespaces, publisher identity, index protocol, immutability, mirrors, rate/size limits, moderation.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Publication protocol

| Rule | Requirement |
|---|---|
| `REG-0001` | Publisher identity is a cryptographic principal bound to one or more namespaces by signed registry metadata and transparency records. |
| `REG-0002` | Package publication requires namespace authority, content digest, manifest/lock/provenance/SBOM, signatures meeting policy threshold, and immutable version identity. |
| `REG-0003` | Namespace transfer uses old-and-new principal signatures, a public waiting period, and explicit package-by-package or namespace-wide scope. Emergency transfer requires registry root threshold and audit. |
| `REG-0004` | Typosquatting defenses include normalized/confusable skeleton checks, reserved names, similarity review, publisher history, and warnings; similarity never automatically transfers ownership. |
| `REG-0005` | Abandoned packages may be archived, adopted under a new namespace, or transferred through the published process. Existing immutable versions are never silently replaced. |
| `REG-0006` | Republication of the same version with different content is prohibited. A corrected release uses a new version. |
| `REG-0007` | Registry APIs are deterministic paginated/signed metadata protocols; clients do not trust transport security alone. |

## 4. Transparency

Every accepted publication, yank, transfer, revocation, and root-metadata change is entered into an append-only verifiable log or offline equivalent whose checkpoint is distributed independently.


---

# OMNI-UPDATE: Secure Update and Revocation

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `distribution-required` |
| Dependencies | `OMNI-REGISTRY`, `OMNI-SECURITY` |
| Date | `2026-08-04` |

## 1. Scope

Rollback/freeze/mix-and-match resistance, metadata expiry, key rotation, revocation, compromise recovery.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Threat model

The update system is designed against rollback, freeze, mix-and-match, wrong-package, fast-forward, endless-data, key compromise below threshold, and repository compromise without sufficient delegated keys.

| Rule | Requirement |
|---|---|
| `UPDATE-0001` | Update metadata uses separately scoped root, timestamp, snapshot, targets, delegation, revocation, and advisory roles with threshold signatures and expiration. |
| `UPDATE-0002` | Root metadata is versioned and updated only through a chain validated from a trusted root with rollback protection. |
| `UPDATE-0003` | Timestamp metadata prevents freeze and detects stale snapshots. Snapshot metadata binds exact versions/hashes of delegated metadata and prevents mix-and-match. |
| `UPDATE-0004` | Target metadata binds package/artifact identity, version, length, digest, and custom compatibility fields, preventing wrong-package substitution. |
| `UPDATE-0005` | Clients enforce monotonically nondecreasing metadata versions, expiration using an authorized clock or offline freshness policy, and maximum metadata/target sizes. |
| `UPDATE-0006` | Endless-data defenses require declared length before download, streaming limits, decompression limits, and hash verification. |
| `UPDATE-0007` | Revocation identifies exact content digests/keys/versions and effective time/severity. A security-revoked artifact is not selected or executed absent an audited emergency waiver. |
| `UPDATE-0008` | Offline mirrors carry the complete signed metadata chain and checkpoints. Mirror transport cannot override signatures or freshness. |


---

# OMNI-REPRO: Reproducible Build and Release Rules

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `distribution-required` |
| Dependencies | `OMNI-BUILD`, `OMNI-TARGET` |
| Date | `2026-08-04` |

## 1. Scope

Build perimeter, timestamps, paths, locales, randomness, parallel order, signatures, independent rebuilds.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Reproducibility classes

- `source-reproducible`: same normalized source/package graph;
- `semantic-reproducible`: same IR and public behavior fingerprints;
- `bit-reproducible`: identical unsigned output bytes;
- `diverse-reproducible`: independent toolchain path yields equivalent and authenticated output.

| Rule | Requirement |
|---|---|
| `REPRO-0001` | The reproducibility perimeter includes source/package trees, spec/data manifests, compiler and tools, target descriptions, libraries/runtime, build actions, external generators, linker/assembler, and signing boundary. |
| `REPRO-0002` | Fixed inputs may include target/profile, build mode, declared source date epoch, pinned seed/model, and authorized environment values; every fixed input appears in provenance. |
| `REPRO-0003` | Timestamps are zero or `SOURCE_DATE_EPOCH`-derived; timezones/locales are fixed; directory/map/set order is canonical; random decisions use recorded deterministic seeds. |
| `REPRO-0004` | Host paths are remapped to virtual source identities. User names, machine names, temporary paths, inode numbers, process IDs, and concurrency timing are excluded. |
| `REPRO-0005` | Parallel compilation merges symbols, diagnostics, archives, and metadata in canonical order independent of task completion. |
| `REPRO-0006` | Object/archive formats use deterministic headers and member order. Linkers use stable layout algorithms or record every layout seed/input. |
| `REPRO-0007` | Reproducible unsigned artifact bytes are computed before platform signing. Signatures are detached or inserted into excluded/normalized containers whose relationship to the unsigned digest is specified. |
| `REPRO-0008` | Two independent builders using the same release inputs SHALL produce byte-identical unsigned artifacts for a bit-reproducible claim. |
| `REPRO-0009` | When platform signing inherently changes bytes, the release records unsigned reproducible digest, signed digest, signer identity, timestamp authority, and deterministic verification mapping. |


---

# OMNI-SECURITY: Security Model and Lifecycle

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-STD-ROOT`, `OMNI-TERMS` |
| Date | `2026-08-04` |

## 1. Scope

Threat model, secure defaults, unsafe/capability requirements, supply chain, vulnerability response, crypto policy.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Security invariants

| Rule | Requirement |
|---|---|
| `SEC-0001` | The language security model combines memory/type/concurrency safety, explicit capabilities, least privilege, supply-chain integrity, hardening, and profile-specific side-channel contracts; no one layer substitutes for another. |
| `SEC-0002` | A threat model is mandatory for the compiler, package/build/update systems, runtime components, standard libraries, profiles, registry, bootstrap, and release infrastructure. |
| `SEC-0003` | Safe-code soundness vulnerabilities, capability escalation, compiler miscompilation, malicious dependencies/build tools, compromised signing keys, hostile inputs, side channels, and device/FFI faults are in scope. |
| `SEC-0004` | Speculative execution, cache/timing, power, and fault-injection leakage are not prevented by ordinary language safety. Constant-time/hardened profiles define additional observations and target-specific mitigations. |
| `SEC-0005` | Constant-time code forbids secret-dependent control flow, memory addresses, variable-time instructions, allocation, scheduling, and observable errors unless a certified primitive masks them. |
| `SEC-0006` | Cryptographic algorithms are selected by named provider/profile and version. Deprecation, disable dates, key sizes, parameter validation, and migration are published; generic unversioned crypto defaults are prohibited. |
| `SEC-0007` | A constant-time claim requires source/IR taint rules, target code audit/testing, compiler transformation restrictions, and provider attestation for the exact artifact. |
| `SEC-0008` | Official releases produce SLSA-compatible provenance, SPDX and/or CycloneDX SBOMs at pinned schema versions, dependency graph, signatures, and transparency checkpoints. |
| `SEC-0009` | Advisories identify affected package/artifact/spec ranges, severity, exploitability, fix/workaround, revocation/yank status, and machine-readable policy data. |
| `SEC-0010` | Policy enforcement may deny known-vulnerable, yanked, unmaintained, unsigned, unreviewed-unsafe, or license-incompatible dependencies according to project rules. |
| `SEC-0011` | Security reports use coordinated disclosure; temporary embargo access is least-privilege and audited. Emergency errata follow OMNI-RELEASE. |
| `SEC-0012` | Unsafe code, FFI, macros/build tools, compiler plugins/protocols, and runtime providers are separately auditable trust boundaries. |
| `SEC-0013` | Hardened targets enable available CFI, protected return flow, stack protection, ASLR/PIE compatibility, RELRO, memory tagging/capabilities, allocator hardening, and compartmentalization without changing source semantics. |

## 4. Capability security

Ambient authority is absent in capability-safe code. A dependency receives only explicit values delegated by its caller/host. Capability authenticity and revocation are provider responsibilities constrained by OMNI-EFFECTS.

## 5. Security lifecycle

Every candidate release requires threat-model update, fuzzing/property/differential results, dependency review, soundness audit, unsafe inventory, reproducibility/DDC evidence, incident-response drill, and unresolved-vulnerability disposition.


---

# OMNI-DIAG: Required Diagnostics and Result Schema

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `distribution-required` |
| Dependencies | `OMNI-RULES`, `OMNI-SOURCE` |
| Date | `2026-08-04` |

## 1. Scope

Mandatory diagnostic classes/codes, spans, expansion traces, machine schema, severity, suppression, SARIF mapping.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Diagnostic classes

| Rule | Requirement |
|---|---|
| `DIAG-0001` | Required diagnostics are errors for malformed source/tokens/grammar, unresolved or ambiguous names, type/effect/ownership violations, invalid const evaluation, unsupported required features, ABI/profile incompatibility, and violated static contracts. |
| `DIAG-0002` | Quality-of-implementation diagnostics include performance remarks, style lints, suspicious-but-valid unsafe patterns, portability warnings, and additional security guidance; they cannot reject strict-conforming code unless promoted by user policy. |
| `DIAG-0003` | Every required diagnostic has a stable code, severity, primary rule ID, source spans, structured arguments, related locations, expansion trace, target/profile context, and optional machine-applicable fixes. |
| `DIAG-0004` | Diagnostic wording may improve within a tool version, but stable codes, schema fields, rule links, and fix semantics remain compatible. |
| `DIAG-0005` | Machine output uses `schemas/diagnostic.schema.json`; SARIF mapping preserves rule ID, locations, code flow, fixes, severity, and artifact identities. |
| `DIAG-0006` | A compiler exits 0 on successful requested action, 1 on source/conformance errors, 2 on invocation/configuration errors, 3 on internal compiler error, 4 on build/tool failure, 5 on policy/security denial, and 6 on target/runtime launch failure. |
| `DIAG-0007` | Machine-output mode writes only framed schema records to stdout; human progress/noise goes to stderr or is disabled. |
| `DIAG-0008` | Strict-conformance mode disables extensions, treats unknown attributes/profiles as errors, verifies external data versions, and emits a conformance claim record. |
| `DIAG-0009` | Error recovery may create placeholder nodes/types for IDE continuation but no executable/object/release IR is emitted from a compilation containing required errors. |

## 4. Stable code families

`E01xx` source/lex, `E02xx` grammar, `E03xx` names, `E04xx` types/numerics, `E05xx` ownership, `E06xx` effects/capabilities, `E07xx` const/macros, `E08xx` concurrency/memory, `E09xx` unsafe/FFI, `E10xx` target/ABI/profile, `E11xx` package/build/security, and `ICE0001` internal compiler failure.


---

# OMNI-FMT: Canonical Formatting

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `distribution-required` |
| Dependencies | `OMNI-GRAMMAR`, `OMNI-SOURCE` |
| Date | `2026-08-04` |

## 1. Scope

Edition-pinned deterministic formatting, comments, generated code, idempotence, migration policy.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Canonical style

| Rule | Requirement |
|---|---|
| `FMT-0001` | The canonical formatter is edition-pinned, deterministic, idempotent, semantics-preserving, comment-preserving, and independent of terminal width unless a fixed width is part of the formatter edition. |
| `FMT-0002` | Edition 1 canonical width is 100 display columns using Unicode East Asian Width from the pinned source-data manifest; tabs are not emitted. |
| `FMT-0003` | Blocks use four-space indentation, opening braces on the declaration/control line, and one statement per line except compact empty bodies. |
| `FMT-0004` | Semicolons are emitted exactly where grammar requires or where optional presence prevents fragile adjacent-token parsing; line breaks never supply semantics. |
| `FMT-0005` | Imports are grouped by package root and sorted by normalized canonical path; the formatter does not remove or add imports except in organizer mode. |
| `FMT-0006` | Comments remain attached to their concrete syntax anchors. The formatter emits visible escapes/annotations for source-invisible or bidi characters according to OMNI-SOURCE. |
| `FMT-0007` | Raw strings remain raw when possible; the formatter chooses the minimum delimiter hash count that preserves content and does not rewrite string data. |
| `FMT-0008` | Formatting an already canonical file yields byte-identical output after LF/source normalization. |
| `FMT-0009` | A formatter edition never changes silently under the same language edition; migrations are explicit and produce reviewable diffs. |

## 4. Machine contract

`omni fmt --check --edition 1` exits 0 only when every file equals canonical bytes. `--emit-diff` emits deterministic unified diffs with normalized paths and LF.


---

# OMNI-DOC: Documentation Syntax and Semantics

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `distribution-required` |
| Dependencies | `OMNI-GRAMMAR`, `OMNI-NAMES`, `OMNI-DIAG` |
| Date | `2026-08-04` |

## 1. Scope

Doc comments, links, examples/doctests, API metadata, version/profile/target resolution.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Documentation grammar and attachment

| Rule | Requirement |
|---|---|
| `DOC-0001` | Outer documentation comments attach to the immediately following item after attributes; inner documentation comments attach to the containing module/item. |
| `DOC-0002` | Doc content is CommonMark-compatible Markdown with Omni extensions for symbol links, effects, capabilities, contracts, target/profile tables, and executable examples. |
| `DOC-0003` | An intra-doc link resolves using the same package/module/name rules as source, optionally with an explicit namespace prefix. Ambiguous or broken links are release errors for public APIs. |
| `DOC-0004` | Code examples declare edition, target/profile, capabilities, expected output/fault, and whether they compile, run, fail, or are illustrative. |
| `DOC-0005` | Doctests execute hermetically with only declared capabilities, deterministic inputs, fixed target emulator/provider, resource limits, and exact package graph. |
| `DOC-0006` | API documentation includes full signature, generics, ownership, effects, capabilities, errors, panics, allocation/blocking, cancellation safety, thread safety, complexity, determinism, target/profile availability, safety obligations, and compatibility history. |
| `DOC-0007` | Private items are omitted by default. Cross-package private documentation requires an explicit authenticated documentation capability and is not published accidentally. |
| `DOC-0008` | Generated documentation records compiler/spec/data versions and source hashes and is reproducible. |

## 4. Example fences

```text
```omni,edition=1,profile=core,compile-pass
...
```
```

Other modes are `compile-fail(code=...)`, `run-pass(output=...)`, `run-panic(category=...)`, `no-run`, and `ignore(reason=...)`.


---

# OMNI-DEBUG: Debug, Unwind, and Source Mapping

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `platform-required` |
| Dependencies | `OMNI-IR`, `OMNI-ABI-*`, `OMNI-DIAG` |
| Date | `2026-08-04` |

## 1. Scope

Optimized debug semantics, source maps, async/actor/region views, crash records, external debug-format mapping.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Mapping contract

| Rule | Requirement |
|---|---|
| `DEBUG-0001` | Debug/source mapping preserves normalized source identity, original-byte mapping, macro expansion stack, inline call stack, variable location ranges, ownership moves, optimized-away status, and async state identity. |
| `DEBUG-0002` | DWARF-based Edition 1 targets pin a stable target-supported DWARF revision in the target supplement; draft DWARF revisions are experimental and cannot be silently required. |
| `DEBUG-0003` | Windows targets use pinned CodeView/PDB specifications and PE/COFF unwind metadata; Apple targets use pinned Mach-O/DWARF/compact-unwind contracts. |
| `DEBUG-0004` | Every unwind-enabled frame has correct unwind information through prologue/epilogue and inline assembly, or the function is declared non-unwindable and boundaries prevent unwind crossing. |
| `DEBUG-0005` | Optimized debug info may report a variable as unavailable but cannot report a stale/moved value as live. Moves and destruction end location validity. |
| `DEBUG-0006` | Async stacks are reconstructed from future/task descriptors and suspension records, with stable logical frame IDs distinct from physical stacks. |
| `DEBUG-0007` | Crash records contain artifact/build IDs, target/profile fingerprint, fault/panic category, stable symbol/source IDs, and integrity metadata; raw addresses are optional and security-policy controlled. |
| `DEBUG-0008` | Debuggers cannot bypass capability/security policy merely because metadata exists; reading protected memory or process state requires debugger authority. |

## 4. Source maps

A source map entry maps machine address/IR operation ranges to normalized source span plus expansion and inline stacks. Generated source includes generator provenance. Mapping is many-to-many; tools SHALL distinguish exact, approximate, optimized-away, and unavailable locations.


---

# OMNI-WIRE: Canonical Wire and Persistence Schemas

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `profile-required` |
| Dependencies | `OMNI-TYPES`, `OMNI-NUM`, `OMNI-SECURITY` |
| Date | `2026-08-04` |

## 1. Scope

Canonical encodings, schema identity/evolution, limits, NaNs, unknown/duplicate fields, golden vectors.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Byte-level encoding

A message is:

```text
4 bytes  "OMW1"
16 bytes schema fingerprint
varuint  payload length
bytes    canonical payload
4 bytes  CRC32C of header+payload (integrity only, not authentication)
```

Authentication/encryption are profile envelopes and do not alter the canonical inner payload.

| Rule | Requirement |
|---|---|
| `WIRE-0001` | Wire/persistent schemas are independent of native layout and use stable schema, type, field, and variant identifiers. |
| `WIRE-0002` | Edition 1 canonical encoding begins with magic/version and encodes records as ascending numeric field ID, each with explicit wire type and length where applicable. |
| `WIRE-0003` | Unsigned integers use minimal unsigned LEB128; signed integers use minimal zigzag-LEB128 unless a fixed-width wire type is declared. Nonminimal encodings are rejected in canonical mode. |
| `WIRE-0004` | Fixed integers/floats are little-endian. Floating values use IEEE storage bits; canonical mode maps every NaN to one quiet canonical NaN per width and preserves signed zero. |
| `WIRE-0005` | Bytes and UTF-8 strings are length-prefixed. Strings are validated UTF-8. Collections have element count and each variable element has bounded length/structure. |
| `WIRE-0006` | Booleans encode as one byte 0 or 1; other values are rejected. |
| `WIRE-0007` | Record field IDs are unique. Duplicate singular fields are errors; repeated fields use an explicit repeated type. Unknown fields are skipped/preserved according to schema policy. |
| `WIRE-0008` | Variant IDs are stable integers. Unknown variants return a typed unknown-variant result or preserved opaque variant only when the schema declares extensibility. |
| `WIRE-0009` | Canonical ordering is ascending field ID, map entries sorted by canonical key bytes, and sets sorted by canonical element bytes. Duplicate canonical map/set keys are errors. |
| `WIRE-0010` | Compatibility classes are backward-readable, forward-readable, bidirectional, migration-required, and breaking. The schema checker computes class from declared changes. |
| `WIRE-0011` | Adding optional fields with defaults is backward-compatible; removing required fields, reusing IDs, changing wire type/meaning, narrowing ranges, or changing canonical defaults is breaking unless versioned migration is required. |
| `WIRE-0012` | Zero-copy views require alignment, endianness, lifetime ownership of the input buffer, validated bounds, and representation compatibility. Otherwise decoding copies/converts. |
| `WIRE-0013` | Decoders enforce schema-declared maximum nesting, total bytes, collection counts, string lengths, allocation budget, and processing fuel before allocating or recursing. |
| `WIRE-0014` | Hostile-input validation completes before constructing a safe value with invariants stronger than the wire representation. |
| `WIRE-0015` | Crash-safe persistent records use checksummed length-delimited frames and commit markers/journal protocol named by the persistence profile; partial frames are ignored/recovered, never interpreted as complete values. |

## 4. Field wire types

`0 varuint`, `1 varsint`, `2 fixed32`, `3 fixed64`, `4 fixed128`, `5 bytes`, `6 record`, `7 packed-sequence`, `8 capability-reference` (provider-authenticated profile only). Field key is `(field_id << 4) | wire_type` encoded as varuint.

## 5. Golden vectors

`conformance/wire-golden-vectors.json` contains canonical encodings for every primitive, boundary, NaN, unknown-field, duplicate, malformed-length, and compatibility case.


---

# OMNI-STAGE0: Stage-0 Language and Compiler Contract

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `bootstrap-required` |
| Dependencies | `OMNI-SOURCE`, `OMNI-GRAMMAR`, `OMNI-EVAL`, `OMNI-LIB-CORE` |
| Date | `2026-08-04` |

## 1. Scope

Exact bootstrap subset, forbidden features, seed interface, target, deterministic output, conformance corpus.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Exact subset

| Rule | Requirement |
|---|---|
| `STAGE0-0001` | Stage-0 is a strict syntactic and semantic subset of Edition 1; every Stage-0 source parses and has identical observations under Edition 1. |
| `STAGE0-0002` | Stage-0 includes UTF-8/ASCII identifiers, modules, structs/enums, fixed integers/bool/byte, arrays/slices, functions, generics without specialization, Result/Option, ownership/borrows, explicit effects, unsafe raw memory, and freestanding/native code generation. |
| `STAGE0-0003` | Stage-0 excludes macros, runtime reflection, async, managed domains, dynamic typing, relations, decimal, scalable vectors/tensors, profile libraries, and user-defined effect handlers. |
| `STAGE0-0004` | Stage-0 grammar is generated by disabling named Edition 1 productions, not maintained as an unrelated grammar. |
| `STAGE0-0005` | Stage-0 uses the same checked arithmetic, evaluation order, initialization/drop, panic-abort policy, provenance, and C/omni_v1 ABI subset. |
| `STAGE0-0006` | The seed compiler consumes canonical Stage-0 source and emits one reference object format/ISA plus deterministic diagnostic records. |
| `STAGE0-0007` | Every Stage-0 restriction is represented by a feature predicate and tested against the Edition 1 parser/model to prove subset inclusion. |

## 4. Reference target

The canonical seed target is `riscv64-unknown-none-elf-omni_v1[lp64]+rv64imac`, with an optional hosted x86-64 seed path for accessibility. A release may add seed targets but retains one immutable canonical path.

## 5. Seed simplifications

Stage-0 uses abort panic/OOM, no unwinder, one bump/system allocator interface, no dynamic linking, no trait objects across ABI, and a simple verified MIR-to-machine lowering. These are subset restrictions, not alternate semantics.


---

# OMNI-BOOT: Bootstrap, DDC, and Trusted Base

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `bootstrap-required` |
| Dependencies | `OMNI-STAGE0`, `OMNI-BUILD`, `OMNI-REPRO`, `OMNI-SECURITY` |
| Date | `2026-08-04` |

## 1. Scope

Seed artifacts, trust graph, diverse double compilation, reproducibility, audit procedure, TCB manifest.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Bootstrap chain

| Rule | Requirement |
|---|---|
| `BOOT-0001` | The bootstrap seed is an immutable content-addressed source/binary package with documented encoding, build platform, required external tools, input hashes, and step-by-step offline procedure. |
| `BOOT-0002` | The trusted bootstrap path consists of seed decoder/compiler, Stage-0 sources, assembler/object writer, linker/image builder, target emulator/hardware, specification data, and verification tools named in the TCB inventory. |
| `BOOT-0003` | Stage A builds compiler B from seed. B builds compiler C from canonical Omni sources. C rebuilds compiler D; C and D must converge byte-for-byte for deterministic stages or to declared semantic equivalence when signing/layout is external. |
| `BOOT-0004` | Diverse double compilation builds the same canonical compiler source with the trusted chain and an independently implemented diverse compiler/toolchain, then uses each result to rebuild and compares canonical outputs. |
| `BOOT-0005` | A diverse compiler qualifies only if its implementation lineage, language/tool dependencies, backend/link path, and build environment are sufficiently independent and documented. |
| `BOOT-0006` | DDC mismatch blocks release, preserves all artifacts/logs, and triggers differential localization; it is never waived merely because one binary passes tests. |
| `BOOT-0007` | The TCB inventory lists exact source/binary digests, role, reason trusted, audit status, replacement strategy, and transitive dependencies for proof checker, parser generator, solver, assembler, linker, Unicode data, crypto, OS/firmware, and signing tools. |
| `BOOT-0008` | Emergency bootstrap recovery uses archived source, specs, data, emulators, and at least two independent seed paths stored in reproducible offline media. |
| `BOOT-0009` | Official compiler artifacts include provenance, SBOM, conformance report, optimization-validation report, and DDC evidence bound to the artifact digest. |

## 4. Seed encoding

The canonical seed source is LF-normalized UTF-8 Stage-0. Seed binary packages use deterministic `tar`-like archives with sorted paths, fixed permissions/timestamps, SHA-256 manifest, and threshold signatures. No opaque installer is part of the canonical path.

## 5. Auditability

The canonical seed compiler prioritizes small code size, straightforward algorithms, and exhaustive tests over optimization performance. The production optimizer/backend are built later and are not required to trust their own output without DDC and validation.


---

# OMNI-CONFORM: Conformance and Certification

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-RULES`, `OMNI-LIB-CORE`, `OMNI-ABI-*` |
| Date | `2026-08-04` |

## 1. Scope

Test formats, oracle rules, coverage, waivers, extensions, certification, independent results, claim validation.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Case schema

Normative schema: `schemas/conformance-case.schema.json`. Result schema: `schemas/conformance-result.schema.json`. Manifest: `conformance/manifest.yaml`.

| Rule | Requirement |
|---|---|
| `CONF-0001` | Each conformance case has stable case ID, rule IDs, suite version, source/artifact inputs, expected classification/result, target/profile applicability, required limits, oracle/model version, and allowed implementation-defined alternatives. |
| `CONF-0002` | Test kinds are parse-pass/fail, static-pass/fail, run-value, run-output, run-fault, memory-litmus, ABI-cross, wire-golden, build-resolution, security-policy, differential, metamorphic, and performance-contract. |
| `CONF-0003` | Coverage includes positive, negative, boundary, interaction, differential, and metamorphic cases for every ratified core rule; high-risk rules additionally require an independent model or implementation. |
| `CONF-0004` | A test is corrected only through a versioned test erratum linked to the rule. Prior suite artifacts remain immutable; corrected conformance claims identify the overlay. |
| `CONF-0005` | Tests do not create semantics. When a test conflicts with normative rules, the test is invalid until corrected. |
| `CONF-0006` | Strict mode disables extension syntax/semantics and rejects artifacts whose unrecognized extension could affect behavior. Namespaced extensions may be tested only in separate claims. |
| `CONF-0007` | An implementation claim names edition, modules, profiles, targets, limits, external ABI/data versions, implementation-defined choices, deviations, and test-result digest. |
| `CONF-0008` | Parser-only, core-subset, profile, platform, distribution, and complete-release claims are distinct; partial implementations cannot claim complete conformity. |
| `CONF-0009` | Certification may be self-attested, independently audited, or authority-certified. The claim record states assurance level, auditor, evidence, waivers, expiration, and revocation status. |
| `CONF-0010` | Waivers cannot excuse a soundness violation or false accepted behavior; they may document unavailable optional hardware/profile tests and narrow the claim. |
| `CONF-0011` | A certification is revoked when evidence is falsified, a blocker soundness/security defect invalidates the claim, or required artifacts become untrusted; revocation is signed and published. |

## 4. Required implementation evidence

Core 1.0 release requires two independently implemented parsers, one executable semantic model, one native implementation, grammar fuzzing, source/model/native differential tests, memory litmus execution, cross-compiler ABI tests, package/build reproducibility tests, and rule coverage with no unresolved P0 gaps.

## 5. Limits

Implementations may declare finite nesting, type size, monomorphization, const fuel, macro token/depth, object size, and diagnostic limits only above release minima. Exceeding a declared limit yields `ImplementationLimit` and cannot miscompile.


---

# OMNI-BENCH: Benchmark and Performance Evidence

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `release-required` |
| Dependencies | `OMNI-TARGET`, `OMNI-PROFILES`, `OMNI-REPRO` |
| Date | `2026-08-04` |

## 1. Scope

Benchmark methodology, equivalence rules, datasets, statistics, energy/latency/size/compile-time reporting.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Evidence rules

| Rule | Requirement |
|---|---|
| `BENCH-0001` | Performance claims compare equivalent algorithms, data structures, numeric/safety policies, error handling, I/O durability, target features, and warm/cold state. |
| `BENCH-0002` | Benchmarks report source, compiler/artifact digests, flags/profiles, hardware/firmware/OS, topology/frequency/power settings, dataset, repetitions, warmup, and raw samples. |
| `BENCH-0003` | Metrics include wall/CPU time, throughput, median and tail latency, peak/live memory, allocations, code size, startup, energy where measurable, storage/network I/O, compile time, and cache artifact size. |
| `BENCH-0004` | Statistics report sample count, central tendency, dispersion/confidence interval, outliers with policy, and practical effect size. Cherry-picked best runs are prohibited. |
| `BENCH-0005` | Adaptive benchmarking chooses run count from a predeclared stopping rule and records it. Benchmark code cannot detect competitors or alter work by implementation identity. |
| `BENCH-0006` | Profile-guided/autotuned builds include profile-collection cost and identify whether the benchmark workload contaminated training data. |
| `BENCH-0007` | Correctness/conformance is checked before timing. A faster result with changed output, precision, fault, durability, or safety contract is not equivalent. |
| `BENCH-0008` | Release gates use representative suites and regression budgets, not a single geometric mean. Regressions may be accepted only with published tradeoff rationale and no hidden metric loss. |

## 4. Anti-gaming review

Benchmark harnesses inspect generated code for eliminated work, validate results, randomize/partition inputs deterministically, separate setup from measured regions, and publish scripts/raw data. Energy claims include measurement uncertainty and idle-baseline method.


---

# OMNI-RELEASE: Edition Release Manifest and Errata

| Field | Value |
|---|---|
| Suite version | `1.0.0-candidate.1` |
| Language edition | `1` |
| Status | Complete normative candidate; not yet ratified or implementation-certified |
| Classification | `core-required` |
| Dependencies | `OMNI-CONFORM`, `OMNI-REPRO`, `OMNI-BOOT` |
| Date | `2026-08-04` |

## 1. Scope

Pinned documents/data/digests, normative references, errata, known deviations, compatibility and migration report.

## 2. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as described by BCP 14 when, and only when, they appear in all capitals.

Every requirement in this document has a stable rule identifier. Informative notes and examples do not create requirements. An implementation claiming conformance to this module SHALL satisfy every applicable REQUIRED rule and SHALL report each implementation-defined choice named by this module.

## 3. Release manifest

| Rule | Requirement |
|---|---|
| `REL-0001` | A release manifest pins every normative module, grammar/data/model/schema/test artifact, target supplement, external normative reference, and cryptographic digest. |
| `REL-0002` | Version axes are: language edition, specification revision, corrigendum/erratum overlay, Unicode/source-data version, text-library data version, ABI version, profile version, wire-schema version, toolchain version, package version, and target-manifest version. |
| `REL-0003` | An edition changes source language compatibility. A revision clarifies/adds nonbreaking normative detail. A corrigendum fixes publication error without intended semantics change. A normative erratum corrects semantics and declares compatibility impact. |
| `REL-0004` | Normative references use exact version/date/revision and archived digest/location. Floating “latest” references are informative only. |
| `REL-0005` | Replacing a normative reference requires compatibility analysis, updated conformance evidence, and a new suite revision or edition as appropriate. |
| `REL-0006` | Errata severities are editorial, diagnostic, compatibility, soundness, and security. Soundness/security errata may invalidate prior claims and trigger revocation. |
| `REL-0007` | An erratum overlay is signed, immutable, rule-linked, and states effective releases, replacement text/data/tests, implementation impact, and migration. |
| `REL-0008` | A corrected-conformance claim names the base manifest plus every applied overlay; silent mutable web text is not a standard release. |
| `REL-0009` | Compatibility reports separately analyze lexical/grammar source, static acceptance, dynamic observations, ABI, wire/persistence, package resolution, build reproducibility, diagnostics/tooling, and profile behavior. |
| `REL-0010` | Release qualification requires no unresolved P0 gaps, no normative placeholders, complete rule-test/model mapping, independent review, security audit, reproducible artifacts, DDC, and signed archival publication. |
| `REL-0011` | Normative external artifacts are archived where license permits or referenced with verified immutable digests and retrieval instructions. |

## 4. Candidate status

This suite is `1.0.0-candidate.1`: the language definition has no intentionally unresolved semantic choices, but ratification, independent implementations, formal proof completion, and certification evidence remain future release gates. Those gates concern confidence and adoption, not missing definitions.

## 5. Publication set

The candidate ZIP, file manifest, SHA-256 list, rule registry, resolved gap register, normative references, and validation report form one immutable candidate publication.
