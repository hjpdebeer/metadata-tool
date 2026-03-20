//! Bulk upload endpoints for data elements.
//!
//! Provides:
//! - `GET  /api/v1/data-dictionary/elements/bulk-upload/template` — download an Excel template
//! - `POST /api/v1/data-dictionary/elements/bulk-upload`           — upload filled-in template
//!
//! No AI enrichment is triggered on bulk-uploaded elements (explicit requirement).
//! Each row is processed independently — partial success is supported.
//! Element code is provided by the user (not auto-generated).

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

/// Column headers in the DataElements sheet (A-P = 16 columns).
const TEMPLATE_HEADERS: &[&str] = &[
    "Element Name",        // A  (0)
    "Element Code",        // B  (1)
    "Description",         // C  (2)
    "Data Type",           // D  (3)
    "Business Definition", // E  (4)
    "Business Rules",      // F  (5)
    "Format Pattern",      // G  (6)
    "Is Nullable",         // H  (7)
    "Is PII",              // I  (8)
    "Domain",              // J  (9)
    "Classification",      // K  (10)
    "Glossary Term",       // L  (11)
    "Owner Email",         // M  (12)
    "Steward Email",       // N  (13)
    "Approver Email",      // O  (14)
    "Org Unit",            // P  (15)
];

/// Instructions for each column (field, description, mandatory, max_length, notes).
const INSTRUCTIONS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Element Name",
        "The display name of the data element",
        "Yes",
        "512",
        "Must be unique",
    ),
    (
        "Element Code",
        "snake_case code for the element",
        "Yes",
        "256",
        "Must be unique, snake_case",
    ),
    (
        "Description",
        "Clear description of the data element",
        "Yes",
        "4000",
        "Plain text only",
    ),
    (
        "Data Type",
        "The logical data type",
        "Yes",
        "",
        "Select from dropdown",
    ),
    (
        "Business Definition",
        "Formal business definition",
        "No",
        "4000",
        "",
    ),
    (
        "Business Rules",
        "Rules governing this element",
        "No",
        "4000",
        "",
    ),
    (
        "Format Pattern",
        "Expected format (e.g., YYYY-MM-DD)",
        "No",
        "256",
        "",
    ),
    (
        "Is Nullable",
        "Whether the element can be null",
        "Yes",
        "",
        "TRUE or FALSE",
    ),
    (
        "Is PII",
        "Whether the element contains PII",
        "No",
        "",
        "TRUE or FALSE (default FALSE)",
    ),
    (
        "Domain",
        "Business domain for the element",
        "Yes",
        "",
        "Select from dropdown",
    ),
    (
        "Classification",
        "Data classification level",
        "Yes",
        "",
        "Select from dropdown",
    ),
    (
        "Glossary Term",
        "Linked business glossary term name",
        "Yes",
        "",
        "Select from dropdown",
    ),
    (
        "Owner Email",
        "Email address of the data owner",
        "Yes",
        "",
        "Must exist in the system",
    ),
    (
        "Steward Email",
        "Email address of the data steward",
        "Yes",
        "",
        "Must exist in the system",
    ),
    (
        "Approver Email",
        "Email address of the approver",
        "Yes",
        "",
        "Must exist in the system",
    ),
    (
        "Org Unit",
        "Responsible organisational unit",
        "Yes",
        "",
        "Select from dropdown",
    ),
];

// ---------------------------------------------------------------------------
// Template download
// ---------------------------------------------------------------------------

/// Download an Excel template for bulk-uploading data elements.
///
/// Returns a `.xlsx` file with three sheets: DataElements (data entry with dropdowns),
/// Valid Values (lookup lists from the database), and Instructions (field documentation).
#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/elements/bulk-upload/template",
    responses(
        (status = 200, description = "Excel template file", content_type = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn download_de_template(State(state): State<AppState>) -> AppResult<Response> {
    let bytes = generate_template(&state.pool).await?;

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"data_element_bulk_upload_template.xlsx\"",
        )
        .body(Body::from(bytes))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to build response: {e}")))
}

/// Fetch all lookup values from DB and build the Excel template in memory.
async fn generate_template(pool: &PgPool) -> AppResult<Vec<u8>> {
    // Fetch all lookup lists in parallel
    let (domains, classifications, glossary_terms, org_units, users_list) = tokio::try_join!(
        fetch_names(
            pool,
            "SELECT domain_name FROM glossary_domains ORDER BY display_order"
        ),
        fetch_names(
            pool,
            "SELECT classification_name FROM data_classifications ORDER BY display_order"
        ),
        fetch_names(
            pool,
            "SELECT term_name FROM glossary_terms WHERE is_current_version = TRUE ORDER BY term_name"
        ),
        fetch_names(
            pool,
            "SELECT unit_name FROM organisational_units ORDER BY display_order"
        ),
        fetch_names(
            pool,
            "SELECT email FROM users WHERE is_active = TRUE ORDER BY display_name"
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

    // Mandatory column indices: all except E(4), F(5), G(6), I(8) are mandatory
    let mandatory_cols: &[u16] = &[0, 1, 2, 3, 7, 9, 10, 11, 12, 13, 14, 15];

    // Each lookup list with its name
    let lookup_lists: Vec<(&str, &[String])> = vec![
        ("Domains", &domains),                 // 0
        ("Classifications", &classifications), // 1
        ("GlossaryTerms", &glossary_terms),    // 2
        ("OrganisationalUnits", &org_units),   // 3
        ("Users", &users_list),                // 4
    ];

    // Dropdown column mappings: (DataElements column index, lookup list index)
    let dropdown_mappings: &[(u16, usize)] = &[
        (9, 0),  // Domain            -> Domains
        (10, 1), // Classification    -> Classifications
        (11, 2), // Glossary Term     -> GlossaryTerms
        (12, 4), // Owner Email       -> Users
        (13, 4), // Steward Email     -> Users
        (14, 4), // Approver Email    -> Users
        (15, 3), // Org Unit          -> OrganisationalUnits
    ];

    // Pre-build the data validations
    let mut validations: Vec<(u16, DataValidation)> = Vec::new();
    for &(app_col, lookup_idx) in dropdown_mappings {
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
            validations.push((app_col, validation));
        }
    }

    // Data type dropdown (static list)
    let data_type_validation = DataValidation::new()
        .allow_list_strings(&[
            "VARCHAR",
            "INTEGER",
            "DECIMAL",
            "DATE",
            "TIMESTAMP",
            "BOOLEAN",
            "TEXT",
            "JSON",
            "UUID",
        ])
        .map_err(xlsx_err)?
        .set_error_title("Invalid value")
        .map_err(xlsx_err)?
        .set_error_message("Please select a valid data type.")
        .map_err(xlsx_err)?;

    // Boolean dropdowns
    let bool_validation = DataValidation::new()
        .allow_list_strings(&["TRUE", "FALSE"])
        .map_err(xlsx_err)?
        .set_error_title("Invalid value")
        .map_err(xlsx_err)?
        .set_error_message("Please enter TRUE or FALSE.")
        .map_err(xlsx_err)?;

    // ===== Sheet 1: DataElements =====
    {
        let de_sheet = workbook.add_worksheet();
        de_sheet.set_name("DataElements").map_err(xlsx_err)?;

        // Write headers
        for (col, &hdr) in TEMPLATE_HEADERS.iter().enumerate() {
            let col = col as u16;
            let fmt = if mandatory_cols.contains(&col) {
                &mandatory_header_format
            } else {
                &header_format
            };
            de_sheet
                .write_string_with_format(0, col, hdr, fmt)
                .map_err(xlsx_err)?;
            de_sheet.set_column_width(col, 22).map_err(xlsx_err)?;
        }
        de_sheet.set_column_width(0, 30).map_err(xlsx_err)?; // Element Name
        de_sheet.set_column_width(1, 30).map_err(xlsx_err)?; // Element Code
        de_sheet.set_column_width(2, 50).map_err(xlsx_err)?; // Description
        de_sheet.set_column_width(4, 40).map_err(xlsx_err)?; // Business Definition
        de_sheet.set_column_width(5, 40).map_err(xlsx_err)?; // Business Rules
        de_sheet.set_column_width(11, 30).map_err(xlsx_err)?; // Glossary Term

        // Apply data validations from lookup lists
        for (app_col, validation) in &validations {
            de_sheet
                .add_data_validation(1, *app_col, MAX_ROWS as u32, *app_col, validation)
                .map_err(xlsx_err)?;
        }

        // Data Type validation (col D = 3)
        de_sheet
            .add_data_validation(1, 3, MAX_ROWS as u32, 3, &data_type_validation)
            .map_err(xlsx_err)?;

        // Is Nullable validation (col H = 7)
        de_sheet
            .add_data_validation(1, 7, MAX_ROWS as u32, 7, &bool_validation)
            .map_err(xlsx_err)?;

        // Is PII validation (col I = 8)
        de_sheet
            .add_data_validation(1, 8, MAX_ROWS as u32, 8, &bool_validation)
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
            valid_sheet.set_column_width(col, 30).map_err(xlsx_err)?;
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

/// Bulk-upload data elements from a filled-in Excel template.
///
/// Accepts a multipart file upload (max 10 MB, up to 1000 rows). Each row is
/// processed independently — partial success is supported. Created elements
/// enter the DRAFT workflow state. No AI enrichment is triggered.
#[utoipa::path(
    post,
    path = "/api/v1/data-dictionary/elements/bulk-upload",
    request_body(content_type = "multipart/form-data", content = String, description = "Excel file upload"),
    responses(
        (status = 200, description = "Upload results", body = BulkUploadResult),
        (status = 413, description = "File too large"),
        (status = 422, description = "Invalid file format")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn bulk_upload_elements(
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
        "application/octet-stream",
    ];
    let has_valid_content_type = content_type
        .as_deref()
        .map(|ct| valid_content_types.iter().any(|v| ct.starts_with(v)))
        .unwrap_or(true);
    let has_valid_extension = file_name
        .as_deref()
        .map(|name| name.to_lowercase().ends_with(".xlsx") || name.to_lowercase().ends_with(".xls"))
        .unwrap_or(true);

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

    // Read the "DataElements" sheet
    let range = workbook
        .worksheet_range("DataElements")
        .map_err(|e| AppError::Validation(format!("cannot read 'DataElements' sheet: {e}")))?;

    let rows: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();

    if rows.is_empty() {
        return Err(AppError::Validation("DataElements sheet is empty".into()));
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
        let all_empty = row.iter().all(|cell| {
            matches!(cell, Data::Empty) || cell_as_string(Some(cell)).trim().is_empty()
        });
        if all_empty {
            break;
        }
        let string_row: Vec<String> = (0..TEMPLATE_HEADERS.len())
            .map(|col| cell_as_string(row.get(col)))
            .collect();
        data_rows.push((idx + 1, string_row)); // 1-based row number
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

    // Fetch DRAFT status_id
    let draft_status_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT status_id FROM entity_statuses WHERE status_code = 'DRAFT'",
    )
    .fetch_one(&state.pool)
    .await?;

    let ctx = DeBulkUploadContext {
        pool: &state.pool,
        user_id: claims.sub,
        draft_status_id,
    };

    let total_rows = data_rows.len();
    let mut successful = 0usize;
    let mut errors: Vec<BulkUploadError> = Vec::new();
    let mut created_ids: Vec<Uuid> = Vec::new();

    // Process each row independently
    for (row_num, cols) in &data_rows {
        let row_result = process_de_row(&ctx, *row_num, cols, &mut created_ids).await;

        match row_result {
            Ok(()) => {
                successful += 1;
            }
            Err(mut errs) => {
                errors.append(&mut errs);
            }
        }
    }

    Ok(axum::Json(BulkUploadResult {
        total_rows,
        successful,
        failed: total_rows - successful,
        errors,
        created_term_ids: created_ids,
    }))
}

/// Context passed to each row during bulk upload processing.
struct DeBulkUploadContext<'a> {
    pool: &'a PgPool,
    user_id: Uuid,
    draft_status_id: Uuid,
}

/// Process a single row from the upload.
async fn process_de_row(
    ctx: &DeBulkUploadContext<'_>,
    row_num: usize,
    cols: &[String],
    created_ids: &mut Vec<Uuid>,
) -> Result<(), Vec<BulkUploadError>> {
    let mut row_errors: Vec<BulkUploadError> = Vec::new();

    // --- Extract and trim values ---
    let element_name = cols[0].trim().to_string();
    let element_code = cols[1].trim().to_string();
    let description = cols[2].trim().to_string();
    let data_type = cols[3].trim().to_string();
    let business_definition = non_empty(&cols[4]);
    let business_rules = non_empty(&cols[5]);
    let format_pattern = non_empty(&cols[6]);
    let is_nullable_str = cols[7].trim().to_string();
    let is_pii_str = non_empty(&cols[8]);
    let domain_val = cols[9].trim().to_string();
    let classification_val = cols[10].trim().to_string();
    let glossary_term_val = cols[11].trim().to_string();
    let owner_email = cols[12].trim().to_string();
    let steward_email = cols[13].trim().to_string();
    let approver_email = cols[14].trim().to_string();
    let org_unit_val = cols[15].trim().to_string();

    // --- Mandatory field validation ---
    if element_name.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Element Name".into()),
            message: "Element Name is required".into(),
        });
    }
    if element_code.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Element Code".into()),
            message: "Element Code is required".into(),
        });
    }
    if description.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Description".into()),
            message: "Description is required".into(),
        });
    }
    if data_type.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Data Type".into()),
            message: "Data Type is required".into(),
        });
    }
    if is_nullable_str.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Is Nullable".into()),
            message: "Is Nullable is required".into(),
        });
    }
    if domain_val.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Domain".into()),
            message: "Domain is required".into(),
        });
    }
    if classification_val.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Classification".into()),
            message: "Classification is required".into(),
        });
    }
    if glossary_term_val.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Glossary Term".into()),
            message: "Glossary Term is required".into(),
        });
    }
    if owner_email.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Owner Email".into()),
            message: "Owner Email is required".into(),
        });
    }
    if steward_email.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Steward Email".into()),
            message: "Steward Email is required".into(),
        });
    }
    if approver_email.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Approver Email".into()),
            message: "Approver Email is required".into(),
        });
    }
    if org_unit_val.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Org Unit".into()),
            message: "Org Unit is required".into(),
        });
    }

    // --- Length validations ---
    if element_name.len() > 512 {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Element Name".into()),
            message: "Element Name exceeds 512 characters".into(),
        });
    }
    if element_code.len() > 256 {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Element Code".into()),
            message: "Element Code exceeds 256 characters".into(),
        });
    }
    if description.len() > 4000 {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Description".into()),
            message: "Description exceeds 4000 characters".into(),
        });
    }
    if business_definition.as_ref().is_some_and(|v| v.len() > 4000) {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Business Definition".into()),
            message: "Business Definition exceeds 4000 characters".into(),
        });
    }
    if business_rules.as_ref().is_some_and(|v| v.len() > 4000) {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Business Rules".into()),
            message: "Business Rules exceeds 4000 characters".into(),
        });
    }
    if format_pattern.as_ref().is_some_and(|v| v.len() > 256) {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Format Pattern".into()),
            message: "Format Pattern exceeds 256 characters".into(),
        });
    }

    // --- Validate element code format (snake_case) ---
    if !element_code.is_empty() {
        let re = regex::Regex::new(r"^[a-z][a-z0-9]*(_[a-z0-9]+)*$").unwrap();
        if !re.is_match(&element_code) {
            row_errors.push(BulkUploadError {
                row: row_num,
                field: Some("Element Code".into()),
                message: format!(
                    "'{}' is not valid snake_case (e.g., customer_account_balance)",
                    element_code
                ),
            });
        }
    }

    // --- Validate data type enum ---
    let valid_data_types = [
        "VARCHAR",
        "INTEGER",
        "DECIMAL",
        "DATE",
        "TIMESTAMP",
        "BOOLEAN",
        "TEXT",
        "JSON",
        "UUID",
    ];
    if !data_type.is_empty() && !valid_data_types.contains(&data_type.as_str()) {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Data Type".into()),
            message: format!("'{}' is not a valid Data Type", data_type),
        });
    }

    // --- Parse boolean fields ---
    let is_nullable = if is_nullable_str.eq_ignore_ascii_case("true") {
        true
    } else if is_nullable_str.eq_ignore_ascii_case("false") {
        false
    } else if !is_nullable_str.is_empty() {
        row_errors.push(BulkUploadError {
            row: row_num,
            field: Some("Is Nullable".into()),
            message: format!("'{}' is not valid — must be TRUE or FALSE", is_nullable_str),
        });
        true // default, won't be used if errors exist
    } else {
        true
    };

    let is_pii = is_pii_str
        .as_deref()
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    // --- Resolve lookups ---
    let domain_opt = non_empty(&domain_val);
    let domain_id = resolve_optional_lookup(
        ctx.pool,
        row_num,
        "Domain",
        &domain_opt,
        "SELECT domain_id FROM glossary_domains WHERE domain_name ILIKE $1",
        &mut row_errors,
    )
    .await;

    let classification_opt = non_empty(&classification_val);
    let classification_id = resolve_optional_lookup(
        ctx.pool,
        row_num,
        "Classification",
        &classification_opt,
        "SELECT classification_id FROM data_classifications WHERE classification_name ILIKE $1",
        &mut row_errors,
    )
    .await;

    let glossary_opt = non_empty(&glossary_term_val);
    let glossary_term_id = resolve_optional_lookup(
        ctx.pool,
        row_num,
        "Glossary Term",
        &glossary_opt,
        "SELECT term_id FROM glossary_terms WHERE term_name ILIKE $1 AND is_current_version = TRUE",
        &mut row_errors,
    )
    .await;

    // Resolve organisational unit
    if !org_unit_val.is_empty() {
        let ou_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM organisational_units WHERE unit_name ILIKE $1)",
        )
        .bind(&org_unit_val)
        .fetch_one(ctx.pool)
        .await
        .unwrap_or(false);

        if !ou_exists {
            row_errors.push(BulkUploadError {
                row: row_num,
                field: Some("Org Unit".into()),
                message: format!("'{}' is not a valid Organisational Unit", org_unit_val),
            });
        }
    }

    // Resolve users by email
    let owner_user_id = resolve_user_by_email(
        ctx.pool,
        row_num,
        "Owner Email",
        &owner_email,
        &mut row_errors,
    )
    .await;
    let steward_user_id = resolve_user_by_email(
        ctx.pool,
        row_num,
        "Steward Email",
        &steward_email,
        &mut row_errors,
    )
    .await;
    let approver_user_id = resolve_user_by_email(
        ctx.pool,
        row_num,
        "Approver Email",
        &approver_email,
        &mut row_errors,
    )
    .await;

    // If there are validation errors, return them all
    if !row_errors.is_empty() {
        return Err(row_errors);
    }

    // --- Insert the data element ---
    let element_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO data_elements (
            element_name, element_code, description, data_type,
            business_definition, business_rules, format_pattern,
            is_nullable, is_pii,
            domain_id, classification_id, glossary_term_id,
            owner_user_id, steward_user_id, approver_user_id,
            organisational_unit,
            status_id, created_by
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16, $17, $18
        )
        RETURNING element_id
        "#,
    )
    .bind(&element_name)                        // $1
    .bind(&element_code)                        // $2
    .bind(&description)                         // $3
    .bind(&data_type)                           // $4
    .bind(business_definition.as_deref())       // $5
    .bind(business_rules.as_deref())            // $6
    .bind(format_pattern.as_deref())            // $7
    .bind(is_nullable)                          // $8
    .bind(is_pii)                               // $9
    .bind(domain_id)                            // $10
    .bind(classification_id)                    // $11
    .bind(glossary_term_id)                     // $12
    .bind(owner_user_id)                        // $13
    .bind(steward_user_id)                      // $14
    .bind(approver_user_id)                     // $15
    .bind(&org_unit_val)                        // $16
    .bind(ctx.draft_status_id)                  // $17
    .bind(ctx.user_id)                          // $18
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
        workflow::ENTITY_DATA_ELEMENT,
        element_id,
        ctx.user_id,
    )
    .await
    {
        tracing::warn!(element_id = %element_id, error = %e, "failed to initiate workflow for bulk-uploaded data element");
    }

    // CS-003: Audit log entry
    sqlx::query(
        "INSERT INTO audit_log (table_name, record_id, action, new_values, changed_by) \
         VALUES ('data_elements', $1, 'INSERT', $2, $3)",
    )
    .bind(element_id)
    .bind(serde_json::json!({"source": "bulk_upload", "row": row_num}))
    .bind(ctx.user_id)
    .execute(ctx.pool)
    .await
    .ok(); // Don't fail the upload if audit logging fails

    created_ids.push(element_id);
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
        return None;
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
