use std::fmt;
use std::str::FromStr;

use thiserror::Error;

// ############
// # Protocol #
// ############
//
// [connect]
// > USER <username>
// < 20 <user_id>
// > LISTFEEDS
// < 21
// < 22 <feed_id> <feed_url> :<feed_name>
// < 25
// > LISTUNREAD
// < 23
// < 24 <entry_id> <feed_id> <feed_url> <entry_title> :<entry_link>
// < 25
// > MARKREAD <entry_id>
// < 28

/// Commands sent to seymour server
pub enum Command {
    /// Select the user user
    User { username: String },

    /// List the current user's subscriptions
    ///
    /// Requires a client to issue a User
    /// command prior.
    ListSubscriptions,

    /// Subscribe the current user to a new feed
    ///
    /// Requires a client to issue a User
    /// command prior.
    Subscribe { url: String },

    /// Unsubscribe the current user from a feed
    ///
    /// Requires a client to issue a User
    /// command prior.
    Unsubscribe { id: i64 },

    /// List the current user's unread feed entries
    ///
    /// Requires a client to issue a User
    /// command prior.
    ListUnread,

    /// Mark a feed entry as read by the current user
    ///
    /// Requires a client to issue a User
    /// command prior.
    MarkRead { id: i64 },
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Command::User { username } => write!(f, "USER {}", username),
            Command::ListSubscriptions => write!(f, "LISTSUBSCRIPTIONS"),
            Command::Subscribe { url } => write!(f, "SUBSCRIBE {}", url),
            Command::Unsubscribe { id } => write!(f, "UNSUBSCRIBE {}", id),
            Command::ListUnread => write!(f, "LISTUNREAD"),
            Command::MarkRead { id } => write!(f, "MARKREAD {}", id),
        }
    }
}

fn check_arguments(parts: &Vec<&str>, expected: usize) -> Result<(), ParseMessageError> {
    if parts.len() > expected + 1 {
        return Err(ParseMessageError::TooManyArguments {
            expected,
            actual: parts.len() - 1,
        });
    }

    Ok(())
}

fn at_position<T: FromStr>(
    parts: &[&str],
    argument_name: &str,
    position: usize,
) -> Result<T, ParseMessageError> {
    let possible = parts
        .get(position)
        .ok_or_else(|| ParseMessageError::MissingArgument(argument_name.to_string()))?;

    possible
        .parse()
        .map_err(|_| ParseMessageError::InvalidIntegerArgument {
            argument: argument_name.to_string(),
            value: possible.to_string(),
        })
}

#[derive(Debug, Error)]
pub enum ParseMessageError {
    #[error("empty message")]
    EmptyMessage,
    #[error("unknown message type \"{0}\"")]
    UnknownType(String),
    #[error("missing argument \"{0}\"")]
    MissingArgument(String),
    #[error("too many arguments (expected {expected}, got {actual})")]
    TooManyArguments { expected: usize, actual: usize },
    #[error("invalid integer value \"{value}\" for argument \"{argument}\"")]
    InvalidIntegerArgument { argument: String, value: String },
}

impl FromStr for Command {
    type Err = ParseMessageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = value.split(' ').collect();

        let command = parts.get(0).ok_or(ParseMessageError::EmptyMessage)?;

        match *command {
            "USER" => {
                check_arguments(&parts, 1)?;

                let username: String = at_position(&parts, "username", 1)?;

                Ok(Command::User { username })
            }
            "LISTSUBSCRIPTIONS" => {
                check_arguments(&parts, 0)?;

                Ok(Command::ListSubscriptions)
            }
            "SUBSCRIBE" => {
                check_arguments(&parts, 1)?;

                let url: String = at_position(&parts, "url", 1)?;

                Ok(Command::Subscribe { url })
            }
            "UNSUBSCRIBE" => {
                check_arguments(&parts, 1)?;

                let id: i64 = at_position(&parts, "id", 1)?;

                Ok(Command::Unsubscribe { id })
            }
            "LISTUNREAD" => {
                check_arguments(&parts, 0)?;

                Ok(Command::ListUnread)
            }
            "MARKREAD" => {
                check_arguments(&parts, 1)?;

                let id: i64 = at_position(&parts, "id", 1)?;

                Ok(Command::MarkRead { id })
            }
            _ => Err(ParseMessageError::UnknownType(command.to_string())),
        }
    }
}

/// Responses sent from seymour server
pub enum Response {
    /// Acknowledgement for selecting current user
    AckUser { id: i64 },

    /// Beginning of a list of subscriptions
    ///
    /// Must be followed by zero or more Subscription lines and
    /// one EndList.
    StartSubscriptionList,

    /// A single subscription entry
    ///
    /// Must be preceeded by one StartSubscriptionList and
    /// followed by one EndList.
    Subscription { id: i64, url: String },

    /// Beginning of a list of feed entries
    ///
    /// Must be followed by zero or more Entry lines and
    /// one EndList.
    StartEntryList,

    /// A single feed entry
    ///
    /// Must be preceeded by one StartEntryList and
    /// followed by one EndList.
    Entry {
        id: i64,
        feed_id: i64,
        feed_url: String,
        title: String,
        url: String,
    },

    /// Ends a list sent by the server
    ///
    /// Must be preceeded by at least either a StartSubscriptionList
    /// or a StartEntryList.
    EndList,

    /// Acknowledgement for subscribing the current user
    /// to a new feed
    AckSubscribe,

    /// Acknowledgement for unsubscribing the current user
    /// from a feed
    AckUnsubscribe,

    /// Acknowledgement for marking a feed entry as read
    /// by the current user
    AckMarkRead,

    /// Error stating that the specified resource was
    /// not found
    ResourceNotFound(String),

    /// Error stating that the command sent was not valid
    BadCommand(String),

    /// Error stating that the command sent requires a
    /// selected user, but no user has been selected
    NeedUser(String),

    /// Error stating that the seymour server hit an
    /// internal problem while attempting to serve
    /// the request
    InternalError(String),
}

impl From<ParseMessageError> for Response {
    fn from(e: ParseMessageError) -> Response {
        Response::BadCommand(e.to_string())
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Response::AckUser { id } => write!(f, "20 {}", id),
            Response::StartSubscriptionList => write!(f, "21"),
            Response::Subscription { id, url } => write!(f, "22 {} {}", id, url),
            Response::StartEntryList => write!(f, "23"),
            Response::Entry {
                id,
                feed_id,
                feed_url,
                title,
                url,
            } => write!(f, "24 {} {} {} {} :{}", id, feed_id, feed_url, url, title),
            Response::EndList => write!(f, "25"),
            Response::AckSubscribe => write!(f, "26"),
            Response::AckUnsubscribe => write!(f, "27"),
            Response::AckMarkRead => write!(f, "28"),

            Response::ResourceNotFound(message) => write!(f, "40 {}", message),
            Response::BadCommand(message) => write!(f, "41 {}", message),
            Response::NeedUser(message) => write!(f, "42 {}", message),

            Response::InternalError(message) => write!(f, "51 {}", message),
        }
    }
}

impl FromStr for Response {
    type Err = ParseMessageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = value.split(' ').collect();

        let response = parts.get(0).ok_or(ParseMessageError::EmptyMessage)?;

        match *response {
            "20" => {
                check_arguments(&parts, 1)?;

                let id: i64 = at_position(&parts, "id", 1)?;

                Ok(Response::AckUser { id })
            }
            "21" => {
                check_arguments(&parts, 0)?;

                Ok(Response::StartSubscriptionList)
            }
            "22" => {
                check_arguments(&parts, 2)?;

                let id: i64 = at_position(&parts, "id", 1)?;
                let url: String = at_position(&parts, "url", 2)?;

                Ok(Response::Subscription { id, url })
            }
            "23" => {
                check_arguments(&parts, 0)?;

                Ok(Response::StartEntryList)
            }
            "24" => {
                let trailing_start = value
                    .find(':')
                    .ok_or_else(|| ParseMessageError::MissingArgument("title".to_string()))?;

                let initial_parts: Vec<&str> = value[..trailing_start].split(' ').collect();

                let id: i64 = at_position(&initial_parts, "id", 1)?;
                let feed_id: i64 = at_position(&initial_parts, "feed_id", 2)?;
                let feed_url: String = at_position(&initial_parts, "feed_url", 3)?;
                let url: String = at_position(&initial_parts, "url", 5)?;

                let title = value[trailing_start + 1..].to_string();

                Ok(Response::Entry {
                    id,
                    feed_id,
                    feed_url,
                    title,
                    url,
                })
            }
            "25" => {
                check_arguments(&parts, 0)?;

                Ok(Response::EndList)
            }
            "26" => {
                check_arguments(&parts, 0)?;

                Ok(Response::AckSubscribe)
            }
            "27" => {
                check_arguments(&parts, 0)?;

                Ok(Response::AckUnsubscribe)
            }
            "28" => {
                check_arguments(&parts, 0)?;

                Ok(Response::AckMarkRead)
            }

            "40" => {
                check_arguments(&parts, 1)?;

                let message: String = at_position(&parts, "message", 1)?;

                Ok(Response::ResourceNotFound(message))
            }
            "41" => {
                check_arguments(&parts, 1)?;

                let message: String = at_position(&parts, "message", 1)?;

                Ok(Response::BadCommand(message))
            }
            "42" => {
                check_arguments(&parts, 1)?;

                let message: String = at_position(&parts, "message", 1)?;

                Ok(Response::NeedUser(message))
            }

            "50" => {
                check_arguments(&parts, 1)?;

                let message: String = at_position(&parts, "message", 1)?;

                Ok(Response::InternalError(message))
            }
            _ => Err(ParseMessageError::UnknownType(response.to_string())),
        }
    }
}
