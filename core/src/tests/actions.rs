//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use crate::{
    repr::{TableRepr, TableEntryRepr, DeckRepr},
    actions::{DeclAction, PlayerAction},
    table::{Table, TableEntry, PlayerTpos},
    game::{Game},
};


pub fn mk_te(s: &str) -> TableEntry {
    TableEntryRepr::new(s).parse().unwrap()
}

pub fn mk_table(s: &str) -> Table {
    TableRepr::new(s).parse().unwrap()
}

#[test]
fn act_t0() {
    let rng = rand::thread_rng(); // not going to be used
    let table_ = TableRepr::new("S4 HT H9").parse().unwrap();
    let hand_ = DeckRepr::new("D5 D9 C3 C9").parse().unwrap();
    let tpos = PlayerTpos(0);
    let game = Game::new_1p_debug(rng, table_, hand_);
    let hand = &game.players[0].hand;
    let table = &game.table;

    println!("hand: {}", hand);
    println!("table: {}", table);

    {
        let decl_act = DeclAction {
            tentries: vec![vec![mk_te("C9")]],
        };
        println!("action: {:?} is be invalid (no cards from table)", decl_act);
        assert!(decl_act.validate_decl(&table, &hand).is_err());
    }

    {
        let decl_act = PlayerAction::Declare(DeclAction {
            tentries: vec![vec![mk_te("♦5"), mk_te("♠4")]],
        });
        let game = game.apply_action(tpos, decl_act).unwrap();
        println!("{}", mk_table("♥T 0:[ ♦5 ♠4 ][ H9 ]:"));
        // NB: equality is based on order, so we might just need to rearrange the result
        assert_eq!(game.table.entries, mk_table("♥T 0:[ ♦5 ♠4 ][ ♥9 ]:").entries);
    }

}

#[test]
fn no_two_decls() {
    let rng = rand::thread_rng(); // not going to be used
    let table_ = TableRepr::new("S2 S5").parse().unwrap();
    let hand_ = DeckRepr::new("D2 H2 D5 H5").parse().unwrap();
    let tpos = PlayerTpos(0);
    let game = Game::new_1p_debug(rng, table_, hand_);
    let hand = &game.players[0].hand;
    let table = &game.table;

    let act1 = PlayerAction::Declare(DeclAction {
        tentries: vec![
            vec![mk_te("D2")],
            vec![mk_te("S2")],
        ],
    });
    println!("hand: {}", hand);
    println!("table: {}", table);
    println!("act1: {:?}", act1);
    let game = game.apply_action(tpos, act1).unwrap();

    let act2 = PlayerAction::Declare(DeclAction {
        tentries: vec![
            vec![mk_te("D5")],
            vec![mk_te("S5")],
        ],
    });
    println!("act2: {:?}", act2);
    let res = game.apply_action(tpos, act2);
    assert!(res.is_err());
    println!("table: {:?}", res);
}
