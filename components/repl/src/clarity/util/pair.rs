/*
 copyright: (c) 2013-2018 by Blockstack PBC, a public benefit corporation.

 This file is part of Blockstack.

 Blockstack is free software. You may redistribute or modify
 it under the terms of the GNU General Public License as published by
 the Free Software Foundation, either version 3 of the License or
 (at your option) any later version.

 Blockstack is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY, including without the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 GNU General Public License for more details.

 You should have received a copy of the GNU General Public License
 along with Blockstack. If not, see <http://www.gnu.org/licenses/>.
*/

// Borrowed from Andrew Poelstra's rust-bitcoin library
/// An iterator that returns pairs of elements
pub struct Pair<I>
where
    I: Iterator,
{
    iter: I,
    last_elem: Option<I::Item>,
}

impl<I: Iterator> Iterator for Pair<I> {
    type Item = (I::Item, I::Item);

    #[inline]
    fn next(&mut self) -> Option<(I::Item, I::Item)> {
        let elem1 = self.iter.next();
        if elem1.is_none() {
            None
        } else {
            let elem2 = self.iter.next();
            if elem2.is_none() {
                self.last_elem = elem1;
                None
            } else {
                Some((elem1.unwrap(), elem2.unwrap()))
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.iter.size_hint() {
            (n, None) => (n / 2, None),
            (n, Some(m)) => (n / 2, Some(m / 2)),
        }
    }
}

impl<I: Iterator> Pair<I> {
    /// Returns the last element of the iterator if there were an odd
    /// number of elements remaining before it was Pair-ified.
    #[inline]
    pub fn remainder(self) -> Option<I::Item> {
        self.last_elem
    }
}

/// Returns an iterator that returns elements of the original iterator 2 at a time
pub trait Pairable: Sized + Iterator {
    /// Returns an iterator that returns elements of the original iterator 2 at a time
    fn pair(self) -> Pair<Self>;
}

impl<I: Iterator> Pairable for I {
    /// Creates an iterator that yields pairs of elements from the underlying
    /// iterator, yielding `None` when there are fewer than two elements to
    /// return.
    #[inline]
    fn pair(self) -> Pair<I> {
        Pair {
            iter: self,
            last_elem: None,
        }
    }
}
