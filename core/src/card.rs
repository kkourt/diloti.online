//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use std::convert::TryFrom;
use super::error as e;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suite {
    /// ♠
    Spade,
    /// ♣
    Club,
    /// ♥
    Heart,
    /// ♦
    Diamond,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rank(u8);

/// Game card
#[derive(PartialEq,Eq)]
pub struct Card {
    pub suite: Suite,
    pub rank: Rank,
}

// TODO: Implement this for other integer types as well. Use a macro.
impl TryFrom<u8> for Rank {
    type Error = e::Error;

    fn try_from(val: u8) -> Result<Rank, e::Error> {
        if val < 1 || val > 13 {
            Err(e::Error::InvalidRankNumber(format!("{}", val)))
        } else {
            Ok(Rank(val))
        }
    }
}

impl Rank {
    pub fn to_symbol(&self) -> char {
        match self.0 {
            1 => 'A',
            2..=9 => ('0' as u8 + self.0) as char,
            10 => 'T',
            11 => 'J',
            12 => 'Q',
            13 => 'K',
            _ => panic!("Invalid rank"),
        }
    }
}

impl Suite {
    pub fn is_red(&self) -> bool {
        match self {
            Self::Spade => false,
            Self::Club => false,
            Self::Heart => true,
            Self::Diamond => true,
        }
    }

    pub fn to_symbol(&self) -> char {
        match self {
            Self::Spade => '♠',
            Self::Club => '♣',
            Self::Heart => '♥',
            Self::Diamond => '♦',
        }
    }
}

impl TryFrom<char> for Rank {
    type Error = e::Error;

    fn try_from(val: char) -> Result<Rank, e::Error> {
        match val {
            'a' | 'A' => Ok(Rank(1)),
            't' | 'T' => Ok(Rank(10)),
            'j' | 'J' => Ok(Rank(11)),
            'q' | 'Q' => Ok(Rank(12)),
            'k' | 'K' => Ok(Rank(13)),
            '1'..='9' => {
                let i: u8 = (val as u8) - ('0' as u8);
                let ret = Rank::try_from(i);
                assert!(ret.is_ok());
                ret
            }
            _ => Err(e::Error::InvalidRankChar(val)),
        }
    }
}

impl TryFrom<char> for Suite {
    type Error = e::Error;

    fn try_from(val: char) -> Result<Suite, e::Error> {
        match val {
            's' | 'S' | '♠' => Ok(Suite::Spade),
            'c' | 'C' | '♣' => Ok(Suite::Club),
            'd' | 'D' | '♦' => Ok(Suite::Diamond),
            'h' | 'H' | '♥' => Ok(Suite::Heart),
            _ => Err(e::Error::InvalidSuiteChar(val)),
        }
    }
}

impl TryFrom<[char; 2]> for Card {
    type Error = e::Error;

    /// suite first
    fn try_from(val: [char; 2]) -> Result<Card, e::Error> {
        let suite = Suite::try_from(val[0])?;
        let rank = Rank::try_from(val[1])?;
        Ok(Card{suite: suite, rank: rank})
    }
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("{}{}", self.suite.to_symbol(), self.rank.to_symbol()))
    }
}

impl std::fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("{}{}", self.suite.to_symbol(), self.rank.to_symbol()))
    }
}

#[test]
fn try_from_tests() {
    // rank
    assert_eq!(Rank::try_from('1').unwrap(), Rank(1));
    assert_eq!(Rank::try_from('A').unwrap(), Rank(1));
    assert!(Rank::try_from('x').is_err());
    // suite
    assert_eq!(Suite::try_from('♥').unwrap(), Suite::Heart);
    assert!(Suite::try_from('x').is_err());
    // card
    assert_eq!(Card::try_from(['♥','T']).unwrap(), Card{suite: Suite::Heart, rank: Rank(10)});
}
