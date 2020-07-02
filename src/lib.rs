#![cfg_attr(not(feature = "mock"), no_std)]
#![feature(proc_macro_hygiene)]
extern crate ontio_std as ostd;
use ostd::abi::{
    Decoder, Encoder, Error, EventBuilder, Sink, Source, VmValueDecoder, VmValueParser,
};
use ostd::contract::governance;
use ostd::contract::governance::{get_peer_info, get_peer_pool};
use ostd::contract::neo;
use ostd::database;
use ostd::macros::base58;
use ostd::prelude::*;
use ostd::runtime::{check_witness, contract_migrate, current_txhash, input, ret, timestamp};

const PRE_TOPIC: &[u8] = b"01";
const PRE_TOPIC_INFO: &[u8] = b"02";
const PRE_VOTED: &[u8] = b"03";
const PRE_TOPIC_HASH: &[u8] = b"04";
const KEY_CUR_HASH_NUM: &[u8] = b"05";

const ADMIN: Address = base58!("AbtTQJYKfQxq4UdygDsbLVjE8uRrJ2H3tP");

mod basic;
use basic::*;

#[cfg(test)]
mod test;
// test net AKzJGcCVr9wVEG95XvP3VnCDRivVjo391r
//local AQWrGrBb6yosjuHDiALkNwVnL9qLanCMdG
const NEO_VOTE_CONTRACT_ADDRESS: Address = base58!("AKzJGcCVr9wVEG95XvP3VnCDRivVjo391r");

/// upgrade contract, only admin has the right to invoke this method
fn migrate(
    code: &[u8],
    vm_ty: U128,
    name: &str,
    version: &str,
    author: &str,
    email: &str,
    desc: &str,
) -> bool {
    assert!(check_witness(&ADMIN));
    let addr = contract_migrate(code, vm_ty as u32, name, version, author, email, desc);
    assert_ne!(addr, Address::new([0u8; 20]));
    true
}

/// query all the consensus and candidate nodes address
fn list_gov_nodes() -> Vec<Address> {
    let peer_pool_map = get_peer_pool();
    let mut res: Vec<Address> = Vec::with_capacity(peer_pool_map.peer_pool_map.len());
    for item in peer_pool_map.peer_pool_map.iter() {
        res.push(item.peer_pubkey_addr);
    }
    res
}

fn get_timestamp() -> u64 {
    return timestamp();
}

/// create topic
/// all the consensus and candidate nodes have the right to create topic
///
fn create_topic(
    gov_node_addr: Address,
    topic_title: &[u8],
    topic_detail: &[u8],
    start_time: U128,
    end_time: U128,
) -> bool {
    assert!(check_witness(&gov_node_addr));
    assert!(is_gov_node(&gov_node_addr));
    assert!(start_time < end_time);
    let cur = timestamp() as U128;
    assert!(cur < end_time);

    let hash = current_txhash();
    let tc = Topic {
        topic_title: topic_title.to_vec(),
        topic_detail: topic_detail.to_vec(),
    };
    let key_topic = get_key(PRE_TOPIC, hash.as_ref());
    database::put(key_topic, tc);
    let key_topic_info = get_key(PRE_TOPIC_INFO, hash.as_ref());
    let info = TopicInfo {
        gov_node_addr,
        topic_title: topic_title.to_vec(),
        topic_detail: topic_detail.to_vec(),
        voters: vec![],
        start_time: start_time as u64,
        end_time: end_time as u64,
        approve: 0,
        reject: 0,
        status: 1,
        hash: hash.clone(),
    };
    database::put(key_topic_info, info);
    let next_hash_key = get_current_hash_num();
    database::put(
        get_key(PRE_TOPIC_HASH, next_hash_key.to_string().as_bytes()),
        &hash,
    );
    database::put(KEY_CUR_HASH_NUM, next_hash_key + 1);
    EventBuilder::new()
        .string("createTopic")
        .h256(&hash)
        .bytearray(topic_title)
        .bytearray(topic_detail)
        .notify();
    true
}

fn get_all_topic_hash_inner() -> Vec<H256> {
    let num = get_current_hash_num();
    let mut res: Vec<H256> = Vec::with_capacity(num as usize);
    for i in 0..num {
        let h = get_hash_by_num(i);
        res.push(h);
    }
    res
}

fn get_hash_by_num(i: u32) -> H256 {
    database::get::<_, H256>(get_key(PRE_TOPIC_HASH, i.to_string().as_bytes())).unwrap()
}

fn get_current_hash_num() -> u32 {
    database::get::<_, u32>(KEY_CUR_HASH_NUM).unwrap_or(0)
}

/// query topic
fn get_topic(hash: &H256) -> Option<Topic> {
    let key = get_key(PRE_TOPIC, hash.as_ref());
    let res = database::get::<_, Topic>(key);
    if let Some(temp) = res {
        return Some(temp);
    } else {
        let topic = neo::call_contract(
            &NEO_VOTE_CONTRACT_ADDRESS,
            ("getTopic", (hash.as_ref() as &[u8],)),
        );
        if let Some(old_topic) = topic {
            let mut parser = VmValueParser::new(old_topic.as_slice());
            let r: Option<Topic> = parser.read().ok();
            r
        } else {
            None
        }
    }
}

fn get_topic_bytes(hash: &H256) -> Vec<u8> {
    let topic = neo::call_contract(
        &NEO_VOTE_CONTRACT_ADDRESS,
        ("getTopic", (hash.as_ref() as &[u8],)),
    );
    if let Some(old_topic) = topic {
        old_topic
    } else {
        vec![]
    }
}

/// cancel topic
/// only the creator of the topic has the right to invoke
fn cancel_topic(hash: &H256) -> bool {
    let topic_info = get_topic_info(hash);
    if let Some(mut info) = topic_info {
        assert_eq!(info.status, 1);
        let cur = timestamp();
        assert!(cur < info.end_time);
        assert!(check_witness(&info.gov_node_addr));
        info.status = 0;
        let key = get_key(PRE_TOPIC_INFO, hash.as_ref());
        database::put(key, info);
    } else {
        panic!("the topic does not exist")
    }
    true
}

/// approve or reject a topic
/// only the consensus and candidate nodes have the right to invoke
/// approve_or_reject is true indicate approve, false indicate reject
fn vote_topic(hash: &H256, voter: Address, approve_or_reject: bool) -> bool {
    assert!(check_witness(&voter));
    assert!(is_gov_node(&voter));
    let info = get_topic_info(hash).expect("not exist topic info");
    assert!(info.status == 1);
    let cur = timestamp();
    assert!(info.start_time < cur);
    assert!(info.end_time > cur);
    let vote_res = get_voted_info(hash, &voter);
    if vote_res == 1 {
        assert!(approve_or_reject == false);
    } else if vote_res == 2 {
        assert!(approve_or_reject == true);
    }
    let weight = get_voter_weight(&voter);
    let vi = VotedInfo {
        voter,
        weight,
        approve_or_reject,
    };
    update_voted_info(hash, vi, info);
    EventBuilder::new()
        .string("voteTopic")
        .h256(hash)
        .address(&voter)
        .bool(approve_or_reject)
        .notify();
    true
}

fn update_voted_info(hash: &H256, info: VotedInfo, mut topic_info: TopicInfo) {
    let mut voted_info = get_all_voted_info(hash);
    let mut has_voted = false;
    let mut approve = 0;
    let mut reject = 0;
    for i in voted_info.iter_mut() {
        let weight = get_voter_weight(&i.voter);
        i.weight = weight;
        if i.approve_or_reject {
            approve += i.weight;
        } else {
            reject += i.weight;
        }
        if i.voter == info.voter {
            i.approve_or_reject = info.approve_or_reject;
            has_voted = true;
        }
    }
    if !has_voted {
        if info.approve_or_reject {
            approve += info.weight;
        } else {
            reject += info.weight;
        }
        voted_info.push(info);
    }
    let key = get_key(PRE_VOTED, hash.as_ref());
    database::put(key, voted_info);
    topic_info.reject = reject;
    topic_info.approve = approve;
    let key = get_key(PRE_TOPIC_INFO, hash.as_ref());
    database::put(key.as_slice(), topic_info);
}

/// ****all user can invoke method ***********
/// query all topic hash
fn list_topic_hash() -> Vec<H256> {
    let res = neo::call_contract(&NEO_VOTE_CONTRACT_ADDRESS, ("listTopics", ()));
    if let Some(r) = res {
        let mut parser = VmValueParser::new(r.as_slice());
        let topics: Vec<Vec<u8>> = parser.read().unwrap();
        let mut temp = get_all_topic_hash_inner();
        for topic_hash in topics.iter() {
            let h: H256 = unsafe { *(topic_hash.as_ptr() as *const H256) };
            temp.push(h);
        }
        return temp;
    }
    vec![]
}

fn get_voter_weight(voter: &Address) -> u64 {
    let item = governance::get_peer_info(voter);
    if &item.peer_pubkey_addr != &Address::new([0u8; 20]) && &item.peer_pubkey_addr == voter {
        return item.init_pos + item.total_pos;
    }
    0
}

fn get_voted_address(hash: &H256) -> Vec<VotedInfo> {
    let voted_info = get_all_voted_info(hash);
    if voted_info.len() != 0 {
        return voted_info;
    }
    let res = neo::call_contract(
        &NEO_VOTE_CONTRACT_ADDRESS,
        ("getVotedAddress", (hash.as_ref() as &[u8],)),
    );
    if let Some(r) = res {
        let mut parser = VmValueParser::new(r.as_slice());
        let info: Vec<VotedInfo> = parser.read().unwrap();
        info
    } else {
        vec![]
    }
}

fn get_voted_address_bytes(hash: &H256) -> Vec<u8> {
    let res = neo::call_contract(
        &NEO_VOTE_CONTRACT_ADDRESS,
        ("getVotedAddress", (hash.as_ref() as &[u8],)),
    );
    if let Some(r) = res {
        return r;
    }
    vec![]
}

fn get_topic_info_list_by_addr(gov_node_addr: &Address) -> Vec<TopicInfo> {
    let hash_list = get_all_topic_hash_inner();
    let mut res = Vec::with_capacity(20);
    for hash in hash_list.iter() {
        let info = get_topic_info(hash).unwrap();
        if &info.gov_node_addr == gov_node_addr {
            res.push(info);
        }
    }
    let neo_res = neo::call_contract(
        &NEO_VOTE_CONTRACT_ADDRESS,
        ("getTopicInfoListByAdmin", (gov_node_addr,)),
    );
    if let Some(neo_r) = neo_res {
        let mut parser = VmValueParser::new(neo_r.as_slice());
        let temp: Vec<TopicInfo> = parser.read().unwrap();
        res.extend(temp);
    }
    res
}

/// 1: approve, 2: reject, other: not voted
fn get_voted_info(hash: &H256, voter: &Address) -> u8 {
    let voted_info = get_all_voted_info(hash);
    for info in voted_info.iter() {
        if &info.voter == voter {
            if info.approve_or_reject {
                return 1;
            } else {
                return 2;
            }
        }
    }
    let res = neo::call_contract(
        &NEO_VOTE_CONTRACT_ADDRESS,
        (
            "getVotedInfo",
            (hash.as_ref() as &[u8], voter.as_ref() as &[u8]),
        ),
    );
    if let Some(r) = res {
        let mut parser = VmValueParser::new(r.as_slice());
        let r = parser.number().unwrap_or_default();
        return r as u8;
    }
    0
}

fn get_voted_info_bytes(hash: &H256, voter: &Address) -> Vec<u8> {
    let res = neo::call_contract(
        &NEO_VOTE_CONTRACT_ADDRESS,
        (
            "getVotedInfo",
            (hash.as_ref() as &[u8], voter.as_ref() as &[u8]),
        ),
    );
    if let Some(r) = res {
        return r;
    }
    vec![]
}

fn get_all_voted_info(hash: &H256) -> Vec<VotedInfo> {
    let key = get_key(PRE_VOTED, hash.as_ref());
    database::get::<_, Vec<VotedInfo>>(key).unwrap_or(vec![])
}

fn is_gov_node(gov_node_addr: &Address) -> bool {
    let peer_info = get_peer_info(gov_node_addr);
    assert_ne!(&peer_info.peer_pubkey_addr, &Address::new([0u8; 20]));
    assert_eq!(&peer_info.peer_pubkey_addr, gov_node_addr);
    true
}

fn get_topic_info(hash: &H256) -> Option<TopicInfo> {
    let key = get_key(PRE_TOPIC_INFO, hash.as_ref());
    let info = database::get::<_, TopicInfo>(key);
    if let Some(temp) = info {
        return Some(temp);
    } else {
        let res = neo::call_contract(
            &NEO_VOTE_CONTRACT_ADDRESS,
            ("getTopicInfo", (hash.as_ref() as &[u8],)),
        );
        if let Some(r) = res {
            let mut parser = VmValueParser::new(r.as_slice());
            let topic_info: Option<TopicInfo> = parser.list().ok();
            return topic_info;
        } else {
            None
        }
    }
}

fn get_topic_info_bytes(hash: &H256) -> Vec<u8> {
    let res = neo::call_contract(
        &NEO_VOTE_CONTRACT_ADDRESS,
        ("getTopicInfo", (hash.as_ref() as &[u8],)),
    );
    if let Some(r) = res {
        return r;
    } else {
        vec![]
    }
}

fn get_key(pre: &[u8], hash: &[u8]) -> Vec<u8> {
    [pre, hash].concat()
}

#[no_mangle]
pub fn invoke() {
    let input = input();
    let mut source = Source::new(&input);
    let action: &[u8] = source.read().unwrap();
    let mut sink = Sink::new(12);
    match action {
        b"migrate" => {
            let (code, vm_ty, name, version, author, email, desc) = source.read().unwrap();
            sink.write(migrate(code, vm_ty, name, version, author, email, desc))
        }
        b"listGovNodes" => {
            sink.write(list_gov_nodes());
        }
        b"listTopics" => {
            sink.write(list_topic_hash());
        }
        b"getTopic" => {
            let hash = source.read().expect("parameter should be H256");
            sink.write(get_topic(hash));
        }
        b"get_topic_bytes" => {
            let hash = source.read().expect("parameter should be H256");
            sink.write(get_topic_bytes(hash));
        }
        b"getTopicInfo" => {
            let hash = source.read().expect("parameter should be H256");
            sink.write(get_topic_info(hash));
        }
        b"get_topic_info_bytes" => {
            let hash = source.read().expect("parameter should be H256");
            sink.write(get_topic_info_bytes(hash));
        }
        b"get_timestamp" => {
            sink.write(get_timestamp());
        }
        b"createTopic" => {
            let (admin, topic_title, topic_detail, start_time, end_time) = source.read().unwrap();
            sink.write(create_topic(
                admin,
                topic_title,
                topic_detail,
                start_time,
                end_time,
            ));
        }
        b"cancelTopic" => {
            let hash = source.read().unwrap();
            sink.write(cancel_topic(hash));
        }
        b"voteTopic" => {
            let (hash, voter, approve_or_reject) = source.read().unwrap();
            sink.write(vote_topic(hash, voter, approve_or_reject));
        }
        b"getVoterWeight" => {
            let voter = source.read().unwrap();
            sink.write(get_voter_weight(voter));
        }
        b"getVotedInfo" => {
            let (hash, voter) = source.read().unwrap();
            sink.write(get_voted_info(hash, voter));
        }
        b"get_voted_info_bytes" => {
            let (hash, voter) = source.read().unwrap();
            sink.write(get_voted_info_bytes(hash, voter));
        }
        b"getVotedAddress" => {
            let hash = source.read().unwrap();
            sink.write(get_voted_address(hash));
        }
        b"get_voted_address_bytes" => {
            let hash = source.read().unwrap();
            sink.write(get_voted_address_bytes(hash));
        }
        b"getTopicInfoListByAddr" => {
            let admin = source.read().unwrap();
            sink.write(get_topic_info_list_by_addr(admin));
        }
        _ => panic!(),
    }
    ret(sink.bytes())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
