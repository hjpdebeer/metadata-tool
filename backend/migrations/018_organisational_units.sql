-- ============================================================================
-- Migration: 018_organisational_units.sql
-- Purpose: Add organisational_units lookup table for dropdown selection
-- ============================================================================

CREATE TABLE organisational_units (
    unit_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    unit_code     VARCHAR(50) NOT NULL UNIQUE,
    unit_name     VARCHAR(256) NOT NULL,
    description   TEXT,
    parent_unit_id UUID REFERENCES organisational_units(unit_id),
    display_order INT NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed with common financial institution organisational units
INSERT INTO organisational_units (unit_code, unit_name, description, display_order) VALUES
    ('GROUP_FINANCE',       'Group Finance',                'Financial planning, reporting, and accounting',                    10),
    ('GROUP_RISK',          'Group Risk Management',        'Enterprise risk management, credit risk, market risk',             20),
    ('GROUP_COMPLIANCE',    'Group Compliance',             'Regulatory compliance, AML/CFT, sanctions',                        30),
    ('INTERNAL_AUDIT',      'Internal Audit',               'Internal audit and assurance',                                     40),
    ('RETAIL_BANKING',      'Retail Banking',               'Personal banking, consumer lending, deposits',                     50),
    ('CORPORATE_BANKING',   'Corporate Banking',            'Corporate and commercial banking services',                        60),
    ('WHOLESALE_BANKING',   'Wholesale Banking',            'Wholesale and institutional banking',                              70),
    ('TREASURY',            'Treasury',                     'Treasury operations, ALM, funding',                                80),
    ('WEALTH_MANAGEMENT',   'Wealth Management',            'Private banking, asset management, advisory',                      90),
    ('ISLAMIC_BANKING',     'Islamic Banking',              'Sharia-compliant banking products and services',                   100),
    ('TRADE_FINANCE',       'Trade Finance',                'Letters of credit, guarantees, supply chain finance',              110),
    ('OPERATIONS',          'Operations',                   'Banking operations, payments, settlements',                        120),
    ('TECHNOLOGY',          'Information Technology',       'IT infrastructure, applications, digital services',                130),
    ('DATA_GOVERNANCE',     'Data Governance',              'Data management, data quality, metadata management',               140),
    ('HUMAN_RESOURCES',     'Human Resources',              'HR, talent management, learning and development',                  150),
    ('LEGAL',               'Legal',                        'Legal services, contracts, regulatory affairs',                     160),
    ('MARKETING',           'Marketing & Communications',   'Marketing, brand management, corporate communications',            170),
    ('CUSTOMER_EXPERIENCE', 'Customer Experience',          'Customer service, complaints, experience management',              180),
    ('DIGITAL',             'Digital Banking',              'Online banking, mobile banking, digital channels',                 190),
    ('STRATEGY',            'Strategy & Transformation',    'Corporate strategy, business transformation',                      200);

CREATE INDEX idx_org_units_parent ON organisational_units(parent_unit_id);
