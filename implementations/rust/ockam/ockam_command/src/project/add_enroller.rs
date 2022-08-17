use clap::Args;

use ockam::Context;
use ockam_api::cloud::project::Enroller;

use crate::node::NodeOpts;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::{stop_node, CommandGlobalOpts};

#[derive(Clone, Debug, Args)]
pub struct AddEnrollerCommand {
    /// Id of the project.
    #[clap(display_order = 1001)]
    pub project_id: String,

    /// Identity id to add as an authorized enroller.
    #[clap(display_order = 1002)]
    pub enroller_identity_id: String,

    /// Description of this enroller, optional.
    #[clap(display_order = 1003)]
    pub description: Option<String>,

    // TODO: add project_name arg that conflicts with project_id
    //  so we can call the get_project_by_name api method
    // /// Name of the project.
    // #[clap(display_order = 1002)]
    // pub project_name: String,
    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(flatten)]
    pub cloud_opts: CloudOpts,
}

impl AddEnrollerCommand {
    pub fn run(opts: CommandGlobalOpts, cmd: AddEnrollerCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AddEnrollerCommand),
) -> crate::Result<()> {
    let res = run_impl(&mut ctx, opts, cmd).await;
    stop_node(ctx).await?;
    res
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: AddEnrollerCommand,
) -> crate::Result<()> {
    let mut rpc = Rpc::new(ctx, &opts, &cmd.node_opts.api_node)?;
    rpc.request(api::project::add_enroller(&cmd)).await?;
    rpc.print_response::<Enroller>()
}