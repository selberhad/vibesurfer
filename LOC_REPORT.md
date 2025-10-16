# Lines of Code Report

**Last Updated**: 2025-10-16 19:01
**Tool**: [cloc](https://github.com/AlDanial/cloc) + wc

---

## Overall Summary

| Metric | Rust Code | Documentation (.md) | Total |
|--------|-----------|---------------------|-------|
| **Lines** | 1,540 | 2,346 | 3,886 |
| **Comments** | 379 | - | 379 |
| **Blank Lines** | 348 | - | 348 |
| **Total Lines** | 2,267 | 2,346 | 4,613 |
| **Files** | 17 | 10 | 27 |

**Documentation Ratio**: 1.52 lines of docs per line of code

---

## Rust Code Breakdown

```
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                            17            348            379           1540
WGSL                             2             36             19            125
-------------------------------------------------------------------------------
SUM:                            19            384            398           1665
-------------------------------------------------------------------------------
```

---

## Rust File Details

| File | Total Lines | Impl Lines | Test Lines | Test % | Status |
|------|-------------|------------|------------|--------|--------|
| `audio/fft.rs` | 91 | 77 | 14 | 15.4% | ✅ |
| `audio/mod.rs` | 11 | 11 | 0 | 0.0% | ✅ |
| `audio/synthesis.rs` | 12 | 12 | 0 | 0.0% | ✅ |
| `audio/system.rs` | 186 | 150 | 36 | 19.4% | ✅ |
| `camera.rs` | 271 | 189 | 82 | 30.3% | ✅ |
| `cli.rs` | 75 | 75 | 0 | 0.0% | ✅ |
| `lib.rs` | 8 | 8 | 0 | 0.0% | ✅ |
| `main.rs` | 255 | 255 | 0 | 0.0% | ⚠️ Large |
| `ocean/mesh.rs` | 231 | 231 | 0 | 0.0% | ⚠️ Large |
| `ocean/mod.rs` | 34 | 17 | 17 | 50.0% | ✅ |
| `ocean/system.rs` | 91 | 67 | 24 | 26.4% | ✅ |
| `params/audio.rs` | 87 | 87 | 0 | 0.0% | ✅ |
| `params/camera.rs` | 233 | 233 | 0 | 0.0% | ⚠️ Large |
| `params/mod.rs` | 17 | 17 | 0 | 0.0% | ✅ |
| `params/ocean.rs` | 84 | 84 | 0 | 0.0% | ✅ |
| `params/render.rs` | 79 | 79 | 0 | 0.0% | ✅ |
| `rendering.rs` | 502 | 502 | 0 | 0.0% | ✅ (infra) |

**⚠️ Warning:** 3 file(s) over 200 impl lines - consider splitting for maintainability

---

## Documentation Files

| File | Lines |
|------|-------|
| `ARCHITECTURE.md` | 353 |
| `CLAUDE.md` | 480 |
| `CODE_MAP.md` | 719 |
| `COVERAGE_REPORT.md` | 68 |
| `FLOWFIELD.md` | 97 |
| `LEARNINGS.md` | 272 |
| `LEXICON.md` | 84 |
| `LOC_REPORT.md` | 99 |
| `README.md` | 124 |
| `VISION.md` | 50 |

---

## Documentation Quality Targets

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Docs/Code Ratio | ≥0.3 | 1.52 | ✅ Excellent |
| README exists | Yes | ✅ | Met |
| ARCHITECTURE.md | Optional | ✅ | Optional |

---

## How to Update This Report

```bash
# Regenerate LOC report
./scripts/generate-loc-report.sh
```

---

*This report is auto-generated from `cloc` and `wc` output.*
*Updated automatically by pre-commit hook when source files change.*
