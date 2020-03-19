
extern crate rand;
extern crate rand_pcg;

use seed::{*, prelude::*};
use rand::SeedableRng;
use core::{deck::Deck, card::Card, game::Game, game::TableEntry};

type XRng = rand_pcg::Pcg64;

/// Initial state

#[derive(Clone)]
enum InitMsg {
    StartGame,
}

struct InitSt {}
impl InitSt {

    fn update_state(&mut self, msg: &InitMsg) -> Option<Model> {
        Some(Model::InGame(
            GameSt::default()
        ))
    }

    fn view(&self) -> Node<Msg> {
        let initmsg = InitMsg::StartGame;
        button![
            simple_ev(Ev::Click, Msg::Init(initmsg)),
            format!("Start")
        ]
    }
}


/// Game state

struct GameSt {
    pub game: Game<XRng>,
}

impl Default for GameSt {
    fn default() -> Self {
        let rng = XRng::from_rng(rand::rngs::OsRng).expect("unable to initalize RNG");
        Self {
            game: Game::new_2p(rng),
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        // Self::Init(InitSt {})
        Self::InGame(GameSt::default())
    }
}

#[derive(Clone)]
enum InGameMsg {
}


impl GameSt {
    fn update_state(&mut self, msg: &InGameMsg) -> Option<Model> {
        unimplemented!()
    }

    fn mk_card_div(&self, card: &Card) -> Node<Msg> {
        let attr = if card.suite.is_red() {
            let mut a = Attrs::empty();
            a.add_multiple(At::Class, &["card", "red"]);
            a
        } else {
            let mut a = Attrs::empty();
            a.add_multiple(At::Class, &["card", "black"]);
            a
        };

        div![ attr, format!("{}", card)]
    }

    fn mk_table_entry_div(&self, te: &TableEntry) -> Node<Msg> {
        match te {
            TableEntry::Decl(x) => unimplemented!(),
            TableEntry::Card(x) => self.mk_card_div(x),
        }
    }

    fn view(&self) -> Node<Msg> {

        let p_id = self.game.turn.0;
        let hand = {
            let p_hand = &self.game.players[p_id as usize].hand;
            let mut cards: Vec<Node<Msg>> = vec![];

            for card in p_hand.cards.iter() {
                let c = self.mk_card_div(card);
                cards.push(c)
            }
            div![
                attrs!{At::Class => "container"},
                p!["Hand"],
                div![ attrs!{At::Class => "hand"}, cards],
            ]
        };

        let table = {
            let mut entries: Vec<Node<Msg>> = vec![];
            for entry in self.game.table.entries.iter() {
                let e = self.mk_table_entry_div(entry);
                entries.push(e)
            }
            div![
                attrs!{At::Class => "container"},
                p!["Table"],
                div![ attrs!{At::Class => "table"},  entries ]
            ]
        };

        div![ table, hand ]
    }
}

/// Demultiplexers

enum Model {
    Init(InitSt),
    InGame(GameSt),
}


#[derive(Clone)]
enum Msg {
    Init(InitMsg),
    InGame(InGameMsg),
}

fn update(msg: Msg, mut model: &mut Model, _: &mut impl Orders<Msg>) {
    let upd_ret = match (&mut model, msg) {
        (&mut Model::Init(st), Msg::Init(ref msg)) => st.update_state(msg),
        (&mut Model::InGame(st), Msg::InGame(ref msg)) => st.update_state(msg),
        _ => panic!("Invalid message for current state"),
    };

    if let Some(newmodel) = upd_ret {
        *model = newmodel;
    }
}

fn view(model: &Model) -> impl View<Msg> {
    match model {
        Model::Init(st) => st.view(),
        Model::InGame(st) => st.view(),
    }

    /*
    button![
        simple_ev(Ev::Click, Msg::Increment),
        format!("Hello, World × {}", model.val)
    ]
    */
}

#[wasm_bindgen(start)]
pub fn render() {
    App::builder(update, view).build_and_start();
}
