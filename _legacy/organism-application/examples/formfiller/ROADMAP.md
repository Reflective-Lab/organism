# FormFiller Roadmap

A reliable, auditable form completion system.

## Target Context (To Define)

- [ ] **Segment**: Consumer / SMB / Enterprise compliance
- [ ] **First 3 Form Families**: e.g., KYC onboarding, vendor questionnaires, HR/benefits
- [ ] **Integration Level**: Browser extension only / API access / Full RPA

---

## Phase 1: Foundation (Current)

### 1.1 Core Infrastructure ✓
- [x] Rust TUI application scaffold
- [x] Profile data model (personal, contact, address, ID)
- [x] Form config model (single-page and multi-step)
- [x] Local JSON storage
- [x] WebDriver-based form filling
- [x] CLI interface (tui, learn, analyze, fill, list)

### 1.2 Test Environment ✓
- [x] Docker-based test forms (12 forms)
- [x] Multi-step wizard forms (Questback/Hermods style)
- [x] Client-side interaction logging
- [x] Server-side log aggregation
- [x] Field suggestion API from logs

### 1.3 Learning System ✓
- [x] Form analyzer (field discovery from URL)
- [x] Data source guessing (Swedish + English field names)
- [x] Auto-generate FormConfig from analysis
- [x] Learn command to update configs from logs

---

## Phase 2: Form Understanding & Field Intelligence

### 2.1 Multi-Source Form Ingestion
- [ ] Web form detection and parsing (current: WebDriver)
- [ ] PDF form field extraction (pdftk, pdf-rs)
- [ ] Scanned PDF / image OCR (tesseract integration)
- [ ] Word document parsing (.docx form fields)
- [ ] Portal/iframe handling

### 2.2 Structure Detection
- [ ] Section detection (fieldsets, headers, visual grouping)
- [ ] Repeating block detection (dependents, employment history)
- [ ] Conditional field detection (show/hide based on values)
- [ ] Signature block detection
- [ ] Multi-page form navigation

### 2.3 Field Semantics Engine
- [ ] Build field ontology (canonical field types)
- [ ] Synonym mapping ("Org nr" = "Company reg no" = "Registration ID")
- [ ] Language detection (Swedish, English, etc.)
- [ ] Context-aware disambiguation ("ID number" → passport vs national ID)

### 2.4 Constraint Extraction
- [ ] Required/optional detection
- [ ] Min/max length from attributes + validation
- [ ] Format detection (IBAN, SSN, personnummer, VAT, phone)
- [ ] Allowed values extraction (dropdowns, radio groups)
- [ ] Date rules (min/max dates, relative dates)
- [ ] Cross-field dependencies

---

## Phase 3: Profile Vault & Data Model

### 3.1 Enhanced Profile Schema
- [ ] Legal name vs preferred name
- [ ] Multiple nationalities
- [ ] ID documents with expiry dates, issuing authority
- [ ] Multiple addresses with validity periods
- [ ] Employment history (not just current)
- [ ] Dependents/family members
- [ ] Business entity data (if applicable)

### 3.2 Provenance & Versioning
- [ ] Source tracking per field ("from passport scan", "user entered")
- [ ] Verification status and date
- [ ] Historical versions ("address as of 2024-01-01")
- [ ] Confidence scores per fact

### 3.3 Document Vault
- [ ] Attachment storage (passport, proof of address, etc.)
- [ ] Document type classification
- [ ] Validity tracking (e.g., "proof of address < 90 days")
- [ ] Auto-match documents to form requirements

### 3.4 Rendering Rules
- [ ] Address formatting per locale
- [ ] Name formatting (middle initial vs full)
- [ ] Phone formatting (E.164 vs local)
- [ ] Date formatting (ISO vs locale)
- [ ] Country code variants ("Sweden" vs "SE" vs "SWE")

---

## Phase 4: Validation & Error Prevention

### 4.1 Pre-Submit Validation
- [ ] Format validation (regex patterns)
- [ ] Cross-field consistency (DOB vs ID document dates)
- [ ] Logical checks (end date after start date)
- [ ] Personnummer checksum validation ✓ (implemented)

### 4.2 Institution-Specific Rules
- [ ] Validation playbooks per form type/vendor
- [ ] Known quirks database ("no spaces in postcode")
- [ ] Value dictionaries per vendor

### 4.3 Confidence Scoring
- [ ] Field-level confidence (0-1)
- [ ] Uncertain fields flagged for review
- [ ] "Ask vs guess" policy enforcement

---

## Phase 5: Review-First UX

### 5.1 TUI Enhancements
- [ ] Diff-style review (what was filled, from where)
- [ ] Field-by-field approval
- [ ] Partial approval workflow
- [ ] Undo/edit before submit

### 5.2 Audit Trail
- [ ] Who filled each field
- [ ] When it was filled
- [ ] What source/evidence was used
- [ ] Exportable audit log

### 5.3 Approval Workflow
- [ ] "Approve to write" mode (never silent submit)
- [ ] Risk-level based approval (low-stakes auto, high-stakes review)
- [ ] Multi-person approval for enterprise

---

## Phase 6: Security, Privacy & Compliance

### 6.1 Access Control
- [ ] Least-privilege data retrieval
- [ ] Per-form data scoping
- [ ] Sensitive field masking in logs

### 6.2 Consent Model
- [ ] Per-category consent (identity, finance, health, employment)
- [ ] Consent tracking and audit
- [ ] Withdrawal handling

### 6.3 Data Protection
- [ ] Encryption at rest (SQLCipher or age)
- [ ] Encryption in transit
- [ ] Secure memory handling for sensitive data
- [ ] Cache expiration policies
- [ ] Data minimization (only store what's needed)

### 6.4 Multi-Tenant (Enterprise)
- [ ] Organization isolation
- [ ] Role-based access
- [ ] Approval policies per org
- [ ] Audit per tenant

---

## Phase 7: Integrations & Automation

### 7.1 Identity & Auth
- [ ] OIDC provider integration
- [ ] Password manager integration (1Password, Bitwarden CLI)
- [ ] BankID integration (Swedish)

### 7.2 Document Sources
- [ ] Google Drive integration
- [ ] OneDrive/SharePoint integration
- [ ] Local filesystem scanning

### 7.3 Business Systems
- [ ] HRIS integration (HR data)
- [ ] ERP/CRM integration (company data)
- [ ] Accounting tool integration

### 7.4 Automation Modes
- [ ] Browser automation (current: WebDriver)
- [ ] API submission where available
- [ ] Headless vs visible browser options
- [ ] Retry and error recovery

### 7.5 Watchlists & Scheduling
- [ ] Recurring form detection
- [ ] Renewal reminders (insurance, compliance)
- [ ] Form availability monitoring (e.g., Hermods slots)

---

## Phase 8: Agent Architecture

### 8.1 Extractor Agent
- [ ] Parse form into structured schema
- [ ] Extract fields, constraints, sections
- [ ] Handle multi-page and conditionals

### 8.2 Mapper Agent
- [ ] Map fields to ontology concepts
- [ ] Select candidate facts from vault
- [ ] Resolve ambiguity

### 8.3 Retriever Agent
- [ ] Pull facts with provenance
- [ ] Pull supporting documents
- [ ] Check validity/freshness

### 8.4 Validator Agent
- [ ] Run deterministic validation
- [ ] Check cross-field consistency
- [ ] Enforce institution rules

### 8.5 Clarifier Agent
- [ ] Generate minimal question set
- [ ] Smart disambiguation questions
- [ ] Remember preferences per vendor

### 8.6 Writer/Executor Agent
- [ ] Write values to form (UI/API/PDF)
- [ ] Report diff of changes
- [ ] Handle errors gracefully

### 8.7 Compliance Agent
- [ ] Enforce PII policies
- [ ] Jurisdiction rules
- [ ] Retention policies
- [ ] Audit logging

---

## Form Families Reference

### Personal & Identity
- Account signup / KYC onboarding
- Travel visa applications
- Government services (residency, permits)
- Healthcare intake and insurance claims

### Employment & HR
- Job applications
- Background checks
- Benefits enrollment
- Timesheets, expense claims, travel requests

### Finance & Banking
- Loan/mortgage applications
- Corporate bank onboarding
- Payment provider onboarding (merchant KYC)
- Tax forms (individual and corporate)

### Legal & Compliance
- NDAs, DPAs, vendor risk questionnaires
- SOC2/ISO evidence requests
- Beneficial ownership declarations
- Import/export and customs docs

### Procurement & B2B
- Supplier onboarding portals
- RFP/RFI responses
- Security questionnaires (SIG, CAIQ)
- Partner ecosystem registrations

### Education & Membership
- University applications
- Grants, scholarship applications
- Professional association memberships

---

## Data Model Reference

### Core Facts (Structured)
```
Identity
├── legal_name (first, middle, last)
├── preferred_name
├── date_of_birth
├── place_of_birth
├── nationalities[]
├── ids[]
│   ├── type (passport, national_id, drivers_license)
│   ├── number
│   ├── issuing_authority
│   ├── issue_date
│   └── expiry_date

Contact
├── emails[]
├── phones[]
└── preferences (language, format)

Addresses[]
├── type (current, previous, billing, shipping)
├── street, city, postal_code, country
├── start_date, end_date
└── proof_document_id

Employment[]
├── employer
├── role
├── start_date, end_date
├── income_range
└── is_current

Business (if applicable)
├── legal_entity_name
├── registration_number
├── vat_number
├── duns
├── directors[]
├── beneficial_owners[]
└── industry_codes (NAICS, NACE)

Financial
├── bank_accounts[]
├── tax_residencies[]
└── invoicing_details

Dependents[]
├── name
├── relationship
├── date_of_birth
```

### Evidence Documents
```
Documents[]
├── type (passport, proof_of_address, employment_letter, etc.)
├── file_path
├── issue_date
├── expiry_date
├── validity_rules ("must be < 90 days old")
└── verified (bool, date, method)
```

### Meta-Data
```
FieldMeta
├── source ("user_entered", "ocr_passport", "hr_system")
├── verified_at
├── confidence (0.0-1.0)
├── valid_from, valid_to
└── jurisdiction_variants{}
```

---

## Next Actions

1. **Define target context** - Which segment? Which 3 form families first?
2. **Implement enhanced vault schema** - Migrate from flat profile to fact graph
3. **Build field ontology** - Start with target form families
4. **Add validation layer** - Format checks, cross-field rules
5. **Implement review UX** - Diff view, approval workflow
