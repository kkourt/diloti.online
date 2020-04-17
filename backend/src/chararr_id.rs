//
// Kornilios Kourtis <kkourt@kkourt.io>
//
// vim: set expandtab softtabstop=4 tabstop=4 shiftwidth=4:
//

use rand::{Rng, distributions::Alphanumeric};

/// Macro to define char array ids.

macro_rules! define_chararr_id {
    ($t:ident, $l:expr) => {

        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub struct $t(pub [char; $l]);

        impl $t {
            pub fn len() -> usize { $l }

            pub fn new_random() -> Self {
                // NB: we could use the MaybeUninitialized stuff to avoid initalization, but I
                // think it's a bit too much
                let mut rarr: [char; $l] = ['x'; $l];
                let iter = rand::thread_rng().sample_iter(&Alphanumeric) .take(rarr.len());
                for (i, c) in iter.enumerate() {
                    rarr[i] = c;
                }

                Self(rarr)
            }

            pub fn from_string(s: &str) -> Option<Self> {
                if s.len() != $l {
                    return None
                }

                // NB: we could use the MaybeUninitialized stuff to avoid initalization, but I
                // think it's a bit too much
                let mut arr: [char; $l] = ['y'; $l];
                for (i,c) in s.chars().enumerate() {
                    arr[i] = c;
                }

                Some(Self(arr))
            }

            pub fn to_string(&self) -> String {
                self.0.iter().cloned().collect::<String>()
            }
        }
    };
}

// NB: there used to be two different types of these ids, hence the macro
define_chararr_id!(GameId, 16);
