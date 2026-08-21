pub mod ids;

use crate::{ArtifactRecord, AuthorityResolutionRecord, EventRecord, PacketRecord};

pub fn base_authority_record() -> AuthorityResolutionRecord {
    crate::proof::base_authority_record()
}

pub fn base_artifact() -> ArtifactRecord {
    crate::proof::base_artifact_record(
        ids::FIXTURE_AFFECTED_ARTIFACT_ID,
        ids::FIXTURE_AFFECTED_SOURCE_REF,
    )
}

pub fn base_packet() -> PacketRecord {
    crate::proof::base_packet_record(
        ids::FIXTURE_AFFECTED_PACKET_ID,
        ids::FIXTURE_AFFECTED_ARTIFACT_ID,
    )
}

pub fn source_deleted_event() -> EventRecord {
    crate::proof::source_deleted_event()
}

pub fn authority_record_changed_event() -> EventRecord {
    crate::proof::authority_record_changed_event_for_test()
}
