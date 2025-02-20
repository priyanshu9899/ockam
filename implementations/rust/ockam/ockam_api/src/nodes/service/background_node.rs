use crate::cli_state::{CliState, StateDirTrait, StateItemTrait};
use crate::nodes::NODEMANAGER_ADDR;
use miette::IntoDiagnostic;
use minicbor::{Decode, Encode};
use ockam_core::api::{Reply, Request};
use ockam_core::{AsyncTryClone, Route};
use ockam_node::api::Client;
use ockam_node::Context;
use ockam_transport_tcp::{TcpConnectionOptions, TcpTransport};
use std::sync::Arc;
use std::time::Duration;

/// This struct represents a node that has been started
/// on the same machine with a given node name
///
/// The methods on this struct allow a user to send requests containing a value of type `T`
/// and expect responses with a value of type `R`
#[derive(Clone)]
pub struct BackgroundNode {
    cli_state: CliState,
    node_name: String,
    to: Route,
    timeout: Option<Duration>,
    tcp_transport: Arc<TcpTransport>,
}

impl BackgroundNode {
    /// Create a new client to send requests to a running background node
    /// This function instantiates a TcpTransport. Since a TcpTransport can only be created once
    /// this function must only be called once
    pub async fn create(
        ctx: &Context,
        cli_state: &CliState,
        node_name: &str,
    ) -> miette::Result<BackgroundNode> {
        let tcp_transport = TcpTransport::create(ctx).await.into_diagnostic()?;
        BackgroundNode::new(&tcp_transport, cli_state, node_name).await
    }

    /// Create a new client to send requests to a running background node
    pub async fn new(
        tcp_transport: &TcpTransport,
        cli_state: &CliState,
        node_name: &str,
    ) -> miette::Result<BackgroundNode> {
        cli_state.nodes.get(node_name)?;
        Ok(BackgroundNode {
            cli_state: cli_state.clone(),
            node_name: node_name.to_string(),
            to: NODEMANAGER_ADDR.into(),
            timeout: None,
            tcp_transport: Arc::new(tcp_transport.async_try_clone().await.into_diagnostic()?),
        })
    }

    // Set a different node name
    pub fn set_node_name(&mut self, node_name: &str) -> &Self {
        self.node_name = node_name.to_string();
        self
    }

    /// Use a default timeout for making requests
    pub fn set_timeout(&mut self, timeout: Duration) -> &Self {
        self.timeout = Some(timeout);
        self
    }

    /// Send a request and expect a decodable response
    pub async fn ask<T, R>(&self, ctx: &Context, req: Request<T>) -> miette::Result<R>
    where
        T: Encode<()>,
        R: for<'b> Decode<'b, ()>,
    {
        self.ask_and_get_reply(ctx, req)
            .await?
            .success()
            .into_diagnostic()
    }

    /// Send a request and expect a decodable response and use a specific timeout
    pub async fn ask_with_timeout<T, R>(
        &self,
        ctx: &Context,
        req: Request<T>,
        timeout: Duration,
    ) -> miette::Result<R>
    where
        T: Encode<()>,
        R: for<'b> Decode<'b, ()>,
    {
        let client = self.make_client_with_timeout(Some(timeout)).await?;
        client
            .ask(ctx, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    /// Send a request but don't decode the response
    pub async fn tell<T>(&self, ctx: &Context, req: Request<T>) -> miette::Result<()>
    where
        T: Encode<()>,
    {
        let client = self.make_client().await?;
        client
            .tell(ctx, req)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()
    }

    /// Send a request and expect either a decodable response or an API error.
    /// This method returns an error if the request cannot be sent of if there is any decoding error
    pub async fn ask_and_get_reply<T, R>(
        &self,
        ctx: &Context,
        req: Request<T>,
    ) -> miette::Result<Reply<R>>
    where
        T: Encode<()>,
        R: for<'b> Decode<'b, ()>,
    {
        let client = self.make_client().await?;
        client.ask(ctx, req).await.into_diagnostic()
    }

    /// Make a route to the node and connect using TCP
    async fn create_route(&self) -> miette::Result<Route> {
        let mut route = self.to.clone();
        let node_state = self.cli_state.nodes.get(&self.node_name)?;
        let port = node_state.config().setup().api_transport()?.addr.port();
        let addr_str = format!("localhost:{port}");
        let addr = self
            .tcp_transport
            .connect(addr_str, TcpConnectionOptions::new())
            .await
            .into_diagnostic()?
            .sender_address()
            .clone();
        route.modify().prepend(addr);
        debug!("Sending requests to {route}");
        Ok(route)
    }

    /// Make a response / request client connected to the node
    pub async fn make_client(&self) -> miette::Result<Client> {
        self.make_client_with_timeout(self.timeout).await
    }

    /// Make a response / request client connected to the node
    /// and specify a timeout for receiving responses
    pub async fn make_client_with_timeout(
        &self,
        timeout: Option<Duration>,
    ) -> miette::Result<Client> {
        let route = self.create_route().await?;
        Ok(Client::new(&route, timeout))
    }
}
