use crate::types::Hostname;
use crate::util::{self, split_to_sorted_vec};

#[test]
fn test_split_empty_str() {
    let t: Vec<Hostname> = split_to_sorted_vec("");
    assert_eq!(t, Vec::<Hostname>::new());
}
