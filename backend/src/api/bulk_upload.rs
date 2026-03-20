//! Bulk upload endpoints for glossary terms.
//!
//! Provides:
//! - `GET  /api/v1/glossary/terms/bulk-upload/template` — download an Excel template
//! - `POST /api/v1/glossary/terms/bulk-upload`           — upload filled-in template
//!
//! No AI enrichment is triggered on bulk-uploaded terms (explicit requirement).
//! Each row is processed independently — partial success is supported.

use axum::Extension;
use axum::body::Body;
use axum::extract::{Multipart, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};
use rust_xlsxwriter::{DataValidation, Format, FormatAlign, FormatBorder, Formula, Workbook};
use sqlx::PgPool;
use std::io::Cursor;
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::glossary::{BulkUploadError, BulkUploadResult};
use crate::error::{AppError, AppResult};
use crate::workflow;

/// Maximum upload file size: 10 MB.
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
/// Maximum data rows to process.
const MAX_ROWS: usize = 1000;

/// Column headers in the Terms sheet (A-AF = 32 columns).
const TEMPLATE_HEADERS: &[&str] = &[
    "Term Name",            // A  (0)
    "Definition",           // B  (1)
    "Definition Notes",     // C  (2)
    "Counter-Examples",     // D  (3)
    "Formula",              // E  (4)
    "Abbreviation",         // F  (5)
    "Domain",               // G  (6)
    "Category",             // H  (7)
    "Data Classification",  // I  (8)
    "Term Type",            // J  (9)
    "Unit of Measure",      // K  (10)
    "Review Frequency",     // L  (11)
    "Visibility",           // M  (12)
    "Language",             // N  (13)
    "Business Term Owner",  // O  (14)
    "Data Steward",         // P  (15)
    "Data Domain Owner",    // Q  (16)
    "Approver",             // R  (17)
    "Organisational Unit",  // S  (18)
    "Parent Term",          // T  (19)
    "Source Reference",     // U  (20)
    "Regulatory Reference", // V  (21)
    "External Reference",   // W  (22)
    "Business Rules",       // X  (23)
    "Examples",             // Y  (24)
    "Used in Reports",      // Z  (25)
    "Used in Policies",     // AA (26)
    "Regulatory Reporting", // AB (27)
    "CBT Flag",             // AC (28)
    "Regulatory Tags",      // AD (29)
    "Subject Areas",        // AE (30)
    "Tags",                 // AF (31)
];

/// Instructions for each column (field, description, mandatory, max_length, notes).
const INSTRUCTIONS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Term Name",
        "The official name of the business term",
        "Yes",
        "256",
        "Must be unique",
    ),
    (
        "Definition",
        "Clear, unambiguous definition of the term",
        "Yes",
        "4000",
        "Plain text only",
    ),
    (
        "Definition Notes",
        "Clarifying notes about the definition",
        "No",
        "4000",
        "",
    ),
    (
        "Counter-Examples",
        "What this term is NOT — helps clarify boundaries",
        "No",
        "4000",
        "",
    ),
    (
        "Formula",
        "Calculation formula for KPIs/metrics",
        "No",
        "2000",
        "",
    ),
    ("Abbreviation", "Short form or acronym", "No", "50", ""),
    (
        "Domain",
        "Business domain the term belongs to",
        "No",
        "",
        "Select from dropdown",
    ),
    (
        "Category",
        "Classification category for the term",
        "No",
        "",
        "Select from dropdown",
    ),
    (
        "Data Classification",
        "Security classification level",
        "No",
        "",
        "Select from dropdown",
    ),
    (
        "Term Type",
        "Type of business term (KPI, concept, etc.)",
        "No",
        "",
        "Select from dropdown",
    ),
    (
        "Unit of Measure",
        "Measurement unit if applicable",
        "No",
        "",
        "Select from dropdown",
    ),
    (
        "Review Frequency",
        "How often this term should be reviewed",
        "No",
        "",
        "Select from dropdown",
    ),
    (
        "Visibility",
        "Who can see this term",
        "No",
        "",
        "Select from dropdown",
    ),
    (
        "Language",
        "Language of the definition",
        "No",
        "",
        "Select from dropdown; default: English",
    ),
    (
        "Business Term Owner",
        "Email address of the business term owner",
        "Yes",
        "",
        "Must exist in the system",
    ),
    (
        "Data Steward",
        "Email address of the data steward",
        "Yes",
        "",
        "Must exist in the system",
    ),
    (
        "Data Domain Owner",
        "Email address of the data domain owner",
        "Yes",
        "",
        "Must exist in the system",
    ),
    (
        "Approver",
        "Email address of the approver",
        "Yes",
        "",
        "Must exist in the system",
    ),
    (
        "Organisational Unit",
        "Responsible organisational unit",
        "Yes",
        "",
        "Select from dropdown",
    ),
    (
        "Parent Term",
        "Name of the parent term (if hierarchical)",
        "No",
        "",
        "Must match existing term name",
    ),
    (
        "Source Reference",
        "External source or origin of the definition",
        "No",
        "2000",
        "",
    ),
    (
        "Regulatory Reference",
        "Regulation or standard citation",
        "No",
        "2000",
        "",
    ),
    (
        "External Reference",
        "URL or external document reference",
        "No",
        "2000",
        "",
    ),
    (
        "Business Rules",
        "Business context and rules related to this term",
        "No",
        "4000",
        "",
    ),
    ("Examples", "Usage examples for the term", "No", "4000", ""),
    (
        "Used in Reports",
        "Names of reports that use this term",
        "No",
        "2000",
        "",
    ),
    (
        "Used in Policies",
        "Policy documents referencing this term",
        "No",
        "2000",
        "",
    ),
    (
        "Regulatory Reporting",
        "Regulatory reporting usage of this term",
        "No",
        "2000",
        "",
    ),
    (
        "CBT Flag",
        "Whether this is a Critical Business Term",
        "No",
        "",
        "TRUE or FALSE",
    ),
    (
        "Regulatory Tags",
        "Applicable regulatory frameworks",
        "No",
        "",
        "Comma-separated from dropdown",
    ),
    (
        "Subject Areas",
        "Business subject areas",
        "No",
        "",
        "Comma-separated from dropdown",
    ),
    (
        "Tags",
        "Freeform keywords/tags",
        "No",
        "",
        "Comma-separated",
    ),
];

// ---------------------------------------------------------------------------
// Template download
// ---------------------------------------------------------------------------

/// Download an Excel template for bulk-uploading glossary terms.
///
/// Returns a `.xlsx` file with three sheets: Terms (data entry with dropdowns),
/// Valid Values (lookup lists from the database), and Instructions (field documentation).
/// Dropdown validations reference the Valid Values sheet, ensuring data integrity.
#[utoipa::path(
    get,
    path = "/api/v1/glossary/terms/bulk-upload/template",
    responses(
        (status = 200, description = "Excel template file", content_type = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn download_template(State(state): State<AppState>) -> AppResult<Response> {
    let bytes = generate_template(&state.pool).await?;

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"glossary_term_template.xlsx\"",
        )
        .body(Body::from(bytes))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to build response: {e}")))
}

/// Fetch all lookup values from DB and build the Excel template in memory.
async fn generate_template(pool: &PgPool) -> AppResult<Vec<u8>> {
    // Fetch all lookup lists in parallel
    let (
        domains,
        categories,
        classifications,
        term_types,
        units,
        frequencies,
        visibility_levels,
        languages,
        org_units,
        user_emails,
        reg_tags,
        subject_areas,
    ) = tokio::try_join!(
        fetch_names(
            pool,
            "SELECT domain_name FROM glossary_domains ORDER BY domain_name"
        ),
        fetch_names(
            pool,
            "SELECT category_name FROM glossary_categories ORDER BY category_name"
        ),
        fetch_names(
            pool,
            "SELECT classification_name FROM data_classifications ORDER BY classification_name"
        ),
        fetch_names(
            pool,
            "SELECT type_name FROM glossary_term_types ORDER BY display_order"
        ),
        fetch_names(
            pool,
            "SELECT unit_name FROM glossary_units_of_measure ORDER BY display_order"
        ),
        fetch_names(
            pool,
            "SELECT frequency_name FROM glossary_review_frequencies ORDER BY display_order"
        ),
        fetch_names(
            pool,
            "SELECT visibility_name FROM glossary_visibility_levels ORDER BY display_order"
        ),
        fetch_names(
            pool,
            "SELECT language_name FROM glossary_languages ORDER BY language_name"
        ),
        fetch_names(
            pool,
            "SELECT unit_name FROM organisational_units ORDER BY display_order"
        ),
        fetch_names(
            pool,
            "SELECT email FROM users WHERE is_active = TRUE AND deleted_at IS NULL ORDER BY display_name"
        ),
        fetch_names(
            pool,
            "SELECT tag_name FROM glossary_regulatory_tags ORDER BY display_order"
        ),
        fetch_names(
            pool,
            "SELECT area_name FROM glossary_subject_areas ORDER BY display_order"
        ),
    )?;

    let mut workbook = Workbook::new();

    // --- Formats ---
    let header_format = Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0x4472C4))
        .set_font_color(rust_xlsxwriter::Color::White)
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin);

    let mandatory_header_format = Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0xFFC000))
        .set_font_color(rust_xlsxwriter::Color::Black)
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin);

    let instruction_header_format = Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0x305496))
        .set_font_color(rust_xlsxwriter::Color::White)
        .set_border(FormatBorder::Thin);

    let list_header_format = Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0x305496))
        .set_font_color(rust_xlsxwriter::Color::White)
        .set_border(FormatBorder::Thin);

    // Mandatory column indices: A(0), B(1), O(14), P(15), Q(16), R(17), S(18)
    let mandatory_cols: &[u16] = &[0, 1, 14, 15, 16, 17, 18];

    // Each lookup list with its name
    let lookup_lists: Vec<(&str, &[String])> = vec![
        ("Domains", &domains),                    // 0
        ("Categories", &categories),              // 1
        ("Classifications", &classifications),    // 2
        ("TermTypes", &term_types),               // 3
        ("UnitsOfMeasure", &units),               // 4
        ("ReviewFrequencies", &frequencies),      // 5
        ("VisibilityLevels", &visibility_levels), // 6
        ("Languages", &languages),                // 7
        ("UserEmails", &user_emails),             // 8
        ("OrganisationalUnits", &org_units),      // 9
        ("RegulatoryTags", &reg_tags),            // 10
        ("SubjectAreas", &subject_areas),         // 11
    ];

    // Dropdown column mappings: (Terms column index, lookup list index)
    let dropdown_mappings: &[(u16, usize)] = &[
        (6, 0),  // Domain         -> Domains
        (7, 1),  // Category       -> Categories
        (8, 2),  // Classification -> Classifications
        (9, 3),  // Term Type      -> TermTypes
        (10, 4), // Unit of Measure -> UnitsOfMeasure
        (11, 5), // Review Freq    -> ReviewFrequencies
        (12, 6), // Visibility     -> VisibilityLevels
        (13, 7), // Language       -> Languages
        (14, 8), // Owner email    -> UserEmails
        (15, 8), // Steward email  -> UserEmails
        (16, 8), // Domain Owner   -> UserEmails
        (17, 8), // Approver       -> UserEmails
        (18, 9), // Org Unit       -> OrganisationalUnits
    ];

    // Pre-build the data validations (no borrow on workbook needed yet)
    let mut validations: Vec<(u16, DataValidation)> = Vec::new();
    for &(terms_col, lookup_idx) in dropdown_mappings {
        let (_name, values) = &lookup_lists[lookup_idx];
        if !values.is_empty() {
            let last_row = values.len() as u32;
            let col_letter = col_to_letter(lookup_idx as u16);
            let formula_str = format!(
                "='Valid Values'!${col_letter}$2:${col_letter}${}",
                last_row + 1
            );
            let validation = DataValidation::new()
                .allow_list_formula(Formula::new(formula_str))
                .set_error_title("Invalid value")
                .map_err(xlsx_err)?
                .set_error_message("Please select a value from the dropdown list.")
                .map_err(xlsx_err)?;
            validations.push((terms_col, validation));
        }
    }

    let cde_validation = DataValidation::new()
        .allow_list_strings(&["TRUE", "FALSE"])
        .map_err(xlsx_err)?
        .set_error_title("Invalid value")
        .map_err(xlsx_err)?
        .set_error_message("Please enter TRUE or FALSE.")
        .map_err(xlsx_err)?;

    // ===== Sheet 1: Terms =====
    {
        let terms_sheet = workbook.add_worksheet();
        terms_sheet.set_name("Terms").map_err(xlsx_err)?;

        // Write headers
        for (col, &hdr) in TEMPLATE_HEADERS.iter().enumerate() {
            let col = col as u16;
            let fmt = if mandatory_cols.contains(&col) {
                &mandatory_header_format
            } else {
                &header_format
            };
            terms_sheet
                .write_string_with_format(0, col, hdr, fmt)
                .map_err(xlsx_err)?;
            terms_sheet.set_column_width(col, 20).map_err(xlsx_err)?;
        }
        terms_sheet.set_column_width(0, 30).map_err(xlsx_err)?;
        terms_sheet.set_column_width(1, 50).map_err(xlsx_err)?;

        // Apply data validations
        for (terms_col, validation) in &validations {
            terms_sheet
                .add_data_validation(1, *terms_col, MAX_ROWS as u32, *terms_col, validation)
                .map_err(xlsx_err)?;
        }

        // CDE Flag validation
        terms_sheet
            .add_data_validation(1, 29, MAX_ROWS as u32, 29, &cde_validation)
            .map_err(xlsx_err)?;
    }

    // ===== Sheet 2: Valid Values =====
    {
        let valid_sheet = workbook.add_worksheet();
        valid_sheet.set_name("Valid Values").map_err(xlsx_err)?;

        for (col_idx, (name, values)) in lookup_lists.iter().enumerate() {
            let col = col_idx as u16;
            valid_sheet
                .write_string_with_format(0, col, *name, &list_header_format)
                .map_err(xlsx_err)?;
            valid_sheet.set_column_width(col, 25).map_err(xlsx_err)?;
            for (row_idx, val) in values.iter().enumerate() {
                valid_sheet
                    .write_string((row_idx + 1) as u32, col, val)
                    .map_err(xlsx_err)?;
            }
        }
    }

    // ===== Sheet 3: Instructions =====
    {
        let instr_sheet = workbook.add_worksheet();
        instr_sheet.set_name("Instructions").map_err(xlsx_err)?;

        let instr_headers = [
            "Field Name",
            "Description",
            "Mandatory",
            "Max Length",
            "Notes",
        ];
        for (col, &hdr) in instr_headers.iter().enumerate() {
            instr_sheet
                .write_string_with_format(0, col as u16, hdr, &instruction_header_format)
                .map_err(xlsx_err)?;
        }
        instr_sheet.set_column_width(0, 25).map_err(xlsx_err)?;
        instr_sheet.set_column_width(1, 55).map_err(xlsx_err)?;
        instr_sheet.set_column_width(2, 12).map_err(xlsx_err)?;
        instr_sheet.set_column_width(3, 12).map_err(xlsx_err)?;
        instr_sheet.set_column_width(4, 40).map_err(xlsx_err)?;

        for (row_idx, &(field, desc, mandatory, max_len, notes)) in INSTRUCTIONS.iter().enumerate()
        {
            let row = (row_idx + 1) as u32;
            instr_sheet.write_string(row, 0, field).map_err(xlsx_err)?;
            instr_sheet.write_string(row, 1, desc).map_err(xlsx_err)?;
            instr_sheet
                .write_string(row, 2, mandatory)
                .map_err(xlsx_err)?;
            instr_sheet
                .write_string(row, 3, max_len)
                .map_err(xlsx_err)?;
            instr_sheet.write_string(row, 4, notes).map_err(xlsx_err)?;
        }
    }

    // Save to buffer
    let buf = workbook.save_to_buffer().map_err(xlsx_err)?;
    Ok(buf)
}

/// Fetch a single-column list of names from the database.
async fn fetch_names(pool: &PgPool, query: &str) -> AppResult<Vec<String>> {
    let rows = sqlx::query_scalar::<_, String>(query)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Convert 0-based column index to Excel column letter(s).
fn col_to_letter(col: u16) -> String {
    let mut result = String::new();
    let mut n = col as u32;
    loop {
        result.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    result
}

/// Convert `rust_xlsxwriter` errors to `AppError`.
fn xlsx_err(e: rust_xlsxwriter::XlsxError) -> AppError {
    AppError::Internal(anyhow::anyhow!("excel generation error: {e}"))
}

// ---------------------------------------------------------------------------
// Bulk upload
// ---------------------------------------------------------------------------

/// Bulk-upload glossary terms from a filled-in Excel template.
///
/// Accepts a multipart file upload (max 10 MB, up to 1000 rows). Each row is
/// processed independently — partial success is supported. Created terms enter
/// the DRAFT workflow state. No AI enrichment is triggered. Each successful
/// insert is recorded in the `audit_log` table.
#[utoipa::path(
    post,
    path = "/api/v1/glossary/terms/bulk-upload",
    request_body(content_type = "multipart/form-data", content = String, description = "Excel file upload"),
    responses(
        (status = 200, description = "Upload results", body = BulkUploadResult),
        (status = 413, description = "File too large"),
        (status = 422, description = "Invalid file format")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn bulk_upload(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> AppResult<axum::Json<BulkUploadResult>> {
    // Extract the file from multipart
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("multipart error: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            file_name = field.file_name().map(|s| s.to_string());
            content_type = field.content_type().map(|s| s.to_string());
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("failed to read file: {e}")))?;
            file_bytes = Some(data.to_vec());
            break;
        }
    }

    let file_bytes = file_bytes
        .ok_or_else(|| AppError::BadRequest("no file field found in multipart upload".into()))?;

    // SEC-024: Validate content type and file extension before processing
    let valid_content_types = [
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "application/vnd.ms-excel",
        "application/octet-stream", // browsers sometimes send this for xlsx
    ];
    let has_valid_content_type = content_type
        .as_deref()
        .map(|ct| valid_content_types.iter().any(|v| ct.starts_with(v)))
        .unwrap_or(true); // if no content type, fall through to format check below
    let has_valid_extension = file_name
        .as_deref()
        .map(|name| name.to_lowercase().ends_with(".xlsx") || name.to_lowercase().ends_with(".xls"))
        .unwrap_or(true); // if no filename, fall through to format check below

    if !has_valid_content_type && !has_valid_extension {
        return Err(AppError::Validation(
            "invalid file type — only .xlsx (Excel) files are accepted".into(),
        ));
    }

    // Check file size
    if file_bytes.len() > MAX_FILE_SIZE {
        return Err(AppError::Validation(format!(
            "file exceeds maximum size of {} MB",
            MAX_FILE_SIZE / (1024 * 1024)
        )));
    }

    // Parse the Excel file
    let cursor = Cursor::new(&file_bytes);
    let mut workbook: Xlsx<_> = open_workbook_from_rs(cursor)
        .map_err(|e| AppError::Validation(format!("invalid Excel file: {e}")))?;

    // Read the "Terms" sheet
    let range = workbook
        .worksheet_range("Terms")
        .map_err(|e| AppError::Validation(format!("cannot read 'Terms' sheet: {e}")))?;

    let rows: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();

    if rows.is_empty() {
        return Err(AppError::Validation("terms sheet is empty".into()));
    }

    // Validate header row
    let header_row = &rows[0];
    for (i, &expected) in TEMPLATE_HEADERS.iter().enumerate() {
        let actual = cell_as_string(header_row.get(i));
        if actual.trim().to_lowercase() != expected.to_lowercase() {
            return Err(AppError::Validation(format!(
                "column {} header mismatch: expected '{}', found '{}'",
                col_to_letter(i as u16),
                expected,
                actual
            )));
        }
    }

    // Collect data rows (skip header, stop at first completely empty row)
    let mut data_rows: Vec<(usize, Vec<String>)> = Vec::new();
    for (idx, row) in rows.iter().enumerate().skip(1) {
        if idx > MAX_ROWS {
            break;
        }
        // Check if row is completely empty
        let all_empty = row.iter().all(|cell| {
            matches!(cell, Data::Empty) || cell_as_string(Some(cell)).trim().is_empty()
        });
        if all_empty {
            break;
        }
        let string_row: Vec<String> = (0..TEMPLATE_HEADERS.len())
            .map(|col| cell_as_string(row.get(col)))
            .collect();
        data_rows.push((idx + 1, string_row)); // 1-based row number (header = row 1)
    }

    if data_rows.is_empty() {
        return Ok(axum::Json(BulkUploadResult {
            total_rows: 0,
            successful: 0,
            failed: 0,
            errors: vec![],
            created_term_ids: vec![],
        }));
    }

    // Fetch DRAFT status_id and default review frequency
    let draft_status_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT status_id FROM entity_statuses WHERE status_code = 'DRAFT'",
    )
    .fetch_one(&state.pool)
    .await?;

    let annual_frequency_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT frequency_id FROM glossary_review_frequencies WHERE frequency_code = 'ANNUAL'",
    )
    .fetch_optional(&state.pool)
    .await?;

    let ctx = BulkUploadContext {
        pool: &state.pool,
        user_id: claims.sub,
        draft_status_id,
        annual_frequency_id,
    };

    let total_rows = data_rows.len();
    let mut successful = 0usize;
    let mut errors: Vec<BulkUploadError> = Vec::new();
    let mut created_term_ids: Vec<Uuid> = Vec::new();

    // Process each row independently
    for (row_num, cols) in &data_rows {
        let row_errors = process_row(&ctx, *row_num, cols, &mut created_term_ids).await;

        match row_errors {
            Ok(()) => {
                successful += 1;
            }
            Err(mut errs) => {
                errors.append(&mut errs);
            }
        }
    }

    // Second pass: resolve parent_term references within the batch.
    // Rows may reference other rows in the same upload as parents.
    for (_row_num, cols) in &data_rows {
        let parent_name = cols.get(20).and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        if let Some(ref name) = parent_name {
            let term_name = cols
                .first()
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            if let Ok(Some(parent_id)) = sqlx::query_scalar::<_, Uuid>(
                "SELECT term_id FROM glossary_terms WHERE term_name ILIKE $1 AND is_current_version = TRUE AND deleted_at IS NULL LIMIT 1",
            )
            .bind(name)
            .fetch_optional(&state.pool)
            .await
            {
                // Find the term we created and update its parent
                let _ = sqlx::query(
                    "UPDATE glossary_terms SET parent_term_id = $1 WHERE term_name ILIKE $2 AND is_current_version = TRUE AND deleted_at IS NULL AND parent_term_id IS NULL",
                )
                .bind(parent_id)
                .bind(&term_name)
                .execute(&state.pool)
                .await;
            }
        }
    }

    Ok(axum::Json(BulkUploadResult {
        total_rows,
        successful,
        failed: total_rows - successful,
        errors,
        created_term_ids,
    }))
}

/// Context passed to each row during bulk upload processing.
struct BulkUploadContext<'a> {
    pool: &'a PgPool,
    user_id: Uuid,
    draft_status_id: Uuid,
    annual_frequency_id: Option<Uuid>,
}

/// Process a single row from the upload. Returns Ok(()) on success,
/// or Err(Vec<BulkUploadError>) with all validation errors for this row.
async fn process_row(
    ctx: &BulkUploadContext<'_>,
    row_num: usize,
    cols: &[String],
    created_ids: &mut Vec<Uuid>,
) -> Result<(), Vec<BulkUploadError>> {
    let mut row_errors: Vec<BulkUploadError> = Vec::new();

    // --- Extract and trim values ---
    let term_name = cols[0].trim().to_string();
    let definition = cols[1].trim().to_string();
    let definition_notes = non_empty(&cols[2]);
    let counter_examples = non_empty(&cols[3]);
    let formula = non_empty(&cols[4]);
    let abbreviation = non_empty(&cols[5]);
    let domain_val = non_empty(&cols[6]);
    let category_val = non_empty(&cols[7]);
    let classification_val = non_empty(&cols[8]);
    let term_type_val = non_empty(&cols[9]);
    let unit_val = non_empty(&cols[10]);
    let frequency_val = non_empty(&cols[11]);
    let visibility_val = non_empty(&cols[12]);
    let language_val = non_empty(&cols[13]);
    let owner_email = cols[14].trim().to_string();
    let steward_email = cols[15].trim().to_string();
    let domain_owner_email = cols[16].trim().to_string();
    let approver_email = cols[17].trim().to_string();
    let org_unit_val = non_empty(&cols[18]);
    let parent_term_val = non_empty(&cols[19]);
    let source_reference = non_empty(&cols[20]);
    let regulatory_reference = non_empty(&cols[21]);
    let external_reference = non_empty(&cols[22]);
    let business_context = non_empty(&cols[23]);
    let examples = non_empty(&cols[24]);
    let used_in_reports = non_empty(&cols[25]);
    let used_in_policies = non_empty(&cols[26]);
    let regulatory_reporting_usage = non_empty(&cols[27]);
    let cbt_flag_str = non_empty(&cols[28]);
    let reg_tags_str = non_empty(&cols[29]);
    let subject_areas_str = non_empty(&cols[30]);
    let tags_str = non_empty(&cols[31]);

    // --- Mandatory field validation ---
    if term_name.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Term Name".into()),
            message: "Term Name is required".into(),
        });
    }
    if definition.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Definition".into()),
            message: "Definition is required".into(),
        });
    }
    if owner_email.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Business Term Owner".into()),
            message: "Business Term Owner email is required".into(),
        });
    }
    if steward_email.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Data Steward".into()),
            message: "Data Steward email is required".into(),
        });
    }
    if domain_owner_email.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Data Domain Owner".into()),
            message: "Data Domain Owner email is required".into(),
        });
    }
    if approver_email.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Approver".into()),
            message: "Approver email is required".into(),
        });
    }
    if org_unit_val.is_none() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Organisational Unit".into()),
            message: "Organisational Unit is required".into(),
        });
    }

    // Length validations
    if term_name.len() > 256 {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Term Name".into()),
            message: "Term Name exceeds 256 characters".into(),
        });
    }
    if definition.len() > 4000 {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Definition".into()),
            message: "Definition exceeds 4000 characters".into(),
        });
    }
    if abbreviation.as_ref().is_some_and(|v| v.len() > 50) {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Abbreviation".into()),
            message: "Abbreviation exceeds 50 characters".into(),
        });
    }

    // --- Resolve lookups ---
    let domain_id = resolve_optional_lookup(
        ctx.pool,
        row_num,
        "Domain",
        &domain_val,
        "SELECT domain_id FROM glossary_domains WHERE domain_name ILIKE $1",
        &mut row_errors,
    )
    .await;

    let category_id = resolve_optional_lookup(
        ctx.pool,
        row_num,
        "Category",
        &category_val,
        "SELECT category_id FROM glossary_categories WHERE category_name ILIKE $1",
        &mut row_errors,
    )
    .await;

    let classification_id = resolve_optional_lookup(
        ctx.pool,
        row_num,
        "Data Classification",
        &classification_val,
        "SELECT classification_id FROM data_classifications WHERE classification_name ILIKE $1",
        &mut row_errors,
    )
    .await;

    let term_type_id = resolve_optional_lookup(
        ctx.pool,
        row_num,
        "Term Type",
        &term_type_val,
        "SELECT term_type_id FROM glossary_term_types WHERE type_name ILIKE $1",
        &mut row_errors,
    )
    .await;

    let unit_id = resolve_optional_lookup(
        ctx.pool,
        row_num,
        "Unit of Measure",
        &unit_val,
        "SELECT unit_id FROM glossary_units_of_measure WHERE unit_name ILIKE $1",
        &mut row_errors,
    )
    .await;

    let frequency_id = resolve_optional_lookup(
        ctx.pool,
        row_num,
        "Review Frequency",
        &frequency_val,
        "SELECT frequency_id FROM glossary_review_frequencies WHERE frequency_name ILIKE $1",
        &mut row_errors,
    )
    .await;

    // Confidence level is managed by the Data Quality module — not set via bulk upload

    let visibility_id = resolve_optional_lookup(
        ctx.pool,
        row_num,
        "Visibility",
        &visibility_val,
        "SELECT visibility_id FROM glossary_visibility_levels WHERE visibility_name ILIKE $1",
        &mut row_errors,
    )
    .await;

    let language_id = resolve_optional_lookup(
        ctx.pool,
        row_num,
        "Language",
        &language_val,
        "SELECT language_id FROM glossary_languages WHERE language_name ILIKE $1",
        &mut row_errors,
    )
    .await;

    // Resolve users by email
    let owner_user_id = resolve_user_by_email(
        ctx.pool,
        row_num,
        "Business Term Owner",
        &owner_email,
        &mut row_errors,
    )
    .await;
    let steward_user_id = resolve_user_by_email(
        ctx.pool,
        row_num,
        "Data Steward",
        &steward_email,
        &mut row_errors,
    )
    .await;
    let domain_owner_user_id = resolve_user_by_email(
        ctx.pool,
        row_num,
        "Data Domain Owner",
        &domain_owner_email,
        &mut row_errors,
    )
    .await;
    let approver_user_id = resolve_user_by_email(
        ctx.pool,
        row_num,
        "Approver",
        &approver_email,
        &mut row_errors,
    )
    .await;

    // Resolve organisational unit text (stored as text, not FK)
    let organisational_unit = org_unit_val.clone();

    // Resolve parent term by name
    // Parent term is resolved AFTER all rows are inserted (second pass)
    // because a child row might reference a parent in the same upload batch.
    // Set to NULL here — the caller handles the second pass.
    let parent_term_id: Option<Uuid> = if parent_term_val.is_some() {
        // Try to resolve from already-existing terms (not in this batch)
        // May be None — second pass will resolve within-batch references
        sqlx::query_scalar::<_, Uuid>(
            "SELECT term_id FROM glossary_terms WHERE term_name ILIKE $1 AND is_current_version = TRUE AND deleted_at IS NULL LIMIT 1",
        )
        .bind(parent_term_val.as_deref().unwrap_or(""))
        .fetch_optional(ctx.pool)
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    // Parse CBT (Critical Business Term) flag
    let is_cbt = cbt_flag_str
        .as_deref()
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    // If there are validation errors, return them all
    if !row_errors.is_empty() {
        return Err(row_errors);
    }

    // Use the resolved review frequency, fall back to ANNUAL default
    let effective_frequency_id = frequency_id.or(ctx.annual_frequency_id);

    // --- Insert the term ---
    let term_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO glossary_terms (
            term_name, definition, definition_notes, counter_examples,
            formula, abbreviation, domain_id, category_id, classification_id,
            term_type_id, unit_of_measure_id, review_frequency_id,
            visibility_id, language_id,
            owner_user_id, steward_user_id, domain_owner_user_id,
            approver_user_id, organisational_unit, parent_term_id,
            source_reference, regulatory_reference, external_reference,
            business_context, examples, used_in_reports, used_in_policies,
            regulatory_reporting_usage, is_cbt,
            status_id, version_number, is_current_version, created_by
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
            $21, $22, $23, $24, $25, $26, $27, $28,
            $29, $30, 1, TRUE, $31
        )
        RETURNING term_id
        "#,
    )
    .bind(&term_name)                       // $1
    .bind(&definition)                      // $2
    .bind(definition_notes.as_deref())      // $3
    .bind(counter_examples.as_deref())      // $4
    .bind(formula.as_deref())               // $5
    .bind(abbreviation.as_deref())          // $6
    .bind(domain_id)                        // $7
    .bind(category_id)                      // $8
    .bind(classification_id)                // $9
    .bind(term_type_id)                     // $10
    .bind(unit_id)                          // $11
    .bind(effective_frequency_id)           // $12
    .bind(visibility_id)                    // $13
    .bind(language_id)                      // $14
    .bind(owner_user_id)                    // $15
    .bind(steward_user_id)                  // $16
    .bind(domain_owner_user_id)             // $17
    .bind(approver_user_id)                 // $18
    .bind(organisational_unit.as_deref())   // $19
    .bind(parent_term_id)                   // $20
    .bind(source_reference.as_deref())      // $21
    .bind(regulatory_reference.as_deref())  // $22
    .bind(external_reference.as_deref())    // $23
    .bind(business_context.as_deref())      // $24
    .bind(examples.as_deref())              // $25
    .bind(used_in_reports.as_deref())       // $26
    .bind(used_in_policies.as_deref())      // $27
    .bind(regulatory_reporting_usage.as_deref()) // $28
    .bind(is_cbt)                           // $29
    .bind(ctx.draft_status_id)              // $30
    .bind(ctx.user_id)                      // $31
    .fetch_one(ctx.pool)
    .await
    .map_err(|e| {
        vec![BulkUploadError {
            row: row_num,
            field: None,
            message: format!("database insert failed: {e}"),
        }]
    })?;

    // Initiate workflow
    if let Err(e) = workflow::service::initiate_workflow(
        ctx.pool,
        workflow::ENTITY_GLOSSARY_TERM,
        term_id,
        ctx.user_id,
    )
    .await
    {
        tracing::warn!(term_id = %term_id, error = %e, "failed to initiate workflow for bulk-uploaded term");
    }

    // CS-003: Audit log entry for bulk-uploaded term
    sqlx::query(
        "INSERT INTO audit_log (table_name, record_id, action, new_values, changed_by) \
         VALUES ('glossary_terms', $1, 'INSERT', $2, $3)",
    )
    .bind(term_id)
    .bind(serde_json::json!({"source": "bulk_upload", "row": row_num}))
    .bind(ctx.user_id)
    .execute(ctx.pool)
    .await
    .ok(); // Don't fail the upload if audit logging fails

    // --- Junction tables ---

    // Regulatory tags (comma-separated)
    if let Some(ref tags_csv) = reg_tags_str {
        for tag_name in tags_csv
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            let tag_id = sqlx::query_scalar::<_, Uuid>(
                "SELECT tag_id FROM glossary_regulatory_tags WHERE tag_name ILIKE $1",
            )
            .bind(tag_name)
            .fetch_optional(ctx.pool)
            .await
            .ok()
            .flatten();

            if let Some(tag_id) = tag_id {
                let _ = sqlx::query(
                    "INSERT INTO glossary_term_regulatory_tags (term_id, tag_id, created_by) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                )
                .bind(term_id)
                .bind(tag_id)
                .bind(ctx.user_id)
                .execute(ctx.pool)
                .await;
            } else {
                tracing::warn!(
                    row = row_num,
                    tag = tag_name,
                    "regulatory tag not found during bulk upload — skipped"
                );
            }
        }
    }

    // Subject areas (comma-separated)
    if let Some(ref areas_csv) = subject_areas_str {
        for area_name in areas_csv
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            let area_id = sqlx::query_scalar::<_, Uuid>(
                "SELECT subject_area_id FROM glossary_subject_areas WHERE area_name ILIKE $1",
            )
            .bind(area_name)
            .fetch_optional(ctx.pool)
            .await
            .ok()
            .flatten();

            if let Some(area_id) = area_id {
                let _ = sqlx::query(
                    "INSERT INTO glossary_term_subject_areas (term_id, subject_area_id, is_primary, created_by) VALUES ($1, $2, FALSE, $3) ON CONFLICT DO NOTHING",
                )
                .bind(term_id)
                .bind(area_id)
                .bind(ctx.user_id)
                .execute(ctx.pool)
                .await;
            } else {
                tracing::warn!(
                    row = row_num,
                    area = area_name,
                    "subject area not found during bulk upload — skipped"
                );
            }
        }
    }

    // Tags (comma-separated, freeform — create if not exists)
    if let Some(ref tags_csv) = tags_str {
        for tag_name in tags_csv
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
        {
            let tag_id = sqlx::query_scalar::<_, Uuid>(
                r#"
                INSERT INTO glossary_tags (tag_name, created_by)
                VALUES ($1, $2)
                ON CONFLICT (tag_name) DO UPDATE SET tag_name = glossary_tags.tag_name
                RETURNING tag_id
                "#,
            )
            .bind(&tag_name)
            .bind(ctx.user_id)
            .fetch_one(ctx.pool)
            .await;

            if let Ok(tag_id) = tag_id {
                let _ = sqlx::query(
                    "INSERT INTO glossary_term_tags (term_id, tag_id, created_by) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                )
                .bind(term_id)
                .bind(tag_id)
                .bind(ctx.user_id)
                .execute(ctx.pool)
                .await;
            }
        }
    }

    created_ids.push(term_id);
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a trimmed string from a calamine `Data` cell value.
fn cell_as_string(cell: Option<&Data>) -> String {
    match cell {
        Some(Data::String(s)) => s.trim().to_string(),
        Some(Data::Float(f)) => {
            // Excel sometimes stores integers as floats
            if (*f - f.round()).abs() < f64::EPSILON {
                format!("{}", *f as i64)
            } else {
                format!("{f}")
            }
        }
        Some(Data::Int(i)) => format!("{i}"),
        Some(Data::Bool(b)) => {
            if *b {
                "TRUE".into()
            } else {
                "FALSE".into()
            }
        }
        Some(Data::DateTime(f)) => format!("{f}"),
        Some(Data::DateTimeIso(s)) => s.clone(),
        Some(Data::DurationIso(s)) => s.clone(),
        Some(Data::Error(e)) => format!("{e:?}"),
        Some(Data::Empty) | None => String::new(),
    }
}

/// Return Some(trimmed) if the string is non-empty, None otherwise.
fn non_empty(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Resolve an optional lookup value (display name -> UUID).
/// If the value is provided but not found, pushes an error.
async fn resolve_optional_lookup(
    pool: &PgPool,
    row_num: usize,
    field_name: &str,
    value: &Option<String>,
    query: &str,
    errors: &mut Vec<BulkUploadError>,
) -> Option<Uuid> {
    let val = match value {
        Some(v) if !v.is_empty() => v,
        _ => return None,
    };

    // Try UUID parse first (in case someone pastes a UUID)
    if let Ok(id) = Uuid::parse_str(val) {
        return Some(id);
    }

    let result = sqlx::query_scalar::<_, Uuid>(query)
        .bind(val)
        .fetch_optional(pool)
        .await;

    match result {
        Ok(Some(id)) => Some(id),
        Ok(None) => {
            errors.push(BulkUploadError {
                row: row_num,
                field: Some(field_name.into()),
                message: format!("'{}' is not a valid {}", val, field_name),
            });
            None
        }
        Err(e) => {
            errors.push(BulkUploadError {
                row: row_num,
                field: Some(field_name.into()),
                message: format!("Failed to resolve {}: {}", field_name, e),
            });
            None
        }
    }
}

/// Resolve a user by email address.
async fn resolve_user_by_email(
    pool: &PgPool,
    row_num: usize,
    field_name: &str,
    email: &str,
    errors: &mut Vec<BulkUploadError>,
) -> Option<Uuid> {
    if email.is_empty() {
        return None; // mandatory check is done separately
    }

    let result = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM users WHERE email ILIKE $1 AND is_active = TRUE",
    )
    .bind(email)
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(id)) => Some(id),
        Ok(None) => {
            errors.push(BulkUploadError {
                row: row_num,
                field: Some(field_name.into()),
                message: format!("User with email '{}' not found", email),
            });
            None
        }
        Err(e) => {
            errors.push(BulkUploadError {
                row: row_num,
                field: Some(field_name.into()),
                message: format!("Failed to resolve user: {}", e),
            });
            None
        }
    }
}
