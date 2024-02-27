///
/// The test_utils module is just for use in tests and should not be used for any non test code
///
use std::env;

/// Set the default environment for the test process
///
/// This **will** irreversibly modify environment variables in the process
pub fn setup_env() {
    env::set_var("AWS_ACCESS_KEY_ID", "deltalake");
    env::set_var("AWS_SECRET_ACCESS_KEY", "weloverust");
    env::set_var("AWS_ENDPOINT", "http://localhost:4566");
    env::set_var("AWS_ALLOW_HTTP", "true");
    env::set_var("BRUTUS_DOCUMENTS_URL", "s3://brutus-data");
}
