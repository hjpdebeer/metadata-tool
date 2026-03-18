-- Business Process Registry

CREATE TABLE process_categories (
    category_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category_name      VARCHAR(256) NOT NULL,
    description        TEXT,
    parent_category_id UUID REFERENCES process_categories(category_id),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE business_processes (
    process_id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    process_name          VARCHAR(256) NOT NULL,
    process_code          VARCHAR(64) NOT NULL UNIQUE,
    description           TEXT NOT NULL,
    detailed_description  TEXT,
    category_id           UUID REFERENCES process_categories(category_id),
    status_id             UUID NOT NULL REFERENCES entity_statuses(status_id),
    owner_user_id         UUID REFERENCES users(user_id),
    parent_process_id     UUID REFERENCES business_processes(process_id),
    is_critical           BOOLEAN NOT NULL DEFAULT FALSE,
    criticality_rationale TEXT,
    frequency             VARCHAR(64),
    regulatory_requirement TEXT,
    sla_description       TEXT,
    documentation_url     TEXT,
    deleted_at            TIMESTAMPTZ,
    created_by            UUID NOT NULL REFERENCES users(user_id),
    updated_by            UUID REFERENCES users(user_id),
    created_at            TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Steps within a business process
CREATE TABLE process_steps (
    step_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    process_id           UUID NOT NULL REFERENCES business_processes(process_id) ON DELETE CASCADE,
    step_number          INT NOT NULL,
    step_name            VARCHAR(256) NOT NULL,
    description          TEXT,
    responsible_role     VARCHAR(128),
    application_id       UUID REFERENCES applications(application_id),
    input_data_elements  JSONB,
    output_data_elements JSONB,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(process_id, step_number)
);

-- Link processes to data elements
CREATE TABLE process_data_elements (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    process_id  UUID NOT NULL REFERENCES business_processes(process_id),
    element_id  UUID NOT NULL REFERENCES data_elements(element_id),
    usage_type  VARCHAR(50) NOT NULL DEFAULT 'BOTH' CHECK(usage_type IN ('INPUT','OUTPUT','BOTH')),
    is_required BOOLEAN NOT NULL DEFAULT TRUE,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(process_id, element_id)
);

-- Link processes to applications
CREATE TABLE process_applications (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    process_id      UUID NOT NULL REFERENCES business_processes(process_id),
    application_id  UUID NOT NULL REFERENCES applications(application_id),
    role_in_process VARCHAR(128),
    description     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(process_id, application_id)
);

-- Add FK from lineage_nodes to processes
ALTER TABLE lineage_nodes
    ADD CONSTRAINT fk_lineage_nodes_process
    FOREIGN KEY (process_id) REFERENCES business_processes(process_id);

-- Trigger: auto-designate CDEs when a process is marked critical
CREATE OR REPLACE FUNCTION auto_designate_cde_for_critical_process()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.is_critical = TRUE AND (OLD.is_critical IS NULL OR OLD.is_critical = FALSE) THEN
        UPDATE data_elements de
        SET
            is_cde = TRUE,
            cde_rationale = COALESCE(cde_rationale, '') ||
                CASE WHEN cde_rationale IS NOT NULL AND cde_rationale != '' THEN '; ' ELSE '' END ||
                'Auto-designated: linked to critical business process "' || NEW.process_name || '"',
            cde_designated_at = CURRENT_TIMESTAMP,
            updated_at = CURRENT_TIMESTAMP
        FROM process_data_elements pde
        WHERE pde.process_id = NEW.process_id
        AND pde.element_id = de.element_id
        AND de.is_cde = FALSE;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_critical_process_cde
    AFTER UPDATE OF is_critical ON business_processes
    FOR EACH ROW
    EXECUTE FUNCTION auto_designate_cde_for_critical_process();

-- Also when linking a data element to an already-critical process
CREATE OR REPLACE FUNCTION auto_designate_cde_on_process_link()
RETURNS TRIGGER AS $$
DECLARE
    v_is_critical BOOLEAN;
    v_process_name VARCHAR(256);
BEGIN
    SELECT is_critical, process_name INTO v_is_critical, v_process_name
    FROM business_processes WHERE process_id = NEW.process_id;

    IF v_is_critical = TRUE THEN
        UPDATE data_elements
        SET
            is_cde = TRUE,
            cde_rationale = COALESCE(cde_rationale, '') ||
                CASE WHEN cde_rationale IS NOT NULL AND cde_rationale != '' THEN '; ' ELSE '' END ||
                'Auto-designated: linked to critical business process "' || v_process_name || '"',
            cde_designated_at = CURRENT_TIMESTAMP,
            updated_at = CURRENT_TIMESTAMP
        WHERE element_id = NEW.element_id AND is_cde = FALSE;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_process_link_cde
    AFTER INSERT ON process_data_elements
    FOR EACH ROW
    EXECUTE FUNCTION auto_designate_cde_on_process_link();

-- Indexes
CREATE INDEX idx_business_processes_category ON business_processes(category_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_business_processes_status ON business_processes(status_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_business_processes_critical ON business_processes(is_critical) WHERE is_critical = TRUE AND deleted_at IS NULL;
CREATE INDEX idx_process_data_elements_process ON process_data_elements(process_id);
CREATE INDEX idx_process_data_elements_element ON process_data_elements(element_id);
