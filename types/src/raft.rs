use std::collections::HashSet;
use crate::error::{
    ClientReadError,
    ClientWriteError,
    ChangeConfigError,
    RaftError,
    InitializeError,
};
use crate::app_data::{
    AppData,
    AppDataResponse,
    NodeId,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot};

pub type ClientWriteResponseTx<D, R> = oneshot::Sender<Result<ClientWriteResponse<R>, ClientWriteError<D>>>;
pub type ClientReadResponseTx = oneshot::Sender<Result<(), ClientReadError>>;
pub type ChangeMembershipTx = oneshot::Sender<Result<(), ChangeConfigError>>;

/// A message coming from the Raft API.
pub enum RaftMsg<D: AppData, R: AppDataResponse> {
    AppendEntries {
        rpc: AppendEntriesRequest<D>,
        tx: oneshot::Sender<Result<AppendEntriesResponse, RaftError>>,
    },
    RequestVote {
        rpc: VoteRequest,
        tx: oneshot::Sender<Result<VoteResponse, RaftError>>,
    },
    InstallSnapshot {
        rpc: InstallSnapshotRequest,
        tx: oneshot::Sender<Result<InstallSnapshotResponse, RaftError>>,
    },
    ClientWriteRequest {
        rpc: ClientWriteRequest<D>,
        tx: ClientWriteResponseTx<D, R>,
    },
    ClientReadRequest {
        tx: ClientReadResponseTx,
    },
    Initialize {
        members: HashSet<NodeId>,
        tx: oneshot::Sender<Result<(), InitializeError>>,
    },
    AddNonVoter {
        id: NodeId,
        tx: ChangeMembershipTx,
    },
    ChangeMembership {
        members: HashSet<NodeId>,
        tx: ChangeMembershipTx,
    },
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////

/// An RPC sent by a cluster leader to replicate log entries (§5.3), and as a heartbeat (§5.2).
#[derive(Debug, Serialize, Deserialize)]
pub struct AppendEntriesRequest<D: AppData> {
    /// The leader's current term.
    pub term: u64,
    /// The leader's ID. Useful in redirecting clients.
    pub leader_id: u64,
    /// The index of the log entry immediately preceding the new entries.
    pub prev_log_index: u64,
    /// The term of the `prev_log_index` entry.
    pub prev_log_term: u64,
    /// The new log entries to store.
    ///
    /// This may be empty when the leader is sending heartbeats. Entries
    /// are batched for efficiency.
    #[serde(bound = "D: AppData")]
    pub entries: Vec<Entry<D>>,
    /// The leader's commit index.
    pub leader_commit: u64,
}

/// The response to an `AppendEntriesRequest`.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// The responding node's current term, for leader to update itself.
    pub term: u64,
    /// Will be true if follower contained entry matching `prev_log_index` and `prev_log_term`.
    pub success: bool,
    /// A value used to implement the _conflicting term_ optimization outlined in §5.3.
    ///
    /// This value will only be present, and should only be considered, when `success` is `false`.
    pub conflict_opt: Option<ConflictOpt>,
}

/// A struct used to implement the _conflicting term_ optimization outlined in §5.3 for log replication.
///
/// This value will only be present, and should only be considered, when an `AppendEntriesResponse`
/// object has a `success` value of `false`.
///
/// This implementation of Raft uses this value to more quickly synchronize a leader with its
/// followers which may be some distance behind in replication, may have conflicting entries, or
/// which may be new to the cluster.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConflictOpt {
    /// The term of the most recent entry which does not conflict with the received request.
    pub term: u64,
    /// The index of the most recent entry which does not conflict with the received request.
    pub index: u64,
}

/// A Raft log entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry<D: AppData> {
    /// This entry's term.
    pub term: u64,
    /// This entry's index.
    pub index: u64,
    /// This entry's payload.
    #[serde(bound = "D: AppData")]
    pub payload: EntryPayload<D>,
}

impl<D: AppData> Entry<D> {
    /// Create a new snapshot pointer from the given data.
    ///
    /// ### index & term
    /// The index and term of the entry being replaced by this snapshot pointer entry.
    ///
    /// ### id
    /// The ID of the associated snapshot.
    ///
    /// ### membership
    /// The cluster membership config which is contained in the snapshot, which will always be the
    /// latest membership covered by the snapshot.
    pub fn new_snapshot_pointer(index: u64, term: u64, id: String, membership: MembershipConfig) -> Self {
        Entry {
            term,
            index,
            payload: EntryPayload::SnapshotPointer(EntrySnapshotPointer { id, membership }),
        }
    }
}

/// Log entry payload variants.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EntryPayload<D: AppData> {
    /// An empty payload committed by a new cluster leader.
    Blank,
    /// A normal log entry.
    #[serde(bound = "D: AppData")]
    Normal(EntryNormal<D>),
    /// A config change log entry.
    ConfigChange(EntryConfigChange),
    /// An entry which points to a snapshot.
    SnapshotPointer(EntrySnapshotPointer),
}

/// A normal log entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntryNormal<D: AppData> {
    /// The contents of this entry.
    #[serde(bound = "D: AppData")]
    pub data: D,
}

/// A log entry holding a config change.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntryConfigChange {
    /// Details on the cluster's membership configuration.
    pub membership: MembershipConfig,
}

/// A log entry pointing to a snapshot.
///
/// This will only be present when read from storage. An entry of this type will never be
/// transmitted from a leader during replication, an `InstallSnapshotRequest`
/// RPC will be sent instead.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntrySnapshotPointer {
    /// The ID of the snapshot, which is application specific, and probably only meaningful to the storage layer.
    pub id: String,
    /// The cluster's membership config covered by this snapshot.
    pub membership: MembershipConfig,
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////

/// A model of the membership configuration of the cluster.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipConfig {
    /// All members of the Raft cluster.
    pub members: HashSet<NodeId>,
    /// All members of the Raft cluster after joint consensus is finalized.
    ///
    /// The presence of a value here indicates that the config is in joint consensus.
    pub members_after_consensus: Option<HashSet<NodeId>>,
}

impl MembershipConfig {
    /// Get an iterator over all nodes in the current config.
    pub fn all_nodes(&self) -> HashSet<u64> {
        let mut all = self.members.clone();
        if let Some(members) = &self.members_after_consensus {
            all.extend(members);
        }
        all
    }

    /// Check if the given NodeId exists in this membership config.
    ///
    /// When in joint consensus, this will check both config groups.
    pub fn contains(&self, x: &NodeId) -> bool {
        self.members.contains(x)
            || if let Some(members) = &self.members_after_consensus {
                members.contains(x)
            } else {
                false
            }
    }

    /// Check to see if the config is currently in joint consensus.
    pub fn is_in_joint_consensus(&self) -> bool {
        self.members_after_consensus.is_some()
    }

    /// Create a new initial config containing only the given node ID.
    pub fn new_initial(id: NodeId) -> Self {
        let mut members = HashSet::new();
        members.insert(id);
        Self {
            members,
            members_after_consensus: None,
        }
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////

/// An RPC sent by candidates to gather votes (§5.2).
#[derive(Debug, Serialize, Deserialize)]
pub struct VoteRequest {
    /// The candidate's current term.
    pub term: u64,
    /// The candidate's ID.
    pub candidate_id: u64,
    /// The index of the candidate’s last log entry (§5.4).
    pub last_log_index: u64,
    /// The term of the candidate’s last log entry (§5.4).
    pub last_log_term: u64,
}

impl VoteRequest {
    /// Create a new instance.
    pub fn new(term: u64, candidate_id: u64, last_log_index: u64, last_log_term: u64) -> Self {
        Self {
            term,
            candidate_id,
            last_log_index,
            last_log_term,
        }
    }
}

/// The response to a `VoteRequest`.
#[derive(Debug, Serialize, Deserialize)]
pub struct VoteResponse {
    /// The current term of the responding node, for the candidate to update itself.
    pub term: u64,
    /// Will be true if the candidate received a vote from the responder.
    pub vote_granted: bool,
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////

/// An RPC sent by the Raft leader to send chunks of a snapshot to a follower (§7).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstallSnapshotRequest {
    /// The leader's current term.
    pub term: u64,
    /// The leader's ID. Useful in redirecting clients.
    pub leader_id: u64,
    /// The snapshot replaces all log entries up through and including this index.
    pub last_included_index: u64,
    /// The term of the `last_included_index`.
    pub last_included_term: u64,
    /// The byte offset where this chunk of data is positioned in the snapshot file.
    pub offset: u64,
    /// The raw bytes of the snapshot chunk, starting at `offset`.
    pub data: Vec<u8>,
    /// Will be `true` if this is the last chunk in the snapshot.
    pub done: bool,
}

/// The response to an `InstallSnapshotRequest`.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallSnapshotResponse {
    /// The receiving node's current term, for leader to update itself.
    pub term: u64,
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////

/// An application specific client request to update the state of the system (§5.1).
///
/// The entry of this payload will be appended to the Raft log and then applied to the Raft state
/// machine according to the Raft protocol.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientWriteRequest<D: AppData> {
    /// The application specific contents of this client request.
    #[serde(bound = "D: AppData")]
    pub entry: EntryPayload<D>,
}

impl<D: AppData> ClientWriteRequest<D> {
    /// Create a new client payload instance with a normal entry type.
    pub fn new(entry: D) -> Self {
        Self::new_base(EntryPayload::Normal(EntryNormal { data: entry }))
    }

    /// Create a new instance.
    pub fn new_base(entry: EntryPayload<D>) -> Self {
        Self { entry }
    }

    /// Generate a new payload holding a config change.
    pub fn new_config(membership: MembershipConfig) -> Self {
        Self::new_base(EntryPayload::ConfigChange(EntryConfigChange { membership }))
    }

    /// Generate a new blank payload.
    ///
    /// This is used by new leaders when first coming to power.
    pub fn new_blank_payload() -> Self {
        Self::new_base(EntryPayload::Blank)
    }
}

/// The response to a `ClientRequest`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientWriteResponse<R: AppDataResponse> {
    /// The log index of the successfully processed client request.
    pub index: u64,
    /// Application specific response data.
    #[serde(bound = "R: AppDataResponse")]
    pub data: R,
}
