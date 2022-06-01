//! Public Raft interface and data types.

use std::collections::HashSet;
use std::sync::Arc;

use common_trait::network::RaftNetwork;
use common_trait::storage::RaftStorage;
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tokio::task::JoinHandle;
use types::raft::RaftMsg;
use crate::config::Config;
use crate::core::RaftCore;
use types::error::{ChangeConfigError, ClientReadError, ClientWriteError, InitializeError, RaftError, RaftResult};
use crate::metrics::RaftMetrics;
use types::app_data::{AppData, AppDataResponse, NodeId};
use types::raft::{
    AppendEntriesRequest,
    AppendEntriesResponse,
    VoteRequest,
    VoteResponse,
    InstallSnapshotRequest,
    InstallSnapshotResponse,
    ClientWriteRequest,
    ClientWriteResponse,
};

struct RaftInner<D: AppData, R: AppDataResponse, N: RaftNetwork<D>, S: RaftStorage<D, R>> {
    tx_api: mpsc::UnboundedSender<RaftMsg<D, R>>,
    rx_metrics: watch::Receiver<RaftMetrics>,
    raft_handle: Mutex<Option<JoinHandle<RaftResult<()>>>>,
    tx_shutdown: Mutex<Option<oneshot::Sender<()>>>,
    marker_n: std::marker::PhantomData<N>,
    marker_s: std::marker::PhantomData<S>,
}

/// The Raft API.
///
/// This type implements the full Raft spec, and is the interface to a running Raft node.
/// Applications building on top of Raft will use this to spawn a Raft task and interact with
/// the spawned task.
///
/// For more information on the Raft protocol, see
/// [the specification here](https://raft.github.io/raft.pdf) (**pdf warning**).
///
/// For details and discussion on this API, see the
/// [Raft API](https://async-raft.github.io/async-raft/raft.html) section of the guide.
///
/// ### clone
/// This type implements `Clone`, and should be cloned liberally. The clone itself is very cheap
/// and helps to facilitate use with async workflows.
///
/// ### shutting down
/// If any of the interfaces returns a `RaftError::ShuttingDown`, this indicates that the Raft node
/// is shutting down (potentially for data safety reasons due to a storage error), and the `shutdown`
/// method should be called on this type to await the shutdown of the node. If the parent
/// application needs to shutdown the Raft node for any reason, calling `shutdown` will do the trick.
pub struct Raft<D: AppData, R: AppDataResponse, N: RaftNetwork<D>, S: RaftStorage<D, R>> {
    inner: Arc<RaftInner<D, R, N, S>>,
}

impl<D: AppData, R: AppDataResponse, N: RaftNetwork<D>, S: RaftStorage<D, R>> Raft<D, R, N, S> {
    /// Create and spawn a new Raft task.
    ///
    /// ### `id`
    /// The ID which the spawned Raft task will use to identify itself within the cluster.
    /// Applications must guarantee that the ID provided to this function is stable, and should be
    /// persisted in a well known location, probably alongside the Raft log and the application's
    /// state machine. This ensures that restarts of the node will yield the same ID every time.
    ///
    /// ### `config`
    /// Raft's runtime config. See the docs on the `Config` object for more details.
    ///
    /// ### `network`
    /// An implementation of the `RaftNetwork` trait which will be used by Raft for sending RPCs to
    /// peer nodes within the cluster. See the docs on the `RaftNetwork` trait for more details.
    ///
    /// ### `storage`
    /// An implementation of the `RaftStorage` trait which will be used by Raft for data storage.
    /// See the docs on the `RaftStorage` trait for more details.
    pub fn new(id: NodeId, config: Arc<Config>, network: Arc<N>, storage: Arc<S>) -> Self {
        let (tx_api, rx_api) = mpsc::unbounded_channel();
        let (tx_metrics, rx_metrics) = watch::channel(RaftMetrics::new_initial(id));
        let (tx_shutdown, rx_shutdown) = oneshot::channel();
        let raft_handle = RaftCore::spawn(id, config, network, storage, rx_api, tx_metrics, rx_shutdown);
        let inner = RaftInner {
            tx_api,
            rx_metrics,
            raft_handle: Mutex::new(Some(raft_handle)),
            tx_shutdown: Mutex::new(Some(tx_shutdown)),
            marker_n: std::marker::PhantomData,
            marker_s: std::marker::PhantomData,
        };
        Self { inner: Arc::new(inner) }
    }

    /// Submit an AppendEntries RPC to this Raft node.
    ///
    /// These RPCs are sent by the cluster leader to replicate log entries (§5.3), and are also
    /// used as heartbeats (§5.2).
    #[tracing::instrument(level = "debug", skip(self, rpc))]
    pub async fn append_entries(&self, rpc: AppendEntriesRequest<D>) -> Result<AppendEntriesResponse, RaftError> {
        let (tx, rx) = oneshot::channel();
        self.inner
            .tx_api
            .send(RaftMsg::AppendEntries { rpc, tx })
            .map_err(|_| RaftError::ShuttingDown)?;
        Ok(rx.await.map_err(|_| RaftError::ShuttingDown).and_then(|res| res)?)
    }

    /// Submit a VoteRequest (RequestVote in the spec) RPC to this Raft node.
    ///
    /// These RPCs are sent by cluster peers which are in candidate state attempting to gather votes (§5.2).
    #[tracing::instrument(level = "debug", skip(self, rpc))]
    pub async fn vote(&self, rpc: VoteRequest) -> Result<VoteResponse, RaftError> {
        let (tx, rx) = oneshot::channel();
        self.inner
            .tx_api
            .send(RaftMsg::RequestVote { rpc, tx })
            .map_err(|_| RaftError::ShuttingDown)?;
        Ok(rx.await.map_err(|_| RaftError::ShuttingDown).and_then(|res| res)?)
    }

    /// Submit an InstallSnapshot RPC to this Raft node.
    ///
    /// These RPCs are sent by the cluster leader in order to bring a new node or a slow node up-to-speed
    /// with the leader (§7).
    #[tracing::instrument(level = "debug", skip(self, rpc))]
    pub async fn install_snapshot(&self, rpc: InstallSnapshotRequest) -> Result<InstallSnapshotResponse, RaftError> {
        let (tx, rx) = oneshot::channel();
        self.inner
            .tx_api
            .send(RaftMsg::InstallSnapshot { rpc, tx })
            .map_err(|_| RaftError::ShuttingDown)?;
        Ok(rx.await.map_err(|_| RaftError::ShuttingDown).and_then(|res| res)?)
    }

    /// Get the ID of the current leader from this Raft node.
    ///
    /// This method is based on the Raft metrics system which does a good job at staying
    /// up-to-date; however, the `client_read` method must still be used to guard against stale
    /// reads. This method is perfect for making decisions on where to route client requests.
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn current_leader(&self) -> Option<NodeId> {
        self.metrics().borrow().current_leader
    }

    /// Check to ensure this node is still the cluster leader, in order to guard against stale reads (§8).
    ///
    /// The actual read operation itself is up to the application, this method just ensures that
    /// the read will not be stale.
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn client_read(&self) -> Result<(), ClientReadError> {
        let (tx, rx) = oneshot::channel();
        self.inner
            .tx_api
            .send(RaftMsg::ClientReadRequest { tx })
            .map_err(|_| ClientReadError::RaftError(RaftError::ShuttingDown))?;
        Ok(rx
            .await
            .map_err(|_| ClientReadError::RaftError(RaftError::ShuttingDown))
            .and_then(|res| res)?)
    }

    /// Submit a mutating client request to Raft to update the state of the system (§5.1).
    ///
    /// It will be appended to the log, committed to the cluster, and then applied to the
    /// application state machine. The result of applying the request to the state machine will
    /// be returned as the response from this method.
    ///
    /// Our goal for Raft is to implement linearizable semantics. If the leader crashes after committing
    /// a log entry but before responding to the client, the client may retry the command with a new
    /// leader, causing it to be executed a second time. As such, clients should assign unique serial
    /// numbers to every command. Then, the state machine should track the latest serial number
    /// processed for each client, along with the associated response. If it receives a command whose
    /// serial number has already been executed, it responds immediately without reexecuting the
    /// request (§8). The `RaftStorage::apply_entry_to_state_machine` method is the perfect place
    /// to implement this.
    ///
    /// These are application specific requirements, and must be implemented by the application which is
    /// being built on top of Raft.
    #[tracing::instrument(level = "debug", skip(self, rpc))]
    pub async fn client_write(&self, rpc: ClientWriteRequest<D>) -> Result<ClientWriteResponse<R>, ClientWriteError<D>> {
        let (tx, rx) = oneshot::channel();
        self.inner
            .tx_api
            .send(RaftMsg::ClientWriteRequest { rpc, tx })
            .map_err(|_| ClientWriteError::RaftError(RaftError::ShuttingDown))?;
        Ok(rx
            .await
            .map_err(|_| ClientWriteError::RaftError(RaftError::ShuttingDown))
            .and_then(|res| res)?)
    }

    /// Initialize a pristine Raft node with the given config.
    ///
    /// This command should be called on pristine nodes — where the log index is 0 and the node is
    /// in NonVoter state — as if either of those constraints are false, it indicates that the
    /// cluster is already formed and in motion. If `InitializeError::NotAllowed` is returned
    /// from this function, it is safe to ignore, as it simply indicates that the cluster is
    /// already up and running, which is ultimately the goal of this function.
    ///
    /// This command will work for single-node or multi-node cluster formation. This command
    /// should be called with all discovered nodes which need to be part of cluster, and as such
    /// it is recommended that applications be configured with an initial cluster formation delay
    /// which will allow time for the initial members of the cluster to be discovered (by the
    /// parent application) for this call.
    ///
    /// If successful, this routine will set the given config as the active config, only in memory,
    /// and will start an election.
    ///
    /// It is recommended that applications call this function based on an initial call to
    /// `RaftStorage.get_initial_state`. If the initial state indicates that the hard state's
    /// current term is `0` and the `last_log_index` is `0`, then this routine should be called
    /// in order to initialize the cluster.
    ///
    /// Once a node becomes leader and detects that its index is 0, it will commit a new config
    /// entry (instead of the normal blank entry created by new leaders).
    ///
    /// Every member of the cluster should perform these actions. This routine is race-condition
    /// free, and Raft guarantees that the first node to become the cluster leader will propagate
    /// only its own config.
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn initialize(&self, members: HashSet<NodeId>) -> Result<(), InitializeError> {
        let (tx, rx) = oneshot::channel();
        self.inner
            .tx_api
            .send(RaftMsg::Initialize { members, tx })
            .map_err(|_| RaftError::ShuttingDown)?;
        Ok(rx
            .await
            .map_err(|_| InitializeError::RaftError(RaftError::ShuttingDown))
            .and_then(|res| res)?)
    }

    /// Synchronize a new Raft node, bringing it up-to-speed (§6).
    ///
    /// Applications built on top of Raft will typically have some peer discovery mechanism for
    /// detecting when new nodes come online and need to be added to the cluster. This API
    /// facilitates the ability to request that a new node be synchronized with the leader, so
    /// that it is up-to-date and ready to be added to the cluster.
    ///
    /// Calling this API will add the target node as a non-voter, starting the syncing process.
    /// Once the node is up-to-speed, this function will return. It is the responsibility of the
    /// application to then call `change_membership` once all of the new nodes are synced.
    ///
    /// If this Raft node is not the cluster leader, then this call will fail.
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn add_non_voter(&self, id: NodeId) -> Result<(), ChangeConfigError> {
        let (tx, rx) = oneshot::channel();
        self.inner
            .tx_api
            .send(RaftMsg::AddNonVoter { id, tx })
            .map_err(|_| RaftError::ShuttingDown)?;
        Ok(rx
            .await
            .map_err(|_| ChangeConfigError::RaftError(RaftError::ShuttingDown))
            .and_then(|res| res)?)
    }

    /// Propose a cluster configuration change (§6).
    ///
    /// This will cause the leader to begin a cluster membership configuration change. If there
    /// are new nodes in the proposed config which are not already registered as non-voters — from
    /// an earlier call to `add_non_voter` — then the new nodes will first be synced as non-voters
    /// before moving the cluster into joint consensus. As this process may take some time, it is
    /// recommended that `add_non_voter` be called first for new nodes, and then once all new nodes
    /// have been synchronized, call this method to start reconfiguration.
    ///
    /// If this Raft node is not the cluster leader, then the proposed configuration change will be
    /// rejected.
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn change_membership(&self, members: HashSet<NodeId>) -> Result<(), ChangeConfigError> {
        let (tx, rx) = oneshot::channel();
        self.inner
            .tx_api
            .send(RaftMsg::ChangeMembership { members, tx })
            .map_err(|_| RaftError::ShuttingDown)?;
        Ok(rx
            .await
            .map_err(|_| ChangeConfigError::RaftError(RaftError::ShuttingDown))
            .and_then(|res| res)?)
    }

    /// Get a handle to the metrics channel.
    pub fn metrics(&self) -> watch::Receiver<RaftMetrics> {
        self.inner.rx_metrics.clone()
    }

    /// Shutdown this Raft node.
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        if let Some(tx) = self.inner.tx_shutdown.lock().await.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.inner.raft_handle.lock().await.take() {
            let _ = handle.await?;
        }
        Ok(())
    }
}

impl<D: AppData, R: AppDataResponse, N: RaftNetwork<D>, S: RaftStorage<D, R>> Clone for Raft<D, R, N, S> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}
