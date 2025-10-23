# Lines of Code Report

**Last Updated**: 2025-10-23 18:04
**Tool**: [cloc](https://github.com/AlDanial/cloc) + wc

---

## Overall Summary

| Metric | Rust Code | Documentation (.md) | Total |
|--------|-----------|---------------------|-------|
| **Lines** | 1,728 | 8,863 | 10,591 |
| **Comments** | 410 | - | 410 |
| **Blank Lines** | 383 | - | 383 |
| **Total Lines** | 2,521 | 8,863 | 11,384 |
| **Files** | 18 | 31 | 49 |

**Documentation Ratio**: 5.13 lines of docs per line of code

---

## Rust Code Breakdown

```
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                            18            383            410           1728
WGSL                             4             85             53            274
-------------------------------------------------------------------------------
SUM:                            22            468            463           2002
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
| `camera.rs` | 263 | 181 | 82 | 31.2% | ✅ |
| `cli.rs` | 88 | 88 | 0 | 0.0% | ✅ |
| `lib.rs` | 9 | 9 | 0 | 0.0% | ✅ |
| `main.rs` | 316 | 316 | 0 | 0.0% | ⚠️ Large |
| `noise.rs` | 27 | 27 | 0 | 0.0% | ✅ |
| `ocean/mesh.rs` | 236 | 236 | 0 | 0.0% | ⚠️ Large |
| `ocean/mod.rs` | 34 | 17 | 17 | 50.0% | ✅ |
| `ocean/system.rs` | 91 | 67 | 24 | 26.4% | ✅ |
| `params/audio.rs` | 87 | 87 | 0 | 0.0% | ✅ |
| `params/camera.rs` | 237 | 237 | 0 | 0.0% | ⚠️ Large |
| `params/mod.rs` | 17 | 17 | 0 | 0.0% | ✅ |
| `params/ocean.rs` | 103 | 103 | 0 | 0.0% | ✅ |
| `params/render.rs` | 79 | 79 | 0 | 0.0% | ✅ |
| `rendering.rs` | 634 | 634 | 0 | 0.0% | ✅ (infra) |

**⚠️ Warning:** 3 file(s) over 200 impl lines - consider splitting for maintainability

---

## Documentation Files

| File | Lines |
|------|-------|
| `ARCHITECTURE.md` | 353 |
| `CAMERA_REFACTOR.md` | 475 |
| `CLAUDE.md` | 305 |
| `CODE_MAP.md` | 719 |
| `COVERAGE_REPORT.md` | 68 |
| `FLOWFIELD.md` | 97 |
| `HANDOFF.md` | 194 |
| `LEARNINGS.md` | 272 |
| `learnings/.ddd/0_compute_shaders_complete.md` | 337 |
| `learnings/.ddd/open_questions.md` | 366 |
| `learnings/gpu_compute_fundamentals.md` | 206 |
| `learnings/wgpu_compute_integration.md` | 343 |
| `learnings/wgsl_compute_patterns.md` | 305 |
| `LEXICON.md` | 84 |
| `LOC_REPORT.md` | 120 |
| `README.md` | 124 |
| `REFACTOR_PLAN.md` | 391 |
| `STUDY_PLAN.md` | 255 |
| `toys/toy1_gpu_noise_match/.ddd/LEARNINGS.md` | 271 |
| `toys/toy1_gpu_noise_match/.ddd/PLAN.md` | 366 |
| `toys/toy1_gpu_noise_match/.ddd/SPEC.md` | 277 |
| `toys/toy2_gpu_terrain_pipeline/.ddd/LEARNINGS.md` | 209 |
| `toys/toy2_gpu_terrain_pipeline/.ddd/PLAN.md` | 553 |
| `toys/toy2_gpu_terrain_pipeline/.ddd/SPEC.md` | 349 |
| `toys/toy3_infinite_camera/.ddd/LEARNINGS.md` | 304 |
| `toys/toy3_infinite_camera/.ddd/PLAN.md` | 384 |
| `toys/toy3_infinite_camera/.ddd/SPEC.md` | 347 |
| `toys/toy3_infinite_camera/MATH_REVIEW.md` | 393 |
| `toys/toy4_spherical_chunks/.ddd/LEARNINGS.md` | 171 |
| `toys/toy4_spherical_chunks/.ddd/SPEC.md` | 175 |
| `VISION.md` | 50 |

---

## Documentation Quality Targets

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Docs/Code Ratio | ≥0.3 | 5.13 | ✅ Excellent |
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
