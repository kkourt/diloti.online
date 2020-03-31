//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use std::clone::Clone;

use super::deck::Deck;
use super::card::Card;

use serde::{Deserialize, Serialize};

// Rules:
//  - https://cardgamesgr.blogspot.com/2014/07/diloti.html
//  - http://alogomouris.blogspot.com/2011/02/blog-post_5755.html

// Design: we move card arounds. Add a custom destructor in the card to check that no card is
// "lost" when the game is played. The state of each card is implicit in which container it is
// stored in.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclAction {
    pub hand_card: Card, // hand card is included twice: here, and also in the card_groups vector
    pub card_groups: Vec<Vec<Card>>,
    // If you make a decleration, you *must* add to it existing cards and declarations of the same
    // value, so I think it's OK if we allow the user to specify this.
    pub decl_groups: Vec<Declaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerAction {
    LayDown(Card),
    Declare(DeclAction),
    RaiseWith,   /* TODO */
    TakeWith,    /* TODO */
}

#[derive(Clone, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Declaration {
    /// groups of cards
    pub cards: Vec<Vec<Card>>,
    /// Last player that made this declaration
    pub player: PlayerTpos,
}

impl Declaration {

    pub fn cards(&self) -> &Vec<Vec<Card>> {
        &self.cards
    }

    pub fn player(&self) -> PlayerTpos {
        self.player
    }

    pub fn value(&self) -> u8 {
        self.cards[0].iter().fold(0, |acc, x| acc + x.rank.0)
    }

    pub fn is_group(&self) -> bool {
        self.cards.len() > 1
    }

    pub fn new(cards: Vec<Vec<Card>>, player: PlayerTpos) -> Option<Declaration> {
        let len = cards.len();
        if len > 1 {
            let val = cards[0].iter().fold(0, |acc, x| acc + x.rank.0);
            for i in 1..len {
                let val_i = cards[i].iter().fold(0, |acc, x| acc + x.rank.0);
                if val_i != val {
                    return None;
                }
            }
        }

        Some(Declaration {
            cards: cards,
            player: player,
        })
    }

    pub fn into_inner(self) -> (Vec<Vec<Card>>, PlayerTpos) {
        (self.cards, self.player)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidAction {
    action: PlayerAction,
    reason: String,
}


/// Player identifier based on its position on the table
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub struct PlayerTpos(pub u8);

#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableEntry {
    Card(Card),
    Decl(Declaration),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Table {
    pub entries: Vec<TableEntry>,
}

#[derive(Clone, Debug)]
struct Player {
    pub hand: Deck,
}


#[derive(Clone, Debug)]
pub struct Game<R: rand::Rng + Clone> {
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

impl TableEntry {
    pub fn unwrap_card(self) -> Card {
        match self {
            TableEntry::Card(c) => c,
            TableEntry::Decl(_) => panic!("unwrap_card() called on a Declaration"),
        }
    }

    pub fn unwrap_decl(self) -> Declaration {
        match self {
            TableEntry::Decl(d) => d,
            TableEntry::Card(_) => panic!("unwrap_decl() called on a Card"),
        }
    }
}

impl Table {
    pub fn remove_card(&mut self, arg: &Card) -> Option<Card> {
        let idx = self.entries.iter().position(|e| {
            match e {
                TableEntry::Card(c) => c == arg,
                _ => false,
            }
        })?;

        Some(self.entries.remove(idx).unwrap_card())
    }

    pub fn remove_decl(&mut self, arg: &Declaration) -> Option<Declaration> {
        let idx = self.entries.iter().position(|e| {
            match e {
                TableEntry::Decl(d) => d == arg,
                _ => false,
            }
        })?;

        Some(self.entries.remove(idx).unwrap_decl())
    }

    pub fn add_decl(&mut self, d: Declaration) {
        self.entries.push(TableEntry::Decl(d))
    }

    pub fn add_card(&mut self, c: Card) {
        self.entries.push(TableEntry::Card(c))
    }
}

impl<R: rand::Rng + Clone> Game<R> {

    pub fn new_1p(rng: R) -> Game<R> {
        Self::init(1, rng)
    }

    pub fn new_1p_debug(rng: R, mut tcards: Vec<Card>, hcards: Vec<Card>) -> Game<R> {
        let table_entries : Vec<TableEntry> = tcards.drain(..).map(|c| TableEntry::Card(c)).collect();
        let players = vec![ Player { hand: Deck { cards: hcards, }, } ];

        Game {
            table: Table { entries: table_entries },
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

    // poor man's transaction
    pub fn apply_action(&self, tpos: PlayerTpos, action: PlayerAction) -> Result<Self, String> {
        let mut new = self.clone();
        new.do_apply_action(tpos, action)?;
        Ok(new)
    }

    // NB: In case of an error, state might be incosistent.
    fn do_apply_action(&mut self, tpos: PlayerTpos, action: PlayerAction) -> Result<(), String> {
        self.get_player_game_view(tpos).validate_action(&action)?;
        match action {
            PlayerAction::LayDown(c) => {
                let card = self.remove_player_card(tpos, &c).ok_or_else(|| "Card does not exist")?;
                self.add_table_card(card);
            },

            PlayerAction::Declare(mut da) => {
                let mut cards : Vec<Vec<Card>> = vec![];
                let mut count = 0;
                for mut cg in da.card_groups.drain(..) {
                    let mut cvec: Vec<Card> = vec![];
                    for c in cg.drain(..) {
                        let card = if count == 0 {
                            self.remove_player_card(tpos, &c).ok_or_else(|| "Hand card does not exist")?
                        } else {
                            self.remove_table_card(&c).ok_or_else(|| "Table card does not exist")?
                        };
                        cvec.push(card);
                        count += 1;
                    }
                    cards.push(cvec);
                }

                for d in da.decl_groups.drain(..) {
                    let decl = self.remove_table_decl(&d).ok_or_else(|| "Table declaration does not exist")?;
                    let (cvec, _) = decl.into_inner();
                    cards.extend_from_slice(&cvec)
                }

                self.add_table_decl(Declaration {
                    cards: cards,
                    player: tpos,
                });

            }

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

    pub fn is_lay_down_valid(&self, c: &Card) -> Result<(), String> {
        // RULE: you are not allow to lay down a figure card, if the same figure already exists on
        // the table.
        if c.rank.is_figure() {
            let matching_figure : Option<&Card> = self.table.entries.iter().find_map(|te| {
                match te {
                    TableEntry::Card(tc) => if tc.rank == c.rank { Some(tc) } else { None },
                    TableEntry::Decl(_) => None,
                }
            });

            match matching_figure {
                Some(tc) => return Err(format!("You cannot lay down a figure card ({}) if the same ({}) exists on the table. You have to take it!", c, tc)),
                _ => (),
            }
        }

        Ok(())
    }

    pub fn is_my_turn(&self) -> bool {
        return self.turn == self.pid;
    }

    pub fn validate_action(&self, action: &PlayerAction) -> Result<(), String> {
        if !self.is_my_turn() {
            return Err("Not this player's turn".into());
        }

        // TODO: check if user has a declaration on the table.

        match action {
            PlayerAction::LayDown(c)  => self.is_lay_down_valid(c),
            PlayerAction::Declare(da) => da.is_valid(),
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

impl DeclAction {
    pub fn is_valid(&self) -> Result<(), String> {

        // size checks
        if self.card_groups.len() == 0 {
            return Err("Buggy Client. Empty card group.".into());
        } else if self.card_groups.len() == 1 && self.card_groups[0].len() == 1{
            return Err("Invalild single declaration".into());
        }

        if self.hand_card != self.card_groups[0][0] {
            return Err("Buggy Client. The hand card should be the first card!".into());
        }

        let val = self.card_groups[0].iter().fold(0, |acc, x| acc + x.rank.0);
        if val > 10 || val < 1 {
            return Err(format!("Invalid declaration value: {}", val));
        }


        for g in self.card_groups[1..].iter() {
            let val_g = g.iter().fold(0, |acc, x| acc + x.rank.0);
            if val != val_g {
                return Err(format!("Not equal group values: {} vs {}", val_g, val));
            }
        }

        for decl in self.decl_groups.iter() {
            if val != decl.value() {
                return Err(format!("Not equal group values: {} vs {}", decl.value(), val));
            }
        }

        Ok(())
    }
}

#[test]
fn pv_validation_tests() {
}
