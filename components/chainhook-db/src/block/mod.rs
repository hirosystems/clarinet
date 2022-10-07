pub mod digestion;
pub mod ingestion;
use chainhook_types::BlockIdentifier;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DigestingCommand {
    DigestSeedBlock(BlockIdentifier),
    GarbageCollect,
    Kill,
    Terminate,
}
