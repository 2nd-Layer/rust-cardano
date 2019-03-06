//! Representation of the block in the mockchain.
use crate::certificate;
use crate::key::{Hash, PrivateKey, Signature, Signed};
use crate::leadership::LeaderId;
use crate::setting;
use crate::transaction::*;
use chain_core::property;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

pub use crate::date::{BlockDate, BlockDateParseError};

/// `Block` is an element of the blockchain it contains multiple
/// transaction and a reference to the parent block. Alongside
/// with the position of that block in the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub slot_id: BlockDate, // FIXME: rename to 'date'
    pub parent_hash: Hash,

    pub contents: Vec<Message>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Transaction(SignedTransaction),

    StakeKeyRegistration(Signed<certificate::StakeKeyRegistration>),
    StakeKeyDeregistration(Signed<certificate::StakeKeyDeregistration>),
    StakeDelegation(Signed<certificate::StakeDelegation>),
    StakePoolRegistration(Signed<certificate::StakePoolRegistration>),
    StakePoolRetirement(Signed<certificate::StakePoolRetirement>),

    // FIXME: Placeholder for the eventual update mechanism. Currently
    // update proposals take effect immediately and there is no
    // signing/voting.
    Update(setting::UpdateProposal),
}

/// `Block` is an element of the blockchain it contains multiple
/// transaction and a reference to the parent block. Alongside
/// with the position of that block in the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedBlock {
    /// Public key used to sign the block.
    pub leader_id: LeaderId,
    /// List of cryptographic signatures that verifies the block.
    pub signature: Signature,
    /// Internal block.
    pub block: Block,
}

/// The mockchain does not have a block header like in the cardano chain.
///
/// Instead we allow a block summary including all the metadata associated
/// to the block that can be useful for a node to know before downloading
/// a block from another node (for example).
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedBlockSummary {
    /// the relative position of the block within the blockchain
    pub slot_id: BlockDate,
    /// the exact position of the block within the blockchain
    pub parent_hash: Hash,
    /// the hash that identify this block (this is the Hash of the `Block`).
    pub hash: Hash,
    /// Public key used to sign the block.
    pub leader_id: LeaderId,
    /// the cryptographic signature that verifies the block.
    pub signature: Signature,
}

impl SignedBlock {
    /// Create a new signed block.
    pub fn new(block: Block, pkey: &PrivateKey) -> Self {
        use chain_core::property::Block;
        let block_id = block.id();
        SignedBlock {
            leader_id: pkey.into(),
            signature: pkey.sign(block_id.as_ref()),
            block: block,
        }
    }

    /// Verify if block is correctly signed by the key.
    /// Return `false` if there is no such signature or
    /// if it can't be verified.
    pub fn verify(&self) -> bool {
        use chain_core::property::Block;
        let block_id = self.block.id();
        self.leader_id.0.verify(block_id.as_ref(), &self.signature)
    }

    /// retrieve the summary of the signed block.
    pub fn summary(&self) -> SignedBlockSummary {
        use chain_core::property::Block;
        SignedBlockSummary {
            slot_id: self.block.slot_id,
            parent_hash: self.block.parent_hash,
            hash: self.id(),
            leader_id: self.leader_id.clone(),
            signature: self.signature.clone(),
        }
    }
}

impl property::Block for Block {
    type Id = Hash;
    type Date = BlockDate;

    /// Identifier of the block, currently the hash of the
    /// serialized transaction.
    fn id(&self) -> Self::Id {
        use chain_core::property::Serialize;
        // TODO: hash creation can be much faster
        let bytes = self
            .serialize_as_vec()
            .expect("expect serialisation in memory to never fail");
        Hash::hash_bytes(&bytes)
    }

    /// Id of the parent block.
    fn parent_id(&self) -> Self::Id {
        self.parent_hash
    }

    /// Date of the block.
    fn date(&self) -> Self::Date {
        self.slot_id
    }
}
impl property::Block for SignedBlock {
    type Id = <Block as property::Block>::Id;
    type Date = <Block as property::Block>::Date;

    /// Identifier of the block, currently the hash of the
    /// serialized transaction.
    fn id(&self) -> Self::Id {
        self.block.id()
    }

    /// Id of the parent block.
    fn parent_id(&self) -> Self::Id {
        self.block.parent_id()
    }

    /// Date of the block.
    fn date(&self) -> Self::Date {
        self.block.date()
    }
}

impl property::HasHeader for SignedBlock {
    type Header = SignedBlockSummary;
    fn header(&self) -> Self::Header {
        self.summary()
    }
}

impl property::Serialize for Block {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::Codec;
        use std::io::Write;

        let mut codec = Codec::from(writer);

        codec.put_u32(self.slot_id.epoch)?;
        codec.put_u32(self.slot_id.slot_id)?;
        codec.write_all(self.parent_hash.as_ref())?;
        codec.put_u16(self.contents.len() as u16)?;
        for t in self.contents.iter() {
            t.serialize(&mut codec)?;
        }

        Ok(())
    }
}
impl property::Header for SignedBlockSummary {
    type Id = <Block as property::Block>::Id;
    type Date = <Block as property::Block>::Date;

    fn id(&self) -> Self::Id {
        self.hash.clone()
    }
    fn date(&self) -> Self::Date {
        self.slot_id
    }
}

impl property::Serialize for SignedBlock {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        self.leader_id.serialize(&mut writer)?;
        self.signature.serialize(&mut writer)?;
        self.block.serialize(&mut writer)
    }
}
impl property::Serialize for SignedBlockSummary {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::Codec;
        use std::io::Write;

        let mut codec = Codec::from(writer);

        codec.put_u32(self.slot_id.epoch)?;
        codec.put_u32(self.slot_id.slot_id)?;
        codec.write_all(self.parent_hash.as_ref())?;
        codec.write_all(self.hash.as_ref())?;
        self.leader_id.serialize(&mut codec)?;
        self.signature.serialize(&mut codec)
    }
}

impl property::Deserialize for Block {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::Codec;
        use std::io::Read;

        let mut codec = Codec::from(reader);

        let epoch = codec.get_u32()?;
        let slot_id = codec.get_u32()?;
        let date = BlockDate { epoch, slot_id };

        let mut hash = [0; 32];
        codec.read_exact(&mut hash)?;
        let hash = Hash::from(cardano::hash::Blake2b256::from(hash));

        let num_messages = codec.get_u16()? as usize;

        let mut block = Block {
            slot_id: date,
            parent_hash: hash,
            contents: Vec::with_capacity(num_messages),
        };
        for _ in 0..num_messages {
            block.contents.push(Message::deserialize(&mut codec)?);
        }

        Ok(block)
    }
}
impl property::Deserialize for SignedBlock {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(mut reader: R) -> Result<Self, Self::Error> {
        let leader_id = LeaderId::deserialize(&mut reader)?;
        let signature = Signature::deserialize(&mut reader)?;
        let block = Block::deserialize(&mut reader)?;

        Ok(SignedBlock {
            leader_id,
            signature,
            block,
        })
    }
}
impl property::Deserialize for SignedBlockSummary {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::Codec;
        use std::io::Read;

        let mut codec = Codec::from(reader);

        let epoch = codec.get_u32()?;
        let slot_id = codec.get_u32()?;
        let slot_id = BlockDate { epoch, slot_id };

        let mut parent_hash = [0; 32];
        codec.read_exact(&mut parent_hash)?;
        let parent_hash = Hash::from(cardano::hash::Blake2b256::from(parent_hash));
        let mut hash = [0; 32];
        codec.read_exact(&mut hash)?;
        let hash = Hash::from(cardano::hash::Blake2b256::from(hash));

        let leader_id = LeaderId::deserialize(&mut codec)?;
        let signature = Signature::deserialize(&mut codec)?;

        Ok(SignedBlockSummary {
            slot_id,
            parent_hash,
            hash,
            leader_id,
            signature,
        })
    }
}

impl property::HasTransaction for Block {
    type Transaction = SignedTransaction;
    fn transactions<'a>(&'a self) -> Box<Iterator<Item = &SignedTransaction> + 'a> {
        Box::new(self.contents.iter().filter_map(|msg| match msg {
            Message::Transaction(tx) => Some(tx),
            _ => None,
        }))
    }

    fn for_each_transaction<F>(&self, mut f: F)
    where
        F: FnMut(&Self::Transaction),
    {
        self.contents.iter().for_each(|msg| match msg {
            Message::Transaction(tx) => f(tx),
            _ => {}
        })
    }
}

impl property::HasTransaction for SignedBlock {
    type Transaction = SignedTransaction;
    fn transactions<'a>(&'a self) -> Box<Iterator<Item = &SignedTransaction> + 'a> {
        self.block.transactions()
    }

    fn for_each_transaction<F>(&self, f: F)
    where
        F: FnMut(&Self::Transaction),
    {
        self.block.for_each_transaction(f)
    }
}

#[derive(FromPrimitive)]
enum MessageTag {
    Transaction = 1,
    StakeKeyRegistration = 2,
    StakeKeyDeregistration = 3,
    StakeDelegation = 4,
    StakePoolRegistration = 5,
    StakePoolRetirement = 6,
    Update = 7,
}

impl property::Serialize for Message {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        match self {
            Message::Transaction(signed) => {
                codec.put_u8(MessageTag::Transaction as u8)?;
                signed.serialize(&mut codec)
            }
            Message::StakeKeyRegistration(signed) => {
                codec.put_u8(MessageTag::StakeKeyRegistration as u8)?;
                signed.serialize(&mut codec)
            }
            Message::StakeKeyDeregistration(signed) => {
                codec.put_u8(MessageTag::StakeKeyDeregistration as u8)?;
                signed.serialize(&mut codec)
            }
            Message::StakeDelegation(signed) => {
                codec.put_u8(MessageTag::StakeDelegation as u8)?;
                signed.serialize(&mut codec)
            }
            Message::StakePoolRegistration(signed) => {
                codec.put_u8(MessageTag::StakePoolRegistration as u8)?;
                signed.serialize(&mut codec)
            }
            Message::StakePoolRetirement(signed) => {
                codec.put_u8(MessageTag::StakePoolRetirement as u8)?;
                signed.serialize(&mut codec)
            }
            Message::Update(proposal) => {
                codec.put_u8(MessageTag::Update as u8)?;
                proposal.serialize(&mut codec)
            }
        }
    }
}

impl property::Deserialize for Message {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        let tag = codec.get_u8()?;
        match MessageTag::from_u8(tag) {
            Some(MessageTag::Transaction) => Ok(Message::Transaction(
                SignedTransaction::deserialize(&mut codec)?,
            )),
            Some(MessageTag::StakeKeyRegistration) => Ok(Message::StakeKeyRegistration(
                Signed::deserialize(&mut codec)?,
            )),
            Some(MessageTag::StakeKeyDeregistration) => Ok(Message::StakeKeyDeregistration(
                Signed::deserialize(&mut codec)?,
            )),
            Some(MessageTag::StakeDelegation) => {
                Ok(Message::StakeDelegation(Signed::deserialize(&mut codec)?))
            }
            Some(MessageTag::StakePoolRegistration) => Ok(Message::StakePoolRegistration(
                Signed::deserialize(&mut codec)?,
            )),
            Some(MessageTag::StakePoolRetirement) => Ok(Message::StakePoolRetirement(
                Signed::deserialize(&mut codec)?,
            )),
            Some(MessageTag::Update) => Ok(Message::Update(setting::UpdateProposal::deserialize(
                &mut codec,
            )?)),
            None => panic!("Unrecognized certificate message tag {}.", tag),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    quickcheck! {
        fn block_serialization_bijection(b: Block) -> TestResult {
            property::testing::serialization_bijection(b)
        }

        fn signed_block_serialization_bijection(b: SignedBlock) -> TestResult {
            property::testing::serialization_bijection(b)
        }

        fn signed_block_summary_serialization_bijection(b: SignedBlockSummary) -> TestResult {
            property::testing::serialization_bijection(b)
        }

        fn summary_is_summary_of_signed_block(block: SignedBlock) -> TestResult {
            use chain_core::property::{Header, HasHeader, Block};

            let summary = block.header();

            TestResult::from_bool(
                summary.id() == block.id() &&
                summary.date() == block.date()
            )
        }
    }

    impl Arbitrary for Message {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match g.next_u32() % 100 {
                0 => Message::StakeKeyRegistration(Arbitrary::arbitrary(g)),
                1 => Message::StakeKeyDeregistration(Arbitrary::arbitrary(g)),
                2 => Message::StakeDelegation(Arbitrary::arbitrary(g)),
                3 => Message::StakePoolRegistration(Arbitrary::arbitrary(g)),
                4 => Message::StakePoolRetirement(Arbitrary::arbitrary(g)),
                _ => Message::Transaction(Arbitrary::arbitrary(g)),
            }
        }
    }

    impl Arbitrary for Block {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Block {
                slot_id: Arbitrary::arbitrary(g),
                parent_hash: Arbitrary::arbitrary(g),
                contents: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SignedBlock {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SignedBlock {
                block: Arbitrary::arbitrary(g),
                leader_id: Arbitrary::arbitrary(g),
                signature: Arbitrary::arbitrary(g),
            }
        }
    }
    impl Arbitrary for SignedBlockSummary {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SignedBlockSummary {
                slot_id: Arbitrary::arbitrary(g),
                parent_hash: Arbitrary::arbitrary(g),
                hash: Arbitrary::arbitrary(g),
                leader_id: Arbitrary::arbitrary(g),
                signature: Arbitrary::arbitrary(g),
            }
        }
    }
    impl Arbitrary for BlockDate {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            BlockDate {
                epoch: Arbitrary::arbitrary(g),
                slot_id: Arbitrary::arbitrary(g),
            }
        }
    }
}