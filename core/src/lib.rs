//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

// XXX: until code stabilizes...
#![allow(dead_code)]
#![allow(unused_variables)]

extern crate rand;

pub mod error;
pub mod card;
pub mod deck;
pub mod game;
pub mod srvcli;

pub use card::{Card, Rank, Suit};
pub use deck::Deck;
pub use game::{Game, PlayerGameView, PlayerAction, TableEntry, Declaration, DeclAction};
