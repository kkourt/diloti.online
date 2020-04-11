//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:

use serde::{Deserialize, Serialize};

use super::card::{Card, Rank, Suit};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Captures(Vec<Capture>);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Capture {
    Card(Card),
    Xeri(Card),
}

pub const NCARDS_SCORE: usize = 4;
pub const XERI_SCORE: usize = 10;
pub const NCARDS: usize = 52;

pub fn card_value(c: &Card) -> usize {
    match c {
        Card { rank: Rank(1),  suit: _ } => 1,
        Card { rank: Rank(10), suit: Suit::Diamond } => 2,
        Card { rank: Rank(2), suit: Suit::Club } => 1,
        _ => 0,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreSheet {
    pub nr_cards: usize,           // 4 points for the team with the more cards
    pub nr_xeres: usize,           // 10 points for each xeri
    pub score_cards: Vec<Card>,    // scoring cards
    pub score: usize,              // total score
}

impl ScoreSheet {
    pub fn new() -> ScoreSheet {
        ScoreSheet {
            nr_cards: 0,
            nr_xeres: 0,
            score_cards: vec![],
            score: 0,
        }
    }

    pub fn has_the_cards(&self) -> bool {
        self.nr_cards > (NCARDS / 2)
    }

    pub fn add_capture(mut self, capture: &Capture) -> Self {
        let card = match capture {
            Capture::Card(c) => c,
            Capture::Xeri(c) => {
                self.nr_xeres += 1;
                self.score += XERI_SCORE;
                c
            },
        };

        self.nr_cards += 1;
        if self.nr_cards == (NCARDS / 2) + 1 {
            self.score  += NCARDS_SCORE;
        }

        match card_value(&card) {
            0 => (),
            v => {
                self.score_cards.push(card.clone());
                self.score += v;
            }
        };

        self
    }
}

impl Capture {
    pub fn unwrap(self) -> Card {
        match self {
            Capture::Card(c) => c,
            Capture::Xeri(c) => c,
        }
    }
}


impl Captures {
    pub fn new() -> Captures {
        Captures(vec![])
    }

    pub fn add_cards_(&mut self, mut cards: Vec<Card>, is_xeri: bool) {
        let mut iter = cards.drain(..);
        if is_xeri {
            let xeri_card = iter.next().unwrap();
            let xeri = Capture::Xeri(xeri_card);
            self.0.push(xeri);
        }

        self.0.extend(iter.map(|x| Capture::Card(x)));
    }

    pub fn add_final_cards(&mut self, cards: Vec<Card>, is_xeri: bool) {
        self.add_cards_(cards, is_xeri);
    }

    pub fn add_cards(&mut self, cards: Vec<Card>, is_xeri: bool) {
        assert!(cards.len() > 1); // at least one from hand, and one from the table
        self.add_cards_(cards, is_xeri);
    }

    pub fn score(&self) -> ScoreSheet {
        self.0.iter().fold(ScoreSheet::new(), |ss, c| ss.add_capture(c))
    }
}
