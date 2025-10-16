# Lines of Code Report

**Last Updated**: 2025-10-16 04:40
**Tool**: [cloc](https://github.com/AlDanial/cloc) + wc

---

## Overall Summary

| Metric | Rust Code | Documentation (.md) | Total |
|--------|-----------|---------------------|-------|
| **Lines** | 1,043 | 1,089 | 2,132 |
| **Comments** | 258 | - | 258 |
| **Blank Lines** | 233 | - | 233 |
| **Total Lines** | 1,534 | 1,089 | 2,623 |
| **Files** | 7 | 7 | 14 |

**Documentation Ratio**: 1.04 lines of docs per line of code

---

## Rust Code Breakdown

```
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                             7            233            258           1043
WGSL                             2             36             19            125
-------------------------------------------------------------------------------
SUM:                             9            269            277           1168
-------------------------------------------------------------------------------
```

---

## Rust File Details

| File | Total Lines | Impl Lines | Test Lines | Test % | Status |
|------|-------------|------------|------------|--------|--------|
| `audio.rs` | 242 | 196 | 46 | 19.0% | ✅ |
| `camera.rs` | 142 | 86 | 56 | 39.4% | ✅ |
| `lib.rs` | 7 | 7 | 0 | 0.0% | ✅ |
| `main.rs` | 199 | 199 | 0 | 0.0% | ✅ |
| `ocean.rs` | 207 | 171 | 36 | 17.4% | ✅ |
| `params.rs` | 345 | 345 | 0 | 0.0% | ✅ (infra) |
| `rendering.rs` | 392 | 392 | 0 | 0.0% | ✅ (infra) |

---

## Documentation Files

| File | Lines |
|------|-------|
| `CLAUDE.md` | 437 |
| `COVERAGE_REPORT.md` | 68 |
| `FLOWFIELD.md` | 97 |
| `LEARNINGS.md` | 272 |
| `LEXICON.md` | 84 |
| `LOC_REPORT.md` | 81 |
| `VISION.md` | 50 |

---

## Documentation Quality Targets

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Docs/Code Ratio | ≥0.3 | 1.04 | ✅ Excellent |
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
