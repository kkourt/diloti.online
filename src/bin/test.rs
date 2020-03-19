use core::game::{Game,PlayerGameView};

fn pr_view(v: &PlayerGameView) {
    println!("Player: {}", v.pid);
    println!("Player's hand: {}", v.own_hand);
    println!("Table: {}", v.table);
}


fn main() {
    println!("Δηλωτή...");

    let rng = rand::thread_rng();
    let game = Game::new_2p(rng);
    loop {
        let turn = game.start_player_turn();
        pr_view(&turn.game_view);

        //
        // TODO: input from player
        // ...
        let _act = unimplemented!();

        /*
        match game.end_player_turn(turn, _act) {
            Err(_x) => (),
            Ok(GameState::NextPlayer(_)) => (),
            Ok(GameState::RoundDone) => (),
            Ok(GameState::GameDone) => (),
        }
        */
    }
}
