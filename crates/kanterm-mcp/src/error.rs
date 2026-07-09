use rmcp::ErrorData;

pub(crate) fn internal(e: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(e.to_string(), None)
}

/// For bad/missing tool arguments, so clients can tell input errors apart from
/// genuine internal faults.
pub(crate) fn bad_param(msg: impl std::fmt::Display) -> ErrorData {
    ErrorData::invalid_params(msg.to_string(), None)
}
