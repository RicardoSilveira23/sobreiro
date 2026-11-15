//! User-facing text for diagnostics.

/// The text when the app could not start and its log holds the details.
pub fn startup_failed(exception_type_name: &str, log_path: &str) -> String {
    format!("Sobreiro couldn't start ({exception_type_name}).\n\nThe details are in {log_path}")
}

/// The text when the app could not start and could not write its log either.
pub fn startup_failed_unlogged(exception_type_name: &str, folder: &str) -> String {
    format!(
        "Sobreiro couldn't start ({exception_type_name}).\n\nIt couldn't write its log either. Check that this folder can be created and written to: {folder}"
    )
}
