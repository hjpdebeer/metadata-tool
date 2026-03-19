# Remediation Plan V2 — Final Pre-Production Review

**Date**: 2026-03-19
**Sources**: Exhaustive compliance review + comprehensive security analysis
**Previous CRITICALs (SEC-001 to SEC-013)**: ALL VERIFIED FIXED

---

## Summary

| Severity | Count | Effort |
|----------|-------|--------|
| CRITICAL | 0 | — |
| HIGH (Security) | 2 | 3-6h |
| MUST FIX (Compliance) | 2 | 8-12h |
| MEDIUM (Security) | 5 | 12-20h |
| SHOULD FIX (Compliance) | 5 | 8-12h |
| LOW/INFO | 11 | Backlog |
| **Total actionable** | **14** | **~31-50h** |

---

## Tier 1 — Block Production Deployment

### SEC-014: Weak Key Derivation for Settings Encryption (HIGH)
**File**: `settings.rs` lines 24-43
**Issue**: Uses `DefaultHasher` (SipHash) to derive AES-256 key — not cryptographic.
**Fix**: Replace with `argon2` (already in deps) or HKDF-SHA256.
**Effort**: 2-4h

### SEC-015: Weak PRNG for Encryption Nonces (HIGH)
**File**: `settings.rs` lines 103-114
**Issue**: Uses `RandomState::new()` for AES-GCM nonces — not cryptographically secure. Nonce reuse is catastrophic for GCM.
**Fix**: Use `getrandom` crate or `rand::rngs::OsRng`.
**Effort**: 1-2h

### CS-001: ADR-0006 Violation — `#[serde(flatten)]` in 6 Domain Structs (MUST FIX)
**Files**: data_dictionary.rs, applications.rs, processes.rs, workflow.rs, users.rs, lineage.rs
**Issue**: ADR-0006 explicitly prohibits `#[serde(flatten)]`. Glossary was refactored but other domains still use old pattern.
**Fix**: Convert each to flat structs with explicit fields (same pattern as `GlossaryTermDetail`).
**Effort**: 4-6h (mechanical, repetitive)

### CS-002: ADR-0006 Violation — N+1 Queries in Detail Handlers (MUST FIX)
**Files**: api/applications.rs `get_application`, api/processes.rs `get_process`
**Issue**: Multiple sequential queries instead of single JOIN. Violates ADR-0006 Pattern 1.
**Fix**: Single JOIN query resolving all FK lookups (same pattern as glossary `get_term`).
**Effort**: 4-6h

---

## Tier 2 — Before Production Release

### SEC-016: No Rate Limiting on Auth Endpoints (MEDIUM)
**File**: main.rs
**Issue**: `/api/v1/auth/dev-login` allows unlimited brute-force attempts.
**Fix**: Custom middleware with IP-based tracking (tower-governor incompatible with axum 0.8).
**Effort**: 4-8h

### SEC-017: Admin Panel Not Role-Guarded at Frontend Route Level (MEDIUM)
**File**: frontend/src/App.tsx line 137
**Issue**: Non-admin users can navigate to `/admin` and see the UI (API calls fail but layout exposed).
**Fix**: Add `RequireAdmin` route guard component checking `user.roles.includes('ADMIN')`.
**Effort**: 1-2h

### SEC-018: Bulk Upload File Size Not Enforced at Axum Layer (MEDIUM)
**File**: api/bulk_upload.rs
**Issue**: 10MB limit checked after file is fully read into memory.
**Fix**: Add `DefaultBodyLimit::max(10 * 1024 * 1024)` layer to the upload route.
**Effort**: 1-2h

### SEC-019: JWT_SECRET Reused as Settings Encryption Key Fallback (MEDIUM)
**File**: db/mod.rs lines 18-19
**Issue**: Key reuse between JWT signing and settings encryption.
**Fix**: Require separate `SETTINGS_ENCRYPTION_KEY` env var in production.
**Effort**: 1-2h

### SEC-020: JWT Tokens in localStorage (MEDIUM)
**File**: frontend/src/services/api.ts
**Issue**: XSS → token theft. Defense-in-depth concern (no XSS vectors found).
**Fix**: Move to httpOnly cookies (significant refactoring).
**Effort**: 4-8h

### CS-003: Missing Audit Logging for Bulk Upload (SHOULD FIX)
**File**: api/bulk_upload.rs
**Issue**: Bulk-created terms bypass audit_log table. Violates Principle 9.
**Fix**: Add audit log entry after each successful row insertion.
**Effort**: 2h

### CS-004: Hardcoded IPv4 for Anthropic API (SHOULD FIX)
**File**: ai/mod.rs lines 311-318
**Issue**: `160.79.104.10` hardcoded — breaks if Anthropic changes DNS.
**Fix**: Document as workaround, add comment with fallback instructions.
**Effort**: 30min

### CS-005: Missing Doc Comments on New Public Functions (SHOULD FIX)
**Files**: api/bulk_upload.rs, api/admin.rs
**Issue**: New functions from bulk upload and admin panel lack `///` docs.
**Fix**: Add doc comments to all public functions.
**Effort**: 2h

### CS-006: `too_many_arguments` Suppression in bulk_upload (SHOULD FIX)
**File**: api/bulk_upload.rs line 533
**Issue**: `process_row` has 8 parameters.
**Fix**: Extract to `BulkUploadContext` struct.
**Effort**: 1h

### CS-007: Inconsistent Error Message Formatting (SHOULD FIX)
**Files**: Various API files
**Issue**: Some use "Application not found" vs "application not found".
**Fix**: Standardize to lowercase (grep + fix).
**Effort**: 1h

---

## Tier 3 — Backlog / Defense-in-Depth

| ID | Finding | Effort |
|----|---------|--------|
| SEC-021 | Dev-login check accepts placeholder tenant ID | 30min |
| SEC-022 | DB password in docker-compose.yml | Documentation |
| SEC-023 | Swagger UI exposed without auth | 1h |
| SEC-024 | No Content-Type validation on bulk upload | 30min |
| SEC-025 | Missing `#[validate]` derive on request types | 2-4h |
| SEC-026 | Argon2 in deps but bcrypt used for passwords | 4-8h |
| SEC-027 | Potential zip bomb via malicious xlsx | Monitoring |
| CS-008 | Shared PaginationParams type | 2h |
| CS-009 | Test coverage gaps (0% on API handlers) | 16-24h |
| CS-010 | Frontend missing React Error Boundary | 1h |
| CS-011 | useCallback dependency optimization | 30min |

---

## Execution Recommendation

**Phase 1 (Immediate — 1-2 days):**
- SEC-014 + SEC-015: Fix cryptographic weaknesses in settings.rs
- SEC-017: Add RequireAdmin route guard (quick win)
- SEC-018: Add body limit layer to upload route (quick win)
- SEC-019: Require separate encryption key (quick win)

**Phase 2 (This sprint — 2-3 days):**
- CS-001 + CS-002: Refactor remaining domains to ADR-0006 (flat structs + single JOIN)
- CS-003: Add audit logging to bulk upload
- CS-005 + CS-007: Doc comments and error message cleanup

**Phase 3 (Next sprint):**
- SEC-016: Rate limiting
- SEC-020: httpOnly cookies
- Tier 3 backlog items

---

## Previously Fixed (Verified)

| Finding | Status |
|---------|--------|
| SEC-001: SQL injection in AI apply | ✅ Exhaustive match, static SQL |
| SEC-002: JWT secret validation | ✅ Rejects < 32 chars and default |
| SEC-003: PostgreSQL trust auth | ✅ Removed, localhost only |
| SEC-004: Dev credentials in UI | ✅ Gated behind DEV env |
| SEC-008: CORS configuration | ✅ Restricted methods/headers |
| SEC-012: Database error leakage | ✅ Generic messages to client |
| SEC-013: Prompt injection | ✅ Sanitization + length limit + tests |
