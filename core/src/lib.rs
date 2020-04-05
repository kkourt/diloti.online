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
pub mod table;
pub mod game;
pub mod srvcli;
pub mod repr;
pub mod actions;
pub mod scoring;

#[cfg(test)]
pub mod tests;

pub use card::{Card, Rank, Suit};
pub use deck::Deck;
pub use table::{Table, TableEntry, Declaration};
pub use game::{Game, GameState, PlayerGameView};
pub use actions::{PlayerAction, DeclAction, DeclActionBuilder, CaptureAction, CaptureActionBuilder};
pub use scoring::{ScoreSheet};
