use super::*;

#[derive(Encoder, Decoder)]
pub struct Topic {
    pub topic_title: Vec<u8>,
    pub topic_detail: Vec<u8>,
}

impl<'a> VmValueDecoder<'a> for Topic {
    fn deserialize(parser: &mut VmValueParser<'a>) -> Result<Self, Error> {
        let ty = parser.source.read_byte()?;
        assert_eq!(ty, 0x10);
        let _ = parser.source.read_u32()?;
        let topic_title = parser.bytearray()?;
        let topic_detail = parser.bytearray()?;
        Ok(Topic {
            topic_title: topic_title.to_vec(),
            topic_detail: topic_detail.to_vec(),
        })
    }
}

#[derive(Encoder, Decoder)]
pub struct VoterWeight {
    pub voter: Address,
    pub weight: U128,
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
pub struct TopicInfo {
    pub gov_node_addr: Address,
    pub topic_title: Vec<u8>,
    pub topic_detail: Vec<u8>,
    pub voters: Vec<VoterWeight>,
    pub start_time: u64,
    pub end_time: u64,
    pub approve: u64,
    pub reject: u64,
    pub status: u8,
    pub hash: H256,
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
            gov_node_addr: addr,
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
pub struct VotedInfo {
    pub voter: Address,
    pub weight: u64,
    pub approve_or_reject: bool,
}

impl<'a> VmValueDecoder<'a> for VotedInfo {
    fn deserialize(parser: &mut VmValueParser<'a>) -> Result<Self, Error> {
        let ty = parser.source.read_byte()?;
        assert_eq!(ty, 0x10);
        let _ = parser.source.read_u32()?;
        let addr_bytes = parser.bytearray()?;
        let addr = unsafe { *(addr_bytes.as_ptr() as *const Address) };
        let approve_or_reject = parser.bool()?;
        Ok(VotedInfo {
            voter: addr.clone(),
            approve_or_reject,
            weight: 0,
        })
    }
}
