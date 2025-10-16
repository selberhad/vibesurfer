# Test Coverage Report

**Last Updated**: 2025-10-16 04:40
**Tool**: cargo-llvm-cov
**Overall Coverage**: **35.58%** lines | **36.84%** regions | **45.61%** functions

## Summary

```
TOTAL                             969               612    36.84%          57                31    45.61%         815               525    35.58%           0                 0         -
```

## Coverage by Module

| Module | Line Coverage | Region Coverage | Functions | Status |
|--------|--------------|-----------------|-----------|--------|
| `audio.rs` | 19.15% | 21.12% | 21.05% | 🔴 Needs Work |
| `camera.rs` | 98.51% | 99.22% | 100.00% | 🟢 Excellent |
| `main.rs` | 0.00% | 0.00% | 0.00% | 🔴 Needs Work |
| `ocean.rs` | 100.00% | 100.00% | 100.00% | 🟢 Excellent |
| `params.rs` | 87.76% | 82.69% | 90.91% | 🟡 Good |
| `rendering.rs` | 0.00% | 0.00% | 0.00% | 🔴 Needs Work |

## Coverage Tiers

### 🟢 Excellent (≥90% lines)
- `camera.rs` - 98.51%
- `ocean.rs` - 100.00%

### 🟡 Good (70-89% lines)
- `params.rs` - 87.76%

### 🟠 Moderate (40-69% lines)

### 🔴 Needs Work (<40% lines)
- `audio.rs` - 19.15%
- `main.rs` - 0.00%
- `rendering.rs` - 0.00%

## Coverage Targets

| Tier | Target | Current | Status |
|------|--------|---------|--------|
| Overall | ≥80% | 35.58% | ⏳ In Progress |
| Critical Paths | ≥95% | Check modules above | Policy |
| New Modules | ≥80% | - | Policy |

## How to Update This Report

```bash
# Regenerate coverage report
./scripts/generate-coverage-report.sh
```

## Quick Commands

```bash
# Run tests with coverage
cargo llvm-cov --html      # Detailed HTML
cargo llvm-cov --summary-only  # Terminal summary

# Update this markdown report
./scripts/generate-coverage-report.sh
```

---

*This report is auto-generated from `cargo llvm-cov` output.*
