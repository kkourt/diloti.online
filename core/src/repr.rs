//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:


// string representation for Table
// Each card is two characters:
//   - first suit (S, C, H, D)
//   - then rank (1-9,T,J,Q,K, and A also works)

// A declaration is 1:[ SR SR SR ][ SR SR ]:
//                  |
//                  ------------- player id

use std::convert::TryFrom;

use super::deck::Deck;
use super::card::Card;
use super::table::{Table, TableEntry, PlayerTpos, Declaration};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeckRepr(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeclRepr(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableEntryRepr(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableRepr(pub String);



/**
 * Deck
 */

impl DeckRepr {
    pub fn new<T: Into<String>>(s: T) -> Self {
        Self(s.into())
    }

    pub fn parse(&self) -> Option<Deck> {
        let iter = self.0.split_whitespace();
        parse_deck(iter)
    }

    pub fn fmt_deck(deck: &Deck, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut sep = "";
        for card in deck.cards.iter() {
            f.write_str(sep)?;
            sep = " ";
            write!(f, "{}", card)?;
        }

        Ok(())
    }
}

fn parse_deck<'a, I>(mut iter: I) -> Option<Deck> where
    I: Iterator<Item=&'a str>,
{
    let mut cards = vec![];

    while let Some(tok) = iter.next() {
        if let Some(card) = Card::try_from(tok).ok() {
            cards.push(card);
        } else {
            return None;
        }
    }

    Some(Deck {
        cards: cards,
    })
}


/**
 * Declaration
 */
impl DeclRepr {
    pub fn new<T: Into<String>>(s: T) -> Self {
        Self(s.into())
    }

    pub fn parse(&self) -> Option<Declaration> {
        let iter = self.0.split_whitespace();
        parse_decl(iter)
    }

    pub fn fmt_declaration(decl: &Declaration, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:", decl.player.0)?;
        let cards_i = decl.cards.iter().peekable();
        for vcards in decl.cards.iter() {
            write!(f, "[")?;
            for card in vcards.iter() {
                write!(f, " {}", card)?;
            }
            write!(f, " ]")?;
        }
        write!(f, ":")
    }
}

fn parse_decl_begin(s: &str) -> Option<PlayerTpos> {
    match s {
        "0:[" => Some(PlayerTpos(0)),
        "1:[" => Some(PlayerTpos(1)),
        "2:[" => Some(PlayerTpos(2)),
        "3:[" => Some(PlayerTpos(3)),
        _ => None,
    }
}

// NB: Yes, this is ugly! I first implemented the parsers as a struct, and then because it looked
// too complex, I implemented them as functions.  At least for the decls, it seems that the best
// option would be the struct, so we don't have to pass the iterator around, but I'll keep it like
// this for now.
fn parse_decl_body<'a, I>(mut iter: I) -> (I,Option<Vec<Vec<Card>>>) where
    I: Iterator<Item=&'a str>,
{
    let mut cards_vec = vec![];
    let mut cards = vec![];
    while let Some(tok) = iter.next() {
        match tok {
            "][" => cards_vec.push(cards.drain(..).collect()),
            "]:" => {
                cards_vec.push(cards.drain(..).collect());
                return (iter, Some(cards_vec));
            },
            x => {
                let card = match Card::try_from(x) {
                    Ok(x) => x,
                    Err(_) => return (iter, None),
                };
                cards.push(card)
            },
        }
    }
    (iter, None)
}

fn parse_decl<'a, I>(mut iter: I) -> Option<Declaration> where
    I: Iterator<Item=&'a str>,
{
    let tok = iter.next()?;
    let tpos = parse_decl_begin(tok)?;
    let cards = parse_decl_body(iter).1?;

    Some(Declaration {
        player: tpos,
        cards: cards,
    })
}

impl TableEntryRepr {
    pub fn new<T: Into<String>>(s: T) -> Self {
        Self(s.into())
    }

    pub fn parse(&self) -> Option<TableEntry> {
        let iter = self.0.split_whitespace();
        parse_table_entry(iter).1
    }

    pub fn fmt_table_entry(entry: &TableEntry, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match entry {
            TableEntry::Card(c) => write!(f, "{}", c),
            TableEntry::Decl(d) => DeclRepr::fmt_declaration(d, f),
        }
    }
}

/**
 * Table
 */

impl TableRepr {
    pub fn new<T: Into<String>>(s: T) -> Self {
        Self(s.into())
    }

    pub fn parse(&self) -> Option<Table> {
        let iter = self.0.split_whitespace();
        parse_table(iter)
    }

    pub fn fmt_table(table: &Table, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut sep = "";
        for entry in table.entries.iter() {
            f.write_str(sep)?;
            sep = " ";
            TableEntryRepr::fmt_table_entry(&entry, f)?;
        }

        Ok(())
    }
}

fn parse_table_entry<'a, I>(mut iter: I) -> (I,Option<TableEntry>) where
    I: Iterator<Item=&'a str>,
{
    let tok = match iter.next() {
        None => return (iter, None),
        Some(x) => x,
    };

    let res = if let Some(card) = Card::try_from(tok).ok() {
        Some(TableEntry::Card(card))
    } else if let Some(tpos) = parse_decl_begin(tok) {
        let ret = parse_decl_body(iter);
        iter = ret.0;
        let cards = match ret.1 {
            None => return (iter, None),
            Some(x) => x,
        };
        Some(TableEntry::Decl(Declaration{
            cards: cards,
            player: tpos,
        }))
    } else {
        None
    };

    (iter, res)
}

fn parse_table<'a, I>(iter: I) -> Option<Table> where
    I: Iterator<Item=&'a str>,
{
    let mut entries = vec![];
    let mut iter_peek = iter.peekable();

    while iter_peek.peek().is_some() {
        let ret = parse_table_entry(iter_peek);
        iter_peek = ret.0;
        let te = ret.1?;
        entries.push(te);
    }

    Some(Table {
        entries: entries,
    })
}

/**
 * Implementation of std::fmt::Display traits
 */

impl std::fmt::Display for Deck {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        DeckRepr::fmt_deck(self, f)
    }
}

impl std::fmt::Debug for TableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        TableEntryRepr::fmt_table_entry(self, f)
    }
}

impl std::fmt::Display for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        TableRepr::fmt_table(self, f)
    }
}
