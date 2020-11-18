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

fn alt_main() {
    let args = env::args().collect::<Vec<_>>();
    println!("{:?}", args);
    let file_name = if let Some(name) = args.get(1) {
        name
    } else {
        println!("provide a file name");
        exit(1);
    };
    let gfa_parser: GFAParser<usize, ()> = GFAParser::new();
    let gfa = gfa_parser.parse_file(file_name).unwrap();

    println!("parsing GFA");
    let mut graph: PackedGraph = Default::default();

    println!("Adding nodes");
    for segment in gfa.segments.iter() {
        assert!(segment.name > 0);
        let seq = &segment.sequence;
        graph.create_handle(seq, segment.name);
    }

    println!("Adding edges");
    for link in gfa.links.iter() {
        let left = Handle::new(link.from_segment, link.from_orient);
        let right = Handle::new(link.from_segment, link.from_orient);
        graph.create_edge(Edge(left, right));
    }

    // count = 0;
    println!("Adding paths");
    let path_index_ids = gfa
        .paths
        .iter()
        .enumerate()
        .map(|(ix, path)| {
            let path_id = graph.create_path(&path.path_name, false);
            (path_id, ix)
        })
        .collect::<HashMap<_, _>>();

    graph.with_all_paths_mut_ctx_chn(|path_id, path_ref| {
        let ix = path_index_ids.get(&path_id).unwrap();
        gfa.paths[*ix]
            .iter()
            .map(|(node, orient)| {
                let handle = Handle::new(node as u64, orient);
                path_ref.append_step(handle)
            })
            .collect()
    });

    println!("final space: {}", graph.total_bytes());
}

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
