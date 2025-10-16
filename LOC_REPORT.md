# Lines of Code Report

**Last Updated**: 2025-10-16 05:34
**Tool**: [cloc](https://github.com/AlDanial/cloc) + wc

---

## Overall Summary

| Metric | Rust Code | Documentation (.md) | Total |
|--------|-----------|---------------------|-------|
| **Lines** | 1,217 | 1,425 | 2,642 |
| **Comments** | 281 | - | 281 |
| **Blank Lines** | 265 | - | 265 |
| **Total Lines** | 1,763 | 1,425 | 3,188 |
| **Files** | 7 | 8 | 15 |

**Documentation Ratio**: 1.17 lines of docs per line of code

---

## Rust Code Breakdown

```
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                             7            265            281           1217
WGSL                             2             36             19            125
-------------------------------------------------------------------------------
SUM:                             9            301            300           1342
-------------------------------------------------------------------------------
```

---

## Rust File Details

| File | Total Lines | Impl Lines | Test Lines | Test % | Status |
|------|-------------|------------|------------|--------|--------|
| `audio.rs` | 269 | 223 | 46 | 17.1% | ⚠️ Large |
| `camera.rs` | 142 | 86 | 56 | 39.4% | ✅ |
| `lib.rs` | 7 | 7 | 0 | 0.0% | ✅ |
| `main.rs` | 257 | 257 | 0 | 0.0% | ⚠️ Large |
| `ocean.rs` | 207 | 171 | 36 | 17.4% | ✅ |
| `params.rs` | 383 | 383 | 0 | 0.0% | ✅ (infra) |
| `rendering.rs` | 498 | 498 | 0 | 0.0% | ✅ (infra) |

**⚠️ Warning:** 2 file(s) over 200 impl lines - consider splitting for maintainability

---

## Documentation Files

| File | Lines |
|------|-------|
| `CLAUDE.md` | 437 |
| `COVERAGE_REPORT.md` | 68 |
| `FLOWFIELD.md` | 97 |
| `HANDOFF.md` | 333 |
| `LEARNINGS.md` | 272 |
| `LEXICON.md` | 84 |
| `LOC_REPORT.md` | 84 |
| `VISION.md` | 50 |

---

## Documentation Quality Targets

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Docs/Code Ratio | ≥0.3 | 1.17 | ✅ Excellent |
| README exists | Yes | ❌ | Missing |
| ARCHITECTURE.md | Optional | ❌ | Optional |

---

## How to Update This Report

```bash
# Regenerate LOC report
./scripts/generate-loc-report.sh
```

---

*This report is auto-generated from `cloc` and `wc` output.*
*Updated automatically by pre-commit hook when source files change.*
