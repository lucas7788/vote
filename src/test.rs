use super::*;

use hexutil::{read_hex, to_hex};

#[test]
fn test() {
    let bs = read_hex(
        "0010010000000020000000bbab28299c699e57db9113d17be63b1f70da58d100100538c35065e69bcda876",
    )
    .unwrap_or_default();

    let mut parser = VmValueParser::new(bs.as_slice());
    let topics: Vec<Vec<u8>> = parser.read().unwrap();
    let mut hash_list = Vec::with_capacity(topics.len());
    for topic_hash in topics.iter() {
        let h: H256 = unsafe { *(topic_hash.as_ptr() as *const H256) };
        println!("hex:{}", to_hex(topic_hash.as_slice()));
        hash_list.push(h);
    }
}
