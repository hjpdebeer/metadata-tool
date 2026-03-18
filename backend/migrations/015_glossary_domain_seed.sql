-- ============================================================================
-- Migration: 015_glossary_domain_seed.sql
-- Purpose: Seed glossary domains and categories for financial institutions
-- ============================================================================

-- Glossary domains — common data domains in financial services
-- Guard: only insert if no domains exist yet (idempotent for reruns)
INSERT INTO glossary_domains (domain_name, description)
SELECT d.domain_name, d.description
FROM (VALUES
    ('Customer',             'Customer identification, demographics, KYC, onboarding, and relationship management data'),
    ('Account',              'Deposit, lending, investment, and other account-level data including balances and status'),
    ('Transaction',          'Payment, transfer, settlement, and other financial transaction data'),
    ('Product',              'Financial products and services including pricing, terms, and eligibility'),
    ('Risk',                 'Credit risk, market risk, operational risk, and enterprise risk management data'),
    ('Compliance',           'Regulatory reporting, AML/CFT, sanctions screening, and compliance monitoring data'),
    ('Operations',           'Back-office processing, reconciliation, exception handling, and operational metrics'),
    ('Financial Reporting',  'General ledger, financial statements, management reporting, and regulatory return data')
) AS d(domain_name, description)
WHERE NOT EXISTS (SELECT 1 FROM glossary_domains);

-- Glossary categories — classification of business terms
-- Guard: only insert if no categories exist yet (idempotent for reruns)
INSERT INTO glossary_categories (category_name, description)
SELECT c.category_name, c.description
FROM (VALUES
    ('Regulatory',    'Terms defined or required by financial regulators and supervisory authorities'),
    ('Financial',     'Terms related to financial instruments, accounting, and monetary concepts'),
    ('Customer',      'Terms describing customer attributes, segments, and relationships'),
    ('Product',       'Terms related to financial products, services, and their characteristics'),
    ('Operational',   'Terms describing business operations, processes, and workflows'),
    ('Technical',     'Terms related to data architecture, systems, and technology infrastructure')
) AS c(category_name, description)
WHERE NOT EXISTS (SELECT 1 FROM glossary_categories);
