-- Data Lineage

CREATE TABLE lineage_node_types (
    node_type_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type_code    VARCHAR(50) NOT NULL UNIQUE,
    type_name    VARCHAR(128) NOT NULL,
    description  TEXT,
    icon_name    VARCHAR(64)
);

INSERT INTO lineage_node_types (type_code, type_name, description, icon_name) VALUES
    ('SOURCE_SYSTEM',   'Source System',    'External or internal source system',             'database'),
    ('DATABASE',        'Database',         'Database instance',                               'server'),
    ('SCHEMA',          'Schema',           'Database schema',                                 'folder'),
    ('TABLE',           'Table/View',       'Database table or view',                          'table'),
    ('COLUMN',          'Column',           'Individual column',                               'columns'),
    ('API',             'API Endpoint',     'REST/SOAP API endpoint',                          'cloud'),
    ('FILE',            'File/Dataset',     'File-based dataset (CSV, Parquet, etc.)',          'file'),
    ('STREAM',          'Data Stream',      'Real-time data stream (Kafka, etc.)',              'activity'),
    ('ETL_JOB',         'ETL/ELT Job',      'Data transformation or loading job',              'shuffle'),
    ('REPORT',          'Report/Dashboard', 'BI report or dashboard',                          'bar-chart'),
    ('APPLICATION',     'Application',      'Business application',                            'monitor'),
    ('PROCESS',         'Business Process', 'Business process step',                           'git-branch'),
    ('MANUAL',          'Manual Process',   'Manual data entry or transformation',             'user');

CREATE TABLE lineage_graphs (
    graph_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    graph_name      VARCHAR(256) NOT NULL,
    graph_type      VARCHAR(20) NOT NULL CHECK(graph_type IN ('BUSINESS','TECHNICAL')),
    description     TEXT,
    scope_type      VARCHAR(50),
    scope_entity_id UUID,
    version_number  INT NOT NULL DEFAULT 1,
    is_current      BOOLEAN NOT NULL DEFAULT TRUE,
    deleted_at      TIMESTAMPTZ,
    created_by      UUID NOT NULL REFERENCES users(user_id),
    updated_by      UUID REFERENCES users(user_id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE lineage_nodes (
    node_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    graph_id       UUID NOT NULL REFERENCES lineage_graphs(graph_id) ON DELETE CASCADE,
    node_type_id   UUID NOT NULL REFERENCES lineage_node_types(node_type_id),
    node_name      VARCHAR(256) NOT NULL,
    node_label     VARCHAR(256),
    description    TEXT,
    system_id      UUID REFERENCES source_systems(system_id),
    table_id       UUID REFERENCES technical_tables(table_id),
    element_id     UUID REFERENCES data_elements(element_id),
    application_id UUID,  -- FK added after applications table
    process_id     UUID,  -- FK added after processes table
    position_x     DOUBLE PRECISION,
    position_y     DOUBLE PRECISION,
    properties     JSONB,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE lineage_edges (
    edge_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    graph_id             UUID NOT NULL REFERENCES lineage_graphs(graph_id) ON DELETE CASCADE,
    source_node_id       UUID NOT NULL REFERENCES lineage_nodes(node_id) ON DELETE CASCADE,
    target_node_id       UUID NOT NULL REFERENCES lineage_nodes(node_id) ON DELETE CASCADE,
    edge_type            VARCHAR(50) NOT NULL DEFAULT 'FLOW',
    transformation_logic TEXT,
    description          TEXT,
    properties           JSONB,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK(source_node_id != target_node_id)
);

-- Indexes
CREATE INDEX idx_lineage_graphs_current ON lineage_graphs(is_current) WHERE is_current = TRUE AND deleted_at IS NULL;
CREATE INDEX idx_lineage_nodes_graph ON lineage_nodes(graph_id);
CREATE INDEX idx_lineage_nodes_type ON lineage_nodes(node_type_id);
CREATE INDEX idx_lineage_edges_graph ON lineage_edges(graph_id);
CREATE INDEX idx_lineage_edges_source ON lineage_edges(source_node_id);
CREATE INDEX idx_lineage_edges_target ON lineage_edges(target_node_id);
