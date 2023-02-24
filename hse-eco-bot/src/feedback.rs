use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FeedbackTopic {
    HseGreen,
    Bot,
    SuggestEcoInitiative,
    ReportGarbageDump,
    Other,
}

impl Display for FeedbackTopic {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::HseGreen => write!(f, "{}", strings::HSE_GREEN),
            Self::Bot => write!(f, "{}", strings::BOT),
            Self::SuggestEcoInitiative => write!(f, "{}", strings::SUGGEST_ECO_INITIATIVE),
            Self::ReportGarbageDump => write!(f, "{}", strings::REPORT_GARBAGE_DUMP),
            Self::Other => write!(f, "{}", strings::OTHER),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct InvalidTopicStrError;

impl Display for InvalidTopicStrError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid feedback topic string")
    }
}

impl FromStr for FeedbackTopic {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            strings::HSE_GREEN => Ok(Self::HseGreen),
            strings::BOT => Ok(Self::Bot),
            strings::SUGGEST_ECO_INITIATIVE => Ok(Self::SuggestEcoInitiative),
            strings::REPORT_GARBAGE_DUMP => Ok(Self::ReportGarbageDump),
            strings::OTHER => Ok(Self::Other),
            _ => Err(()),
        }
    }
}

mod strings {
    pub const HSE_GREEN: &str = "hsegreen";
    pub const BOT: &str = "bot";
    pub const SUGGEST_ECO_INITIATIVE: &str = "suggest";
    pub const REPORT_GARBAGE_DUMP: &str = "report-dump";
    pub const OTHER: &str = "other";
}
