//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use std::clone::Clone;

use super::deck::Deck;
use super::card::Card;
use super::table::{Table, Declaration, PlayerTpos, TableEntry};
use super::actions::{PlayerAction, DeclAction};

use serde::{Deserialize, Serialize};

// Rules:
//  - https://cardgamesgr.blogspot.com/2014/07/diloti.html
//  - http://alogomouris.blogspot.com/2011/02/blog-post_5755.html

// Design: we move card arounds. Add a custom destructor in the card to check that no card is
// "lost" when the game is played. The state of each card is implicit in which container it is
// stored in.


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaiseAction {
    pub hand_card: Card,
    pub decl: Declaration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeAction {
    pub hand_card: Card,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub hand: Deck,
}

#[derive(Clone, Debug)]
pub struct Game<R: rand::Rng + Clone> {
    pub(crate) table: Table,
    pub(crate) main_deck: Deck,
    pub(crate) players: Vec<Player>,

    pub(crate) turn: PlayerTpos,

    rng: R,
}


/// This a player's point of view of the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerGameView {
    pub pid: PlayerTpos,
    pub table: Table,
    pub own_hand: Deck,
    pub turn: PlayerTpos,

    pub main_deck_sz: usize,
    pub player_decks_sz: Vec<usize>,
}

// Some terminology:
//  - Each player has a turn
//  - A round is one deal of cards
//    (players are dealt 6 cards and take turns until they run out of cards)
//  - A game is playing rounds until the deck is done.
//  - A match is playing games until the max score is reached
//
// NB: (52 - 4) / 6 = 8, so there are 4 rounds on a 4-player game and 8 rounds on a 2 player game
//
pub enum GameState {
    NextPlayer(PlayerTpos),
    RoundDone,
    GameDone,
}

impl<R: rand::Rng + Clone> Game<R> {

    pub fn new_1p(rng: R) -> Game<R> {
        Self::init(1, rng)
    }

    pub fn new_1p_debug(rng: R, table: Table, hand: Deck) -> Game<R> {
        let players = vec![
            Player { hand: hand },
        ];

        Game {
            table: table,
            main_deck: Deck::empty(),
            players: players,
            turn: PlayerTpos(0),
            rng: rng,
        }
    }

    pub fn new_2p(rng: R) -> Game<R> {
        Self::init(2, rng)
    }

    pub fn new_4p(rng: R) -> Game<R> {
        Self::init(4, rng)
    }


    fn init(nplayers: usize, rng: R) -> Game<R> {
        assert!(nplayers == 1 || nplayers == 2 || nplayers == 4);

        let deck = Deck::full_52();

        let mut game = Game {
            table: Table { entries: vec![] },
            main_deck: deck,
            players: (0..nplayers).map( |_i| Player { hand: Deck::empty() } ).collect(),
            turn: PlayerTpos(0),
            rng: rng,
        };

        game.deal();
        game
    }

    fn deal(&mut self) {
        let hand_size = 6;
        let table_size = 4;

        self.main_deck.shuffle(&mut self.rng);

        for _ in 0..hand_size {
            for p in 0..self.players.len() {
                let card = self.main_deck.pop().unwrap();
                self.players[p].hand.push(card)
            }
        }

        for _ in 0..table_size {
            let card = self.main_deck.pop().unwrap();
            self.table.entries.push(TableEntry::Card(card));
        }

    }

    fn get_player(&self, tpos: PlayerTpos) -> Option<&Player> {
        self.players.get(tpos.0 as usize)
    }

    fn get_player_mut(&mut self, tpos: PlayerTpos) -> Option<&mut Player> {
        self.players.get_mut(tpos.0 as usize)
    }

    pub fn remove_player_card(&mut self, tpos:  PlayerTpos, c: &Card) -> Option<Card> {
        let player : &mut Player = self.get_player_mut(tpos)?;
        let pos = player.iter_hand_cards().position(|x| c == x )?;
        Some(player.hand.cards.remove(pos))
    }

    pub fn remove_table_card(&mut self, c: &Card) -> Option<Card> {
        self.table.remove_card(c)
    }

    pub fn remove_table_decl(&mut self, d: &Declaration) -> Option<Declaration> {
        self.table.remove_decl(d)
    }

    pub fn add_table_decl(&mut self, d: Declaration) {
        self.table.add_decl(d)
    }

    pub fn add_table_card(&mut self, c: Card) {
        self.table.add_card(c)
    }

    pub fn get_player_game_view(&self, pid: PlayerTpos) -> PlayerGameView {
        PlayerGameView {
            pid: pid,
            table: self.table.clone(),
            own_hand: self.players[pid.0 as usize].hand.clone(),
            turn: self.turn,

            main_deck_sz: self.main_deck.ncards(),
            player_decks_sz: self.players.iter().map(|p| p.hand.ncards()).collect(),
        }
    }

    fn next_turn(&mut self) {
        let nplayers = self.players.len() as u8;
        self.turn = PlayerTpos( (self.turn.0 + 1) % nplayers);
    }

    pub fn apply_action(&self, tpos: PlayerTpos, action: PlayerAction) -> Result<Self, String> {
        // poor man's transaction
        let mut new = self.clone();
        new.do_apply_action(tpos, action)?;
        Ok(new)
    }

    // NB: In case of an error, state might be incosistent.
    fn do_apply_action(&mut self, tpos: PlayerTpos, action: PlayerAction) -> Result<(), String> {

        {
            let pview = self.get_player_game_view(tpos);
            action.validate(&pview)?
        }

        match action {
            PlayerAction::LayDown(c) => {
                let card = self.remove_player_card(tpos, &c).ok_or_else(|| "Card does not exist")?;
                self.add_table_card(card);
            },
            PlayerAction::Declare(da) => self.do_apply_decl_action(tpos, &da)?,
            PlayerAction::Capture(ca) => unimplemented!(),
        }
        self.next_turn();
        Ok(())
    }

    fn do_apply_decl_action(&mut self, tpos: PlayerTpos, da: &DeclAction) -> Result<(), String> {
        let mut decl_cards : Vec<Vec<Card>> = vec![];
        for (i, entries_v) in da.tentries.iter().enumerate() {
            let entries_v_len = entries_v.len();
            let mut cards_v = vec![];
            for (j, te) in entries_v.iter().enumerate() {
                match ((i,j), te) {
                    // Hand card (by convention it's the first entry)
                    ((0,0), TableEntry::Card(c)) => {
                        let hand_card = self.remove_player_card(tpos, c).ok_or_else(|| "Hand card does not exist")?;
                        cards_v.push(hand_card);
                    },

                    // Any card (by convention it's the first entry)
                    (_, TableEntry::Card(c)) => {
                        let table_card =  self.remove_table_card(c).ok_or_else(|| "Table card does not exist")?;
                        cards_v.push(table_card);
                    },

                    // Plain declarations can be combined with other cards
                    (_, TableEntry::Decl(d)) if d.is_plain() => {
                        let decl = self.remove_table_decl(d).ok_or_else(|| "Table declaration does not exist")?;
                        let (decl_cards, _) = decl.into_inner();
                        assert!(decl_cards.len() == 1); // declaration is plain
                        cards_v.extend_from_slice(&decl_cards[0]);
                    },

                    // Group declarations cannot be combined with other cards, and have to
                    // be on their own
                    ((_,0), TableEntry::Decl(d)) if d.is_group() => {
                        if entries_v_len != 1 {
                            return Err("Invalid declaration: group declaration cannot be combined with other cards".to_string());
                        }
                        assert!(cards_v.len() == 0); // should be true since j is 0
                        let decl = self.remove_table_decl(d).ok_or_else(|| "Table declaration does not exist")?;
                        let (cvv, _) = decl.into_inner();
                        decl_cards.extend_from_slice(&cvv);
                        break;
                    },

                    // TODO: if this ever hits, add more info
                    _ => return Err("Invalid declaration".to_string()),
                }
            }

            // NB: len might be 0 in case of group declaration
            if cards_v.len() > 0 {
                decl_cards.push(cards_v.drain(..).collect());
            }
        }

        let decl = Declaration {
            cards: decl_cards,
            player: tpos,
        };

        self.add_table_decl(decl);
        Ok(())
    }
}

impl Player {
    pub fn iter_hand_cards(&self) -> impl Iterator<Item=&Card> {
        self.hand.cards.iter()
    }
}

impl PlayerGameView {

    pub fn iter_hand_cards(&self) -> impl Iterator<Item=&Card> {
        self.own_hand.cards.iter()
    }

    pub fn enum_hand_cards(&self) -> impl Iterator<Item=(usize, &Card)> {
        self.own_hand.cards.iter().enumerate()
    }

    pub fn get_hand_card(&self, idx: usize) -> &Card {
        &self.own_hand.cards[idx]
    }

    pub fn iter_table_entries(&self) -> impl Iterator<Item=&TableEntry> {
        self.table.entries.iter()
    }

    pub fn enum_table_entries(&self) -> impl Iterator<Item=(usize, &TableEntry)> {
        self.table.entries.iter().enumerate()
    }

    pub fn get_table_entry(&self, idx: usize) -> &TableEntry {
        &self.table.entries[idx]
    }


    pub fn card_in_hand(&self, c: &Card) -> bool {
        self.iter_hand_cards().find(|hc| c == *hc).is_some()
    }

    pub fn is_my_turn(&self) -> bool {
        return self.turn == self.pid;
    }
}

