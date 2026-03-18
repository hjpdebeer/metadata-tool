# Implementation Plan

Enterprise Metadata Management Tool - Implementation Roadmap

**Version**: 1.0
**Date**: 2026-03-18
**Author**: Hendrik de Beer (AI-assisted via Claude Code)

---

## Programme Summary

**Objective**: Transform the existing project scaffold into a fully functional enterprise metadata lifecycle management platform. The scaffold provides the foundation (database schema, API stubs, frontend routing, domain models); this plan sequences the work required to implement all functionality.

**Scope**:

*In Scope*:
- Complete implementation of 40+ API endpoint stubs across 7 metadata domains
- Authentication flow (Microsoft Entra ID SSO + JWT)
- Role-based access control middleware
- Generic workflow engine for all entity types
- AI-powered metadata enrichment (Claude primary, OpenAI fallback)
- Email notifications via Microsoft Graph API
- Frontend pages for all domains with CRUD operations
- Data lineage visualisation (React Flow)
- Audit trail and logging
- Unit and integration tests

*Out of Scope*:
- Multi-tenancy
- Data discovery/cataloguing automation (future phase)
- External API consumers (API gateway, rate limiting)
- Mobile application
- On-premises deployment tooling

**Key Milestones**:

| Milestone | Target | Description |
|-----------|--------|-------------|
| M1: First Vertical Slice | Sprint 3 | End-to-end flow: create glossary term, approve via workflow, view on frontend |
| M2: Core Domains Complete | Sprint 6 | Glossary, Data Dictionary, Workflow fully functional |
| M3: All Domains Functional | Sprint 10 | All 7 domains with full CRUD and workflow integration |
| M4: AI & Lineage | Sprint 12 | AI enrichment + lineage visualisation complete |
| M5: Production Ready | Sprint 14 | Notifications, polish, testing complete |

**Success Criteria**:
- All 40+ API endpoints return real data (no "not implemented" responses)
- 80%+ code coverage on backend business logic
- All domain entities flow through workflow lifecycle
- AI enrichment generates suggestions for glossary terms and data elements
- Lineage graphs render interactively with impact analysis
- End-to-end integration tests pass

**Assumptions**:
- A1: Single developer (AI-assisted via Claude Code) - 8 hours/day, 5 days/week
- A2: Microsoft Entra ID tenant available for SSO configuration
- A3: Claude API and OpenAI API keys available
- A4: Microsoft Graph API credentials available for email
- A5: PostgreSQL 17 available locally via Docker Compose

**Constraints**:
- C1: Must comply with 14 foundational principles (METADATA_TOOL_PRINCIPLES.md)
- C2: Must follow coding standards (CODING_STANDARDS.md)
- C3: Pre-commit checks must pass (fmt, clippy, test, doc, frontend build)
- C4: No proprietary bank references (Principle 14: bank-agnostic)
- C5: All security remediations are non-negotiable

---

## Work Item Register

### Phase 0: Foundation

| WI-ID | Title | Type | Priority | Effort | Dependencies | Status |
|-------|-------|------|----------|--------|--------------|--------|
| WI-001 | Implement JWT token issuance and validation | Backend | P0 | 3d | - | Not Started |
| WI-002 | Add password_hash column, seed dev users, dev-login endpoint | Backend | P0 | 2d | WI-001 | Not Started |
| WI-003 | Implement require_auth middleware | Backend | P0 | 1d | WI-001 | Not Started |
| WI-004 | Implement require_role middleware (RBAC) | Backend | P0 | 2d | WI-003 | Not Started |
| WI-005 | Seed initial workflow states and definitions | Backend | P0 | 1d | - | Not Started |
| WI-006 | Implement workflow state transition logic | Backend | P1 | 3d | WI-005 | Not Started |
| WI-007 | Implement workflow task creation and assignment | Backend | P1 | 2d | WI-006 | Not Started |
| WI-008 | Implement workflow history recording | Backend | P1 | 1d | WI-006 | Not Started |
| WI-009 | Create test utilities module (fixtures, db setup) | Backend | P1 | 1d | - | Not Started |
| WI-010 | Add AuthProvider context and dev-login page to frontend | Frontend | P0 | 2d | WI-002 | Not Started |

> **Note (2026-03-18):** WI-002 was revised. Entra ID SSO is deferred to Phase 7 (Sprint 13) alongside Microsoft Graph notifications — both depend on Microsoft tenant configuration that is impractical during development. Sprint 1-2 uses a dev-mode login (email + bcrypt password) that is automatically disabled when `ENTRA_TENANT_ID` is configured. See WI-134 below.

### Phase 1: Glossary Vertical Slice

| WI-ID | Title | Type | Priority | Effort | Dependencies | Status |
|-------|-------|------|----------|--------|--------------|--------|
| WI-011 | Implement list_terms with pagination and full-text search | Backend | P0 | 2d | WI-003 | Not Started |
| WI-012 | Implement get_term | Backend | P0 | 1d | WI-003 | Not Started |
| WI-013 | Implement create_term (with workflow initiation) | Backend | P0 | 2d | WI-006, WI-007 | Not Started |
| WI-014 | Implement update_term (with versioning) | Backend | P0 | 2d | WI-012 | Not Started |
| WI-015 | Implement list_domains | Backend | P0 | 0.5d | WI-003 | Not Started |
| WI-016 | Implement term relationships CRUD | Backend | P1 | 2d | WI-012 | Not Started |
| WI-017 | Implement term aliases CRUD | Backend | P1 | 1d | WI-012 | Not Started |
| WI-018 | Add validation rules to glossary request types | Backend | P1 | 1d | WI-013 | Not Started |
| WI-019 | Frontend: GlossaryPage list view (real data) | Frontend | P0 | 2d | WI-011 | Not Started |
| WI-020 | Frontend: Glossary term detail page | Frontend | P0 | 1d | WI-012 | Not Started |
| WI-021 | Frontend: Create/Edit term form | Frontend | P0 | 2d | WI-013, WI-014 | Not Started |
| WI-022 | Frontend: Term relationship viewer | Frontend | P2 | 2d | WI-016 | Not Started |
| WI-023 | Unit tests for glossary API handlers | Backend | P1 | 2d | WI-011-014 | Not Started |
| WI-024 | Integration tests for glossary endpoints | Backend | P1 | 1d | WI-023 | Not Started |

### Phase 1: Workflow Engine

| WI-ID | Title | Type | Priority | Effort | Dependencies | Status |
|-------|-------|------|----------|--------|--------------|--------|
| WI-025 | Implement my_pending_tasks | Backend | P0 | 1d | WI-007 | Not Started |
| WI-026 | Implement get_instance | Backend | P0 | 1d | WI-006 | Not Started |
| WI-027 | Implement transition endpoint | Backend | P0 | 2d | WI-006, WI-008 | Not Started |
| WI-028 | Implement complete_task | Backend | P0 | 2d | WI-007, WI-027 | Not Started |
| WI-029 | Implement workflow SLA calculation | Backend | P2 | 1d | WI-025 | Not Started |
| WI-030 | Frontend: Task inbox page | Frontend | P0 | 2d | WI-025 | Not Started |
| WI-031 | Frontend: Workflow instance detail page | Frontend | P1 | 1d | WI-026 | Not Started |
| WI-032 | Frontend: Approval/rejection form | Frontend | P0 | 1d | WI-027, WI-028 | Not Started |
| WI-033 | Unit tests for workflow handlers | Backend | P1 | 2d | WI-025-028 | Not Started |

### Phase 2: Data Dictionary

| WI-ID | Title | Type | Priority | Effort | Dependencies | Status |
|-------|-------|------|----------|--------|--------------|--------|
| WI-034 | Implement list_elements with filters | Backend | P0 | 2d | WI-003 | Not Started |
| WI-035 | Implement get_element (full view with tech metadata) | Backend | P0 | 2d | WI-034 | Not Started |
| WI-036 | Implement create_element (with workflow) | Backend | P0 | 2d | WI-006 | Not Started |
| WI-037 | Implement update_element | Backend | P1 | 2d | WI-035 | Not Started |
| WI-038 | Implement list_cde (Critical Data Elements) | Backend | P0 | 1d | WI-034 | Not Started |
| WI-039 | Implement list_source_systems | Backend | P1 | 1d | WI-003 | Not Started |
| WI-040 | Implement technical metadata CRUD (schemas, tables, columns) | Backend | P1 | 3d | WI-035 | Not Started |
| WI-041 | Implement naming standards validation engine | Backend | P0 | 2d | - | Not Started |
| WI-042 | Wire naming validation into column creation | Backend | P0 | 1d | WI-040, WI-041 | Not Started |
| WI-043 | Frontend: Data Dictionary list page | Frontend | P0 | 2d | WI-034 | Not Started |
| WI-044 | Frontend: Data element detail page | Frontend | P0 | 2d | WI-035 | Not Started |
| WI-045 | Frontend: Create/Edit data element form | Frontend | P0 | 2d | WI-036, WI-037 | Not Started |
| WI-046 | Frontend: CDE management view | Frontend | P1 | 1d | WI-038 | Not Started |
| WI-047 | Frontend: Technical metadata browser | Frontend | P2 | 3d | WI-040 | Not Started |
| WI-048 | Unit tests for data dictionary handlers | Backend | P1 | 2d | WI-034-040 | Not Started |

### Phase 3: Data Quality

| WI-ID | Title | Type | Priority | Effort | Dependencies | Status |
|-------|-------|------|----------|--------|--------------|--------|
| WI-049 | Implement list_dimensions | Backend | P0 | 0.5d | WI-003 | Not Started |
| WI-050 | Implement list_rules with filters | Backend | P0 | 1d | WI-003 | Not Started |
| WI-051 | Implement create_rule (with workflow) | Backend | P0 | 2d | WI-006 | Not Started |
| WI-052 | Implement update_rule | Backend | P1 | 1d | WI-050 | Not Started |
| WI-053 | Implement get_assessments | Backend | P1 | 1d | WI-050 | Not Started |
| WI-054 | Implement create_assessment (manual assessment entry) | Backend | P1 | 2d | WI-053 | Not Started |
| WI-055 | Implement get_element_scores | Backend | P1 | 2d | WI-053 | Not Started |
| WI-056 | Frontend: Data Quality dashboard | Frontend | P0 | 2d | WI-049, WI-050 | Not Started |
| WI-057 | Frontend: Quality rule management | Frontend | P0 | 2d | WI-050, WI-051 | Not Started |
| WI-058 | Frontend: Assessment results view | Frontend | P1 | 2d | WI-053 | Not Started |
| WI-059 | Frontend: Quality scores dashboard | Frontend | P2 | 2d | WI-055 | Not Started |
| WI-060 | Unit tests for data quality handlers | Backend | P1 | 2d | WI-049-055 | Not Started |

### Phase 4: Applications & Processes

| WI-ID | Title | Type | Priority | Effort | Dependencies | Status |
|-------|-------|------|----------|--------|--------------|--------|
| WI-061 | Implement list_applications | Backend | P0 | 1d | WI-003 | Not Started |
| WI-062 | Implement get_application | Backend | P0 | 1d | WI-061 | Not Started |
| WI-063 | Implement create_application (with workflow) | Backend | P0 | 2d | WI-006 | Not Started |
| WI-064 | Implement update_application | Backend | P1 | 1d | WI-062 | Not Started |
| WI-065 | Implement application interfaces CRUD | Backend | P2 | 2d | WI-062 | Not Started |
| WI-066 | Implement application-data element links | Backend | P1 | 1d | WI-062 | Not Started |
| WI-067 | Implement list_processes | Backend | P0 | 1d | WI-003 | Not Started |
| WI-068 | Implement get_process | Backend | P0 | 1d | WI-067 | Not Started |
| WI-069 | Implement create_process (with workflow) | Backend | P0 | 2d | WI-006 | Not Started |
| WI-070 | Implement process steps CRUD | Backend | P1 | 2d | WI-068 | Not Started |
| WI-071 | Implement list_critical_processes | Backend | P0 | 1d | WI-067 | Not Started |
| WI-072 | Implement process-data element links (triggers CDE propagation) | Backend | P0 | 2d | WI-068 | Not Started |
| WI-073 | Implement process-application links | Backend | P1 | 1d | WI-068 | Not Started |
| WI-074 | Frontend: Applications list page | Frontend | P0 | 2d | WI-061 | Not Started |
| WI-075 | Frontend: Application detail page | Frontend | P0 | 2d | WI-062 | Not Started |
| WI-076 | Frontend: Create/Edit application form | Frontend | P0 | 2d | WI-063, WI-064 | Not Started |
| WI-077 | Frontend: Processes list page | Frontend | P0 | 2d | WI-067 | Not Started |
| WI-078 | Frontend: Process detail page | Frontend | P0 | 2d | WI-068 | Not Started |
| WI-079 | Frontend: Create/Edit process form | Frontend | P0 | 2d | WI-069 | Not Started |
| WI-080 | Frontend: Process step editor | Frontend | P2 | 3d | WI-070 | Not Started |
| WI-081 | Unit tests for applications handlers | Backend | P1 | 1d | WI-061-066 | Not Started |
| WI-082 | Unit tests for processes handlers | Backend | P1 | 2d | WI-067-073 | Not Started |

### Phase 5: Data Lineage

| WI-ID | Title | Type | Priority | Effort | Dependencies | Status |
|-------|-------|------|----------|--------|--------------|--------|
| WI-083 | Implement list_graphs | Backend | P0 | 1d | WI-003 | Not Started |
| WI-084 | Implement get_graph (nodes + edges for visualization) | Backend | P0 | 2d | WI-083 | Not Started |
| WI-085 | Implement create_graph | Backend | P0 | 1d | WI-003 | Not Started |
| WI-086 | Implement add_node | Backend | P0 | 1d | WI-085 | Not Started |
| WI-087 | Implement add_edge | Backend | P0 | 1d | WI-086 | Not Started |
| WI-088 | Implement update_node, delete_node | Backend | P1 | 1d | WI-086 | Not Started |
| WI-089 | Implement update_edge, delete_edge | Backend | P1 | 1d | WI-087 | Not Started |
| WI-090 | Implement impact_analysis (upstream/downstream traversal) | Backend | P0 | 3d | WI-084 | Not Started |
| WI-091 | Frontend: Lineage graph page (React Flow integration) | Frontend | P0 | 4d | WI-084 | Not Started |
| WI-092 | Frontend: Node/edge styling by type | Frontend | P1 | 2d | WI-091 | Not Started |
| WI-093 | Frontend: Impact analysis overlay | Frontend | P0 | 2d | WI-090, WI-091 | Not Started |
| WI-094 | Frontend: Graph creation/editing UI | Frontend | P1 | 3d | WI-091 | Not Started |
| WI-095 | Unit tests for lineage handlers | Backend | P1 | 2d | WI-083-090 | Not Started |

### Phase 6: AI Integration

| WI-ID | Title | Type | Priority | Effort | Dependencies | Status |
|-------|-------|------|----------|--------|--------------|--------|
| WI-096 | Implement Claude API client | Backend | P0 | 2d | - | Not Started |
| WI-097 | Implement OpenAI API client (fallback) | Backend | P1 | 1d | WI-096 | Not Started |
| WI-098 | Implement AI enrichment for glossary terms | Backend | P0 | 2d | WI-096 | Not Started |
| WI-099 | Implement AI enrichment for data elements | Backend | P0 | 2d | WI-096 | Not Started |
| WI-100 | Implement AI suggestion storage (PENDING status) | Backend | P0 | 1d | WI-098 | Not Started |
| WI-101 | Implement AI suggestion accept/reject/modify | Backend | P0 | 2d | WI-100 | Not Started |
| WI-102 | Implement AI feedback recording | Backend | P1 | 1d | WI-101 | Not Started |
| WI-103 | Frontend: AI enrichment button on term detail | Frontend | P0 | 1d | WI-098 | Not Started |
| WI-104 | Frontend: AI suggestion review panel | Frontend | P0 | 2d | WI-100, WI-101 | Not Started |
| WI-105 | Frontend: AI suggestion accept/reject UI | Frontend | P0 | 1d | WI-101 | Not Started |
| WI-106 | Frontend: AI feedback form | Frontend | P2 | 1d | WI-102 | Not Started |
| WI-107 | Unit tests for AI handlers | Backend | P1 | 2d | WI-096-102 | Not Started |

### Phase 7: Notifications

| WI-ID | Title | Type | Priority | Effort | Dependencies | Status |
|-------|-------|------|----------|--------|--------------|--------|
| WI-108 | Implement Microsoft Graph API client | Backend | P0 | 2d | - | Not Started |
| WI-109 | Implement email notification queue | Backend | P0 | 2d | WI-108 | Not Started |
| WI-110 | Implement notification on workflow task assignment | Backend | P0 | 1d | WI-109, WI-007 | Not Started |
| WI-111 | Implement notification on workflow completion | Backend | P1 | 1d | WI-109 | Not Started |
| WI-112 | Implement notification preferences API | Backend | P1 | 2d | WI-003 | Not Started |
| WI-113 | Implement in-app notification storage | Backend | P2 | 1d | - | Not Started |
| WI-114 | Frontend: Notification bell with unread count | Frontend | P2 | 1d | WI-113 | Not Started |
| WI-115 | Frontend: Notification preferences page | Frontend | P2 | 1d | WI-112 | Not Started |
| WI-116 | Unit tests for notification handlers | Backend | P1 | 1d | WI-108-113 | Not Started |
| WI-134 | Implement Entra ID SSO flow (login, callback, token exchange) | Backend | P1 | 3d | WI-001 | Not Started |
| WI-135 | Configure dev-login auto-disable when ENTRA_TENANT_ID is set | Backend | P1 | 0.5d | WI-134 | Not Started |

> **Note (2026-03-18):** WI-134 and WI-135 were moved here from Phase 0. Entra ID SSO requires a configured Microsoft tenant, which is impractical during development. The dev-mode login (WI-002) provides full auth functionality for development and testing. When Entra is configured, the dev-login endpoint is automatically disabled.

### Phase 8: Admin & Polish

| WI-ID | Title | Type | Priority | Effort | Dependencies | Status |
|-------|-------|------|----------|--------|--------------|--------|
| WI-117 | Implement list_users | Backend | P0 | 1d | WI-003 | Not Started |
| WI-118 | Implement get_user | Backend | P0 | 0.5d | WI-117 | Not Started |
| WI-119 | Implement update_user (role assignment) | Backend | P0 | 1d | WI-118 | Not Started |
| WI-120 | Implement list_roles | Backend | P0 | 0.5d | WI-003 | Not Started |
| WI-121 | Populate meta_tables and meta_columns (self-describing) | Backend | P1 | 2d | - | Not Started |
| WI-122 | Implement audit log query endpoint | Backend | P2 | 1d | - | Not Started |
| WI-123 | Implement naming standards management API | Backend | P2 | 2d | WI-041 | Not Started |
| WI-124 | Frontend: User management page | Frontend | P0 | 2d | WI-117, WI-119 | Not Started |
| WI-125 | Frontend: Role assignment UI | Frontend | P0 | 1d | WI-119, WI-120 | Not Started |
| WI-126 | Frontend: Naming standards configuration | Frontend | P2 | 2d | WI-123 | Not Started |
| WI-127 | Frontend: Dashboard with real statistics | Frontend | P0 | 2d | All list endpoints | Not Started |
| WI-128 | Frontend: Recent activity feed | Frontend | P1 | 1d | WI-122 | Not Started |
| WI-129 | End-to-end integration tests | Backend | P0 | 3d | All endpoints | Not Started |
| WI-130 | Performance testing and optimization | Backend | P2 | 2d | WI-129 | Not Started |
| WI-131 | Security review and hardening | Backend | P0 | 2d | WI-129 | Not Started |
| WI-132 | Documentation: API usage guide | Docs | P1 | 1d | All endpoints | Not Started |
| WI-133 | Documentation: Deployment guide | Docs | P1 | 1d | WI-131 | Not Started |

---

## Dependency Map

| WI-ID | Depends On | Dependency Type | Risk if Blocked |
|-------|------------|-----------------|-----------------|
| WI-002 | WI-001 | Finish-to-Start | Critical - no auth without JWT |
| WI-003 | WI-001 | Finish-to-Start | Critical - all protected routes blocked |
| WI-004 | WI-003 | Finish-to-Start | High - no RBAC until auth works |
| WI-006 | WI-005 | Finish-to-Start | Critical - workflow engine blocked |
| WI-007 | WI-006 | Finish-to-Start | High - no task assignment |
| WI-008 | WI-006 | Finish-to-Start | Medium - audit gap |
| WI-010 | WI-002 | Finish-to-Start | Critical - frontend auth blocked |
| WI-013 | WI-006, WI-007 | Finish-to-Start | High - create without workflow |
| WI-019 | WI-011 | Finish-to-Start | High - no data to display |
| WI-027 | WI-006, WI-008 | Finish-to-Start | High - no transitions possible |
| WI-036 | WI-006 | Finish-to-Start | High - elements without workflow |
| WI-041 | - | Independent | Medium - naming not enforced |
| WI-042 | WI-040, WI-041 | Finish-to-Start | Medium - columns bypass validation |
| WI-072 | WI-068 | Finish-to-Start | High - CDE triggers not tested |
| WI-090 | WI-084 | Finish-to-Start | High - impact analysis blocked |
| WI-091 | WI-084 | Finish-to-Start | High - no visualization data |
| WI-096 | - | Independent | Low - can stub for testing |
| WI-098 | WI-096 | Finish-to-Start | High - AI enrichment blocked |
| WI-109 | WI-108 | Finish-to-Start | High - no email sending |
| WI-110 | WI-109, WI-007 | Finish-to-Start | Medium - tasks without notification |
| WI-129 | All endpoints | Finish-to-Start | Low - can run partial |

---

## Delivery Roadmap

### Phase 0: Foundation
**Duration**: Sprints 1-2 (4 weeks)
**Objective**: Establish authentication, authorisation, and workflow engine - the infrastructure all features depend on.

**Deliverables**:
- WI-001 through WI-010

**Exit Criteria**:
- [ ] Users can log in via Microsoft Entra ID SSO
- [ ] JWT tokens are issued and validated
- [ ] Protected routes reject unauthenticated requests (401)
- [ ] RBAC middleware rejects unauthorised requests (403)
- [ ] Workflow state transitions work in isolation
- [ ] Frontend AuthProvider stores JWT and user info
- [ ] Test utilities module ready for use

**Key Risks**:
- Entra ID configuration issues (mitigation: use test tenant, document setup)
- JWT library compatibility (mitigation: proven library, early testing)

---

### Phase 1: First Vertical Slice
**Duration**: Sprint 3 (2 weeks)
**Objective**: Deliver a complete end-to-end flow: create a glossary term, approve it via workflow, view it on the frontend. This proves the architecture works.

**Deliverables**:
- WI-011 through WI-024, WI-025 through WI-033

**Exit Criteria**:
- [ ] Glossary terms can be created via API (Draft state)
- [ ] Terms can be submitted for review (workflow transition)
- [ ] Reviewers see tasks in their pending list
- [ ] Reviewers can approve/reject tasks
- [ ] Approved terms become ACCEPTED
- [ ] Frontend displays glossary list with pagination
- [ ] Frontend displays term details
- [ ] Frontend allows creating new terms
- [ ] Frontend displays task inbox with pending approvals
- [ ] Unit tests pass for all implemented handlers

**Key Risks**:
- Workflow complexity (mitigation: simplest path first, hardcoded definitions)
- Full-text search performance (mitigation: PostgreSQL GIN indexes already in schema)

---

### Phase 2: Data Dictionary
**Duration**: Sprints 4-5 (4 weeks)
**Objective**: Complete the data dictionary domain, including CDE designation and naming standards validation.

**Deliverables**:
- WI-034 through WI-048

**Exit Criteria**:
- [ ] Data elements can be created, updated, viewed
- [ ] Elements flow through workflow approval
- [ ] CDE list endpoint returns critical data elements
- [ ] Technical metadata (schemas, tables, columns) can be managed
- [ ] Naming standards validation runs on column creation
- [ ] Frontend shows data dictionary with CDE indicators
- [ ] Unit tests pass

**Key Risks**:
- CDE trigger behaviour (mitigation: integration tests, DB-level verification)
- Naming regex complexity (mitigation: use existing patterns from migration)

---

### Phase 3: Data Quality
**Duration**: Sprint 6 (2 weeks)
**Objective**: Quality rules and assessments, connecting to data elements.

**Deliverables**:
- WI-049 through WI-060

**Exit Criteria**:
- [ ] Quality dimensions are queryable
- [ ] Quality rules can be created and linked to elements
- [ ] Assessment results can be recorded
- [ ] Quality scores can be retrieved per element
- [ ] Frontend shows quality dashboard
- [ ] Unit tests pass

**Key Risks**:
- Assessment data model complexity (mitigation: start with manual entry only)

---

### Phase 4: Applications & Processes
**Duration**: Sprints 7-8 (4 weeks)
**Objective**: Complete the application registry and process registry, including CDE propagation via critical processes.

**Deliverables**:
- WI-061 through WI-082

**Exit Criteria**:
- [ ] Applications can be managed with workflow
- [ ] Processes can be managed with workflow
- [ ] Critical processes auto-designate linked elements as CDEs (trigger works)
- [ ] Process steps can be added/edited
- [ ] Process-element and process-application links work
- [ ] Frontend pages for both domains functional
- [ ] Unit tests pass

**Key Risks**:
- CDE propagation trigger correctness (mitigation: specific test cases)
- Process step editor complexity (mitigation: defer to P2, simple list first)

---

### Phase 5: Data Lineage
**Duration**: Sprints 9-10 (4 weeks)
**Objective**: Lineage graphs with visualization and impact analysis.

**Deliverables**:
- WI-083 through WI-095

**Exit Criteria**:
- [ ] Graphs can be created with nodes and edges
- [ ] Graph data returned in React Flow compatible format
- [ ] Impact analysis traverses upstream/downstream
- [ ] Frontend renders interactive lineage graph
- [ ] Nodes colored by type
- [ ] Impact analysis highlights affected nodes
- [ ] Unit tests pass

**Key Risks**:
- Graph traversal performance (mitigation: depth limits, indexed queries)
- React Flow learning curve (mitigation: reference examples, start simple)

---

### Phase 6: AI Integration
**Duration**: Sprints 11-12 (4 weeks)
**Objective**: AI-powered metadata enrichment with human review.

**Deliverables**:
- WI-096 through WI-107

**Exit Criteria**:
- [ ] Claude API client works with financial services prompts
- [ ] OpenAI fallback activates on Claude failure
- [ ] AI suggestions stored with PENDING status
- [ ] Users can accept/reject/modify suggestions
- [ ] Accepted suggestions update the entity
- [ ] Frontend shows AI enrichment UI
- [ ] Feedback can be recorded
- [ ] Unit tests pass (mocked AI responses)

**Key Risks**:
- API rate limits (mitigation: queue requests, retry logic)
- Prompt engineering (mitigation: iterate on prompts, test with real terms)
- Cost management (mitigation: use cheaper models for dev, monitor usage)

---

### Phase 7: Notifications
**Duration**: Sprint 13 (2 weeks)
**Objective**: Email notifications for workflow events.

**Deliverables**:
- WI-108 through WI-116

**Exit Criteria**:
- [ ] Microsoft Graph API client sends emails
- [ ] Queue prevents blocking API responses
- [ ] Task assignments trigger email notifications
- [ ] Workflow completions trigger notifications
- [ ] Notification preferences can be set
- [ ] Unit tests pass

**Key Risks**:
- Graph API auth complexity (mitigation: service principal with minimal scope)
- Email deliverability (mitigation: test with real mailbox)

---

### Phase 8: Admin & Production Readiness
**Duration**: Sprint 14 (2 weeks)
**Objective**: User management, self-describing metadata, testing, and production hardening.

**Deliverables**:
- WI-117 through WI-133

**Exit Criteria**:
- [ ] User management UI works
- [ ] Roles can be assigned to users
- [ ] meta_tables and meta_columns populated
- [ ] Dashboard shows real statistics
- [ ] End-to-end integration tests pass
- [ ] Security review completed, no critical findings
- [ ] API usage guide and deployment guide written
- [ ] Performance acceptable under load

**Key Risks**:
- Test coverage gaps (mitigation: systematic test matrix)
- Undiscovered security issues (mitigation: dependency audit, OWASP checklist)

---

## Sprint Plan

### Sprint 1: Auth Foundation
**Duration**: 2 weeks
**Theme**: Authentication infrastructure (dev-mode — Entra SSO deferred to Sprint 13)

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-001 | JWT token issuance and validation | 5 | - | - | Tokens issued with claims, validation rejects expired/invalid tokens |
| WI-002 | Dev-mode login (password_hash, seed users, endpoint) | 5 | - | WI-001 | POST /api/v1/auth/dev-login accepts email+password, returns JWT. Seeded admin + test users available |
| WI-003 | require_auth middleware | 3 | - | WI-001 | Protected routes return 401 without valid JWT, Claims injected into request extensions |
| WI-009 | Test utilities module | 2 | - | - | Fixture builders, test DB setup helper available |

**Sprint Goal**: Users can log in via dev-mode (email + password) and receive a valid JWT. Protected routes enforce authentication.
**Capacity**: 15 points
**Committed**: 15 points

---

### Sprint 2: Workflow Foundation
**Duration**: 2 weeks
**Theme**: Workflow engine and RBAC

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-004 | require_role middleware | 3 | - | WI-003 | Endpoints reject users without required role (403) |
| WI-005 | Seed workflow states and definitions | 2 | - | - | All entity types have active workflow definitions with transitions |
| WI-006 | Workflow state transition logic | 5 | - | WI-005 | Valid transitions succeed, invalid return error, entity status updated |
| WI-007 | Workflow task creation and assignment | 3 | - | WI-006 | Tasks created on UNDER_REVIEW state, assigned to DATA_STEWARD role |
| WI-008 | Workflow history recording | 2 | - | WI-006 | Every transition logged with user, timestamp, comments |
| WI-010 | Frontend AuthProvider + dev-login page | 3 | - | WI-002 | Login page with email/password, JWT stored, user context available, logout works |

**Sprint Goal**: Workflow engine can transition entities through states with task assignment. Frontend has working login.
**Capacity**: 18 points
**Committed**: 18 points

---

### Sprint 3: Glossary Vertical Slice
**Duration**: 2 weeks
**Theme**: End-to-end glossary flow

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-011 | list_terms with pagination | 3 | - | WI-003 | Returns paginated results with full-text search |
| WI-012 | get_term | 2 | - | WI-003 | Returns term by ID, 404 if not found |
| WI-013 | create_term with workflow | 3 | - | WI-006, WI-007 | Creates term in DRAFT, initiates workflow |
| WI-014 | update_term with versioning | 3 | - | WI-012 | Updates term, creates new version if ACCEPTED |
| WI-015 | list_domains | 1 | - | WI-003 | Returns all glossary domains |
| WI-025 | my_pending_tasks | 2 | - | WI-007 | Returns tasks for authenticated user |
| WI-027 | transition endpoint | 3 | - | WI-006, WI-008 | Validates and executes state transition |
| WI-028 | complete_task | 3 | - | WI-007, WI-027 | Completes task, updates instance state |

**Sprint Goal**: A glossary term can be created and approved through the workflow.
**Capacity**: 20 points
**Committed**: 20 points

---

### Sprint 4: Glossary Frontend + Data Dictionary Backend
**Duration**: 2 weeks
**Theme**: Frontend completion and next domain backend

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-019 | Frontend: GlossaryPage list view | 3 | - | WI-011 | Displays terms, pagination, search works |
| WI-020 | Frontend: Term detail page | 2 | - | WI-012 | Shows all term fields |
| WI-021 | Frontend: Create/Edit term form | 3 | - | WI-013, WI-014 | Form submits, validation errors shown |
| WI-030 | Frontend: Task inbox | 3 | - | WI-025 | Shows pending tasks, links to entities |
| WI-032 | Frontend: Approval form | 2 | - | WI-027, WI-028 | Approve/reject with comments |
| WI-034 | list_elements | 3 | - | WI-003 | Returns paginated data elements |
| WI-035 | get_element (full view) | 3 | - | WI-034 | Returns element with tech metadata counts |

**Sprint Goal**: Frontend users can manage glossary terms and see their tasks.
**Capacity**: 19 points
**Committed**: 19 points

---

### Sprint 5: Data Dictionary Completion
**Duration**: 2 weeks
**Theme**: Data dictionary and naming standards

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-036 | create_element with workflow | 3 | - | WI-006 | Creates element in DRAFT, initiates workflow |
| WI-037 | update_element | 2 | - | WI-035 | Updates element fields |
| WI-038 | list_cde | 2 | - | WI-034 | Returns only CDEs |
| WI-041 | Naming standards validation engine | 3 | - | - | Validates names against patterns from DB |
| WI-040 | Technical metadata CRUD | 5 | - | WI-035 | Schemas, tables, columns CRUD |
| WI-042 | Wire naming validation | 2 | - | WI-040, WI-041 | Column creation validates naming |
| WI-043 | Frontend: Data Dictionary list | 3 | - | WI-034 | Displays elements, CDE indicators |

**Sprint Goal**: Data dictionary is fully manageable with naming standards enforced.
**Capacity**: 20 points
**Committed**: 20 points

---

### Sprint 6: Data Quality + Dictionary Frontend
**Duration**: 2 weeks
**Theme**: Data quality domain

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-044 | Frontend: Data element detail | 3 | - | WI-035 | Shows element with tech metadata |
| WI-045 | Frontend: Create/Edit element form | 3 | - | WI-036, WI-037 | Form works, validation shown |
| WI-049 | list_dimensions | 1 | - | WI-003 | Returns 6 quality dimensions |
| WI-050 | list_rules | 2 | - | WI-003 | Returns quality rules with filters |
| WI-051 | create_rule | 3 | - | WI-006 | Creates rule with workflow |
| WI-053 | get_assessments | 2 | - | WI-050 | Returns assessment history |
| WI-055 | get_element_scores | 3 | - | WI-053 | Returns quality scores per dimension |
| WI-056 | Frontend: Data Quality dashboard | 3 | - | WI-049, WI-050 | Displays dimensions, rule counts |

**Sprint Goal**: Data quality rules can be managed and scores viewed.
**Capacity**: 20 points
**Committed**: 20 points

---

### Sprint 7: Applications
**Duration**: 2 weeks
**Theme**: Application registry

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-057 | Frontend: Quality rule management | 3 | - | WI-050, WI-051 | List and create rules |
| WI-061 | list_applications | 2 | - | WI-003 | Returns paginated applications |
| WI-062 | get_application | 2 | - | WI-061 | Returns application details |
| WI-063 | create_application | 3 | - | WI-006 | Creates with workflow |
| WI-064 | update_application | 2 | - | WI-062 | Updates application |
| WI-066 | Application-data element links | 2 | - | WI-062 | Links elements to applications |
| WI-074 | Frontend: Applications list | 3 | - | WI-061 | Displays applications |
| WI-075 | Frontend: Application detail | 3 | - | WI-062 | Shows application with links |

**Sprint Goal**: Applications can be registered and linked to data elements.
**Capacity**: 20 points
**Committed**: 20 points

---

### Sprint 8: Processes + CDE Propagation
**Duration**: 2 weeks
**Theme**: Process registry with CDE triggers

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-076 | Frontend: Application form | 3 | - | WI-063, WI-064 | Create/edit works |
| WI-067 | list_processes | 2 | - | WI-003 | Returns paginated processes |
| WI-068 | get_process | 2 | - | WI-067 | Returns process details |
| WI-069 | create_process | 3 | - | WI-006 | Creates with workflow |
| WI-071 | list_critical_processes | 2 | - | WI-067 | Returns only critical processes |
| WI-072 | Process-element links (CDE trigger) | 3 | - | WI-068 | Links trigger CDE propagation |
| WI-077 | Frontend: Processes list | 3 | - | WI-067 | Displays processes |
| WI-078 | Frontend: Process detail | 3 | - | WI-068 | Shows process with CDE impact |

**Sprint Goal**: Critical business processes automatically designate linked elements as CDEs.
**Capacity**: 21 points
**Committed**: 21 points

---

### Sprint 9: Lineage Backend
**Duration**: 2 weeks
**Theme**: Lineage graphs and traversal

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-079 | Frontend: Process form | 3 | - | WI-069 | Create/edit works |
| WI-083 | list_graphs | 2 | - | WI-003 | Returns lineage graphs |
| WI-084 | get_graph (visualization data) | 3 | - | WI-083 | Returns nodes and edges for React Flow |
| WI-085 | create_graph | 2 | - | WI-003 | Creates empty graph |
| WI-086 | add_node | 2 | - | WI-085 | Adds node to graph |
| WI-087 | add_edge | 2 | - | WI-086 | Adds edge between nodes |
| WI-090 | impact_analysis | 5 | - | WI-084 | Traverses graph upstream/downstream |

**Sprint Goal**: Lineage graphs can be built and impact analysis works.
**Capacity**: 19 points
**Committed**: 19 points

---

### Sprint 10: Lineage Frontend
**Duration**: 2 weeks
**Theme**: Interactive lineage visualization

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-091 | Frontend: Lineage graph (React Flow) | 8 | - | WI-084 | Renders interactive graph |
| WI-092 | Frontend: Node styling by type | 3 | - | WI-091 | Different colors/shapes per type |
| WI-093 | Frontend: Impact analysis overlay | 5 | - | WI-090, WI-091 | Highlights affected nodes |

**Sprint Goal**: Users can visualize and explore data lineage interactively.
**Capacity**: 16 points
**Committed**: 16 points

---

### Sprint 11: AI Integration Backend
**Duration**: 2 weeks
**Theme**: AI enrichment services

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-094 | Frontend: Graph editing UI | 5 | - | WI-091 | Add nodes/edges via UI |
| WI-096 | Claude API client | 3 | - | - | Sends prompts, parses responses |
| WI-097 | OpenAI fallback | 2 | - | WI-096 | Activates on Claude failure |
| WI-098 | AI enrichment for glossary | 3 | - | WI-096 | Generates definition suggestions |
| WI-099 | AI enrichment for data elements | 3 | - | WI-096 | Generates descriptions, classifications |
| WI-100 | AI suggestion storage | 2 | - | WI-098 | Stores suggestions as PENDING |
| WI-101 | AI suggestion accept/reject | 3 | - | WI-100 | Updates entity on accept |

**Sprint Goal**: AI can generate suggestions that are stored for human review.
**Capacity**: 21 points
**Committed**: 21 points

---

### Sprint 12: AI Frontend + Notifications Backend
**Duration**: 2 weeks
**Theme**: AI UI and notifications

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-102 | AI feedback recording | 2 | - | WI-101 | Stores user ratings |
| WI-103 | Frontend: AI enrichment button | 2 | - | WI-098 | Triggers enrichment from detail page |
| WI-104 | Frontend: AI suggestion panel | 3 | - | WI-100, WI-101 | Displays suggestions |
| WI-105 | Frontend: Accept/reject UI | 2 | - | WI-101 | Buttons update suggestion status |
| WI-108 | Microsoft Graph API client | 3 | - | - | Sends email via Graph |
| WI-109 | Email notification queue | 3 | - | WI-108 | Queue processes async |
| WI-110 | Notification on task assignment | 2 | - | WI-109, WI-007 | Sends email when task created |
| WI-111 | Notification on completion | 2 | - | WI-109 | Sends email on workflow done |

**Sprint Goal**: AI suggestions reviewable on frontend, email notifications sent.
**Capacity**: 19 points
**Committed**: 19 points

---

### Sprint 13: Admin
**Duration**: 2 weeks
**Theme**: User management and preferences

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-112 | Notification preferences API | 3 | - | WI-003 | Users can set preferences |
| WI-117 | list_users | 2 | - | WI-003 | Returns paginated users |
| WI-118 | get_user | 1 | - | WI-117 | Returns user details |
| WI-119 | update_user (role assignment) | 2 | - | WI-118 | Updates user roles |
| WI-120 | list_roles | 1 | - | WI-003 | Returns all roles |
| WI-121 | Self-describing metadata | 3 | - | - | Populates meta_tables, meta_columns |
| WI-124 | Frontend: User management | 3 | - | WI-117, WI-119 | List and manage users |
| WI-125 | Frontend: Role assignment | 2 | - | WI-119, WI-120 | Assign roles to users |
| WI-127 | Frontend: Dashboard statistics | 3 | - | All list endpoints | Real counts on dashboard |

**Sprint Goal**: Admins can manage users and roles, dashboard shows real data.
**Capacity**: 20 points
**Committed**: 20 points

---

### Sprint 14: Production Readiness
**Duration**: 2 weeks
**Theme**: Testing, security, documentation

| Story ID | Title | Points | Owner | Dependencies | Acceptance Criteria |
|----------|-------|--------|-------|--------------|---------------------|
| WI-128 | Frontend: Recent activity | 2 | - | WI-122 | Shows recent changes |
| WI-129 | End-to-end integration tests | 5 | - | All endpoints | All major flows tested |
| WI-130 | Performance testing | 3 | - | WI-129 | Load test, optimize bottlenecks |
| WI-131 | Security review | 3 | - | WI-129 | Dependency audit, OWASP checklist |
| WI-132 | API usage guide | 2 | - | All endpoints | Complete API documentation |
| WI-133 | Deployment guide | 2 | - | WI-131 | Production deployment documented |
| WI-122 | Audit log query endpoint | 2 | - | - | Returns audit log entries |

**Sprint Goal**: Production-ready with documentation and security review complete.
**Capacity**: 19 points
**Committed**: 19 points

---

## RAID Log

### Risks

| RISK-ID | Description | Prob | Impact | Score | Mitigation | Owner | Status |
|---------|-------------|------|--------|-------|------------|-------|--------|
| R-001 | Entra ID SSO configuration complex or delayed | Medium | High | 6 | Use test tenant, document setup early, fallback to local auth for dev | - | Open |
| R-002 | Workflow engine too complex for single developer | Medium | High | 6 | Simplest implementation first, iterate; defer advanced features | - | Open |
| R-003 | AI API costs exceed budget | Low | Medium | 3 | Monitor usage, use cheaper models for dev, implement caching | - | Open |
| R-004 | AI prompt engineering requires extensive iteration | Medium | Medium | 4 | Start with proven prompts, test with real data, allow time in sprint | - | Open |
| R-005 | React Flow learning curve slows lineage UI | Medium | Medium | 4 | Use official examples, start with simple graph, defer editing | - | Open |
| R-006 | CDE propagation triggers have edge cases | Medium | High | 6 | Integration tests for all trigger scenarios, DB-level verification | - | Open |
| R-007 | Microsoft Graph API auth configuration delays notifications | Medium | Medium | 4 | Document setup, use service principal, test early | - | Open |
| R-008 | Performance issues with large datasets | Low | Medium | 3 | Use pagination consistently, index all FK columns, test with volume | - | Open |
| R-009 | Frontend build complexity as app grows | Low | Low | 2 | Follow established patterns, lazy loading, review bundle size | - | Open |
| R-010 | Test coverage gaps lead to production bugs | Medium | High | 6 | Systematic test matrix, mandatory tests for each handler, integration tests | - | Open |
| R-011 | Security vulnerabilities in dependencies | Low | Critical | 4 | Run cargo-audit weekly, address critical findings immediately | - | Open |
| R-012 | Single point of failure (single developer) | High | High | 9 | Comprehensive documentation, AI-maintainable code, no undocumented knowledge | - | Accepted |

### Assumptions

| ASSM-ID | Assumption | If Wrong, Impact | Validation Action |
|---------|------------|------------------|-------------------|
| A-001 | Entra ID test tenant available | Auth development blocked | Confirm tenant access before Sprint 1 |
| A-002 | Claude API key available and has quota | AI features delayed | Confirm API access before Sprint 11 |
| A-003 | Microsoft Graph API credentials available | Email notifications delayed | Confirm credentials before Sprint 12 |
| A-004 | PostgreSQL 17 runs without issues | Schema changes needed | Test early with Docker Compose |
| A-005 | 8 hours/day developer availability | Timeline extends | Adjust sprint points if capacity lower |
| A-006 | Workflow requirements stable | Rework required | Document all workflow rules early |
| A-007 | Existing schema is correct and complete | Migration work needed | Review schema against domain models |

### Issues

| ISSUE-ID | Description | Raised | Impact | Resolution | Owner | Due | Status |
|----------|-------------|--------|--------|------------|-------|-----|--------|
| - | No current issues | - | - | - | - | - | - |

### Dependencies

| DEP-ID | Description | Provider | Due Date | Status | Risk if Late |
|--------|-------------|----------|----------|--------|--------------|
| D-001 | Entra ID tenant configuration | IT/Self | Sprint 1 Start | Not Started | Auth development blocked |
| D-002 | Claude API key and quota | Anthropic | Sprint 11 Start | Not Started | AI features delayed |
| D-003 | OpenAI API key (fallback) | OpenAI | Sprint 11 Start | Not Started | No AI fallback |
| D-004 | Microsoft Graph API credentials | IT/Self | Sprint 12 Start | Not Started | Email blocked |

---

## Metrics Dashboard

| Metric | Target | Actual | Trend |
|--------|--------|--------|-------|
| Velocity (SP/sprint) | 18-20 | - | - |
| Sprint Goal Achievement | 90% | - | - |
| Defect Escape Rate | <5% | - | - |
| Code Coverage (backend) | 80% | - | - |
| Open Critical Defects | 0 | - | - |
| RAID Items (open) | <10 | 12 | - |
| API Endpoints Implemented | 40+ | 0 | - |
| Pre-commit Check Pass Rate | 100% | - | - |

---

## Priority Rationale

The sequencing follows these principles:

### 1. Foundation First (Sprints 1-2)
Authentication and workflow are cross-cutting concerns that every feature depends on. Without JWT validation, no protected endpoint can be tested realistically. Without the workflow engine, no entity can move through its lifecycle. These must come first.

### 2. Vertical Slice Over Horizontal Slice (Sprint 3)
Rather than implementing all backend endpoints before any frontend work, Sprint 3 delivers a complete end-to-end flow for glossary terms. This:
- Validates the architecture works
- Provides immediate visible progress
- Uncovers integration issues early
- Creates a pattern for subsequent domains

### 3. Domain Complexity Ordering (Sprints 4-10)
Domains are sequenced by dependency and complexity:

1. **Glossary** (Sprint 3-4): Foundation domain, referenced by others
2. **Data Dictionary** (Sprint 5-6): References glossary terms, most used domain
3. **Data Quality** (Sprint 6): References data elements
4. **Applications** (Sprint 7): Simpler than processes
5. **Processes** (Sprint 8): Complex due to CDE propagation triggers
6. **Lineage** (Sprint 9-10): Most complex UI, benefits from stable backend

### 4. AI and Notifications Later (Sprints 11-13)
AI enrichment and notifications are enhancement features. Core CRUD and workflow must work first. Deferring these:
- Allows external API credentials to be obtained
- Provides stable entities for AI to enrich
- Ensures notifications have events to trigger on

### 5. Admin and Polish Last (Sprints 13-14)
User management, dashboards, and documentation benefit from all features being complete. Testing and security review should happen when the codebase is stable.

### Single Developer Optimisation
The plan accounts for a single developer (AI-assisted) by:
- Limiting sprint points to achievable levels
- Grouping related work to minimise context switching
- Alternating between backend and frontend within sprints
- Building test utilities early for efficient testing
- Creating reusable patterns (first domain establishes patterns for others)

---

## Critical Path

The critical path runs through:

1. **WI-001** (JWT) -> **WI-002** (SSO) -> **WI-003** (require_auth) -> All protected endpoints
2. **WI-005** (Workflow seed) -> **WI-006** (Transitions) -> **WI-007** (Tasks) -> All create endpoints
3. **WI-084** (get_graph) -> **WI-090** (impact_analysis) -> **WI-091** (Frontend graph)
4. **WI-096** (Claude client) -> **WI-098** (Glossary AI) -> **WI-100** (Suggestion storage)

Any delay on the critical path delays the project. Near-critical paths include:
- Naming standards (WI-041 -> WI-042) - can run in parallel
- Graph API (WI-108 -> WI-109 -> WI-110) - can run in parallel

---

## Change Log

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-03-18 | Hendrik de Beer | Initial plan created |
