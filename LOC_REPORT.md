# Lines of Code Report

**Last Updated**: 2025-10-16 17:02
**Tool**: [cloc](https://github.com/AlDanial/cloc) + wc

---

## Overall Summary

| Metric | Rust Code | Documentation (.md) | Total |
|--------|-----------|---------------------|-------|
| **Lines** | 1,412 | 1,094 | 2,506 |
| **Comments** | 344 | - | 344 |
| **Blank Lines** | 318 | - | 318 |
| **Total Lines** | 2,074 | 1,094 | 3,168 |
| **Files** | 7 | 7 | 14 |

**Documentation Ratio**: 0.77 lines of docs per line of code

---

## Rust Code Breakdown

```
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                             7            318            344           1412
WGSL                             2             36             19            125
-------------------------------------------------------------------------------
SUM:                             9            354            363           1537
-------------------------------------------------------------------------------
```

---

## Rust File Details

| File | Total Lines | Impl Lines | Test Lines | Test % | Status |
|------|-------------|------------|------------|--------|--------|
| `audio.rs` | 277 | 231 | 46 | 16.6% | ⚠️ Large |
| `camera.rs` | 209 | 128 | 81 | 38.8% | ✅ |
| `lib.rs` | 7 | 7 | 0 | 0.0% | ✅ |
| `main.rs` | 296 | 296 | 0 | 0.0% | ⚠️ Large |
| `ocean.rs` | 326 | 290 | 36 | 11.0% | ⚠️ Large |
| `params.rs` | 457 | 457 | 0 | 0.0% | ✅ (infra) |
| `rendering.rs` | 502 | 502 | 0 | 0.0% | ✅ (infra) |

**⚠️ Warning:** 3 file(s) over 200 impl lines - consider splitting for maintainability

---

## Documentation Files

| File | Lines |
|------|-------|
| `CLAUDE.md` | 437 |
| `COVERAGE_REPORT.md` | 68 |
| `FLOWFIELD.md` | 97 |
| `LEARNINGS.md` | 272 |
| `LEXICON.md` | 84 |
| `LOC_REPORT.md` | 86 |
| `VISION.md` | 50 |

---

## Documentation Quality Targets

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Docs/Code Ratio | ≥0.3 | 0.77 | ✅ Excellent |
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
