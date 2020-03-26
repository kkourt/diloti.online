//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use std::convert::TryFrom;
use super::error as e;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Suit {
    /// ♠
    Spade,
    /// ♣
    Club,
    /// ♥
    Heart,
    /// ♦
    Diamond,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rank(u8);

/// Game card
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

#[derive(PartialEq,Eq, Copy, Clone)]
pub struct CardClone {
    pub suit: Suit,
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

    pub fn is_figure(&self) -> bool {
        match self.0 {
            1..=10 => false,
            11..=13 => true,
            _ => panic!("Invalid rank"),
        }
    }
}

impl Suit {
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

impl Card {
    pub fn get_clone(&self) -> CardClone {
        CardClone {
            suit: self.suit,
            rank: self.rank,
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

impl TryFrom<char> for Suit {
    type Error = e::Error;

    fn try_from(val: char) -> Result<Suit, e::Error> {
        match val {
            's' | 'S' | '♠' => Ok(Suit::Spade),
            'c' | 'C' | '♣' => Ok(Suit::Club),
            'd' | 'D' | '♦' => Ok(Suit::Diamond),
            'h' | 'H' | '♥' => Ok(Suit::Heart),
            _ => Err(e::Error::InvalidSuitChar(val)),
        }
    }
}

impl TryFrom<[char; 2]> for Card {
    type Error = e::Error;

    /// suit first
    fn try_from(val: [char; 2]) -> Result<Card, e::Error> {
        let suit = Suit::try_from(val[0])?;
        let rank = Rank::try_from(val[1])?;
        Ok(Card{suit: suit, rank: rank})
    }
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("{}{}", self.suit.to_symbol(), self.rank.to_symbol()))
    }
}

impl std::fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("{}{}", self.suit.to_symbol(), self.rank.to_symbol()))
    }
}

impl std::fmt::Display for CardClone {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("{}{}", self.suit.to_symbol(), self.rank.to_symbol()))
    }
}

impl std::fmt::Debug for CardClone {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("{}{}", self.suit.to_symbol(), self.rank.to_symbol()))
    }
}

#[test]
fn try_from_tests() {
    // rank
    assert_eq!(Rank::try_from('1').unwrap(), Rank(1));
    assert_eq!(Rank::try_from('A').unwrap(), Rank(1));
    assert!(Rank::try_from('x').is_err());
    // suit
    assert_eq!(Suit::try_from('♥').unwrap(), Suit::Heart);
    assert!(Suit::try_from('x').is_err());
    // card
    assert_eq!(Card::try_from(['♥','T']).unwrap(), Card{suit: Suit::Heart, rank: Rank(10)});
}
