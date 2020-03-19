//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//


use rand::prelude::SliceRandom;
use std::convert::TryFrom;

use super::card::{Rank,Suite,Card};

/// A Deck is an ordered collection of cards
pub struct Deck {
    pub cards: Vec<Card>,
}

impl Deck {
    pub fn empty() -> Deck {
        Deck { cards: vec![] }
    }

    pub fn full_52() -> Deck {
        let mut cards = vec![];
        for suite in [Suite::Spade, Suite::Club, Suite::Heart, Suite::Diamond].iter() {
            for rank in 1..=13 {
                cards.push(Card {
                    suite: *suite,
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
}

impl std::fmt::Display for Deck {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_list().entries(self.cards.iter()).finish()
    }
}
