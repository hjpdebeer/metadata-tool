# ADR-0006: Standardised Data Access Patterns

**Status**: Proposed
**Date**: 2026-03-19
**Context**: The glossary enhancement revealed inconsistent patterns for reading and writing data. Different endpoints return different response shapes, AI suggestion acceptance uses ad-hoc field matching, and lookup resolution is inconsistent. This creates bugs, makes the codebase harder to maintain, and violates Principle 13 (AI-Maintained Codebase).

---

## Decision

Establish three standardised patterns for all domain entities, enforced consistently across all API endpoints.

### Pattern 1: Read (Detail View)

Every entity has ONE detail response type that includes:
- All entity columns
- All resolved FK lookup names (joined in a single SQL query, not separate lookups)
- All junction/relationship data

**Implementation**: A single SQL query with LEFT JOINs for all lookup tables, followed by separate queries for junction tables (these are always arrays).

```sql
-- Single query resolves all FK lookups
SELECT
    gt.*,
    gd.domain_name,
    gc.category_name,
    gtt.type_name AS term_type_name,
    gum.unit_name AS unit_of_measure_name,
    dc.classification_name,
    grf.frequency_name AS review_frequency_name,
    gcl.level_name AS confidence_level_name,
    gvl.visibility_name,
    gl.language_name,
    pt.term_name AS parent_term_name,
    uo.display_name AS owner_name,
    us.display_name AS steward_name,
    udo.display_name AS domain_owner_name,
    ua.display_name AS approver_name
FROM glossary_terms gt
LEFT JOIN glossary_domains gd ON gd.domain_id = gt.domain_id
LEFT JOIN glossary_categories gc ON gc.category_id = gt.category_id
LEFT JOIN glossary_term_types gtt ON gtt.term_type_id = gt.term_type_id
LEFT JOIN glossary_units_of_measure gum ON gum.unit_id = gt.unit_of_measure_id
LEFT JOIN data_classifications dc ON dc.classification_id = gt.classification_id
LEFT JOIN glossary_review_frequencies grf ON grf.frequency_id = gt.review_frequency_id
LEFT JOIN glossary_confidence_levels gcl ON gcl.confidence_id = gt.confidence_level_id
LEFT JOIN glossary_visibility_levels gvl ON gvl.visibility_id = gt.visibility_id
LEFT JOIN glossary_languages gl ON gl.language_id = gt.language_id
LEFT JOIN glossary_terms pt ON pt.term_id = gt.parent_term_id
LEFT JOIN users uo ON uo.user_id = gt.owner_user_id
LEFT JOIN users us ON us.user_id = gt.steward_user_id
LEFT JOIN users udo ON udo.user_id = gt.domain_owner_user_id
LEFT JOIN users ua ON ua.user_id = gt.approver_user_id
WHERE gt.term_id = $1 AND gt.deleted_at IS NULL
```

**Response type**: One flat struct with all columns + resolved names. No `#[serde(flatten)]` — explicit fields only.

```rust
pub struct GlossaryTermDetail {
    // All 45 entity columns
    pub term_id: Uuid,
    pub term_name: String,
    // ... all columns ...

    // Resolved lookup names (from JOINs)
    pub domain_name: Option<String>,
    pub category_name: Option<String>,
    pub term_type_name: Option<String>,
    // ... all resolved names ...

    // Junction data (from separate queries)
    pub regulatory_tags: Vec<RegulatoryTagRef>,
    pub subject_areas: Vec<SubjectAreaRef>,
    pub tags: Vec<TagRef>,
    pub linked_processes: Vec<ProcessRef>,
}
```

### Pattern 2: Write (Create / Update)

Every entity has ONE update mechanism that handles ALL field types uniformly:

**Text columns**: Direct SQL UPDATE with COALESCE for partial updates.

**FK lookup columns**: Accept EITHER the UUID ID directly OR the display name. If a display name is provided, resolve it to the ID via the lookup table. This allows both UI dropdowns (which send IDs) and AI suggestions (which send display names) to use the same update path.

```rust
/// Resolve a lookup field value. Accepts either a UUID string (direct ID)
/// or a display name (resolved via lookup table).
async fn resolve_lookup(
    pool: &PgPool,
    value: &str,
    lookup_query: &str,  // e.g., "SELECT term_type_id FROM glossary_term_types WHERE type_name ILIKE $1"
) -> Option<Uuid> {
    // Try parsing as UUID first (from UI dropdown)
    if let Ok(id) = Uuid::parse_str(value) {
        return Some(id);
    }
    // Otherwise resolve by display name (from AI suggestion)
    sqlx::query_scalar::<_, Uuid>(lookup_query)
        .bind(value)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}
```

**Junction columns**: Accept comma-separated display names OR arrays of IDs. Resolve names to IDs, then INSERT with ON CONFLICT DO NOTHING.

### Pattern 3: AI Suggestion Application

AI suggestions go through the SAME update mechanism as user edits. The `apply_suggestion_to_entity` function should:

1. Determine the field type (text column, FK lookup, junction table)
2. Route to the appropriate handler:
   - Text → direct column update
   - FK lookup → resolve display name to ID, then update column
   - Junction → parse comma-separated, resolve each, insert junction rows
3. All using the same `resolve_lookup` helper

This eliminates the separate code paths for AI vs UI updates.

### Pattern 4: Frontend Contract

The frontend always works with ONE response type per entity. For glossary:
- List: `GlossaryTermListItem` (lightweight, from JOINed list query)
- Detail: `GlossaryTermDetail` (comprehensive, flat struct with all resolved names)
- Create/Update: sends field values (IDs for dropdowns, text for text fields)

No nested types. No `#[serde(flatten)]`. Explicit fields only.

---

## Consequences

- **Consistency**: Every entity follows the same read/write patterns
- **AI + UI parity**: Both use the same update mechanism
- **Single source of truth**: One detail query, one response type, one update path
- **Easier to extend**: Adding a new field means adding it to the query, the struct, and the update — same three places every time
- **Breaking change**: Requires refactoring the current glossary implementation to remove `#[serde(flatten)]` and consolidate the read/write paths

## Implementation Priority

1. Refactor glossary detail endpoint (remove flatten, use single JOIN query)
2. Refactor glossary update to handle both ID and display name inputs
3. Refactor AI suggestion acceptance to use the unified update path
4. Apply the same patterns to data_dictionary, data_quality, etc.
