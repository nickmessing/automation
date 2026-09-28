use chrono::NaiveDate;

/// Everything that can go wrong, with the same messages lab02 prints.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("'{0}' is not a currency code")]
    InvalidCurrency(String),
    #[error("'{0}' is not a valid date")]
    InvalidDate(String),
    #[error("{0} is in the future")]
    FutureDate(NaiveDate),
    #[error("start date {from} is after end date {to}")]
    RangeOrder { from: NaiveDate, to: NaiveDate },
    #[error("{0}")]
    Usage(&'static str),
    #[error("missing required parameter '{0}'")]
    MissingParam(&'static str),
    #[error("API error: {message} (HTTP {status})")]
    Api { status: u16, message: String },
    #[error("request to {url} failed: {reason}")]
    Request { url: String, reason: String },
    #[error("API returned no rates for this range")]
    EmptyRange,
    #[error("could not save data: {0}")]
    Save(String),
}

impl Error {
    /// A hint on how to fix the problem, if there is an obvious one.
    pub fn help(&self) -> Option<&'static str> {
        match self {
            Error::InvalidCurrency(_) => Some("use a 3-letter ISO 4217 code, e.g. EUR, USD, MDL"),
            Error::InvalidDate(_) => Some("use the YYYY-MM-DD format, e.g. 2024-03-15"),
            Error::FutureDate(_) => Some("pick today or an earlier date"),
            Error::RangeOrder { .. } => Some("swap the dates"),
            Error::Usage(_) | Error::MissingParam(_) => None,
            Error::Api { status: 404, .. } => Some("no data for this date, it may be before the currency was tracked"),
            Error::Api { status: 422, .. } => Some("check the currency codes and date"),
            Error::Api { .. } => None,
            Error::Request { .. } => Some("check your internet connection"),
            Error::EmptyRange => Some("the range may be before the currency was tracked"),
            Error::Save(_) => Some("check that the data directory is writable"),
        }
    }

    /// HTTP status for the web API: upstream errors keep theirs, input errors are 4xx.
    pub fn status(&self) -> u16 {
        match self {
            Error::Api { status, .. } => *status,
            Error::Request { .. } => 502,
            Error::EmptyRange => 404,
            Error::Usage(_) | Error::MissingParam(_) => 400,
            Error::Save(_) => 500,
            _ => 422,
        }
    }
}
