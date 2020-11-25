use handlegraph_cli::{
    interface::{LoadGFAMsg, LoadGFAView},
    io::load_gfa,
    mmap_gfa::{LineIndices, LineType, MmapGFA},
};

use tokio::{io, sync::mpsc, time::sleep};

use std::env;
use std::process::exit;

use anyhow::Result;

use succinct::SpaceUsage;

use gfa::{gfa::GFA, parser::GFAParser};

use handlegraph::{
    handle::{Edge, Handle, NodeId},
    handlegraph::HandleGraphRef,
    mutablehandlegraph::*,
    // pathgraph::PathHandleGraph,
    pathhandlegraph::*,
};

use handlegraph::packedgraph::{
    paths::{PackedGraphPaths, PackedPath, StepUpdate},
    PackedGraph,
};

use handlegraph::hashgraph::HashGraph;

use bstr::{BStr, ByteSlice, ByteVec};

use std::collections::HashMap;

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    println!("{:?}", args);
    let file_name = if let Some(name) = args.get(1) {
        name
    } else {
        println!("provide a file name");
        exit(1);
    };

    let mut mmap_gfa = MmapGFA::new(file_name)?;

    let indices = mmap_gfa.build_index()?;

    println!(" ~ Indices ~ ");
    println!("Segments");
    for &s in indices.segments.iter() {
        let line = mmap_gfa.read_line_at(s)?;
        let bstr_line = line.trim().as_bstr();
        println!("    {:>4} - {}", s, bstr_line);
    }
    println!();

    println!("Links");
    for &s in indices.links.iter() {
        let line = mmap_gfa.read_line_at(s)?;
        let bstr_line = line.trim().as_bstr();
        println!("    {:>4} - {}", s, bstr_line);
    }
    println!();

    println!("Paths");
    for &s in indices.paths.iter() {
        let line = mmap_gfa.read_line_at(s)?;
        let bstr_line = line.trim().as_bstr();
        println!("    {:>4} - {}", s, bstr_line);
    }
    println!();

    Ok(())
}

fn _main() {
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
    // let mut graph: HashGraph = Default::default();

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

    /*
    for path in gfa.paths.iter() {
        let name = &path.path_name;
        let path_id = graph.create_path_handle(name, false);
        for (seg, orient) in path.iter() {
            let handle = Handle::new(seg, orient);
            graph.append_step(&path_id, handle);
        }
    }
    */

    println!("Adding paths");
    let path_index_ids = gfa
        .paths
        .iter()
        .enumerate()
        .filter_map(|(ix, path)| {
            let path_id = graph.create_path(&path.path_name, false)?;
            Some((path_id, ix))
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

    // println!("final space: {}", graph.total_bytes());
}

#[tokio::main]
async fn __main() {
    let args = env::args().collect::<Vec<_>>();
    println!("{:?}", args);
    let file_name = if let Some(name) = args.get(1) {
        name
    } else {
        println!("provide a file name");
        exit(1);
    };
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
}
