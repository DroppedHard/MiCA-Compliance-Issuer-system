use super::ValidationError;
pub(crate) fn validate_minor_amount(name: &str, value: &str) -> Result<(), ValidationError> {
    match value.parse::<u64>() {
        Ok(v) if v > 0 => Ok(()),
        _ => Err(ValidationError(format!(
            "{name} musi być dodatnią liczbą całkowitą"
        ))),
    }
}
pub(crate) fn validate_decimal_amount(name: &str, value: &str) -> Result<(), ValidationError> {
    match value.parse::<f64>() {
        Ok(v) if v.is_finite() && v > 0.0 => Ok(()),
        _ => Err(ValidationError(format!("{name} musi być dodatnią kwotą"))),
    }
}
