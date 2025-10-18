# Lines of Code Report

**Last Updated**: 2025-10-18 15:52
**Tool**: [cloc](https://github.com/AlDanial/cloc) + wc

---

## Overall Summary

| Metric | Rust Code | Documentation (.md) | Total |
|--------|-----------|---------------------|-------|
| **Lines** | 1,580 | 6,859 | 8,439 |
| **Comments** | 395 | - | 395 |
| **Blank Lines** | 356 | - | 356 |
| **Total Lines** | 2,331 | 6,859 | 9,190 |
| **Files** | 18 | 22 | 40 |

**Documentation Ratio**: 4.34 lines of docs per line of code

---

## Rust Code Breakdown

```
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                            18            356            395           1580
WGSL                             3             49             26            169
-------------------------------------------------------------------------------
SUM:                            21            405            421           1749
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
| `camera.rs` | 278 | 196 | 82 | 29.5% | ✅ |
| `cli.rs` | 75 | 75 | 0 | 0.0% | ✅ |
| `lib.rs` | 9 | 9 | 0 | 0.0% | ✅ |
| `main.rs` | 280 | 280 | 0 | 0.0% | ⚠️ Large |
| `noise.rs` | 27 | 27 | 0 | 0.0% | ✅ |
| `ocean/mesh.rs` | 231 | 231 | 0 | 0.0% | ⚠️ Large |
| `ocean/mod.rs` | 34 | 17 | 17 | 50.0% | ✅ |
| `ocean/system.rs` | 91 | 67 | 24 | 26.4% | ✅ |
| `params/audio.rs` | 87 | 87 | 0 | 0.0% | ✅ |
| `params/camera.rs` | 237 | 237 | 0 | 0.0% | ⚠️ Large |
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
| `CLAUDE.md` | 305 |
| `CODE_MAP.md` | 719 |
| `COVERAGE_REPORT.md` | 68 |
| `FLOWFIELD.md` | 97 |
| `HANDOFF.md` | 280 |
| `LEARNINGS.md` | 272 |
| `learnings/.ddd/0_compute_shaders_complete.md` | 337 |
| `learnings/.ddd/open_questions.md` | 362 |
| `learnings/gpu_compute_fundamentals.md` | 330 |
| `learnings/wgpu_compute_integration.md` | 736 |
| `learnings/wgsl_compute_patterns.md` | 559 |
| `LEXICON.md` | 84 |
| `LOC_REPORT.md` | 112 |
| `README.md` | 124 |
| `STUDY_PLAN.md` | 255 |
| `toys/toy1_gpu_noise_match/.ddd/LEARNINGS.md` | 271 |
| `toys/toy1_gpu_noise_match/.ddd/PLAN.md` | 366 |
| `toys/toy1_gpu_noise_match/.ddd/SPEC.md` | 277 |
| `toys/toy2_gpu_terrain_pipeline/.ddd/LEARNINGS.md` | 209 |
| `toys/toy2_gpu_terrain_pipeline/.ddd/PLAN.md` | 553 |
| `toys/toy2_gpu_terrain_pipeline/.ddd/SPEC.md` | 349 |
| `VISION.md` | 50 |

---

## Documentation Quality Targets

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Docs/Code Ratio | ≥0.3 | 4.34 | ✅ Excellent |
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
