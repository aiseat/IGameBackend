use tokio_postgres::Error;

pub fn is_db_zero_line_error(err: &Error) -> bool {
    if err.to_string() == "query returned an unexpected number of rows" {
        return true;
    }
    false
}
