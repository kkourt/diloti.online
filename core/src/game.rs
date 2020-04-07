//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use std::clone::Clone;

use serde::{Deserialize, Serialize};

use super::deck::Deck;
use super::card::Card;
use super::table::{Table, Declaration, PlayerTpos, TableEntry};
use super::actions::{PlayerAction, DeclAction, CaptureAction, PerformedAction};
use super::scoring::{Captures, ScoreSheet};


// Rules: we are using a variant where:
//  - if a card can be added to a new declaration, it is automatically added (even if the player
//  does not state it)
//  - Capturing captures all cards and declarations on the table with the same value.
//
// Extended rules:
//  - https://www.pagat.com/fishing/diloti.html
//  - https://cardgamesgr.blogspot.com/2014/07/diloti.html
//  - http://alogomouris.blogspot.com/2011/02/blog-post_5755.html

// Design notes: we move card arounds. The state of each card is implicit in which container it is
// stored in. I wanted to avoid "copying" the cards between server and client. The two solutions I
// came up with were: i) use indices and a global version for the game, ii) use "card copies" that
// have the same contents as the cards but a different types that can be cloned. To simplify
// things, however, I ended just copying Cards for now. Also, users send the server all the cards
// when they want to designate a declaration on the table which can be optimized in future
// versions.


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub hand: Deck,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Team {
    pub captures: Captures,
    pub score: usize,
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GameState {
    NextTurn(PlayerTpos),
    RoundDone,
    GameDone(Vec<ScoreSheet>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game<R: rand::Rng + Clone> {
    pub(crate) table: Table,
    pub(crate) main_deck: Deck,
    pub(crate) players: Vec<Player>,

    pub(crate) teams: Vec<Team>,
    pub(crate) last_team_captured: usize,

    pub(crate) state: GameState,
    pub (crate) first_player: PlayerTpos,

    pub(crate) last_action: Option<PerformedAction>,

    rng: R,
}


/// This a player's point of view of the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerGameView {
    pub pid: PlayerTpos,
    pub table: Table,
    pub own_hand: Deck,
    pub state: GameState,

    pub last_action: Option<PerformedAction>,
    pub main_deck_sz: usize,
    pub player_decks_sz: Vec<usize>,
}

impl<R: rand::Rng + Clone> Game<R> {

    pub fn new_1p(rng: R) -> Game<R> {
        Self::init(1, rng)
    }

    pub fn new_1p_debug(rng: R, table: Table, hand: Deck) -> Game<R> {
        let players = vec![
            Player { hand: hand },
        ];
        let first_p = PlayerTpos(0);

        Game {
            table: table,
            main_deck: Deck::empty(),
            players: players,
            first_player: first_p,
            state: GameState::NextTurn(first_p),
            rng: rng,
            teams: vec![Team::default()],
            last_team_captured: 0, // NB: technically not true
            last_action: None,
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
        let nteams = if nplayers == 1 { 1 } else { 2 };

        let deck = Deck::full_52();
        let first_player = PlayerTpos(0);

        let mut game = Game {
            table: Table { entries: vec![] },
            main_deck: deck,
            players: (0..nplayers).map( |_i| Player { hand: Deck::empty() } ).collect(),
            first_player: first_player,
            state: GameState::NextTurn(first_player),
            rng: rng,
            teams: (0..nteams).map( |_| Team::default()).collect(),
            last_team_captured: 0,
            last_action: None,
        };

        game.shuffle_deck();
        game.deal_hands();
        game.deal_table();
        game
    }

    fn team_idx(&mut self, tpos: PlayerTpos) -> usize {
        let idx = (tpos.0 % 2) as usize;
        assert!(idx < self.teams.len());
        idx
    }

    fn update_captures(&mut self, tpos: PlayerTpos, captured_cards: Vec<Card>) {
        let idx = self.team_idx(tpos);
        let is_xeri = self.table.nentries() == 0;
        self.teams[idx].captures.add_cards(captured_cards, is_xeri);
        self.last_team_captured = idx;
    }

    fn finalize_captures(&mut self) {
        let idx = self.last_team_captured;
        let cards = self.table.remove_all_cards();
        let is_xeri = false;
        self.teams[idx].captures.add_final_cards(cards, is_xeri);
    }

    fn shuffle_deck(&mut self) {
        self.main_deck.shuffle(&mut self.rng);
    }

    pub fn new_round(&mut self) {
        assert!(self.state.is_round_done());
        self.deal_hands();
        self.state = GameState::NextTurn(self.first_player);
    }

    pub fn deal_hands(&mut self) {
        assert!(self.all_players_done());
        assert!(self.main_deck.ncards() > 0);
        let hand_size = 6;
        for _ in 0..hand_size {
            for p in 0..self.players.len() {
                let card = self.main_deck.pop().unwrap();
                self.players[p].hand.push(card)
            }
        }
    }

    fn deal_table(&mut self) {
        let table_size = 4;
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
            state: self.state.clone(),
            last_action: self.last_action.clone(),

            main_deck_sz: self.main_deck.ncards(),
            player_decks_sz: self.players.iter().map(|p| p.hand.ncards()).collect(),
        }
    }

    pub fn all_players_done(&self) -> bool {
        self.players.iter().all(|p| p.hand.ncards() == 0)
    }

    fn next_turn(&mut self) {
        if let GameState::NextTurn(curr_tpos) = self.state {
            let nplayers = self.players.len() as u8;
            let next_tpos = PlayerTpos((curr_tpos.0 + 1) % nplayers);
            if self.get_player(next_tpos).unwrap().hand.ncards() > 0 {
                self.state = GameState::NextTurn(next_tpos);
            } else if self.main_deck.ncards() > 0 {
                assert!(self.all_players_done());
                self.state = GameState::RoundDone;
            } else {
                assert!(self.all_players_done());
                self.finalize_captures();
                let mut sheets = vec![];
                for team in self.teams.iter_mut() {
                    sheets.push(team.update_score())
                }
                self.state = GameState::GameDone(sheets);
            }
        } else {
            panic!("Invalid call of next_turn()")
        }
    }

    pub fn apply_action(&self, tpos: PlayerTpos, action: PlayerAction) -> Result<Self, String> {
        // poor man's transaction: we copy everything, try to apply the action, and then either
        // return an error or the new state. This is because any errors during action application
        // could leave the game in an incosistent state. It should be possible to ensure
        // that there are no errors by doing a perfect validation, but we do not do this
        // currently.
        let mut new = self.clone();
        let performed_act = new.do_apply_action(tpos, action)?;
        new.last_action = Some(performed_act);
        new.next_turn();
        Ok(new)
    }

    pub fn state(&self) -> &GameState {
        return &self.state
    }

    pub fn score(&self) -> Vec<ScoreSheet> {
        self.teams.iter().map(|x| x.captures.score()).collect()
    }

    // NB: In case of an error, state might be incosistent.
    fn do_apply_action(&mut self, tpos: PlayerTpos, action: PlayerAction) -> Result<PerformedAction, String> {
        {
            let pview = self.get_player_game_view(tpos);
            action.validate(&pview)?
        }

        match action {
            PlayerAction::LayDown(c) => {
                let card = self.remove_player_card(tpos, &c).ok_or_else(|| "Card does not exist")?;
                self.add_table_card(card);
                Ok(PerformedAction {
                    action: PlayerAction::LayDown(c),
                    player: tpos,
                    forced_cards: vec![],
                    xeri: false,
                })
            },
            PlayerAction::Declare(da) => self.do_apply_decl_action(tpos, da),
            PlayerAction::Capture(ca) => self.do_apply_capture_action(tpos, ca),
        }
    }

    /// enforce obligations on a declaration
    fn decl_enforce_obligations(&mut self, da: &DeclAction, decl_cards: &mut Vec<Vec<Card>>) -> Vec<Card> {
        // NB: not sure if this is all of it, but for simplicity let's do the following:
        // whenever there is a *new* declaration, then existing cards on the table with the same
        // value are dragged in it.
        let mut ret: Vec<Card> = vec![];

        // Not a new declaration, do nothing
        if da.has_decl() {
            return ret;
        }

        let val = da.value();
        while let Some(card) = self.table.remove_card_with_value(val) {
            ret.push(card.clone());
            decl_cards.push(vec![card])
        }

        ret

    }

    fn do_apply_capture_action(&mut self, tpos: PlayerTpos, ca: CaptureAction) -> Result<PerformedAction, String> {
        let mut captured_cards : Vec<Card> = vec![];

        let hand_card = self.remove_player_card(tpos, &ca.handcard).ok_or_else(|| "Hand card does not exist")?;
        captured_cards.push(hand_card);

        for te in ca.tentries.iter().flatten() {
            match te {
                TableEntry::Card(c) => {
                    let table_card =  self.remove_table_card(&c).ok_or_else(|| "Table card does not exist")?;
                    captured_cards.push(table_card);
                },
                TableEntry::Decl(d) => {
                    let mut table_decl = self.remove_table_decl(&d).ok_or_else(|| "Table declaration does not exist")?;
                    for c in table_decl.cards.drain(..).flatten() {
                        captured_cards.push(c);
                    }
                },
            }
        }

        // obligatory captures
        let val = ca.value();
        let mut forced_cards = vec![];
        while let Some(te) = self.table.remove_entry_with_value(val) {
            match te {
                TableEntry::Card(c) => {
                    forced_cards.push(c);
                    //captured_cards.push(c);
                },
                TableEntry::Decl(mut d) => {
                    for c in d.cards.drain(..).flatten() {
                        forced_cards.push(c);
                        //captured_cards.push(c);
                    }
                },
            }
        }
        captured_cards.extend_from_slice(&forced_cards[..]);

        // update scoring structures
        self.update_captures(tpos, captured_cards);
        let xeri = self.table.nentries() == 0;

        Ok(PerformedAction {
            action: PlayerAction::Capture(ca),
            player: tpos,
            forced_cards: forced_cards,
            xeri: xeri,
        })
    }

    fn do_apply_decl_action(&mut self, tpos: PlayerTpos, da: DeclAction) -> Result<PerformedAction, String> {


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

        let forced_cards = self.decl_enforce_obligations(&da, &mut decl_cards);

        let decl = Declaration {
            cards: decl_cards,
            player: tpos,
        };
        self.add_table_decl(decl);

        Ok(PerformedAction {
            action: PlayerAction::Declare(da),
            player: tpos,
            forced_cards: forced_cards,
            xeri: false,
        })
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
        match self.state {
            GameState::NextTurn(tpos) if tpos == self.pid => true,
            _ => false,
        }
    }

    pub fn active_tpos(&self) -> Option<PlayerTpos> {
        match self.state {
            GameState::NextTurn(tpos) => Some(tpos),
            _ => None,
        }
    }
}


impl Default for Team {
    fn default() -> Self {
        Team {
            captures: Captures::new(),
            score: 0,
        }
    }
}

impl GameState {
    pub fn is_round_done(&self) -> bool {
        match self {
            GameState::RoundDone => true,
            _ => false,
        }
    }
}

impl Team {
    pub fn update_score(&mut self) -> ScoreSheet {
        let sheet = self.captures.score();
        self.score += sheet.score;
        self.captures = Captures::new();
        sheet
    }
}
