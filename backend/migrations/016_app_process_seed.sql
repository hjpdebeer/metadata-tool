-- Seed process categories (idempotent)
-- application_classifications are already seeded in migration 007

INSERT INTO process_categories (category_name, description)
SELECT 'Core Banking', 'Core banking operations and transaction processing'
WHERE NOT EXISTS (SELECT 1 FROM process_categories WHERE category_name = 'Core Banking');

INSERT INTO process_categories (category_name, description)
SELECT 'Payments', 'Payment processing, clearing, and settlement'
WHERE NOT EXISTS (SELECT 1 FROM process_categories WHERE category_name = 'Payments');

INSERT INTO process_categories (category_name, description)
SELECT 'Lending', 'Lending, credit origination, and loan servicing'
WHERE NOT EXISTS (SELECT 1 FROM process_categories WHERE category_name = 'Lending');

INSERT INTO process_categories (category_name, description)
SELECT 'Treasury', 'Treasury operations, liquidity management, and trading'
WHERE NOT EXISTS (SELECT 1 FROM process_categories WHERE category_name = 'Treasury');

INSERT INTO process_categories (category_name, description)
SELECT 'Risk Management', 'Risk identification, assessment, and mitigation'
WHERE NOT EXISTS (SELECT 1 FROM process_categories WHERE category_name = 'Risk Management');

INSERT INTO process_categories (category_name, description)
SELECT 'Customer Service', 'Customer onboarding, servicing, and support'
WHERE NOT EXISTS (SELECT 1 FROM process_categories WHERE category_name = 'Customer Service');
