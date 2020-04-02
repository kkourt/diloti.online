//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use serde::{Deserialize, Serialize};
use super::card::Card;


/// Player identifier based on their position on the table
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlayerTpos(pub u8);

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct Declaration {
    /// groups of cards
    pub cards: Vec<Vec<Card>>,
    /// Last player that made this declaration
    pub player: PlayerTpos,
}

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TableEntry {
    Card(Card),
    Decl(Declaration),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Table {
    pub entries: Vec<TableEntry>,
}

impl Declaration {

    pub fn cards(&self) -> &Vec<Vec<Card>> {
        &self.cards
    }

    pub fn player(&self) -> PlayerTpos {
        self.player
    }

    pub fn value(&self) -> u8 {
        assert!(self.cards.len() > 0);
        self.cards[0].iter().fold(0, |acc, x| acc + x.rank.0)
    }

    pub fn is_group(&self) -> bool {
        self.cards.len() > 1
    }

    pub fn is_plain(&self) -> bool {
        self.cards.len() == 1
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

    pub fn merge_table_entry(&mut self, te: TableEntry) {
        assert!(self.value() == te.value());
        match te {
            TableEntry::Card(c) => self.cards.push(vec![c]),
            TableEntry::Decl(mut d) => {
                for vcards in d.cards.drain(..) {
                    self.cards.push(vcards);
                }
            }
        }
    }
}

impl TableEntry {
    pub fn unwrap_card(self) -> Card {
        match self {
            TableEntry::Card(c) => c,
            TableEntry::Decl(_) => panic!("unwrap_card() called on a Declaration"),
        }
    }

    pub fn is_card(&self) -> bool {
        match self {
            TableEntry::Card(_) => true,
            TableEntry::Decl(_) => false,
        }
    }

    pub fn is_decl(&self) -> bool {
        match self {
            TableEntry::Decl(_) => true,
            TableEntry::Card(_) => false,
        }
    }

    pub fn unwrap_decl(self) -> Declaration {
        match self {
            TableEntry::Decl(d) => d,
            TableEntry::Card(_) => panic!("unwrap_decl() called on a Card"),
        }
    }

    pub fn ref_decl(&self) -> &Declaration {
        match self {
            TableEntry::Decl(d) => d,
            TableEntry::Card(_) => panic!("ref_decl() called on a Card"),
        }
    }

    pub fn value(&self) -> u8 {
        match self {
            TableEntry::Decl(d) => d.value(),
            TableEntry::Card(c) => c.rank.0,
        }
    }

   pub fn card_or_else<E, F: FnOnce() -> E>(self, errfn: F) -> Result<Card, E> {
        match self {
            TableEntry::Card(c) => Ok(c),
            TableEntry::Decl(_) => Err(errfn()),
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

    pub fn find_decl_from(&self, tpos: PlayerTpos) -> Option<&Declaration> {
        self.entries.iter().find_map(|e| {
            match e {
                TableEntry::Decl(d) if d.player == tpos => Some(d),
                _ => None,
            }
        })
    }

    pub fn remove_card_with_value(&mut self, val: u8) -> Option<Card> {
        let pos = self.entries.iter().position(|x| x.is_card() && x.value() == val)?;
        Some(self.entries.remove(pos).unwrap_card())
    }

    /// remove the first entry found with the given value, or return None if value does not exist
    pub fn remove_entry_with_value(&mut self, val: u8) -> Option<TableEntry> {
        let pos = self.entries.iter().position(|x| x.value() == val)?;
        Some(self.entries.remove(pos))
    }
}

impl std::fmt::Display for PlayerTpos {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let PlayerTpos(pid) = *self;
        write!(f, "P{}", pid)
    }
}
