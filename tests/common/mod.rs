use precomputed_context_core::{ArtifactRecord, AuthorityResolutionRecord, PacketRecord};

#[cfg(feature = "test-support")]
use precomputed_context_core::fixture_support;

#[cfg(not(feature = "test-support"))]
use precomputed_context_core::proof;

#[allow(dead_code)]
pub fn base_authority_record() -> AuthorityResolutionRecord {
    #[cfg(feature = "test-support")]
    {
        fixture_support::base_authority_record()
    }

    #[cfg(not(feature = "test-support"))]
    {
        proof::base_authority_record()
    }
}

pub fn base_artifact() -> ArtifactRecord {
    #[cfg(feature = "test-support")]
    {
        fixture_support::base_artifact()
    }

    #[cfg(not(feature = "test-support"))]
    {
        proof::base_artifact_record("art-001", "src-tauri/src")
    }
}

pub fn base_packet() -> PacketRecord {
    #[cfg(feature = "test-support")]
    {
        fixture_support::base_packet()
    }

    #[cfg(not(feature = "test-support"))]
    {
        proof::base_packet_record("pkt-001", "art-001")
    }
}
