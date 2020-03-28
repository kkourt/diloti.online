//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use super::deck::Deck;
use super::card::{Card, CardClone};

use serde::{Deserialize, Serialize};

// Rules:
//  - https://cardgamesgr.blogspot.com/2014/07/diloti.html
//  - http://alogomouris.blogspot.com/2011/02/blog-post_5755.html

// Design: we move card arounds. Add a custom destructor in the card to check that no card is
// "lost" when the game is played. The state of each card is implicit in which container it is
// stored in.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerAction {
    LayDown(CardClone),
    DeclareWith, /* TODO */
    TakeWith, /* TODO */
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidAction {
    action: PlayerAction,
    reason: String,
}


/// Player identifier based on its position on the table
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct PlayerTpos(pub u8);

#[derive(Clone, Serialize, Deserialize)]
pub struct Declaration {
    /// groups of cards
    cards: Vec<Vec<Card>>,
    /// Last player that made this declaration
    player: PlayerTpos,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum TableEntry {
    Card(Card),
    Decl(Declaration),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct  Table {
    pub entries: Vec<TableEntry>,
}

struct Player {
    pub hand: Deck,
}


pub struct Game<R: rand::Rng> {
    table: Table,
    main_deck: Deck,
    players: Vec<Player>,

    pub turn: PlayerTpos,

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

impl<R: rand::Rng> Game<R> {

    pub fn new_2p(rng: R) -> Game<R> {
        Self::init(2, rng)
    }

    pub fn new_4p(rng: R) -> Game<R> {
        Self::init(4, rng)
    }


    fn init(nplayers: usize, rng: R) -> Game<R> {
        assert!(nplayers == 2 || nplayers == 4);

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

    pub fn remove_player_card(&mut self, tpos:  PlayerTpos, cc: CardClone) -> Option<Card> {
        let player : &mut Player = self.get_player_mut(tpos)?;
        let pos = player.iter_hand_cards().position(|x| cc.is_card(x) )?;
        Some(player.hand.cards.remove(pos))
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

    pub fn apply_action(&mut self, tpos: PlayerTpos, action: &PlayerAction) -> Result<(), String> {
        self.get_player_game_view(tpos).validate_action(action)?;
        match action {
            PlayerAction::LayDown(cc) => {
                let card = self.remove_player_card(tpos, *cc).ok_or_else(|| "Card does not exist")?;
                self.table.entries.push(TableEntry::Card(card));
            },
            _ => unimplemented!(),
        }
        self.next_turn();
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

    pub fn iter_table_entries(&self) -> impl Iterator<Item=&TableEntry> {
        self.table.entries.iter()
    }

    pub fn card_in_hand(&self, cc: CardClone) -> bool {
        self.iter_hand_cards().find(|c| c.rank == cc.rank && c.suit == cc.suit).is_some()
    }

    pub fn is_lay_down_valid(&self, cc: CardClone) -> Result<(), String> {
        // RULE: you are not allow to lay down a figure card, if the same figure already exists on
        // the table.
        if cc.rank.is_figure() {
            let matching_figure : Option<&Card> = self.table.entries.iter().find_map(|te| {
                match te {
                    TableEntry::Card(c) => if c.rank == cc.rank { Some(c) } else { None },
                    TableEntry::Decl(_) => None,
                }
            });

            match matching_figure {
                Some(c) => return Err(format!("You cannot lay down a figure card ({}) if the same ({}) exists on the table. You have to take it!", cc, c)),
                _ => (),
            }
        }

        Ok(())
    }

    pub fn is_my_turn(&self) -> bool {
        return self.turn == self.pid;
    }

    // NB: we put all validation logic here so that clients that only have this object can perform
    // it.
    pub fn validate_action(&self, action: &PlayerAction) -> Result<(), String> {
        // TODO: verify version once
        if !self.is_my_turn() {
            return Err("Not this player's turn".into());
        }

        match action {
            PlayerAction::LayDown(cc) => self.is_lay_down_valid(*cc),
            _ => unimplemented!(),
        }
    }
}

impl std::fmt::Display for PlayerTpos {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let PlayerTpos(pid) = *self;
        write!(f, "P{}", pid)
    }
}

impl std::fmt::Debug for TableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Card(c) => write!(f, "{}", c),
            Self::Decl(d) => unimplemented!(),
        }
    }
}

impl std::fmt::Display for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_list().entries(self.entries.iter()).finish()
    }
}

#[test]
fn pv_validation_tests() {
}
