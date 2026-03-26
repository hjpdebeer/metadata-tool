-- Clear all entity data while retaining users, roles, and lookup/seed tables.
-- Uses TRUNCATE CASCADE to handle circular FK dependencies cleanly.

-- Workflow + notifications
TRUNCATE workflow_history, workflow_tasks, workflow_instances CASCADE;
TRUNCATE in_app_notifications, notification_queue, notification_preferences CASCADE;

-- AI
TRUNCATE ai_feedback, ai_suggestions CASCADE;

-- Audit
TRUNCATE audit_log CASCADE;

-- Quality
TRUNCATE quality_scores, quality_assessments, quality_rules CASCADE;

-- Lineage
TRUNCATE lineage_edges, lineage_nodes, lineage_graphs CASCADE;

-- Process junction tables
TRUNCATE process_data_elements, process_applications, process_steps, glossary_term_processes CASCADE;

-- Application junction tables
TRUNCATE application_data_elements, application_interfaces CASCADE;

-- Technical metadata
TRUNCATE technical_columns, technical_tables, technical_schemas CASCADE;

-- Glossary junction tables
TRUNCATE glossary_term_aliases, glossary_term_regulatory_tags, glossary_term_subject_areas, glossary_term_relationships CASCADE;

-- Core entity tables
TRUNCATE glossary_terms, data_elements, applications, business_processes CASCADE;

-- Other
TRUNCATE api_keys, ingestion_log CASCADE;
