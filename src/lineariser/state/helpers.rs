//! Private helpers to interactive with the lineariser state.

use crate::lineariser::state::LState;
use crate::utils::SingleUse;

impl LState {
    /// Increment the id and return the one that can be used.
    ///
    /// This function ensures that every id is unique.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "todo: fail when no more ids available"
    )]
    pub(super) const fn get_and_bump_symbol_id(&mut self) -> SingleUse<usize> {
        let old = self.next_symbol_id;
        self.next_symbol_id += 1;
        SingleUse::from(old)
    }

    /// Resets the symbol id to the given value.
    pub(super) const fn reset_symbol_id(&mut self, value: SingleUse<usize>) {
        if let Some(old) = value.try_into_value() {
            self.next_symbol_id = old;
        }
    }
}
