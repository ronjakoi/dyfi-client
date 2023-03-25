// Dyfi-client, a dynamic DNS updater for the dy.fi service.
// Copyright (C) 2020-2023  Ronja Koistinen

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::HashSet;

// Joins a HashSet into a String without first collecting the set into a Vec
// or using the itertools crate
fn _join_set(set: &HashSet<String>, separator: char) -> String {
    let out_size = set.iter().map(|s| s.len() + 1).sum();
    let mut out = String::with_capacity(out_size);
    let mut set_iter = set.iter();
    if let Some(first) = set_iter.next() {
        out.push_str(first);
    }
    for s in set_iter {
        out.push(separator);
        out.push_str(s);
    }
    out
}

pub fn split_to_sorted_vec(s: &str) -> Vec<String> {
    if s.is_empty() {
        return vec![];
    }
    let mut out: Vec<_> =
        s.split(',').map(std::string::ToString::to_string).collect();
    out.sort();
    out
}
