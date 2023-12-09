use std::fmt::{Display, self};
use std::str::FromStr;
use std::time::Duration;

use humantime::parse_duration;
use serde::{de};


#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum RefreshCondition {
    /// Never refresh, this is the default.
    #[default]
    Never,
    /// Always refresh.
    Always,
    /// Refresh after a given duration.
    /// The syntax fir the duration is described roughlyat
    /// https://docs.rs/humantime/latest/humantime/fn.parse_duration.html
    /// examples:
    ///  - 1month
    ///  - 3days 12hours
    ///  - 3d 12h
    ///  - 1day
    ///  - 30min
    Duration(Duration),
    Ask,
}

serde_with::serde_conv!(
    pub(crate) RefreshConditionAsString,
    RefreshCondition,
    |condition: &RefreshCondition| match condition {
        RefreshCondition::Never => "never".to_string(),
        RefreshCondition::Always => "always".to_string(),
        RefreshCondition::Ask => "ask".to_string(),
        RefreshCondition::Duration(duration) => humantime::format_duration(*duration).to_string(),
    },
    |value: String| RefreshCondition::from_str(&value)
);

impl FromStr for RefreshCondition {
    type Err = ParseRefreshConditionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match s {
            "never" => Ok(RefreshCondition::Never),
            "always" => Ok(RefreshCondition::Always),
            "ask" => Ok(RefreshCondition::Ask),
            value => match parse_duration(value) {
                Ok(result) => Ok(RefreshCondition::Duration(result)),
                Err(error) => Err(ParseRefreshConditionError(error.to_string()))
            }
        }
    }
}

#[derive(Debug)]
pub struct ParseRefreshConditionError(String);

impl de::Error for ParseRefreshConditionError {
    fn custom<T: Display>(msg: T) -> Self {
        ParseRefreshConditionError(msg.to_string())
    }
}

impl Display for ParseRefreshConditionError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseRefreshConditionError(msg) => formatter.write_str(msg),
        }
    }
}

impl std::error::Error for ParseRefreshConditionError {}
