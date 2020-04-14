//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use seed::{*, prelude::*,};


use core::srvcli::PlayerTpos;
use crate::Msg;

pub trait ToElem {
    fn to_elem(&self) -> Node<Msg>;
}

pub fn iter_to_elem<T: ToElem, I: Iterator<Item=T>>(start: &str, iter: I, end: &str) -> Node<Msg> {
    iter_to_elem_sep(start, iter, " ", end)
}

pub fn iter_to_elem_sep<T: ToElem, I: Iterator<Item=T>>(start: &str, mut iter: I, sep: &str, end: &str) -> Node<Msg> {
    let mut span = span![span![start]];
        let i0 = iter.next();
        if let Some(x0) = i0 {
            span.add_child(x0.to_elem());
            for x in iter {
                span.add_child(span![sep]);
                span.add_child(x.to_elem());
            }
        }
        span.add_child(span![end]);
        span
}

impl ToElem for core::Card {
    fn to_elem(&self) -> Node<Msg> {
        let span = if self.suit.is_red() {
            span![ style!{"color" => "red"}, format!("{}", self) ]
        } else {
            span![ style!{"color" => "black"}, format!("{}", self) ]
        };

        span
    }
}

impl ToElem for core::Deck {
    fn to_elem(&self) -> Node<Msg> {
        let mut span = span![span!["["]];
        for (i,card) in self.cards.iter().enumerate() {
            span.add_child(card.to_elem());
            if i + 1 < self.cards.len() {
                span.add_child(span![" "]);
            }
        }
        span.add_child(span!["]"]);

        span
    }
}

impl ToElem for core::TableEntry {
    fn to_elem(&self) -> Node<Msg> {
        match self {
            core::TableEntry::Card(c) => c.to_elem(),
            core::TableEntry::Decl(d) => {
                span![ style!{"color" => "blue"}, format!("\u{2605}{}", d.value()) ]
            },
        }
    }
}

impl ToElem for core::DeclActionBuilder {
    fn to_elem(&self) -> Node<Msg> {
        let mut span = span![];

        if self.current.len() > 0 {
            let curr = span![
                // NB: Can we avoid the cloned here?
                iter_to_elem("→", self.current.iter().cloned(), ""),
                " "
            ];
            span.add_child(curr);
        }

        if self.action.tentries.len() > 0 {
            for tev in self.action.tentries.iter() {
                // NB: Can we avoid the cloned here?
                let e = iter_to_elem(" (", tev.iter().cloned(), ") ");
                span.add_child(e);
            }
        }

        span
    }
}

impl ToElem for core::CaptureActionBuilder {
    fn to_elem(&self) -> Node<Msg> {
        let mut span = span![];

        if self.current.len() > 0 {
            let curr = span![
                // NB: Can we avoid the cloned here?
                iter_to_elem("→", self.current.iter().cloned(), ""),
                " "
            ];
            span.add_child(curr);
        }

        if self.action.tentries.len() > 0 {
            for tev in self.action.tentries.iter() {
                // NB: Can we avoid the cloned here?
                let e = iter_to_elem(" (", tev.iter().cloned(), ") ");
                span.add_child(e);
            }
        }

        span
    }
}

impl ToElem for core::ScoreSheet {
    fn to_elem(&self) -> Node<Msg> {
        let details = if self.score == 0 { span![""] } else {
            let mut nodes: Vec<Vec<Node<Msg>>> = vec![];
            if self.has_the_cards() {
                nodes.push(
                    vec![
                        span![format!("{} (cards: {})", core::scoring::NCARDS_SCORE, self.nr_cards)]
                    ]
                )
            }

            if self.nr_xeres > 0 {
                nodes.push(
                    vec![
                        span![format!("{} ({} {})",
                            self.nr_xeres*core::scoring::XERI_SCORE,
                            self.nr_xeres,
                            if self.nr_xeres == 1 {"ξερή"} else {"ξερές"},

                        )]
                    ]
                )
            }

            for c in self.score_cards.iter() {
                nodes.push(
                    vec![
                        span![format!("{} (", core::scoring::card_value(c))],
                        c.to_elem(),
                        span![")"],
                    ]
                )
            }

            let mut span = span![];
            let sep = vec![span![" + "]];
            let mut joined = (&nodes[..]).join(&sep[..]);
            for n in joined.drain(..) {
                span.add_child(n);
            }
            span
        };

        span![details]
    }
}

/*
fn tpos_char(tpos: srvcli::PlayerTpos) -> char {
    match tpos.0 {
        0 => '\u{278A}',
        1 => '\u{278B}',
        2 => '\u{278C}',
        3 => '\u{278D}',
        _ => panic!("Invalid tpos: {:?}", tpos),
    }
}
*/

pub fn tpos_char(tpos: PlayerTpos) -> char {
    match tpos.0 {
        0 => '\u{278A}', // black (1)
        1 => '\u{2781}', // white (2)
        2 => '\u{278C}', // black (3)
        3 => '\u{2783}', // white (4)
        _ => panic!("Invalid tpos: {:?}", tpos),
    }
}

