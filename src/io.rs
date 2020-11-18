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

use succinct::SpaceUsage;

use gfa::{
    gfa as gfa_types,
    // gfa::{Line, Link, Orientation, Path, Segment, GFA},
    gfa::{Line, GFA},
    optfields::OptFields,
    parser::{GFAParser, GFAParserBuilder, GFAResult},
};

use anyhow::{bail, Result};
use bstr::{BString, ByteVec};

use crate::interface::{LoadGFAMsg, LoadGFAView};

use tokio::sync::mpsc;

pub async fn load_gfa(
    // gfa_path: &std::path::Path,
    gfa_path: &str,
    send: mpsc::Sender<LoadGFAMsg>,
) -> Result<PackedGraph> {
    use tokio::{
        fs::File,
        io::{self, AsyncBufRead, AsyncBufReadExt, AsyncRead, BufReader},
        time::sleep,
    };

    let gfa_parser: GFAParser<usize, ()> = GFAParser::new();

    let file = File::open(gfa_path).await?;
    let mut reader = BufReader::new(file);
    let mut buf = Vec::with_capacity(1024);

    let mut graph = PackedGraph::default();

    loop {
        // sleep(std::time::Duration::from_secs(1)).await;
        buf.clear();
        let res = reader.read_until(0xA, &mut buf).await?;
        if res == 0 {
            send.send(LoadGFAMsg::Done).await?;
            break;
        }

        match gfa_parser.parse_gfa_line(&buf[0..res]) {
            Ok(parsed) => match parsed {
                Line::Header(_) => (),
                Line::Segment(seg) => {
                    graph.create_handle(&seg.sequence, seg.name as u64);
                    send.send(LoadGFAMsg::Node).await?;
                }
                Line::Link(link) => {
                    let from =
                        Handle::new(link.from_segment as u64, link.from_orient);
                    let to =
                        Handle::new(link.to_segment as u64, link.to_orient);
                    graph.create_edge(Edge(from, to));
                    send.send(LoadGFAMsg::Edge).await?;
                }
                Line::Containment(_) => (),
                Line::Path(path) => {
                    let path_id = graph.create_path(&path.path_name, false);
                    graph.with_path_mut_ctx(path_id, |path_ref| {
                        path.iter()
                            .map(|(node, orient)| {
                                let handle = Handle::new(node as u64, orient);
                                path_ref.append_step(handle)
                            })
                            .collect()
                    });
                    send.send(LoadGFAMsg::Path).await?;
                }
            },
            // Err(err) if err.can_safely_continue(&self.tolerance) => (),
            Err(err) => bail!("Cannot parse GFA file: {:?}", err),
        }

        let bytes = graph.total_bytes();
        send.send(LoadGFAMsg::Bytes(bytes)).await?;
    }

    /*

    // later I'll use these to store offsets to links/paths that need
    // to be added later, rather than store the entire objects while
    // it's not necessary -- however, given the current state of
    // rs-gfa, that would require parsing these things twice, hence
    // holding off on it
    // let mut link_offsets: Vec<u64> = Vec::new();
    // let mut path_offsets: Vec<u64> = Vec::new();

    type GFALink = gfa_types::Link<usize, ()>;
    type GFAPath = gfa_types::Path<usize, ()>;

    let mut links: Vec<GFALink> = Vec::new();
    let mut paths: Vec<GFAPath> = Vec::new();

    for line in lines {
        let line = line?;

        match gfa_parser.parse_gfa_line(line.as_ref()) {
            Ok(parsed) => {
                match parsed {
                    Line::Header(head) => {
                        unimplemented!();
                    }
                    Line::Segment(seg) => {
                        unimplemented!();
                    }
                    Line::Link(seg) => {
                        unimplemented!();
                    }
                    Line::Containment(seg) => {
                        unimplemented!();
                    }
                    Line::Path(seg) => {
                        unimplemented!();
                    }
                }
                unimplemented!();
            }
            // Err(err) if err.can_safely_continue(&self.tolerance) => (),
            Err(err) => bail!("Cannot parse GFA file: {:?}", err),
        }
    }
    */

    Ok(graph)
}
