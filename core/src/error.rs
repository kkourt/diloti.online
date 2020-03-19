//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//


#[derive(Debug)]
pub enum Error {
    /// Number -> Rank conversion failed (argument is given number)
    InvalidRankNumber(String),
    /// Char -> Rank conversion failed (argument is given character)
    InvalidRankChar(char),
    /// Char -> Suite conversion failed (argument is given character)
    InvalidSuiteChar(char),
}
