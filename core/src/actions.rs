//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use serde::{Deserialize, Serialize};

use super::card::Card;
use super::deck::Deck;
use super::table::{Table, TableEntry, Declaration, PlayerTpos};
use super::game::{PlayerGameView};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerAction {
    LayDown(Card),
    Declare(DeclAction),
    Capture(CaptureAction),
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformedAction {
    pub action: PlayerAction,
    pub player: PlayerTpos,
    pub forced_cards: Vec<Card>,
    pub xeri: bool,
}

// NB: by convention, the first card is the handcard stored as a TableEntry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclAction {
    pub tentries: Vec<Vec<TableEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureAction {
    pub handcard: Card,
    pub tentries: Vec<Vec<TableEntry>>,
}

#[derive(Debug, Clone)]
pub enum GetSingleRes<T> {
    Zero,
    OnlyOne(T),
    MoreThanOne,
}


impl DeclAction {

    // by convention, the first card is the handcard stored as a TableEntry
    pub fn handcard(&self) -> Card {
        self.tentries[0][0].clone().unwrap_card()
    }

    /// value of the first group
    pub fn value(&self) -> u8 {
        self.tentries[0].iter().fold(0, |acc, entry| acc + entry.value())
    }

    /// Returns true of all groups have the same value
    pub fn same_value(&self) -> bool {
        let val = self.value();
        for te_vec in self.tentries[1..].iter() {
            let val_g = te_vec.iter().fold(0, |acc, te| acc + te.value());
            if val != val_g {
                return false;
            }
        }

        return true;
    }

    /// Is the value bounded to 1-10?
    pub fn validate_bounded(&self) -> Result<(), String> {
        let val = self.value();
        if val > 0 && val <= 10 {
            Ok(())
        } else {
            Err(format!("Invalid delaration: invalid value: {}", val))
        }
    }

    pub fn has_decl(&self) -> bool {
        self.tentries
            .iter()
            .flatten()
            .find(|te| te.is_decl()).is_some()
    }

    pub fn get_single_decl(&self) -> GetSingleRes<&Declaration> {
        let tentries: Vec<&TableEntry> = self.tentries
            .iter()
            .flatten()
            .filter(|x| x.is_decl())
            .take(2)
            .collect();

        match tentries.len() {
            0 => GetSingleRes::Zero,
            1 => GetSingleRes::OnlyOne(tentries[0].ref_decl()),
            2 => GetSingleRes::MoreThanOne,
            _ => panic!("Unexpected"),
        }
    }

    pub fn get_decl(&self) -> Option<Declaration> {
        match self.get_single_decl() {
            GetSingleRes::Zero => None,
            GetSingleRes::OnlyOne(d) => Some(d.clone()),
            GetSingleRes::MoreThanOne => panic!("Invalid decl"),
        }
    }

    pub fn validate_decl_base(&self, hand: &Deck) -> Result<(), String> {
        if (self.tentries.len() == 0) || (self.tentries[0].len() == 0) {
            return Err("Invalid declaration: empty".to_string());
        }

        if !self.tentries[0][0].is_card() {
            return Err("First entry is not a card".to_string());
        }

        if !self.same_value() {
            return Err("Invalid declaration: Not all groups have the same value".to_string());
        }

        if self.tentries.iter().flatten().nth(1).is_none() {
            return Err("Invalid declaration: Need more than one cards".to_string());
        }

        self.validate_bounded()?;

        let value = self.value();
        let value_in_hand = hand.cards
            .iter()
            .find(|c| **c != self.handcard() && c.rank.0 == value)
            .is_some();

        if !value_in_hand {
            return Err(format!("Invalid declaration: There is no value {} card in your hand", value));
        }

        Ok(())
    }

    pub fn validate_decl(&self, _table: &Table, hand: &Deck) -> Result<(), String> {

        self.validate_decl_base(hand)?;

        match self.get_single_decl() {
            GetSingleRes::Zero => (),
            GetSingleRes::OnlyOne(d) if d.value() != self.value() && d.is_group() => return Err("You may not raise group declarations".to_string()),
            GetSingleRes::MoreThanOne => return Err("You may not combine more than one declaration to form a new one".to_string()),
            _ => (),
        }

        Ok(())
    }

    // - users can only add to their declaration (but not raise)
    pub fn validate_decl_continuation(&self, decl: &Declaration, _table: &Table, hand: &Deck) -> Result<(), String> {

        self.validate_decl_base(hand)?;

        match self.get_single_decl() {
            GetSingleRes::Zero => return Err("You cannot create a new declaration if you have one on the table".to_string()),
            GetSingleRes::OnlyOne(d) if d != decl => return Err("You cannot act on a declaration other than your lastest one".to_string()),
            GetSingleRes::OnlyOne(d) if d.value() != self.value() => return Err("You may not raise your declaration".to_string()),
            GetSingleRes::MoreThanOne => return Err("You may not combine more than one declaration to form a new one".to_string()),
            _ => (),
        }

        Ok(())
    }
}

fn validate_laydown(card: Card, table: &Table) -> Result<(), String> {
    let matching_card = table.entries
        .iter()
        .find(|te| te.value() == card.rank.0)
        .is_some();

        if matching_card {
            return Err(format!("You cannot lay down a card ({}) if a card or declaration with the same value exists on the table.", card));
        }

        Ok(())
}

impl CaptureAction {
    pub fn validate_capture(&self, table: &Table) -> Result<(), String> {
        if (self.tentries.len() == 0) || (self.tentries[0].len() == 0) {
            return Err("Invalid capture: empty".to_string());
        }

        if !self.same_value() {
            return Err("Invalid capture: Not all groups have the same value".to_string());
        }

        for tvec in self.tentries.iter() {
            if tvec.iter().find(|x| x.is_decl()).is_some()  && tvec.len() < 1 {
                return Err("Invalid capture: declarations can only be captured on their own".to_string());
            }
        }

        if self.handcard.rank.is_figure() {
            for te_vec in self.tentries.iter() {
                if te_vec.len() > 1 {
                    return Err("Invalid capture: Figures cannot be used to capture multiple cards".to_string());
                }
            }

            let ncaptured = self.tentries.len();
            let ntable = table.iter_cards_with_val(self.value()).count();
            match (ncaptured, ntable) {
                (1, 1) => (),
                (1, 2) => (),
                (2, 2) => return Err("invalid capture: if 2 same figures exist on the table, only one can be captured.".to_string()),
                // For the two cases below, we could force the action as we do with other
                // obligations, but this is tricky and also counter-intuitive to the player because
                // it's a very special case, so we just invalidate the action.
                (1, 3) => return Err("invalid capture: if 3 same figures exist on the table, all three must be captured.".to_string()),
                (2, 3) => return Err("invalid capture: if 3 same figures exist on the table, all three must be captured.".to_string()),
                (3, 3) => (),
                _ => return Err("Huh?".to_string()),
            }

        }

        Ok(())
    }

    pub fn value(&self) -> u8 {
        self.handcard.rank.0
    }

    pub fn same_value(&self) -> bool {
        let val = self.value();
        for te_vec in self.tentries.iter() {
            let val_g = te_vec.iter().fold(0, |acc, te| acc + te.value());
            if val != val_g {
                return false;
            }
        }

        return true;
    }

    // NB: Build an iterator for this...
    pub fn get_table_cards(&self) -> Vec<Card> {
        let mut ret = vec![];
        let iter = self.tentries.iter().flatten();
        for te in iter {
            match te {
                TableEntry::Card(c) => ret.push(c.clone()),
                TableEntry::Decl(d) => {
                    for c in d.cards.iter().flatten() {
                        ret.push(c.clone())
                    }
                }
            }
        }

        ret
    }
}


impl PlayerAction {

    /// Validate action given a player's view
    ///
    /// NB: This function does not check whether the referenced cards exist in the table. This will
    /// happen later.
    pub fn validate(&self, view: &PlayerGameView) -> Result<(), String> {
        use PlayerAction::*;
        if !view.is_my_turn() {
            return Err("Not this player's turn".into());
        }

        // RULE: if a user has made a declaration, they are only allowed to:
        // - capture (their declaration or otherwise)
        // - add to their declaration (but not raise)
        let player_decl = view.table.find_decl_from(view.pid);
        match (player_decl, self) {
            (None,    LayDown(c))  => validate_laydown(c.clone(), &view.table),
            (Some(_), LayDown(c))  => Err("Cannot lay down a card when a declaration of yours exists.".to_string()),
            (_,       Capture(ca)) => ca.validate_capture(&view.table),
            (None,    Declare(da)) => da.validate_decl(&view.table, &view.own_hand),
            (Some(d), Declare(da)) => da.validate_decl_continuation(&d, &view.table, &view.own_hand),
        }
    }
}

/**
 * Builders
 */


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclActionBuilder {
    pub value: u8,
    pub current: Vec<TableEntry>,
    pub entries_set: std::collections::HashSet<TableEntry>,
    pub action: DeclAction,
}

impl DeclActionBuilder {

    pub fn new(hcard: &Card, value: u8) -> Result<DeclActionBuilder, String> {
        if value < 1 || value > 10 {
            return Err(format!("Invalid value: {}", value))
        }

        let mut current = vec![];
        let mut tentries = vec![];

        let tentry = TableEntry::Card(hcard.clone());
        if hcard.rank.0 > value {
            return Err(format!("Rank of hand card {} is larger than the declaration value {}", hcard.rank.0, value));
        } else if hcard.rank.0 == value {
            tentries.push(vec![tentry]);
        } else {
            current.push(tentry);
        }

        let ret = DeclActionBuilder {
            value: value,
            current: current,
            entries_set: std::collections::HashSet::new(),
            action: DeclAction { tentries: tentries }
        };

        Ok(ret)
    }

    pub fn hand_card(&self) -> Card {
        if self.action.tentries.len() > 0 {
            self.action.tentries[0][0].clone().unwrap_card()
        } else {
            self.current[0].clone().unwrap_card()
        }
    }

    pub fn reset(&mut self) {
        *self = DeclActionBuilder::new(&self.hand_card(), self.value).unwrap();
    }

    pub fn has_decl(&self) -> bool {
        self.entries_set.iter().find(|te| te.is_decl()).is_some()
    }

    pub fn is_ready(&self) -> bool {
        if self.current.len() != 0 {
            return false;
        }

        // need at least one table entry
        if self.entries_set.len() == 0 {
            return false;
        }

        true
    }

    pub fn add_table_entry(&mut self, tentry: &TableEntry) -> Result<(), String> {
        let ret = self.do_add_table_entry(tentry);
        if ret.is_ok() {
            self.entries_set.insert(tentry.clone());
        }

        ret
    }

    fn do_add_table_entry(&mut self, tentry: &TableEntry) -> Result<(), String> {

        let current_value = self.current_value() + tentry.value();
        if current_value > self.value {
            return Err("Cannot add entry to current declaration (it will exceed declared value)".to_string());
        }

        match tentry {
            TableEntry::Decl(tdecl) if self.has_decl() =>
                Err("Cannot add more than two declarations".to_string()),
            TableEntry::Decl(tdecl) if tdecl.is_group() && (tdecl.value() != self.value) =>
                Err("Cannot add a group declaration to a raiase".to_string()),
            _ => Ok(()),
        }?;

        self.current.push(tentry.clone());
        assert_eq!(current_value, self.current_value());
        if current_value == self.value {
            let curr = self.current.drain(..).collect();
            self.action.tentries.push(curr);
        }

        Ok(())
    }

    pub fn has_tentry(&self, tentry: &TableEntry) -> bool {
        self.entries_set.contains(tentry)
    }

    fn current_value(&self) -> u8 {
        self.current.iter().fold(0, |acc, x| acc + x.value())
    }

    pub fn make_decl_action(&self) -> DeclAction {
        assert!(self.is_ready());
        self.action.clone()
    }

    pub fn make_action(&self) -> PlayerAction {
        PlayerAction::Declare(self.make_decl_action())
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureActionBuilder {
    pub action: CaptureAction,
    pub current: Vec<TableEntry>,
    pub entries_set: std::collections::HashSet<TableEntry>,
}

impl CaptureActionBuilder {

    pub fn new(hcard: &Card) -> CaptureActionBuilder {
        CaptureActionBuilder {
            action: CaptureAction {
                handcard: hcard.clone(),
                tentries: vec![],
            },
            current: vec![],
            entries_set: std::collections::HashSet::new(),
        }
    }

    pub fn reset(&mut self) {
        *self = CaptureActionBuilder::new(&self.action.handcard);
    }

    pub fn is_ready(&self) -> bool {
        if self.current.len() != 0 {
            return false;
        }

        // need at least one table entry
        if self.action.tentries.len() == 0 {
            return false;
        }

        true
    }

    fn current_value(&self) -> u8 {
        self.current.iter().fold(0, |acc, x| acc + x.value())
    }

    pub fn add_table_entry(&mut self, tentry: &TableEntry) -> Result<(), String> {
        let ret = self.do_add_table_entry(tentry);
        if ret.is_ok() {
            self.entries_set.insert(tentry.clone());
        }

        ret
    }

    pub fn has_tentry(&self, tentry: &TableEntry) -> bool {
        self.entries_set.contains(tentry)
    }

    fn do_add_table_entry(&mut self, tentry: &TableEntry) -> Result<(), String> {
        let handc = &self.action.handcard;
        let val = self.action.value();
        if handc.rank.is_figure() {
            match tentry {
                TableEntry::Card(c) if c.rank != handc.rank => Err(format!("You cannot capture {} with {}", c, handc)),
                TableEntry::Decl(_) => Err("Cannot capture declarations with a figure".to_string()),
                _ => Ok(())
            }?;
        }

        match tentry {

            TableEntry::Decl(tdecl) if self.current.len() == 0 && tdecl.value() == val => {
                self.action.tentries.push(vec![tentry.clone()]);
                Ok(())
            }

            TableEntry::Decl(tdecl) => {
                Err("Declarations must be picked up on their own".to_string())
            },

            TableEntry::Card(tdecl) => {
                let curr_val = self.current_value() + tentry.value();
                if curr_val > val {
                    Err("Cannot add entry to current declaration (it will exceed declared value)".to_string())
                } else {
                    self.current.push(tentry.clone());
                    assert_eq!(self.current_value(), curr_val);
                    if curr_val == val {
                        let curr = self.current.drain(..).collect();
                        self.action.tentries.push(curr);
                    }
                    Ok(())
                }
            }
        }
    }


    fn make_capture_action(&self) -> CaptureAction {
        assert!(self.is_ready());
        self.action.clone()
    }

    pub fn make_action(&self) -> PlayerAction {
        PlayerAction::Capture(self.make_capture_action())
    }
}
