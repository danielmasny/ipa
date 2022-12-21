#![allow(dead_code)] // TODO: remove
#![allow(clippy::mutable_key_type)] // `HelperIdentity` cannot be modified

pub mod http;

mod error;

pub use error::Error;

use crate::helpers::Role;
use crate::net::ByteArrStream;
use crate::{
    helpers::{network::MessageChunks, HelperIdentity},
    protocol::QueryId,
};
use async_trait::async_trait;
use futures::Stream;
use std::collections::HashMap;
use tokio::sync::oneshot;

pub trait TransportCommandData {
    type RespData;
    fn name() -> &'static str;
    fn respond(self, query_id: QueryId, data: Self::RespData) -> Result<(), Error>;
}

#[derive(Debug)]
pub struct CreateQueryData {
    pub field_type: String,
    pub helper_positions: [HelperIdentity; 3],
    callback: oneshot::Sender<(QueryId, <Self as TransportCommandData>::RespData)>,
}

impl CreateQueryData {
    #[must_use]
    pub fn new(
        field_type: String,
        helper_positions: [HelperIdentity; 3],
        callback: oneshot::Sender<(QueryId, <Self as TransportCommandData>::RespData)>,
    ) -> Self {
        CreateQueryData {
            field_type,
            helper_positions,
            callback,
        }
    }
}

impl TransportCommandData for CreateQueryData {
    type RespData = HelperIdentity;
    fn name() -> &'static str {
        "CreateQuery"
    }

    fn respond(self, query_id: QueryId, data: Self::RespData) -> Result<(), Error> {
        self.callback
            .send((query_id, data))
            .map_err(|_| Error::CallbackFailed {
                command_name: Self::name(),
                query_id,
            })
    }
}

#[derive(Debug)]
pub struct PrepareQueryData {
    pub query_id: QueryId,
    pub field_type: String,
    pub helper_positions: [HelperIdentity; 3],
    pub helpers_to_roles: HashMap<HelperIdentity, Role>,
    callback: oneshot::Sender<()>,
}

impl PrepareQueryData {
    #[must_use]
    pub fn new(
        query_id: QueryId,
        field_type: String,
        helper_positions: [HelperIdentity; 3],
        helpers_to_roles: HashMap<HelperIdentity, Role>,
        callback: oneshot::Sender<()>,
    ) -> Self {
        PrepareQueryData {
            query_id,
            field_type,
            helper_positions,
            helpers_to_roles,
            callback,
        }
    }
}

impl TransportCommandData for PrepareQueryData {
    type RespData = ();
    fn name() -> &'static str {
        "PrepareQuery"
    }
    fn respond(self, query_id: QueryId, _: Self::RespData) -> Result<(), Error> {
        self.callback.send(()).map_err(|_| Error::CallbackFailed {
            command_name: Self::name(),
            query_id,
        })
    }
}

#[derive(Debug)]
pub struct StartMulData {
    pub query_id: QueryId,
    pub data_stream: ByteArrStream,
    callback: oneshot::Sender<()>,
}

impl StartMulData {
    pub fn new(
        query_id: QueryId,
        data_stream: ByteArrStream,
        callback: oneshot::Sender<()>,
    ) -> Self {
        StartMulData {
            query_id,
            data_stream,
            callback,
        }
    }
}

impl TransportCommandData for StartMulData {
    type RespData = ();
    fn name() -> &'static str {
        "StartMul"
    }
    fn respond(self, query_id: QueryId, _: Self::RespData) -> Result<(), Error> {
        self.callback.send(()).map_err(|_| Error::CallbackFailed {
            command_name: Self::name(),
            query_id,
        })
    }
}

#[derive(Debug)]
pub struct MulData {
    pub query_id: QueryId,
    pub field_type: String,
    pub data_stream: ByteArrStream,
}

impl MulData {
    pub fn new(query_id: QueryId, field_type: String, data_stream: ByteArrStream) -> Self {
        Self {
            query_id,
            field_type,
            data_stream,
        }
    }
}

impl TransportCommandData for MulData {
    type RespData = ();
    fn name() -> &'static str {
        "Mul"
    }
    fn respond(self, _: QueryId, _: Self::RespData) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct NetworkEventData {
    pub query_id: QueryId,
    pub roles_to_helpers: [HelperIdentity; 3],
    pub message_chunks: MessageChunks,
}

impl NetworkEventData {
    pub fn new(
        query_id: QueryId,
        roles_to_helpers: [HelperIdentity; 3],
        message_chunks: MessageChunks,
    ) -> Self {
        Self {
            query_id,
            roles_to_helpers,
            message_chunks,
        }
    }
}

impl TransportCommandData for NetworkEventData {
    type RespData = ();
    fn name() -> &'static str {
        "NetworkEvent"
    }
    fn respond(self, _: QueryId, _: Self::RespData) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum TransportCommand {
    // `Administration` Commands

    // Helper which receives this command becomes the de facto leader of the query setup. It will:
    // * generate `query_id`
    // * assign roles to each helper, generating a role mapping `helper id` -> `role`
    // * store `query_id` -> (`context_type`, `field_type`, `secret share mapping`, `role mapping`)
    // * inform other helpers of new `query_id` and associated data
    // * respond with `query_id` and helper which should receive `Start*` command
    CreateQuery(CreateQueryData),

    // Helper which receives this message is a follower of the query setup. It will receive this
    // message from the leader, who has received the `CreateQuery` command. It will:
    // * store `query_id` -> (`context_type`, `field_type`, `secret share mapping`, `role mapping`)
    // * respond with ack
    PrepareQuery(PrepareQueryData),

    // Helper which receives this message is the leader of the mul protocol, as chosen by the leader
    // of the `CreateQuery` command. It will:
    // * retrieve (`context_type`, `field_type`, `secret share mapping`, `role mapping`)
    // * assign `Transport` using `secret share mapping` and `role mapping`
    // * break apart incoming data into 3 different streams, 1 for each helper
    // * send 2 of the streams to other helpers
    // * run the protocol using final stream of data, `context_type`, `field_type`
    StartMul(StartMulData),

    // Helper which receives this message is a follower of the mul protocol. It will:
    // * retrieve (`context_type`, `field_type`, `secret share mapping`, `role mapping`)
    // * assign `Transport` using `secret share mapping` and `role mapping`
    // * run the protocol using incoming stream of data, `context_type`, `field_type`
    Mul(MulData),

    // `Query` Commands

    // `MessageChunks` to be sent over the network
    NetworkEvent(NetworkEventData),
}

/// Users of a [`Transport`] must subscribe to a specific type of command, and so must pass this
/// type as argument to the `subscribe` function
#[allow(dead_code)] // will use this soon
pub enum SubscriptionType {
    /// Commands for managing queries
    Administration,
    /// Commands intended for a running query
    Query(QueryId),
}

#[async_trait]
pub trait Transport: Sync {
    type CommandStream: Stream<Item = TransportCommand> + Send + Unpin + 'static;

    /// To be called by an entity which will handle the events as indicated by the
    /// [`SubscriptionType`]. There should be only 1 subscriber per type.
    /// # Panics
    /// May panic if attempt to subscribe to the same [`SubscriptionType`] twice
    fn subscribe(&self, subscription_type: SubscriptionType) -> Self::CommandStream;

    /// To be called when an entity wants to send commands to the `Transport`.
    async fn send(
        &self,
        destination: &HelperIdentity,
        command: TransportCommand,
    ) -> Result<(), Error>;
}
