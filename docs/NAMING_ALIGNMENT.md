# Frontend ↔ Backend ↔ Database Naming Alignment

This document tracks the alignment between UI labels, API field names, and database columns.
Any mismatch is a bug. All three layers MUST use consistent naming.

## Glossary Term Fields

| UI Label | API Field Name | DB Column | DB Lookup Table | Notes |
|----------|---------------|-----------|-----------------|-------|
| Term Name | term_name | term_name | - | Text |
| Term Code | term_code | term_code | - | Auto-generated |
| Definition | definition | definition | - | Text |
| Definition Notes | definition_notes | definition_notes | - | Text |
| Counter-Examples | counter_examples | counter_examples | - | Text |
| Formula | formula | formula | - | Text |
| Abbreviation | abbreviation | abbreviation | - | Text (max 50) |
| Domain | domain | domain_id | glossary_domains | FK Lookup |
| Category | category | category_id | glossary_categories | FK Lookup |
| Data Classification | data_classification | classification_id | data_classifications | FK Lookup |
| Term Type | term_type | term_type_id | glossary_term_types | FK Lookup |
| Unit of Measure | unit_of_measure | unit_of_measure_id | glossary_units_of_measure | FK Lookup |
| Review Frequency | review_frequency | review_frequency_id | glossary_review_frequencies | FK Lookup |
| Confidence Level | confidence_level | confidence_level_id | glossary_confidence_levels | FK Lookup |
| Visibility | visibility | visibility_id | glossary_visibility_levels | FK Lookup |
| Language | language | language_id | glossary_languages | FK Lookup |
| Business Term Owner | owner | owner_user_id | users | User FK |
| Data Steward | steward | steward_user_id | users | User FK |
| Data Domain Owner | domain_owner | domain_owner_user_id | users | User FK |
| Approver | approver | approver_user_id | users | User FK |
| Organisational Unit | organisational_unit | organisational_unit | - | Text |
| Parent Term | parent_term | parent_term_id | glossary_terms | Self FK |
| Source Reference | source_reference | source_reference | - | Text |
| Regulatory Reference | regulatory_reference | regulatory_reference | - | Text |
| External Reference | external_reference | external_reference | - | Text |
| Business Rules | business_context | business_context | - | Text |
| Examples | examples | examples | - | Text |
| Used in Reports | used_in_reports | used_in_reports | - | Text |
| Used in Policies | used_in_policies | used_in_policies | - | Text |
| Regulatory Reporting | regulatory_reporting_usage | regulatory_reporting_usage | - | Text |
| CDE Flag | is_cde | is_cde | - | Boolean |
| Golden Source | golden_source | golden_source | - | Text |
| Regulatory Tags | regulatory_tags | glossary_term_regulatory_tags | glossary_regulatory_tags | Junction M2M |
| Subject Areas | subject_areas | glossary_term_subject_areas | glossary_subject_areas | Junction M2M |
| Tags | tags | glossary_term_tags | glossary_tags | Junction M2M |

## Rule: Frontend Label → API Field → DB Column

For **lookup fields** (FK to a lookup table):
- UI shows: the `display_name` from the lookup table
- API detail response includes: both `{field}_id` (UUID) and `{field}_name` (resolved display name)
- API create/update accepts: the UUID
- AI prompt sends: the full lookup list with `{id, name}` pairs
- AI returns: the UUID directly in `suggested_value`
- The `field_name` in AI suggestions matches the **concept name** (e.g., `domain`, not `domain_id`)
