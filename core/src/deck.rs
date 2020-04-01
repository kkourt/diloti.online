//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//


use rand::prelude::SliceRandom;
use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use super::card::{Rank,Suit,Card};
use super::error as e;

/// A Deck is an ordered collection of cards
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Deck {
    pub cards: Vec<Card>,
}

impl Deck {

    pub fn empty() -> Deck {
        Deck { cards: vec![] }
    }

    pub fn full_52() -> Deck {
        let mut cards = vec![];
        for suit in [Suit::Spade, Suit::Club, Suit::Heart, Suit::Diamond].iter() {
            for rank in 1..=13 {
                cards.push(Card {
                    suit: *suit,
                    rank: Rank::try_from(rank).unwrap()
                });
            }
        }

        Deck { cards: cards }
    }

    pub fn pop(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    pub fn push(&mut self, card: Card) {
        self.cards.push(card)
    }

    pub fn shuffle<R>(&mut self, rng: &mut R)
    where R: rand::Rng {
        self.cards.shuffle(rng);
    }

    pub fn ncards(&self) -> usize {
        return self.cards.len()
    }

    pub fn to_inner(self) -> Vec<Card> {
        self.cards
    }
}

impl TryFrom<&str> for Deck {
    type Error = e::Error;

    fn try_from(s: &str) -> Result<Deck, e::Error> {
        let mut vec : Vec<Card> = vec![];
        for cs in s.split_whitespace() {
            let card = Card::try_from(cs)?;
            vec.push(card)
        }

        Ok(Deck { cards: vec })
    }

}

