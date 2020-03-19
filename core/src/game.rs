//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use super::deck::Deck;
use super::card::Card;

// Rules:
//  - https://cardgamesgr.blogspot.com/2014/07/diloti.html
//  - http://alogomouris.blogspot.com/2011/02/blog-post_5755.html

// Design: we move card arounds. Add a custom destructor in the card to check that no card is
// "lost" when the game is played. The state of each card is implicit in which container it is
// stored in.

// NB: User messages and actions need to refer to objects in the game state (e.g., cards)
// I use indices in the game state array together with a version number as a sanity check.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct GameVer(u64);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct HandCardIdx(usize, GameVer);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct TableEntryIdx(usize, GameVer);

pub struct Player {
    pub hand: Deck,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct PlayerId(pub u8);

#[derive(Clone)]
pub struct Declaration {
    cards: Vec<Vec<Card>>,
    player: PlayerId,
    sum: u8,
}

#[derive(Clone)]
pub enum TableEntry {
    Card(Card),
    Decl(Declaration),
}

#[derive(Clone)]
pub struct  Table {
    pub entries: Vec<TableEntry>,
}

pub struct Game<R: rand::Rng> {
    pub table: Table,
    pub main_deck: Deck,
    pub players: Vec<Player>,

    pub turn: PlayerId,

    rng: R,
    ver: GameVer,
}

#[derive(Debug)]
pub enum PlayerAction {
    Play(HandCardIdx),
    DeclareWith,
    TakeWith,
}

pub struct InvalidAction {
    action: PlayerAction,
    reason: String,
}

/// This a player's point of view of the game
pub struct PlayerGameView {
    pub pid: PlayerId,
    pub table: Table,
    pub own_hand: Deck,
    pub ver: GameVer,

    pub main_deck_sz: usize,
    pub player_decks_sz: Vec<usize>,
}

pub struct PlayerTurn {
    pub game_view: PlayerGameView,
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
    NextPlayer(PlayerId),
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


    fn init(nplayers: usize, rng: R) -> Game<R>
    {
        assert!(nplayers == 2 || nplayers == 4);

        let deck = Deck::full_52();

        let mut game = Game {
            table: Table { entries: vec![] },
            main_deck: deck,
            players: (0..nplayers).map( |_i| Player { hand: Deck::empty() } ).collect(),
            turn: PlayerId(0),
            rng: rng,
            ver: GameVer(0),
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

    pub fn get_player_game_view(&self) -> PlayerGameView {
        PlayerGameView {
            pid: self.turn,
            table: self.table.clone(),
            own_hand: self.players[self.turn.0 as usize].hand.clone(),
            ver: self.ver,

            main_deck_sz: self.main_deck.ncards(),
            player_decks_sz: self.players.iter().map(|p| p.hand.ncards()).collect(),
        }
    }

    pub fn start_player_turn(&self) -> PlayerTurn {
        PlayerTurn {
            game_view: self.get_player_game_view(),
        }
    }

    pub fn  end_player_turn(&self, turn: PlayerTurn, action: PlayerAction) -> Result<GameState, InvalidAction> {
        unimplemented!()
    }

}

impl PlayerGameView {

    pub fn get_hand_card(&self, cidx: &HandCardIdx) -> &Card {
        let HandCardIdx(idx, ver) = cidx;
        if *ver != self.ver {
            panic!("mismatching versions: {:?} vs {:?}", ver, self.ver);
        }
        &self.own_hand.cards[*idx]
    }

    pub fn iter_hand_cards(&self) -> impl Iterator<Item=(HandCardIdx, &Card)> {
        let ver = self.ver;
        self.own_hand.cards
            .iter()
            .enumerate()
            .map(move |(i,c)| (HandCardIdx(i, ver), c))
    }

    pub fn iter_table_entries(&self) -> impl Iterator<Item=(TableEntryIdx, &TableEntry)> {
        let ver = self.ver;
        self.table.entries
            .iter()
            .enumerate()
            .map(move |(i,e)| (TableEntryIdx(i, ver), e))
    }

}

impl std::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let PlayerId(pid) = *self;
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
