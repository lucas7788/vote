#![cfg_attr(not(feature = "mock"), no_std)]
#![feature(proc_macro_hygiene)]
extern crate ontio_std as ostd;
use ostd::abi::{
    Decoder, Encoder, Error, EventBuilder, Sink, Source, VmValueDecoder, VmValueParser,
};
use ostd::console::debug;
use ostd::contract::governance;
use ostd::contract::governance::get_peer_pool;
use ostd::contract::neo;
use ostd::database;
use ostd::macros::base58;
use ostd::prelude::*;
use ostd::runtime::{check_witness, contract_migrate, input, ret, sha256, timestamp};

const PRE_TOPIC: &[u8] = b"01";
const PRE_TOPIC_INFO: &[u8] = b"02";
const KEY_ALL_TOPIC_HASH: &[u8] = b"03";
const PRE_VOTED: &[u8] = b"04";
const SUPER_ADMIN: Address = base58!("AbtTQJYKfQxq4UdygDsbLVjE8uRrJ2H3tP");

#[cfg(test)]
mod test;
// test net AKzJGcCVr9wVEG95XvP3VnCDRivVjo391r
//local AQWrGrBb6yosjuHDiALkNwVnL9qLanCMdG
const NEO_VOTE_CONTRACT_ADDRESS: Address = base58!("AQWrGrBb6yosjuHDiALkNwVnL9qLanCMdG");

#[derive(Encoder, Decoder)]
struct Topic {
    topic_title: Vec<u8>,
    topic_detail: Vec<u8>,
}

#[derive(Encoder, Decoder)]
struct VoterWeight {
    voter: Address,
    weight: U128,
}

impl<'a> VmValueDecoder<'a> for VoterWeight {
    fn deserialize(parser: &mut VmValueParser<'a>) -> Result<Self, Error> {
        let ty = parser.source.read_byte()?;
        assert_eq!(ty, 0x10);
        let _ = parser.source.read_u32()?;
        let addr_bytes = parser.bytearray()?;
        let addr = unsafe { *(addr_bytes.as_ptr() as *const Address) };
        let weight_bytes = parser.bytearray()?;
        let weight = unsafe { *(weight_bytes.as_ptr() as *const U128) };
        Ok(VoterWeight {
            voter: addr,
            weight,
        })
    }
}

#[derive(Encoder, Decoder)]
struct TopicInfo {
    admin: Address,
    topic_title: Vec<u8>,
    topic_detail: Vec<u8>,
    voters: Vec<VoterWeight>,
    start_time: u64,
    end_time: u64,
    approve: u64,
    reject: u64,
    status: u8,
    hash: H256,
}

impl<'a> VmValueDecoder<'a> for TopicInfo {
    fn deserialize(parser: &mut VmValueParser<'a>) -> Result<Self, Error> {
        let addr_bytes = parser.bytearray()?;
        let addr = unsafe { *(addr_bytes.as_ptr() as *const Address) };
        let topic_title = parser.bytearray()?;
        let topic_detail = parser.bytearray()?;
        // skip voters
        let voters: Vec<VoterWeight> = parser.read()?;
        let start_time_bytes = parser.bytearray()?;
        let start_time = unsafe { *(start_time_bytes.as_ptr() as *const U128) };
        let end_time_bytes = parser.bytearray()?;
        let end_time = unsafe { *(end_time_bytes.as_ptr() as *const U128) };
        let approve = parser.number()?;
        let reject = parser.number()?;
        let status = parser.number()?;
        let hash_bytes = parser.bytearray()?;
        let hash = unsafe { *(hash_bytes.as_ptr() as *const H256) };
        Ok(TopicInfo {
            admin: addr,
            topic_title: topic_title.to_vec(),
            topic_detail: topic_detail.to_vec(),
            voters,
            start_time: start_time as u64,
            end_time: end_time as u64,
            approve: approve as u64,
            reject: reject as u64,
            status: status as u8,
            hash,
        })
    }
}

#[derive(Encoder, Decoder)]
struct VotedInfo {
    voter: Address,
    weight: u64,
    approve_or_reject: bool,
}

impl<'a> VmValueDecoder<'a> for VotedInfo {
    fn deserialize(parser: &mut VmValueParser<'a>) -> Result<Self, Error> {
        let addr = parser.address()?;
        let approve_or_reject = parser.bool()?;
        Ok(VotedInfo {
            voter: addr.clone(),
            approve_or_reject,
            weight: 0,
        })
    }
}

fn migrate(
    code: &[u8],
    vm_ty: U128,
    name: &str,
    version: &str,
    author: &str,
    email: &str,
    desc: &str,
) -> bool {
    assert!(check_witness(&SUPER_ADMIN));
    let addr = contract_migrate(code, vm_ty as u32, name, version, author, email, desc);
    assert_ne!(addr, Address::new([0u8; 20]));
    true
}

fn list_admins() -> Vec<Address> {
    let peer_pool_map = governance::get_peer_pool();
    let mut res: Vec<Address> = vec![];
    for item in peer_pool_map.peer_pool_map.iter() {
        res.push(item.address);
    }
    res
}

fn create_topic(
    admin: Address,
    topic_title: &[u8],
    topic_detail: &[u8],
    start_time: u64,
    end_time: u64,
) -> bool {
    assert!(check_witness(&admin));
    assert!(is_admin(&admin));
    assert!(start_time < end_time);
    let content: Vec<u8> = [topic_title, topic_detail].concat();
    let hash = sha256(content);
    let key_topic = get_key(PRE_TOPIC, hash.as_ref());
    let topic = database::get::<_, Topic>(key_topic.as_slice());
    assert!(topic.is_none());
    let tc = Topic {
        topic_title: topic_title.to_vec(),
        topic_detail: topic_detail.to_vec(),
    };
    database::put(key_topic, tc);
    let key_topic_info = get_key(PRE_TOPIC_INFO, hash.as_ref());
    let info = TopicInfo {
        admin,
        topic_title: topic_title.to_vec(),
        topic_detail: topic_detail.to_vec(),
        voters: vec![],
        start_time,
        end_time,
        approve: 0,
        reject: 0,
        status: 1,
        hash: hash.clone(),
    };
    database::put(key_topic_info, info);
    let mut all_hash: Vec<H256> = database::get(KEY_ALL_TOPIC_HASH).unwrap_or(vec![]);
    all_hash.push(hash.clone());
    database::put(KEY_ALL_TOPIC_HASH, all_hash);
    EventBuilder::new()
        .string("createTopic")
        .h256(&hash)
        .bytearray(topic_title)
        .bytearray(topic_detail)
        .notify();
    true
}

fn get_topic(hash: &H256) -> Option<Topic> {
    let key = get_key(PRE_TOPIC, hash.as_ref());
    let res = database::get::<_, Topic>(key);
    if let Some(temp) = res {
        return Some(temp);
    } else {
        let topic = neo::call_contract(&NEO_VOTE_CONTRACT_ADDRESS, ("getTopic", (hash,)));
        if let Some(old_topic) = topic {
            let mut parser = VmValueParser::new(old_topic.as_slice());
            let r: Vec<Vec<u8>> = parser.list().unwrap();
            Some(Topic {
                topic_title: r[0].clone(),
                topic_detail: r[1].clone(),
            })
        } else {
            None
        }
    }
}

fn cancel_topic(hash: &H256) -> bool {
    let topic_info = get_topic_info(hash);
    if let Some(mut info) = topic_info {
        assert_eq!(info.status, 1);
        assert!(check_witness(&info.admin));
        info.status = 0;
        let key = get_key(PRE_TOPIC_INFO, hash.as_ref());
        database::put(key, info);
    } else {
        panic!("no the topic")
    }
    true
}

fn vote_topic(hash: &H256, voter: Address, approve_or_reject: bool) -> bool {
    assert!(check_witness(&voter));
    assert!(is_admin(&voter));
    let mut info = get_topic_info(hash).expect("not exist topic info");
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
    if approve_or_reject {
        info.approve += weight;
        if vote_res == 2 {
            info.reject -= weight;
        }
    } else {
        info.reject += weight;
        if vote_res == 1 {
            info.approve -= weight;
        }
    }
    let vi = VotedInfo {
        voter,
        weight,
        approve_or_reject,
    };
    update_voted_info(hash, vi);
    EventBuilder::new()
        .string("voteTopic")
        .h256(hash)
        .address(&voter)
        .bool(approve_or_reject)
        .notify();
    true
}

fn update_voted_info(hash: &H256, info: VotedInfo) {
    let mut voted_info = get_all_voted_info(hash);
    let mut has_voted = false;
    for i in voted_info.iter_mut() {
        if i.voter == info.voter {
            i.approve_or_reject = info.approve_or_reject;
            has_voted = true;
            break;
        }
    }
    if !has_voted {
        voted_info.push(info)
    }
    let key = get_key(PRE_VOTED, hash.as_ref());
    database::put(key, voted_info);
}

/// ****all user can invoke method ***********
/// query all topic hash
fn list_topic_hash() -> Vec<H256> {
    let res = neo::call_contract(&NEO_VOTE_CONTRACT_ADDRESS, ("listTopics", ()));
    if let Some(r) = res {
        let mut parser = VmValueParser::new(r.as_slice());
        let topics: Vec<Vec<u8>> = parser.read().unwrap();
        let mut temp = database::get::<_, Vec<H256>>(KEY_ALL_TOPIC_HASH).unwrap_or(vec![]);
        for topic_hash in topics.iter() {
            let h: H256 = unsafe { *(topic_hash.as_ptr() as *const H256) };
            temp.push(h);
        }
        return temp;
    }
    vec![]
}

fn get_voter_weight(voter: &Address) -> u64 {
    let peer_pool_map = governance::get_peer_pool();
    for item in peer_pool_map.peer_pool_map.iter() {
        if &item.address == voter {
            return item.init_pos + item.total_pos;
        }
    }
    0
}

fn get_voted_address(hash: &H256) -> Vec<VotedInfo> {
    let voted_info = get_all_voted_info(hash);
    if voted_info.len() != 0 {
        return voted_info;
    }
    let res = neo::call_contract(&NEO_VOTE_CONTRACT_ADDRESS, ("getVotedAddress", (hash,)));
    if let Some(r) = res {
        let mut parser = VmValueParser::new(r.as_slice());
        let info: Vec<Vec<u8>> = parser.read().unwrap();
        let mut r: Vec<VotedInfo> = vec![];
        for item in info.iter() {
            let mut parser = VmValueParser::new(item);
            let vi = parser.list().unwrap();
            r.push(vi);
        }
        r
    } else {
        vec![]
    }
}

fn get_topic_info_list_by_admin(admin: &Address) -> Vec<TopicInfo> {
    let hash_list = database::get::<_, Vec<H256>>(KEY_ALL_TOPIC_HASH).unwrap_or(vec![]);
    let mut res = Vec::with_capacity(20);
    for hash in hash_list.iter() {
        let info = get_topic_info(hash).unwrap();
        if &info.admin == admin {
            res.push(info);
        }
    }
    let neo_res = neo::call_contract(
        &NEO_VOTE_CONTRACT_ADDRESS,
        ("getTopicInfoListByAdmin", (admin,)),
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
    let res = neo::call_contract(&NEO_VOTE_CONTRACT_ADDRESS, ("getVotedInfo", (hash, voter)));
    if let Some(r) = res {
        let mut parser = VmValueParser::new(r.as_slice());
        let r = parser.number().unwrap_or_default();
        return r as u8;
    }
    0
}

fn get_all_voted_info(hash: &H256) -> Vec<VotedInfo> {
    let key = get_key(PRE_VOTED, hash.as_ref());
    database::get::<_, Vec<VotedInfo>>(key).unwrap_or(vec![])
}

fn is_admin(admin: &Address) -> bool {
    let peer_pool_map = get_peer_pool();
    for item in peer_pool_map.peer_pool_map.iter() {
        if &item.address == admin {
            return true;
        }
    }
    false
}

fn get_topic_info(hash: &H256) -> Option<TopicInfo> {
    let key = get_key(PRE_TOPIC_INFO, hash.as_ref());
    let info = database::get::<_, TopicInfo>(key);
    if let Some(temp) = info {
        return Some(temp);
    } else {
        debug("22222");
        let res = neo::call_contract(
            &NEO_VOTE_CONTRACT_ADDRESS,
            ("getTopicInfo", (hash.as_ref() as &[u8],)),
        );
        if let Some(r) = res {
            debug("111");
            debug(&format!("{}", r.len()));
            debug(str::from_utf8(r.as_slice()).unwrap_or_default());
            let mut parser = VmValueParser::new(r.as_slice());
            let topic_info: TopicInfo = parser.list().unwrap();
            return Some(topic_info);
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
        debug("111");
        debug(&format!("{}", r.len()));
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
        b"listAdmins" => {
            sink.write(list_admins());
        }
        b"listTopics" => {
            sink.write(list_topic_hash());
        }
        b"getTopic" => {
            let hash = source.read().expect("parameter should be H256");
            sink.write(get_topic(hash));
        }
        b"getTopicInfo" => {
            let hash = source.read().expect("parameter should be H256");
            sink.write(get_topic_info(hash));
        }
        b"get_topic_info_bytes" => {
            let hash = source.read().expect("parameter should be H256");
            sink.write(get_topic_info_bytes(hash))
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
        b"getVotedInfo" => {
            let (hash, voter) = source.read().unwrap();
            sink.write(get_voted_info(hash, voter));
        }
        b"get_voted_address" => {
            let hash = source.read().unwrap();
            sink.write(get_voted_address(hash));
        }
        b"getTopicInfoListByAdmin" => {
            let admin = source.read().unwrap();
            sink.write(get_topic_info_list_by_admin(admin));
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
