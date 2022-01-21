// -- Functions called by guest, exported by host
/// The waPC protocol function `__console_log`
pub const HOST_CONSOLE_LOG: &str = "__console_log";
/// The waPC protocol function `__host_call`
pub const HOST_CALL: &str = "__host_call";
/// The waPC protocol function `__guest_request`
pub const GUEST_REQUEST_FN: &str = "__guest_request";
/// The waPC protocol function `__host_response`
pub const HOST_RESPONSE_FN: &str = "__host_response";
/// The waPC protocol function `__host_response_len`
pub const HOST_RESPONSE_LEN_FN: &str = "__host_response_len";
/// The waPC protocol function `__guest_response`
pub const GUEST_RESPONSE_FN: &str = "__guest_response";
/// The waPC protocol function `__guest_error`
pub const GUEST_ERROR_FN: &str = "__guest_error";
/// The waPC protocol function `__host_error`
pub const HOST_ERROR_FN: &str = "__host_error";
/// The waPC protocol function `__host_error_len`
pub const HOST_ERROR_LEN_FN: &str = "__host_error_len";

// -- Functions called by host, exported by guest
/// The waPC protocol function `__guest_call`
pub const GUEST_CALL: &str = "__guest_call";
/// The waPC protocol function `wapc_init`
pub const WAPC_INIT: &str = "wapc_init";
/// The waPC protocol function `_start`
pub const TINYGO_START: &str = "_start";

/// Start functions to attempt to call - order is important
pub const REQUIRED_STARTS: [&str; 2] = [TINYGO_START, WAPC_INIT];
