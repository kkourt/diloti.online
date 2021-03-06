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
    /// Char -> Suit conversion failed (argument is given character)
    InvalidSuitChar(char),
    /// str -> Card coversion failed (str has a different length than two)
    InvalidStringLen,
}
