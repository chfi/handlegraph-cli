use tui_handlegraph::{
    interface::{LoadGFAMsg, LoadGFAView},
    io::load_gfa,
};

use tokio::{io, sync::mpsc, time::sleep};

use std::env;
use std::process::exit;

use succinct::SpaceUsage;

use gfa::{gfa::GFA, parser::GFAParser};

use handlegraph::{
    handle::{Edge, Handle, NodeId},
    handlegraph::HandleGraphRef,
    mutablehandlegraph::*,
    pathhandlegraph::*,
    // pathgraph::PathHandleGraph,
};

use handlegraph::packedgraph::{
    PackedGraph, PackedGraphPaths, PackedPath, StepUpdate,
};

use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let args = env::args().collect::<Vec<_>>();
    println!("{:?}", args);
    let file_name = if let Some(name) = args.get(1) {
        name
    } else {
        println!("provide a file name");
        exit(1);
    };
    // let file_name = "lil.gfa";
    let (send, recv) = mpsc::channel::<LoadGFAMsg>(10000);
    let mut view = LoadGFAView::new(&file_name);

    tokio::spawn(async move {
        let mut sout = std::io::stdout();
        view.render_loop(&mut sout, recv).await;
    });

    let graph = load_gfa(&file_name, send).await;

    if let Ok(graph) = graph {
        println!("\n\ngraph loaded");
    }

    // sleep(std::time::Duration::from_secs(5)).await;
}
