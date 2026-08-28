use crate::{
    application::{CaspReportSource, CaspReportingError},
    domain::CaspDailyReport,
};
use async_trait::async_trait;
pub struct HttpCaspReportSource {
    client: reqwest::Client,
    base_url: String,
}
impl HttpCaspReportSource {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').into(),
        }
    }
}
#[async_trait]
impl CaspReportSource for HttpCaspReportSource {
    async fn fetch(&self, from: &str, to: &str) -> Result<CaspDailyReport, CaspReportingError> {
        let response = self
            .client
            .get(format!(
                "{}/api/v1/reports/daily-transactions?from={from}&to={to}",
                self.base_url
            ))
            .send()
            .await
            .map_err(source)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CaspReportingError::Source(format!("HTTP {status}: {body}")));
        }
        response.json().await.map_err(source)
    }
}
fn source(error: impl std::fmt::Display) -> CaspReportingError {
    CaspReportingError::Source(error.to_string())
}
