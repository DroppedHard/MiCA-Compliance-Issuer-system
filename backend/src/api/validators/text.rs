use serde::Deserialize;
use std::{marker::PhantomData, str::FromStr};

pub(crate) trait TextRule {
    const FIELD: &'static str;
    const MAX_CHARS: usize;
}

pub(crate) struct ValidatedText<R> {
    value: String,
    rule: PhantomData<R>,
}

impl<R: TextRule> ValidatedText<R> {
    fn parse(value: String) -> Result<Self, String> {
        let value = value.trim();
        let length = value.chars().count();
        if length == 0 || length > R::MAX_CHARS {
            return Err(format!(
                "{} musi zawierać od 1 do {} znaków",
                R::FIELD,
                R::MAX_CHARS
            ));
        }

        Ok(Self {
            value: value.to_owned(),
            rule: PhantomData,
        })
    }
}

impl<R: TextRule> AsRef<str> for ValidatedText<R> {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl<R: TextRule> From<ValidatedText<R>> for String {
    fn from(value: ValidatedText<R>) -> Self {
        value.value
    }
}

impl<'de, R: TextRule> Deserialize<'de> for ValidatedText<R> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

pub(crate) enum GenericIdRule {}
impl TextRule for GenericIdRule {
    const FIELD: &'static str = "identifier";
    const MAX_CHARS: usize = 128;
}

pub(crate) enum ReasonRule {}
impl TextRule for ReasonRule {
    const FIELD: &'static str = "reason";
    const MAX_CHARS: usize = 500;
}

pub(crate) type GenericId = ValidatedText<GenericIdRule>;
pub(crate) type OperationId = GenericId;
pub(crate) type Reason = ValidatedText<ReasonRule>;

pub(crate) struct EvmAddress(alloy::primitives::Address);

impl EvmAddress {
    fn parse(value: String) -> Result<Self, String> {
        alloy::primitives::Address::from_str(value.trim())
            .map(Self)
            .map_err(|_| "adres musi być poprawnym adresem Ethereum".to_owned())
    }
}

impl From<EvmAddress> for String {
    fn from(value: EvmAddress) -> Self {
        value.0.to_checksum(None)
    }
}

impl<'de> Deserialize<'de> for EvmAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_id_accepts_128_characters() {
        let json = format!("\"{}\"", "a".repeat(128));
        assert!(serde_json::from_str::<OperationId>(&json).is_ok());
    }

    #[test]
    fn operation_id_rejects_more_than_128_characters() {
        let json = format!("\"{}\"", "a".repeat(129));
        let error = match serde_json::from_str::<OperationId>(&json) {
            Ok(_) => panic!("an identifier longer than 128 characters was accepted"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("od 1 do 128 znaków"));
    }

    #[test]
    fn operation_id_limit_counts_characters_not_utf8_bytes() {
        let json = format!("\"{}\"", "ą".repeat(128));
        assert!(serde_json::from_str::<OperationId>(&json).is_ok());
    }

    #[test]
    fn ethereum_address_is_parsed_and_normalized() {
        let address: EvmAddress =
            serde_json::from_str("\"0x0000000000000000000000000000000000000001\"").unwrap();
        let normalized: String = address.into();
        assert_eq!(normalized, "0x0000000000000000000000000000000000000001");
    }
}
