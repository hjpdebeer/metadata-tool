# Remediation Plan — Code Review + Security Assessment

**Date**: 2026-03-19
**Sources**: Post-compliance code review + comprehensive InfoSec assessment
**Total Findings**: 9 code standards + 20 security = 29 findings

---

## Priority 0 — Immediate (Before Next Feature Work)

### SEC-001: SQL Injection via Dynamic Column Names in AI Suggestion Apply
**Severity**: CRITICAL | **Effort**: 4-8h
**Location**: `api/ai.rs` — `apply_suggestion_to_entity`
**Issue**: `format!()` interpolates `field_name` into SQL column position. Although an allow-list exists, this is inherently unsafe.
**Fix**: Replace `format!()` with exhaustive `match` that uses static SQL strings per column. No user-influenced string in SQL structure.

### SEC-002: Weak Default JWT Secret with No Runtime Validation
**Severity**: CRITICAL | **Effort**: 2h
**Location**: `config.rs`, `.env.example`
**Issue**: Default secret `change-this-to-a-secure-random-string` could reach production. No minimum length check.
**Fix**: Add startup validation: reject if < 32 chars or equals the default. Document: `openssl rand -base64 48`.

### SEC-003: PostgreSQL Exposed with Trust Authentication
**Severity**: CRITICAL | **Effort**: 1h
**Location**: `docker-compose.yml`
**Issue**: `POSTGRES_HOST_AUTH_METHOD: trust` + port exposed to all interfaces.
**Fix**: Remove trust auth, bind port to `127.0.0.1:5432:5432`, require password.

### SEC-004: Dev Credentials Pre-filled in Frontend
**Severity**: CRITICAL (for production) | **Effort**: 2h
**Location**: `frontend/src/pages/LoginPage.tsx`
**Issue**: `admin@example.com` / `metadata123` hardcoded in form `initialValues`.
**Fix**: Gate behind `import.meta.env.DEV` — strip in production builds.

### SEC-008: Overly Permissive CORS (Any Methods/Headers)
**Severity**: HIGH | **Effort**: 1h
**Location**: `main.rs`
**Issue**: `allow_methods(Any)` and `allow_headers(Any)`.
**Fix**: Restrict to `[GET, POST, PUT, DELETE, OPTIONS]` and `[Authorization, Content-Type, Accept]`.

### SEC-012: Database Errors Leak Schema Information
**Severity**: MEDIUM | **Effort**: 2h
**Location**: `error.rs`
**Issue**: Full `sqlx::Error` message returned to client reveals table/column names.
**Fix**: Log full error server-side, return generic "a database error occurred" to client.

---

## Priority 1 — This Sprint (Before Production)

### SEC-005: JWT Tokens in localStorage (XSS Risk)
**Severity**: HIGH | **Effort**: 8-16h
**Issue**: `localStorage` is readable by any JavaScript. XSS → token theft.
**Fix**: Move to `httpOnly` + `SameSite=Strict` cookies set by the backend. Requires changes to auth flow, middleware, and frontend.

### SEC-006: No Rate Limiting on Auth Endpoints
**Severity**: HIGH | **Effort**: 4-8h
**Issue**: Unlimited login attempts enable brute force.
**Fix**: Add `tower-governor` rate limiting (5 req/s per IP on auth endpoints). Account lockout after 10 failed attempts.

### SEC-009: AI API Keys in Cleartext Memory
**Severity**: HIGH | **Effort**: 4h
**Issue**: API keys stored as plain `String` in `AppConfig`, could leak via debug logs.
**Fix**: Use `secrecy::SecretString` wrapper that redacts on `Debug`/`Display`.

### SEC-013: AI Prompt Injection
**Severity**: MEDIUM | **Effort**: 8h
**Issue**: User-provided term names/definitions embedded directly in AI prompts.
**Fix**: Sanitize input (strip instruction-like patterns), add monitoring for unusual AI responses. The field allow-list mitigates the worst case.

### SEC-017: No Security Response Headers
**Severity**: LOW | **Effort**: 2h
**Fix**: Add `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, `Content-Security-Policy`.

### SEC-016: Swagger UI Exposed Without Auth
**Severity**: LOW | **Effort**: 1h
**Fix**: Disable in production via `cfg!(debug_assertions)` check, or add basic auth.

---

## Priority 2 — Next Sprint

### SEC-010: Insecure Default Database Password
**Effort**: 1h | **Fix**: Document strong password requirement, add validation.

### SEC-011: No JWT Token Refresh Mechanism
**Effort**: 16-24h | **Fix**: Short-lived access tokens (15 min) + refresh tokens with server-side storage.

### SEC-014: Missing Input Validation on Search Parameters
**Effort**: 2h | **Fix**: Max 500 chars on search query parameters.

### SEC-015: SSL Mode Prefer Instead of Require
**Effort**: 1h | **Fix**: Environment-based SSL mode (`Prefer` for dev, `Require` for production).

### SEC-020: Timing Attack on Authentication
**Effort**: 2h | **Fix**: Always perform dummy password comparison when user not found.

### SEC-019: No Audit Logging for Admin Actions
**Effort**: 8h | **Fix**: Persist role assignments, user updates to `audit_log` table.

---

## Code Standards Findings (from code review)

### CS-001: Missing Clone Derive on ~20 Request Types
**Severity**: Should Fix | **Effort**: 2h
**Fix**: Add `Clone` to all public request types across API modules.

### CS-002: Pagination Inconsistency (lineage uses limit/offset)
**Severity**: Should Fix | **Effort**: 2h
**Fix**: Standardize lineage to use `page/page_size` like all other endpoints.

### CS-003: Hardcoded Workflow State Strings
**Severity**: Should Fix | **Effort**: 2h
**Fix**: Replace string literals with constants from `workflow/mod.rs`.

### CS-004: Test .unwrap() Without Context
**Severity**: Consider | **Effort**: 1h
**Fix**: Use `.expect("description")` in tests for better failure messages.

---

## Summary

| Priority | Findings | Estimated Effort |
|----------|----------|-----------------|
| P0 — Immediate | 6 (4 CRITICAL + 1 HIGH + 1 MEDIUM) | 12-16 hours |
| P1 — This Sprint | 6 (2 HIGH + 2 MEDIUM + 2 LOW) | 27-39 hours |
| P2 — Next Sprint | 6 (all MEDIUM/LOW) | 30-38 hours |
| Code Standards | 4 (all Should Fix/Consider) | 7 hours |
| **Total** | **22 actionable** | **76-100 hours** |

---

## Recommendation

Execute **P0 immediately** — these 6 items take ~12-16 hours and address all CRITICAL findings. The SQL injection fix (SEC-001) and JWT secret validation (SEC-002) are the highest risk. The CORS and error message fixes (SEC-008, SEC-012) are quick wins.

**P1 can be a dedicated security sprint** before any production deployment. The httpOnly cookie migration (SEC-005) is the largest item and has the most architectural impact.

**P2 and Code Standards** can be addressed incrementally alongside feature work.
