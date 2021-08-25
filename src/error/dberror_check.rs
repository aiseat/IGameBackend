use tokio_postgres::Error;

pub fn is_db_zero_line_error(err: &Error) -> bool {
    if err
        .to_string()
        .contains("query returned an unexpected number of rows")
    {
        return true;
    }
    false
}

pub fn is_db_dup_unique_error(err: &Error) -> bool {
    if err
        .to_string()
        .contains("duplicate key value violates unique constraint")
    {
        return true;
    }
    false
}
