# Implementation Plan: Bulk Upload and Admin Panel

**Version**: 1.0
**Date**: 2026-03-19
**Author**: Claude (Opus 4.5) on behalf of Hendrik de Beer
**Status**: Draft for Review

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Feature 1: Bulk Upload of Glossary Terms](#2-feature-1-bulk-upload-of-glossary-terms)
   - [2.1 Architecture Decisions](#21-architecture-decisions)
   - [2.2 Template Design](#22-template-design)
   - [2.3 API Endpoint Specifications](#23-api-endpoint-specifications)
   - [2.4 Frontend Component Design](#24-frontend-component-design)
   - [2.5 Work Items](#25-work-items)
   - [2.6 Acceptance Criteria](#26-acceptance-criteria)
3. [Feature 2: Admin Panel](#3-feature-2-admin-panel)
   - [3.1 Architecture Decisions](#31-architecture-decisions)
   - [3.2 Section Specifications](#32-section-specifications)
   - [3.3 API Endpoint Specifications](#33-api-endpoint-specifications)
   - [3.4 Frontend Layout Design](#34-frontend-layout-design)
   - [3.5 Work Items](#35-work-items)
   - [3.6 Acceptance Criteria](#36-acceptance-criteria)
4. [Execution Order](#4-execution-order)
5. [Dependencies](#5-dependencies)
6. [Risk Register](#6-risk-register)
7. [Appendix A: Excel Template Column Specification](#appendix-a-excel-template-column-specification)
8. [Appendix B: System Settings Schema](#appendix-b-system-settings-schema)

---

## 1. Executive Summary

This document specifies the implementation plan for two major features:

1. **Bulk Upload**: Enable users to upload multiple glossary terms via an Excel template, reducing manual data entry time and improving onboarding efficiency for enterprise deployments.

2. **Admin Panel**: Provide administrators with a centralised interface to manage lookup tables, system configuration, users, and notifications without requiring server restarts or database access.

**Estimated Total Effort**: 12-15 person-days (assuming familiarity with the codebase)

---

## 2. Feature 1: Bulk Upload of Glossary Terms

### 2.1 Architecture Decisions

#### 2.1.1 Crate Selection

| Purpose | Selected Crate | Rationale |
|---------|---------------|-----------|
| Excel parsing | `calamine` 0.28 | Read-only, lightweight, well-maintained. Supports .xlsx natively. 2.7M downloads/month on crates.io. |
| Excel writing | `rust_xlsxwriter` 0.84 | Feature-rich, supports data validation, conditional formatting, named ranges. Actively maintained by the same author as Python XlsxWriter. |

**Alternative considered**: `umya-spreadsheet` (read/write combined) was rejected because it has slower parse times for large files and a less mature API for data validation.

#### 2.1.2 Transaction Strategy

**Decision**: Partial success with detailed error report.

**Rationale**:
- Financial institutions often have hundreds of terms to upload. Failing the entire batch on a single error is too punishing.
- Each successfully uploaded row receives its own workflow instance (DRAFT status).
- Failed rows are reported with specific error details; users can fix and re-upload the failed subset.
- This aligns with Principle 6 (AI-Assisted, Human-Governed) by giving users control over corrections.

**Implementation**:
- Parse entire file into memory first (validation pass).
- For each row passing validation, INSERT within an individual transaction scope.
- Collect successes and failures; return summary to frontend.

#### 2.1.3 Error Handling

| Error Type | Response | HTTP Status |
|------------|----------|-------------|
| File too large (>10 MB) | `{ "error": { "code": "PAYLOAD_TOO_LARGE", "message": "..." } }` | 413 |
| Invalid file format | `{ "error": { "code": "INVALID_FILE_FORMAT", "message": "..." } }` | 422 |
| Parsing error | `{ "error": { "code": "PARSE_ERROR", "message": "..." } }` | 422 |
| Partial success | `{ "total_rows": N, "successful": X, "failed": Y, "errors": [...] }` | 200 |
| No valid rows | `{ "total_rows": N, "successful": 0, "failed": N, "errors": [...] }` | 200 |

#### 2.1.4 Constraints

| Constraint | Value | Rationale |
|------------|-------|-----------|
| Max file size | 10 MB | Prevents memory exhaustion; 10MB can hold ~50,000 rows |
| Max rows per upload | 1,000 | Balances performance with practicality; larger batches should be split |
| Max concurrent uploads | 1 per user | Prevents duplicate term creation race conditions |

### 2.2 Template Design

The Excel template (.xlsx) contains three sheets.

#### Sheet 1: Terms (Data Entry)

All columns correspond to the 45-field glossary specification. See [Appendix A](#appendix-a-excel-template-column-specification) for the complete column list.

**Column header format**: Display name exactly matching UI labels (e.g., "Term Name", "Definition", "Domain").

**Mandatory columns** (highlighted yellow):
- Term Name
- Definition
- Business Term Owner (email)
- Data Steward (email)
- Data Domain Owner (email)
- Approver (email)
- Organisational Unit

**Dropdown columns** (data validation lists referencing Valid Values sheet):
- Domain
- Category
- Data Classification
- Term Type
- Unit of Measure
- Review Frequency
- Confidence Level
- Visibility
- Language
- Organisational Unit
- Regulatory Tags (multi-select not natively supported; comma-separated)
- Subject Areas (comma-separated)

#### Sheet 2: Instructions

| Column | Description | Mandatory | Max Length | Notes |
|--------|-------------|-----------|------------|-------|
| Term Name | The official name of the business term | Yes | 256 | Must be unique |
| Definition | Clear, unambiguous definition | Yes | 4000 | Plain text only |
| ... | ... | ... | ... | ... |

Full table with all 45 fields, each with:
- Description
- Mandatory/Optional
- Maximum character length
- Allowed values or format requirements
- Example value

#### Sheet 3: Valid Values

Named ranges for each lookup table:

| Named Range | Values |
|-------------|--------|
| `Domains` | Customer, Account, Transaction, Product, Risk, Compliance, Operations, Financial Reporting |
| `Categories` | Core Identifier, Attribute, Measure, Reference, Classification, Relationship |
| `TermTypes` | KPI / Financial Metric, Business Concept, Regulatory Term, Technical Term, Process Term, Product Term, Risk Term, Compliance Term |
| `Classifications` | Public, Internal, Confidential, Restricted |
| `UnitsOfMeasure` | Percentage, Currency, Count, Ratio, Days, Months, Years, Boolean, Text, Date, Date and Time, Rate, Score, Index |
| `ReviewFrequencies` | Monthly, Quarterly, Semi-Annual, Annual, Biennial |
| `ConfidenceLevels` | High, Medium, Low |
| `VisibilityLevels` | Enterprise-Wide, Domain-Specific, Restricted |
| `Languages` | English, Arabic, French, German, Spanish, Chinese, Hindi, Portuguese |
| `OrganisationalUnits` | (All from organisational_units table) |
| `RegulatoryTags` | BCBS 239, IFRS 9, IFRS 17, Basel III, FATCA, CRS, GDPR, PCI DSS, SOX, AML/CFT, MiFID II, DORA, Local Regulation |
| `SubjectAreas` | Retail Banking, Corporate Banking, Investment Banking, Wealth Management, Treasury, Risk Management, Compliance, Finance, Operations, Technology, Human Resources, Legal, Audit |

### 2.3 API Endpoint Specifications

#### 2.3.1 Download Template

```
GET /api/v1/glossary/terms/bulk-upload/template
```

**Authentication**: Required (any authenticated user)

**Response**:
- Content-Type: `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`
- Content-Disposition: `attachment; filename="glossary_term_template.xlsx"`
- Body: Binary Excel file

**Implementation notes**:
- Template is generated dynamically (not a static file).
- Lookup values are fetched from the database at request time to ensure freshness.
- Template generation is idempotent and cacheable for short periods (e.g., 5 minutes).

#### 2.3.2 Bulk Upload

```
POST /api/v1/glossary/terms/bulk-upload
```

**Authentication**: Required

**Request**:
- Content-Type: `multipart/form-data`
- Body: Single file upload with field name `file`

**Response** (200 OK):
```json
{
  "total_rows": 150,
  "successful": 147,
  "failed": 3,
  "errors": [
    {
      "row": 23,
      "field": "Domain",
      "message": "Invalid domain 'CustomerData'. Valid values: Customer, Account, Transaction, Product, Risk, Compliance, Operations, Financial Reporting"
    },
    {
      "row": 45,
      "field": "Business Term Owner",
      "message": "User with email 'unknown@example.com' not found"
    },
    {
      "row": 45,
      "field": "Term Name",
      "message": "Term name is required"
    }
  ],
  "created_term_ids": ["uuid1", "uuid2", ...]
}
```

**Processing logic**:

1. **Validate file**:
   - Check file extension is `.xlsx`
   - Check file size <= 10 MB
   - Attempt to open as Excel workbook

2. **Parse "Terms" sheet**:
   - Read header row (row 1)
   - Validate header columns match expected template
   - Read data rows (row 2 onwards), stop at first empty row

3. **For each row**:
   a. Validate mandatory fields are present and non-empty
   b. Resolve lookup display names to UUIDs using the same `resolve_lookup()` pattern as AI enrichment (CODING_STANDARDS Section 15.6):
      - First try exact match on display name
      - Fall back to case-insensitive match
      - If no match, record error
   c. Resolve user fields (owner, steward, domain_owner, approver) by email:
      - Look up user by email in `users` table
      - If not found, record error
   d. Validate naming standards where applicable
   e. Validate field lengths (e.g., abbreviation <= 50 chars)

4. **For each valid row**:
   a. Fetch DRAFT status_id
   b. Fetch default review_frequency_id (ANNUAL)
   c. INSERT into glossary_terms
   d. Insert junction records for regulatory_tags and subject_areas (comma-separated parsing)
   e. Call `workflow::service::initiate_workflow()`
   f. **Do NOT trigger AI enrichment** (explicit requirement)

5. **Return summary**

### 2.4 Frontend Component Design

#### 2.4.1 Component: BulkUploadButton

Location: `frontend/src/components/BulkUploadButton.tsx`

A button that opens the bulk upload modal. Placed on GlossaryPage next to "New Term".

```tsx
<Space>
  <BulkUploadButton onSuccess={handleBulkUploadSuccess} />
  <Button type="primary" icon={<PlusOutlined />} onClick={() => navigate('/glossary/new')}>
    New Term
  </Button>
</Space>
```

#### 2.4.2 Component: BulkUploadModal

Location: `frontend/src/components/BulkUploadModal.tsx`

Modal/Drawer with:

1. **Header**: "Bulk Upload Glossary Terms"

2. **Section 1: Download Template**
   - Text: "Download the Excel template with instructions and valid dropdown values."
   - Button: "Download Template" (triggers GET /template)
   - On click: opens browser download

3. **Section 2: Upload File**
   - Ant Design `<Upload.Dragger>` component
   - Accept: `.xlsx` only
   - Max size display: "Max file size: 10 MB, Max rows: 1,000"
   - Upload state: idle / uploading / complete / error

4. **Section 3: Results** (visible after upload completes)
   - Summary: "147 of 150 terms uploaded successfully"
   - Success message with green checkmark if all succeeded
   - Error table (if any errors):
     - Columns: Row, Field, Error Message
     - Sortable and filterable
     - Export errors as CSV button (for large error lists)
   - Action buttons:
     - "Close" (always)
     - "View Created Terms" (navigates to glossary list, filtered to show recent)

#### 2.4.3 Service: glossaryApi additions

```typescript
// In frontend/src/services/glossaryApi.ts

export interface BulkUploadError {
  row: number;
  field: string;
  message: string;
}

export interface BulkUploadResult {
  total_rows: number;
  successful: number;
  failed: number;
  errors: BulkUploadError[];
  created_term_ids: string[];
}

export const glossaryApi = {
  // ... existing methods ...

  downloadBulkUploadTemplate(): Promise<void> {
    return api.get('/glossary/terms/bulk-upload/template', {
      responseType: 'blob',
    }).then((response) => {
      const url = window.URL.createObjectURL(new Blob([response.data]));
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', 'glossary_term_template.xlsx');
      document.body.appendChild(link);
      link.click();
      link.remove();
    });
  },

  uploadBulkTerms(file: File): Promise<AxiosResponse<BulkUploadResult>> {
    const formData = new FormData();
    formData.append('file', file);
    return api.post('/glossary/terms/bulk-upload', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    });
  },
};
```

### 2.5 Work Items

| ID | Task | Estimate | Dependencies |
|----|------|----------|--------------|
| BU-01 | Add `calamine` and `rust_xlsxwriter` to workspace dependencies | 0.5h | - |
| BU-02 | Create `backend/src/api/bulk_upload.rs` with template generation endpoint | 4h | BU-01 |
| BU-03 | Implement dynamic template generation with all lookup tables | 3h | BU-02 |
| BU-04 | Implement Excel parsing logic for Terms sheet | 4h | BU-01 |
| BU-05 | Implement row validation and lookup resolution | 4h | BU-04 |
| BU-06 | Implement term creation with workflow initiation (no AI) | 3h | BU-05 |
| BU-07 | Implement junction table handling (regulatory_tags, subject_areas) | 2h | BU-06 |
| BU-08 | Add utoipa OpenAPI annotations to new endpoints | 1h | BU-02, BU-06 |
| BU-09 | Register new routes in `main.rs` | 0.5h | BU-08 |
| BU-10 | Write unit tests for Excel parsing | 2h | BU-04 |
| BU-11 | Write unit tests for lookup resolution | 1h | BU-05 |
| BU-12 | Create `BulkUploadButton` component | 1h | - |
| BU-13 | Create `BulkUploadModal` component with template download | 2h | BU-12 |
| BU-14 | Implement file upload with progress and error handling | 3h | BU-13 |
| BU-15 | Implement results display with error table | 2h | BU-14 |
| BU-16 | Add glossaryApi service methods | 1h | - |
| BU-17 | Integration test: end-to-end upload flow | 2h | All |
| **Total** | | **~36h (4.5 days)** | |

### 2.6 Acceptance Criteria

#### AC-BU-01: Template Download
- **Given** an authenticated user on the Glossary page
- **When** they click "Download Template"
- **Then** an Excel file downloads with three sheets: Terms, Instructions, Valid Values

#### AC-BU-02: Template Freshness
- **Given** an admin has added a new Domain "Analytics" via the Admin Panel
- **When** a user downloads the template
- **Then** "Analytics" appears in the Valid Values sheet and Domain dropdown

#### AC-BU-03: Successful Upload
- **Given** a user uploads a valid template with 50 rows
- **When** all rows pass validation
- **Then** 50 terms are created in DRAFT status, 50 workflow instances are initiated, and the response shows `successful: 50, failed: 0`

#### AC-BU-04: Partial Failure
- **Given** a user uploads a template with 10 rows, 2 of which have invalid Domain values
- **When** the upload completes
- **Then** 8 terms are created, 2 errors are returned with row numbers and field names, and the frontend displays the error table

#### AC-BU-05: Mandatory Field Validation
- **Given** a user uploads a row with empty "Definition"
- **When** the upload processes that row
- **Then** the row fails with error "Definition is required" and no term is created

#### AC-BU-06: User Resolution
- **Given** a user uploads a row with "Business Term Owner" = "alice@example.com"
- **When** alice@example.com exists in the users table
- **Then** the term is created with `owner_user_id` set to Alice's UUID

#### AC-BU-07: No AI Enrichment
- **Given** terms are created via bulk upload
- **When** the upload completes
- **Then** no AI enrichment is triggered and no `ai_suggestions` records are created

#### AC-BU-08: File Size Limit
- **Given** a user attempts to upload a file > 10 MB
- **When** the request is sent
- **Then** HTTP 413 is returned with error "File exceeds maximum size of 10 MB"

#### AC-BU-09: Row Limit
- **Given** a user uploads a template with 1,500 rows
- **When** the upload processes
- **Then** only the first 1,000 rows are processed and a warning is included in the response

---

## 3. Feature 2: Admin Panel

### 3.1 Architecture Decisions

#### 3.1.1 Settings Storage

**Decision**: Store system settings in a `system_settings` database table with encryption for sensitive values.

**Rationale**:
- Allows configuration changes without server restarts.
- Enables audit trail for configuration changes.
- Separates sensitive values (API keys, secrets) from non-sensitive values.
- Aligns with Principle 9 (Audit Everything).

**Table design**:
```sql
CREATE TABLE system_settings (
    setting_key   VARCHAR(128) PRIMARY KEY,
    setting_value TEXT NOT NULL,
    is_encrypted  BOOLEAN NOT NULL DEFAULT FALSE,
    category      VARCHAR(64) NOT NULL,
    display_name  VARCHAR(256) NOT NULL,
    description   TEXT,
    validation_regex VARCHAR(512),
    updated_by    UUID REFERENCES users(user_id),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

#### 3.1.2 Encryption Strategy

**Decision**: Use AES-256-GCM with a key derived from `JWT_SECRET` (or a separate `SETTINGS_ENCRYPTION_KEY` environment variable if provided).

**Rationale**:
- AES-256-GCM provides authenticated encryption (prevents tampering).
- Deriving from JWT_SECRET avoids adding another mandatory env var.
- The key never leaves the server; encrypted values are useless if extracted from the database.

**Implementation**:
- Use the `aes-gcm` crate for encryption/decryption.
- Each encrypted value includes a random 12-byte nonce prepended to the ciphertext.
- On read, the backend decrypts before returning to the admin UI.
- On write, the backend encrypts before storing.

**Note**: The API will NEVER return the raw decrypted value of secrets to the frontend. It returns a masked value (e.g., `sk-ant-...****...3x9f`) for display purposes only. Full values are only used server-side.

#### 3.1.3 Configuration Reload

**Decision**: Hot-reload configuration from database on every AI call or authenticated request (with 60-second cache).

**Rationale**:
- Immediate effect is expected when changing API keys.
- 60-second cache prevents excessive DB queries.
- Cache invalidation is triggered on settings update.

**Implementation**:
- Add `SettingsCache` to `AppState` (wrapped in `RwLock`).
- On settings GET/PUT, invalidate cache.
- AI module reads API keys from cache, falling back to env vars if not set in DB.

### 3.2 Section Specifications

#### 3.2.1 Lookup Table Management

**Tables to manage** (13 total):

| Table | Key Columns | Notes |
|-------|-------------|-------|
| `glossary_domains` | domain_id, domain_code, domain_name, description | domain_code required for term_code generation |
| `glossary_categories` | category_id, category_name, description | |
| `glossary_term_types` | term_type_id, type_code, type_name, description | |
| `data_classifications` | classification_id, classification_code, classification_name | Shared with data_dictionary |
| `glossary_units_of_measure` | unit_id, unit_code, unit_name, unit_symbol | |
| `glossary_review_frequencies` | frequency_id, frequency_code, frequency_name, months_interval | |
| `glossary_confidence_levels` | confidence_id, level_code, level_name | |
| `glossary_visibility_levels` | visibility_id, visibility_code, visibility_name | |
| `glossary_languages` | language_id, language_code, language_name | |
| `glossary_regulatory_tags` | tag_id, tag_code, tag_name, jurisdiction | |
| `glossary_subject_areas` | subject_area_id, area_code, area_name | |
| `glossary_tags` | tag_id, tag_name | User-created, freeform |
| `organisational_units` | unit_id, unit_code, unit_name, parent_unit_id | Hierarchical |

**Operations per table**:
- List (GET with pagination and search)
- Create (POST)
- Update (PUT)
- Delete (DELETE with usage check)

**Delete protection**: Before deleting a lookup value, check if it is referenced by any entity. If in use, return 409 Conflict with the count of referencing entities.

#### 3.2.2 System Configuration

**Settings categories and keys**:

| Category | Key | Display Name | Encrypted | Validation |
|----------|-----|--------------|-----------|------------|
| AI | `anthropic_api_key` | Anthropic API Key | Yes | Starts with `sk-ant-` |
| AI | `anthropic_model` | Anthropic Model | No | `claude-3-5-sonnet-latest` or `claude-3-5-haiku-latest` |
| AI | `openai_api_key` | OpenAI API Key | Yes | Starts with `sk-` |
| AI | `openai_model` | OpenAI Model | No | Model identifier |
| Auth | `jwt_secret` | JWT Secret | Yes | Min 32 characters |
| Auth | `entra_tenant_id` | Entra Tenant ID | No | UUID format |
| Auth | `entra_client_id` | Entra Client ID | No | UUID format |
| Auth | `entra_client_secret` | Entra Client Secret | Yes | Non-empty |
| Auth | `entra_redirect_uri` | Entra Redirect URI | No | Valid URL |
| Email | `graph_client_id` | Graph Client ID | No | UUID format |
| Email | `graph_client_secret` | Graph Client Secret | Yes | Non-empty |
| Email | `graph_tenant_id` | Graph Tenant ID | No | UUID format |
| Email | `notification_sender_email` | Notification Sender | No | Valid email |
| App | `frontend_url` | Frontend URL | No | Valid URL |
| App | `default_review_frequency` | Default Review Frequency | No | Lookup code |

**Test Connection buttons**:
- **Anthropic**: Sends a minimal API request (list models) to verify the key is valid.
- **OpenAI**: Sends a minimal API request (list models) to verify the key is valid.
- **Microsoft Graph**: Attempts to get organization info to verify credentials.

#### 3.2.3 User Management

**Already implemented at** `/admin/users` (see `UserManagementPage.tsx`). Will be integrated into the Admin Panel as a tab.

**Existing functionality**:
- List users with search and role filter
- View user details
- Activate/deactivate users
- Assign/remove roles

**No additional functionality required** for this phase.

#### 3.2.4 Notification Preferences

**Already implemented at** `/admin/notifications` (see `NotificationPreferencesPage.tsx`). Will be integrated into the Admin Panel as a tab.

**Existing functionality**:
- Configure which events trigger notifications
- Set notification channels (in-app, email)
- Define templates for notification content

**No additional functionality required** for this phase.

### 3.3 API Endpoint Specifications

#### 3.3.1 Lookup Table CRUD

For each lookup table, the pattern is:

```
GET    /api/v1/admin/lookups/{table_name}                    # List with pagination
POST   /api/v1/admin/lookups/{table_name}                    # Create
PUT    /api/v1/admin/lookups/{table_name}/{id}               # Update
DELETE /api/v1/admin/lookups/{table_name}/{id}               # Delete with usage check
GET    /api/v1/admin/lookups/{table_name}/{id}/usage-count   # Check usage before delete
```

**Table name mappings** (kebab-case in URL):

| URL Segment | DB Table |
|-------------|----------|
| `domains` | `glossary_domains` |
| `categories` | `glossary_categories` |
| `term-types` | `glossary_term_types` |
| `classifications` | `data_classifications` |
| `units-of-measure` | `glossary_units_of_measure` |
| `review-frequencies` | `glossary_review_frequencies` |
| `confidence-levels` | `glossary_confidence_levels` |
| `visibility-levels` | `glossary_visibility_levels` |
| `languages` | `glossary_languages` |
| `regulatory-tags` | `glossary_regulatory_tags` |
| `subject-areas` | `glossary_subject_areas` |
| `tags` | `glossary_tags` |
| `organisational-units` | `organisational_units` |

**All endpoints require ADMIN role.**

**Example: Create Domain**

```
POST /api/v1/admin/lookups/domains
```

Request:
```json
{
  "domain_code": "ANALYTICS",
  "domain_name": "Analytics",
  "description": "Data analytics and reporting domain"
}
```

Response (201 Created):
```json
{
  "domain_id": "uuid",
  "domain_code": "ANALYTICS",
  "domain_name": "Analytics",
  "description": "Data analytics and reporting domain"
}
```

**Example: Delete with Usage Check**

```
DELETE /api/v1/admin/lookups/domains/{domain_id}
```

Response (409 Conflict if in use):
```json
{
  "error": {
    "code": "IN_USE",
    "message": "Cannot delete domain 'Customer': it is referenced by 42 glossary terms"
  }
}
```

#### 3.3.2 System Settings

```
GET  /api/v1/admin/settings                        # List all settings (values masked for encrypted)
GET  /api/v1/admin/settings/{key}                  # Get single setting (masked if encrypted)
PUT  /api/v1/admin/settings/{key}                  # Update setting value
POST /api/v1/admin/settings/test-connection/{key}  # Test connection for API keys
```

**All endpoints require ADMIN role.**

**Example: Get Settings**

```
GET /api/v1/admin/settings
```

Response:
```json
{
  "settings": [
    {
      "key": "anthropic_api_key",
      "value": "sk-ant-...****...3x9f",
      "is_encrypted": true,
      "category": "AI",
      "display_name": "Anthropic API Key",
      "description": "API key for Claude AI enrichment",
      "is_set": true,
      "updated_at": "2026-03-19T10:00:00Z",
      "updated_by_name": "Hendrik de Beer"
    },
    {
      "key": "frontend_url",
      "value": "https://metadata.example.com",
      "is_encrypted": false,
      "category": "App",
      "display_name": "Frontend URL",
      "is_set": true,
      "updated_at": "2026-03-19T10:00:00Z",
      "updated_by_name": "Hendrik de Beer"
    }
  ]
}
```

**Example: Update Setting**

```
PUT /api/v1/admin/settings/anthropic_api_key
```

Request:
```json
{
  "value": "sk-ant-api03-newkey..."
}
```

Response:
```json
{
  "key": "anthropic_api_key",
  "is_set": true,
  "updated_at": "2026-03-19T10:05:00Z"
}
```

**Example: Test Connection**

```
POST /api/v1/admin/settings/test-connection/anthropic_api_key
```

Response (success):
```json
{
  "success": true,
  "message": "Successfully connected to Anthropic API"
}
```

Response (failure):
```json
{
  "success": false,
  "message": "Authentication failed: Invalid API key"
}
```

### 3.4 Frontend Layout Design

#### 3.4.1 Route: /admin

**Component**: `AdminPanel.tsx`

**Layout**: Ant Design `Tabs` component with vertical tabs on the left.

```
+---------------------------------------------------------------+
| Admin Panel                                                    |
+---------------+-----------------------------------------------+
|               |                                               |
| Lookup Tables | [Content area changes based on selected tab]  |
|               |                                               |
| System Config |                                               |
|               |                                               |
| Users         |                                               |
|               |                                               |
| Notifications |                                               |
|               |                                               |
+---------------+-----------------------------------------------+
```

#### 3.4.2 Tab: Lookup Tables

**Component**: `AdminLookupTables.tsx`

**Layout**: Left sidebar with table list, right content area with table view.

```
+---------------------------------------------------------------+
| [Search box]                      + Add New                    |
+------------------+--------------------------------------------+
| Domains          | Domains                                    |
| Categories       +--------------------------------------------+
| Term Types       | [Table with columns:]                     |
| Classifications  | Code | Name | Description | Actions        |
| Units of Measure | CUS  | Customer | Customer... | Edit Delete|
| Review Freq.     | ACC  | Account  | Account...  | Edit Delete|
| Confidence       | ...  | ...      | ...         | ...        |
| Visibility       +--------------------------------------------+
| Languages        | [Pagination]                               |
| Regulatory Tags  |                                            |
| Subject Areas    |                                            |
| Tags             |                                            |
| Org Units        |                                            |
+------------------+--------------------------------------------+
```

**Interactions**:
- Click table name in sidebar to view its contents
- "Add New" opens a modal form
- "Edit" opens same modal in edit mode
- "Delete" shows confirmation with usage count

#### 3.4.3 Tab: System Configuration

**Component**: `AdminSystemConfig.tsx`

**Layout**: Grouped by category with collapsible sections.

```
+---------------------------------------------------------------+
| System Configuration                                           |
+---------------------------------------------------------------+
| v AI Configuration                                             |
|   +-----------------------------------------------------------+
|   | Anthropic API Key                                          |
|   | [sk-ant-...****...3x9f] [Reveal] [Test Connection]         |
|   |                                                            |
|   | Anthropic Model                                            |
|   | [claude-3-5-sonnet-latest        v]                        |
|   |                                                            |
|   | OpenAI API Key                                             |
|   | [Not configured] [Set Value] [Test Connection]             |
|   +-----------------------------------------------------------+
|                                                                |
| v Authentication                                               |
|   +-----------------------------------------------------------+
|   | JWT Secret                                                 |
|   | [********************] [Rotate]                            |
|   |                                                            |
|   | Entra Tenant ID                                            |
|   | [00000000-0000-0000-0000-000000000000]                     |
|   +-----------------------------------------------------------+
|                                                                |
| v Email Notifications                                          |
|   ...                                                          |
|                                                                |
| v Application                                                  |
|   ...                                                          |
+---------------------------------------------------------------+
```

**Interactions**:
- "Reveal" temporarily shows the full value (timeout after 30 seconds)
- "Test Connection" sends a test request and shows success/failure toast
- "Set Value" / "Edit" opens an input modal
- "Rotate" (for JWT Secret) generates a new random value with confirmation warning

#### 3.4.4 Tab: Users

**Component**: Reuse existing `UserManagementPage.tsx` content, embedded within the tab.

#### 3.4.5 Tab: Notifications

**Component**: Reuse existing `NotificationPreferencesPage.tsx` content, embedded within the tab.

### 3.5 Work Items

| ID | Task | Estimate | Dependencies |
|----|------|----------|--------------|
| AP-01 | Create migration 019_system_settings.sql | 1h | - |
| AP-02 | Add `aes-gcm` to workspace dependencies | 0.5h | - |
| AP-03 | Create `backend/src/settings/mod.rs` with encryption/decryption | 3h | AP-01, AP-02 |
| AP-04 | Create SettingsCache in AppState with RwLock | 2h | AP-03 |
| AP-05 | Create `backend/src/api/admin.rs` with settings CRUD | 4h | AP-03 |
| AP-06 | Implement test-connection endpoints for API keys | 3h | AP-05 |
| AP-07 | Update AI module to read keys from settings cache | 2h | AP-04 |
| AP-08 | Create generic lookup table CRUD handler | 4h | - |
| AP-09 | Implement usage-count check for lookup deletion | 2h | AP-08 |
| AP-10 | Add utoipa OpenAPI annotations to admin endpoints | 2h | AP-05, AP-08 |
| AP-11 | Register admin routes in `main.rs` | 0.5h | AP-10 |
| AP-12 | Create `AdminPanel.tsx` with tab layout | 2h | - |
| AP-13 | Create `AdminLookupTables.tsx` with sidebar navigation | 3h | AP-12 |
| AP-14 | Create lookup table CRUD modal component | 3h | AP-13 |
| AP-15 | Implement delete confirmation with usage count | 1h | AP-14 |
| AP-16 | Create `AdminSystemConfig.tsx` with category groupings | 3h | AP-12 |
| AP-17 | Implement masked value display with reveal toggle | 1h | AP-16 |
| AP-18 | Implement test connection UI with feedback | 2h | AP-16 |
| AP-19 | Integrate existing UserManagementPage into admin panel | 1h | AP-12 |
| AP-20 | Integrate existing NotificationPreferencesPage into admin panel | 1h | AP-12 |
| AP-21 | Add adminApi service methods | 2h | - |
| AP-22 | Update App.tsx routing for /admin | 0.5h | AP-12 |
| AP-23 | Write unit tests for encryption/decryption | 1h | AP-03 |
| AP-24 | Write unit tests for settings cache | 1h | AP-04 |
| AP-25 | Integration test: admin endpoints authorization | 1h | All backend |
| **Total** | | **~47h (6 days)** | |

### 3.6 Acceptance Criteria

#### AC-AP-01: Admin Access Control
- **Given** a user without ADMIN role
- **When** they navigate to /admin
- **Then** they are shown an "Access Denied" message or redirected

#### AC-AP-02: Lookup Table List
- **Given** an admin navigates to Lookup Tables > Domains
- **When** the page loads
- **Then** all domains are listed with code, name, and description columns

#### AC-AP-03: Lookup Table Create
- **Given** an admin clicks "Add New" for Domains
- **When** they fill in code="ANALYTICS", name="Analytics" and submit
- **Then** the new domain appears in the list and is available in the glossary term form

#### AC-AP-04: Lookup Table Delete Protection
- **Given** Domain "Customer" is used by 42 glossary terms
- **When** an admin attempts to delete it
- **Then** a confirmation shows "This domain is used by 42 terms. Deletion is not allowed."

#### AC-AP-05: Settings Masking
- **Given** an admin views System Configuration
- **When** the page loads
- **Then** encrypted values (API keys, secrets) are displayed as masked strings (e.g., `sk-ant-...****...3x9f`)

#### AC-AP-06: Settings Reveal
- **Given** an admin clicks "Reveal" on Anthropic API Key
- **When** the reveal button is clicked
- **Then** the full key is shown for 30 seconds, then automatically masked again

#### AC-AP-07: Settings Update
- **Given** an admin updates the Anthropic API Key
- **When** they save the new value
- **Then** the setting is encrypted and stored, the cache is invalidated, and subsequent AI calls use the new key

#### AC-AP-08: Test Connection Success
- **Given** a valid Anthropic API Key is configured
- **When** an admin clicks "Test Connection"
- **Then** a success toast appears: "Successfully connected to Anthropic API"

#### AC-AP-09: Test Connection Failure
- **Given** an invalid API key is configured
- **When** an admin clicks "Test Connection"
- **Then** an error toast appears with the specific error message

#### AC-AP-10: Integrated User Management
- **Given** an admin navigates to Admin Panel > Users
- **When** the tab loads
- **Then** the existing user management functionality is displayed within the admin panel

---

## 4. Execution Order

### Phase 1: Foundation (Days 1-2)

**Backend infrastructure that both features depend on.**

1. **AP-01**: Create `system_settings` migration
2. **AP-02**: Add `aes-gcm` dependency
3. **AP-03**: Implement settings encryption/decryption module
4. **AP-04**: Create SettingsCache in AppState
5. **BU-01**: Add `calamine` and `rust_xlsxwriter` dependencies

**Rationale**: The settings infrastructure enables configuration-driven behaviour. Excel dependencies are quick to add.

### Phase 2: Admin Panel Backend (Days 3-4)

6. **AP-05**: Settings CRUD endpoints
7. **AP-06**: Test connection endpoints
8. **AP-07**: Update AI module to use settings cache
9. **AP-08**: Generic lookup table CRUD handler
10. **AP-09**: Usage-count check for deletion
11. **AP-10**: OpenAPI annotations
12. **AP-11**: Register admin routes

**Rationale**: Backend must be complete before frontend can be developed. Admin panel is prioritised because it enables configuration without .env changes.

### Phase 3: Admin Panel Frontend (Days 5-6)

13. **AP-12**: AdminPanel with tabs
14. **AP-13**: AdminLookupTables
15. **AP-14**: Lookup CRUD modal
16. **AP-15**: Delete confirmation with usage
17. **AP-16**: AdminSystemConfig
18. **AP-17**: Masked value display
19. **AP-18**: Test connection UI
20. **AP-19**: Integrate UserManagementPage
21. **AP-20**: Integrate NotificationPreferencesPage
22. **AP-21**: adminApi service
23. **AP-22**: Routing

**Rationale**: Frontend follows backend. Integration of existing pages is low effort.

### Phase 4: Bulk Upload Backend (Days 7-8)

24. **BU-02**: Template generation endpoint
25. **BU-03**: Dynamic template with lookups
26. **BU-04**: Excel parsing logic
27. **BU-05**: Row validation and lookup resolution
28. **BU-06**: Term creation with workflow
29. **BU-07**: Junction table handling
30. **BU-08**: OpenAPI annotations
31. **BU-09**: Register routes

**Rationale**: Bulk upload depends on lookup tables being manageable via admin panel. Also, the dynamic template generation needs to fetch from DB.

### Phase 5: Bulk Upload Frontend (Days 9-10)

32. **BU-12**: BulkUploadButton
33. **BU-13**: BulkUploadModal
34. **BU-14**: File upload with progress
35. **BU-15**: Results display
36. **BU-16**: glossaryApi methods

### Phase 6: Testing (Days 11-12)

37. **AP-23**: Unit tests for encryption
38. **AP-24**: Unit tests for settings cache
39. **AP-25**: Integration tests for admin auth
40. **BU-10**: Unit tests for Excel parsing
41. **BU-11**: Unit tests for lookup resolution
42. **BU-17**: Integration test for upload flow

---

## 5. Dependencies

### New Rust Crates

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `calamine` | 0.28 | Excel (.xlsx) parsing | MIT license, no security advisories |
| `rust_xlsxwriter` | 0.84 | Excel (.xlsx) writing | MIT license, no security advisories |
| `aes-gcm` | 0.10 | AES-256-GCM encryption | RustCrypto, well-audited |

**Add to `/Users/hjpdebeer/Projects/metadata-tool/Cargo.toml`**:

```toml
[workspace.dependencies]
# ... existing deps ...

# Excel processing
calamine = "0.28"
rust_xlsxwriter = "0.84"

# Encryption
aes-gcm = "0.10"
```

**Add to `/Users/hjpdebeer/Projects/metadata-tool/backend/Cargo.toml`**:

```toml
[dependencies]
# ... existing deps ...
calamine.workspace = true
rust_xlsxwriter.workspace = true
aes-gcm.workspace = true
```

### Frontend Dependencies

No new dependencies required. Ant Design's `Upload.Dragger` component is already available.

---

## 6. Risk Register

| ID | Risk | Likelihood | Impact | Mitigation |
|----|------|------------|--------|------------|
| R-01 | Excel parsing fails on files created by non-Microsoft tools | Medium | Low | Test with LibreOffice, Google Sheets exports. `calamine` handles most variants. |
| R-02 | Large uploads (1000 rows) cause memory pressure | Low | Medium | Stream parsing (calamine supports this). Set row limit enforced at parse time. |
| R-03 | Concurrent uploads create duplicate terms | Medium | High | Add database unique constraint on `(term_name, domain_id)` or mutex per user. |
| R-04 | Settings encryption key rotation invalidates all stored secrets | Medium | High | Document key rotation procedure. Provide "re-encrypt all settings" admin function. |
| R-05 | Test connection exposes network errors in unhelpful ways | Low | Low | Catch specific error types and provide user-friendly messages. |
| R-06 | Admin panel performance degrades with large lookup tables | Low | Low | Implement pagination and search on all lookup lists. |
| R-07 | Deleting lookup values breaks existing entity references | Low | High | Usage check with FK constraint prevents orphan references. |
| R-08 | JWT Secret rotation logs out all users | Medium | Medium | Document impact. Implement graceful rotation with old-key fallback for 24h. |
| R-09 | Excel data validation dropdown limits exceeded (255 items per dropdown in older Excel) | Low | Low | Modern Excel supports more. Document limitation for very large lookup tables. |
| R-10 | Browser caches stale template after lookup table changes | Low | Low | Add Cache-Control: no-cache headers to template endpoint. |

---

## Appendix A: Excel Template Column Specification

### Terms Sheet Columns

| Column | Header | Mandatory | Max Length | Validation | Notes |
|--------|--------|-----------|------------|------------|-------|
| A | Term Name | Yes | 256 | Non-empty | Primary identifier |
| B | Definition | Yes | 4000 | Non-empty | Plain text |
| C | Definition Notes | No | 4000 | - | Clarifying notes |
| D | Counter-Examples | No | 4000 | - | What this term is NOT |
| E | Formula | No | 2000 | - | For KPIs/metrics |
| F | Abbreviation | No | 50 | - | Short form |
| G | Domain | No | - | Dropdown | From Valid Values |
| H | Category | No | - | Dropdown | From Valid Values |
| I | Data Classification | No | - | Dropdown | From Valid Values |
| J | Term Type | No | - | Dropdown | From Valid Values |
| K | Unit of Measure | No | - | Dropdown | From Valid Values |
| L | Review Frequency | No | - | Dropdown | From Valid Values |
| M | Confidence Level | No | - | Dropdown | From Valid Values |
| N | Visibility | No | - | Dropdown | From Valid Values |
| O | Language | No | - | Dropdown | Default: English |
| P | Business Term Owner | Yes | - | Email | Must exist in users table |
| Q | Data Steward | Yes | - | Email | Must exist in users table |
| R | Data Domain Owner | Yes | - | Email | Must exist in users table |
| S | Approver | Yes | - | Email | Must exist in users table |
| T | Organisational Unit | Yes | - | Dropdown | From Valid Values |
| U | Parent Term | No | - | Text | Term name of parent (resolved after all rows processed) |
| V | Source Reference | No | 2000 | - | External source |
| W | Regulatory Reference | No | 2000 | - | Regulation citation |
| X | External Reference | No | 2000 | - | URL or doc reference |
| Y | Business Rules | No | 4000 | - | Business context/rules |
| Z | Examples | No | 4000 | - | Usage examples |
| AA | Used in Reports | No | 2000 | - | Report names |
| AB | Used in Policies | No | 2000 | - | Policy references |
| AC | Regulatory Reporting | No | 2000 | - | Regulatory report usage |
| AD | CDE Flag | No | - | TRUE/FALSE | Is Critical Data Element |
| AE | Golden Source | No | 500 | - | Authoritative source system |
| AF | Regulatory Tags | No | - | Comma-separated | Multiple from Valid Values |
| AG | Subject Areas | No | - | Comma-separated | Multiple from Valid Values |
| AH | Tags | No | - | Comma-separated | Freeform keywords |

---

## Appendix B: System Settings Schema

### Migration: 019_system_settings.sql

```sql
-- ============================================================================
-- Migration: 019_system_settings.sql
-- Purpose: System settings table for admin-configurable options
-- ============================================================================

CREATE TABLE system_settings (
    setting_key     VARCHAR(128) PRIMARY KEY,
    setting_value   TEXT NOT NULL,
    is_encrypted    BOOLEAN NOT NULL DEFAULT FALSE,
    category        VARCHAR(64) NOT NULL,
    display_name    VARCHAR(256) NOT NULL,
    description     TEXT,
    validation_regex VARCHAR(512),
    updated_by      UUID REFERENCES users(user_id),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_system_settings_category ON system_settings(category);

-- Seed with default structure (values will be empty until set by admin)
INSERT INTO system_settings (setting_key, setting_value, is_encrypted, category, display_name, description, validation_regex)
VALUES
    -- AI
    ('anthropic_api_key', '', TRUE, 'AI', 'Anthropic API Key', 'API key for Claude AI enrichment (primary provider)', '^sk-ant-'),
    ('anthropic_model', 'claude-3-5-sonnet-latest', FALSE, 'AI', 'Anthropic Model', 'Model to use for AI enrichment', NULL),
    ('openai_api_key', '', TRUE, 'AI', 'OpenAI API Key', 'API key for OpenAI (fallback provider)', '^sk-'),
    ('openai_model', 'gpt-4o', FALSE, 'AI', 'OpenAI Model', 'Model to use for AI enrichment fallback', NULL),

    -- Auth
    ('jwt_secret', '', TRUE, 'Auth', 'JWT Secret', 'Secret key for signing JWT tokens (min 32 chars)', '.{32,}'),
    ('entra_tenant_id', '', FALSE, 'Auth', 'Entra Tenant ID', 'Microsoft Entra ID tenant', '^[0-9a-f-]{36}$'),
    ('entra_client_id', '', FALSE, 'Auth', 'Entra Client ID', 'OAuth application client ID', '^[0-9a-f-]{36}$'),
    ('entra_client_secret', '', TRUE, 'Auth', 'Entra Client Secret', 'OAuth application client secret', NULL),
    ('entra_redirect_uri', '', FALSE, 'Auth', 'Entra Redirect URI', 'OAuth redirect URI after login', '^https?://'),

    -- Email
    ('graph_tenant_id', '', FALSE, 'Email', 'Graph Tenant ID', 'Microsoft Graph API tenant', '^[0-9a-f-]{36}$'),
    ('graph_client_id', '', FALSE, 'Email', 'Graph Client ID', 'Microsoft Graph application ID', '^[0-9a-f-]{36}$'),
    ('graph_client_secret', '', TRUE, 'Email', 'Graph Client Secret', 'Microsoft Graph client secret', NULL),
    ('notification_sender_email', '', FALSE, 'Email', 'Notification Sender', 'Email address for sending notifications', '^[^@]+@[^@]+$'),

    -- App
    ('frontend_url', 'http://localhost:5173', FALSE, 'App', 'Frontend URL', 'Base URL for the frontend application', '^https?://'),
    ('default_review_frequency', 'ANNUAL', FALSE, 'App', 'Default Review Frequency', 'Default review frequency for new terms', NULL);

COMMENT ON TABLE system_settings IS 'Admin-configurable system settings with optional encryption for sensitive values';
```

---

**Document End**

*This plan is ready for review by the project owner. Upon approval, implementation can begin following the execution order specified in Section 4.*
