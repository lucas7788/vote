use super::*;

use hexutil::{read_hex, to_hex};
use ostd::abi::VmValueBuilder;
use ostd::contract::governance::PeerPoolMap;
use ostd::mock::build_runtime;

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

#[test]
fn test_peer_pool_map() {
    let data = read_hex("0100000001000000000000000000000000000000000000000000000000000000000000000140420f0000000000400d030000000000").unwrap_or_default();
    let mut source = Source::new(data.as_slice());
    let peer_pool_map: PeerPoolMap = source.read().unwrap();

    let data = read_hex("070000000300000042000000000000003033653831386236356136366439383361393934393765303663363535326565353036373232396538356261316365633630633534373764633364353638656434337da44cd3f40927c0139ef006511c59efe49ef68902000000000000000000000000000000000200000042000000000000003033616664393230613362346365326537313735613332633064303932313533643161313165663565306463633134653731633835313031623935353138643564373afacc35e63616740daa47e7808b2798a6ad94b702000000000000000000000000000000000500000042000000000000003033616630343063303961663565303663663936366637336663393965386634333732663135313066653665343337363832343435326139396238353639356139637bd65775cb9ca7289d2aa401affefad654200dee020000000000000000000000000000000006000000420000000000000030333465653261343336386539393966633763303465376533613930373331363264343737313233383266313639306436613637653765316334373563643066663347f8e93645e7426a8a2c76ccd46b962181315e4d0200000000000000000000000000000000010000004200000000000000303333343863386665363465316465666234303836373662366533323030333862643265353932633830326532376333643765383865363832373030373663326637c35fdde6f8ae1a72fe1fe706840082a0f825809a020000000000000000000000000000000007000000420000000000000030333237663965306662336238393430323763353263616633643331643961633566363736643363663839326339333361633130376564373434376662366536356295a94d6323822d708bd7a7627850e9d85e93255a0200000000000000000000000000000000040000004200000000000000303233373565343465353030663963666538626432663461666134613031366138613930323536373939366339313962396431636534663564346639333066313435275a8a95344a5958effc3efc5ae72120a305c59b0200000000000000000000000000000000").unwrap_or_default();
    let mut source = Source::new(data.as_slice());
    let peer_pool_map: PeerPoolMap = source.read().unwrap();
}

#[test]
fn test_parse_topic_info() {
    let data = read_hex("00100a0000000014000000dca1305cc8fc2b3d3127a2c4849b43301545d84e000b000000746573745f7469746c6532000c000000746573745f64657461696c32100100000010020000000014000000dca1305cc8fc2b3d3127a2c4849b43301545d84e000200000010270004000000f055ec5e000400000090dced5e040000000000000000000000000000000004000000000000000000000000000000000401000000000000000000000000000000002000000056ff666d80219e1c2c81e95644a5911748096d37210873fa97523258906387a4").unwrap_or_default();
    let mut parser = VmValueParser::new(data.as_slice());
    let topic_info: TopicInfo = parser.list().unwrap();
}

#[test]
fn test_topic_info() {
    let data = read_hex("010334805df2a8b2fc6b07acbecbe2c44f59b2952c0b746f7069635f7469746c650d746f7069635f636f6e74656e740010ad045f0000000050ef135f0000000040420f0000000000000000000000000001fa53f8ef6f8270564df17791778a5fbc05b24f5d7e8203e7d72dc4a8551b2667").unwrap_or_default();
    let mut source = Source::new(data.as_slice());
    let boo: bool = source.read().unwrap();
    assert!(boo);
    let topic_info: TopicInfo = source.read().unwrap();
    println!("{}", topic_info.approve);
}

#[test]
fn test_voter_info() {
    let data = read_hex("6400100200000010020000000014000000961e12a400c10ddc9c88aa3b9fce5405152a6c470402000000000000000000000000000000100200000000140000005bf56926791d1da9b1992ca95adad31471f2a6810401000000000000000000000000000000").unwrap_or_default();
    let mut source = Source::new(data.as_slice());
    let da: Vec<u8> = source.read().unwrap();
    let mut parser = VmValueParser::new(da.as_slice());
    let info: Vec<VotedInfo> = parser.read().unwrap();
}

#[test]
fn test_topic() {
    let data =
        read_hex("1a0010020000000005000000666972737400050000006669727374").unwrap_or_default();
    let mut source = Source::new(data.as_slice());
    let da: Vec<u8> = source.read().unwrap();
    let mut parser = VmValueParser::new(da.as_slice());
    let info: Topic = parser.read().unwrap();
    println!("{}", to_hex(&info.topic_title).as_str());
    println!("{}", to_hex(&info.topic_detail).as_str());
}

#[test]
fn test_create_topic() {
    let admin = Address::repeat_byte(1);
    let topic_title = b"title";
    let topc_detail = b"detail";
    assert!(create_topic(admin, topic_title, topc_detail, 1, 4));
    let data = read_hex("ec58bd841665cfc687036fa7c537814aa96f2e4cc5071b5f1b312d75843bdd43")
        .unwrap_or_default();
    let hash = unsafe { *(data.as_ptr() as *const H256) };
    let topic = get_topic(&hash).unwrap();
    assert_eq!(topic.topic_title, b"title");
    let topic_info = get_topic_info(&hash).unwrap();
    assert_eq!(topic_info.topic_title, b"title");

    let voter = Address::repeat_byte(2);

    let handle = build_runtime();
    handle.timestamp(2);
    handle.witness(&[voter]);
    assert!(vote_topic(&hash, voter, true));
}
