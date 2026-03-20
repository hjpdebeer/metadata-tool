# Metadata Attribute Change Checklist

This checklist MUST be followed when adding, removing, or renaming any metadata attribute on a governed entity (Business Glossary Term, Application, Data Element, Quality Rule, Business Process).

Incomplete propagation leads to runtime errors (missing columns, stale references, broken templates). Every item below is a potential failure point.

---

## 1. Database Layer

- [ ] **Migration**: ADD/ALTER/DROP column with correct type, nullability, and default
- [ ] **3NF compliance**: If the field references a set of values, create a lookup table with FK — do NOT use free text for enumerable values
- [ ] **Indexes**: Create/update indexes if the field is searchable or filterable
- [ ] **Triggers**: Update any triggers that reference the table (search vector, auto-calculate, naming validation)
- [ ] **Column comments**: Add `COMMENT ON COLUMN` for documentation
- [ ] **Constraints**: Unique constraints, check constraints, FK constraints as appropriate
- [ ] **Seed data**: If it's a lookup table, seed with initial values

## 2. Backend Domain Model (`domain/{entity}.rs`)

- [ ] **Base struct** (e.g., `GlossaryTerm`, `Application`) — used by `RETURNING *`
- [ ] **Detail row struct** (e.g., `GlossaryTermDetailRow`, `ApplicationDetailRow`) — includes resolved lookup names from JOINs
- [ ] **Detail view struct** (e.g., `GlossaryTermDetail`, `ApplicationFullView`) — the API response
- [ ] **`from_row_and_junctions()`** — mapping from row to view
- [ ] **Create request struct** — if the field is user-provided at creation
- [ ] **Update request struct** — if the field is editable
- [ ] **Search/list request struct** — if the field is filterable
- [ ] **List item struct** — if the field is shown in the list view

## 3. Backend API (`api/{entity}.rs`)

- [ ] **Column constants** (e.g., `GLOSSARY_TERM_COLUMNS`) — must match the base struct exactly
- [ ] **GET detail query**: Add to SELECT, add LEFT JOIN if it's a lookup FK, add resolved name alias
- [ ] **GET list query**: Add to SELECT if shown in list view
- [ ] **POST create handler**: Add to INSERT column list, VALUES, and `.bind()` calls
- [ ] **PUT update handler**: Add COALESCE line, `.bind()` call, renumber all `$N` parameters
- [ ] **Amend handler**: Copy field to new version in INSERT
- [ ] **Discard handler**: No change needed (hard deletes entire row)
- [ ] **Visibility filter**: Update if the field affects access control

## 4. AI Integration (`ai/mod.rs` + `api/ai.rs`)

### If the field IS AI-suggestible:
- [ ] **AI prompt** (`ai/mod.rs`): Add field description to the prompt text
- [ ] **fetch_entity_data** (`api/ai.rs`): Add to SELECT query and JSON object
- [ ] **existing_fields tracking**: Add `if row.field.is_some() { existing.push(...) }`
- [ ] **apply_suggestion_to_entity**: Add static SQL UPDATE for the field
- [ ] **Lookup resolution**: If it's a lookup field, embed the lookup table in the prompt (Section 15.6 pattern)

### If the field is NOT AI-suggestible:
- [ ] **AI prompt**: Do NOT include in the prompt
- [ ] **Filter exclusion list**: Add to the `matches!()` exclusion in the suggestion filter
- [ ] **fetch_entity_data**: Still include in SELECT (for the `GlossaryTerm` struct) but do NOT include in the JSON sent to AI

### If removing a previously-suggestible field:
- [ ] **AI prompt**: Remove from prompt text
- [ ] **fetch_entity_data**: Remove from JSON object (keep in SELECT if still in struct)
- [ ] **existing_fields**: Remove the check
- [ ] **apply_suggestion_to_entity**: Remove the match arm
- [ ] **Filter exclusion**: Add to exclusion list (AI may hallucinate old field names)
- [ ] **Clean up**: Delete any PENDING suggestions for the removed field from `ai_suggestions` table

## 5. Bulk Upload (`api/bulk_upload.rs` / `api/app_bulk_upload.rs`)

- [ ] **Template headers array**: Add/remove/rename column header
- [ ] **Template instructions array**: Add/remove/rename with mandatory/optional, max length, notes
- [ ] **Column indices**: ALL indices after the change point must be renumbered
- [ ] **Valid Values sheet**: Add lookup list if it's a dropdown field
- [ ] **Dropdown validation**: Add `DataValidation` mapping if dropdown
- [ ] **Row parsing**: Update column index for the field
- [ ] **Mandatory validation**: Add check if the field is required
- [ ] **Lookup resolution**: Add `resolve_optional_lookup` / `resolve_lookup` if FK
- [ ] **INSERT SQL**: Add to column list and VALUES, renumber all `$N` parameters
- [ ] **`.bind()` calls**: Add bind in correct position, renumber all subsequent

## 6. Frontend Types (`services/{entity}Api.ts`)

- [ ] **Base interface** (e.g., `GlossaryTerm`, `Application`)
- [ ] **Detail view interface** (e.g., `GlossaryTermDetailView`, `ApplicationFullView`)
- [ ] **Resolved name field** (e.g., `field_name` for display if it's a lookup FK)
- [ ] **Create request interface** (if user-provided at creation)
- [ ] **Update request interface** (if editable)
- [ ] **List item interface** (if shown in list view)
- [ ] **Search params interface** (if filterable)
- [ ] **Lookup type interface** (if it's a new lookup table)
- [ ] **API method** (if it's a new lookup endpoint)

## 7. Frontend Pages

### Detail Page (`pages/{Entity}Detail.tsx`)
- [ ] Display field in the correct collapsible section
- [ ] Resolve lookup name (use `detail.field_name` not `detail.field_id`)
- [ ] Never display UUIDs — use resolved names from the API response

### Edit Form (`pages/{Entity}Form.tsx`)
- [ ] Add Form.Item with correct component (Input, Select, Switch, DatePicker, etc.)
- [ ] Load field value from detail response in `fetchExisting`
- [ ] Add to the update diff field list
- [ ] For Select dropdowns: load options in `fetchReferenceData`, create options array
- [ ] For lookups: ensure options load BEFORE field values (sequential loading pattern)
- [ ] If NOT user-editable (e.g., confidence_level): do NOT add form field, do NOT include in diff list

### Create Form
- [ ] Include field only if it's user-provided at creation time
- [ ] Most fields should NOT be in the simplified create form (Name + Description → AI Enrich)

### List Page (`pages/{Entity}Page.tsx`)
- [ ] Add table column if the field should be visible in the list
- [ ] Add filter if the field is filterable

### Ownership Card (Detail Page)
- [ ] If it's an ownership/user field: use `labelInValue` pattern, sync from resolved names
- [ ] Include in `handleSaveOwnership` with `.value` extraction

## 8. Workflow / Governance Decisions

- [ ] **User-editable or system-managed?** — System-managed fields (confidence_level, approved_at, next_review_date) must NOT appear in edit forms
- [ ] **AI-suggestible?** — Only text fields and lookup fields where AI can make meaningful suggestions. Never: ownership, dates, flags, FKs to user-created entities (like golden_source_app_id)
- [ ] **Read-only display?** — Some fields display on the detail page but can only be set by specific modules (e.g., confidence_level by Data Quality)
- [ ] **Amendment copy?** — Does the field need to be copied when creating a version-based amendment?
- [ ] **Bulk upload?** — Should the field be included in the bulk upload template? Mandatory or optional?

---

## Quick Reference: Common Field Types

| Field Type | DB Column | Lookup Table | AI Suggestible | Bulk Upload | Form Component |
|------------|-----------|-------------|----------------|-------------|----------------|
| Free text | VARCHAR/TEXT | No | Yes (if meaningful) | Yes | Input / TextArea |
| Lookup FK | UUID FK | Yes (3NF) | Yes (embed in prompt) | Yes (dropdown) | Select |
| User FK | UUID FK | No (users table) | No | Yes (email dropdown) | Select (labelInValue) |
| Boolean flag | BOOLEAN | No | No (governance decision) | Yes (TRUE/FALSE) | Switch |
| Date | DATE/TIMESTAMPTZ | No | No | Yes (if user-set) | DatePicker |
| System-managed | Any | N/A | No | No | Read-only display only |
| Calculated | Any | N/A | No | No | Not in form |

---

## Version History

| Date | Change | Author |
|------|--------|--------|
| 2026-03-20 | Initial version | Hendrik de Beer + Claude |
