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

use tokio::{
    fs::File,
    io::{
        self, AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncSeek,
        AsyncSeekExt, BufReader,
    },
    time::sleep,
};

use std::io::SeekFrom;

async fn read_segments(
    file: &mut File,
    send: &mut mpsc::Sender<LoadGFAMsg>,
    graph: &mut PackedGraph,
) -> Result<()> {
    let gfa_parser: GFAParser<usize, ()> = GFAParser::new();

    file.seek(SeekFrom::Start(0)).await?;

    let mut reader = BufReader::new(file);
    let mut buf = Vec::with_capacity(1024);

    loop {
        buf.clear();
        let res = reader.read_until(0xA, &mut buf).await?;
        if res == 0 {
            break;
        }

        match gfa_parser.parse_gfa_line(&buf[0..res]) {
            Ok(parsed) => match parsed {
                Line::Segment(seg) => {
                    graph.create_handle(&seg.sequence, seg.name as u64);
                    send.send(LoadGFAMsg::Node).await?;
                }
                _ => (),
            },
            Err(err) => bail!("Cannot parse GFA file: {:?}", err),
        }

        let bytes = graph.total_bytes();
        send.send(LoadGFAMsg::Bytes(bytes)).await?;
    }
    Ok(())
}

async fn read_links(
    file: &mut File,
    send: &mut mpsc::Sender<LoadGFAMsg>,
    graph: &mut PackedGraph,
) -> Result<()> {
    let gfa_parser: GFAParser<usize, ()> = GFAParser::new();

    file.seek(SeekFrom::Start(0)).await?;

    let mut reader = BufReader::new(file);
    let mut buf = Vec::with_capacity(1024);

    loop {
        buf.clear();
        let res = reader.read_until(0xA, &mut buf).await?;
        if res == 0 {
            break;
        }

        match gfa_parser.parse_gfa_line(&buf[0..res]) {
            Ok(parsed) => match parsed {
                Line::Link(link) => {
                    let from =
                        Handle::new(link.from_segment as u64, link.from_orient);
                    let to =
                        Handle::new(link.to_segment as u64, link.to_orient);
                    graph.create_edge(Edge(from, to));
                    send.send(LoadGFAMsg::Edge).await?;
                }
                _ => (),
            },
            Err(err) => bail!("Cannot parse GFA file: {:?}", err),
        }

        let bytes = graph.total_bytes();
        send.send(LoadGFAMsg::Bytes(bytes)).await?;
    }
    Ok(())
}

async fn read_paths(
    file: &mut File,
    send: &mut mpsc::Sender<LoadGFAMsg>,
    graph: &mut PackedGraph,
) -> Result<()> {
    let gfa_parser: GFAParser<usize, ()> = GFAParser::new();

    file.seek(SeekFrom::Start(0)).await?;

    let mut reader = BufReader::new(file);
    let mut buf = Vec::with_capacity(1024);

    loop {
        buf.clear();
        let res = reader.read_until(0xA, &mut buf).await?;
        if res == 0 {
            break;
        }

        match gfa_parser.parse_gfa_line(&buf[0..res]) {
            Ok(parsed) => match parsed {
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
                _ => (),
            },
            Err(err) => bail!("Cannot parse GFA file: {:?}", err),
        }

        let bytes = graph.total_bytes();
        send.send(LoadGFAMsg::Bytes(bytes)).await?;
    }

    Ok(())
}

pub async fn load_gfa(
    // gfa_path: &std::path::Path,
    gfa_path: &str,
    mut send: mpsc::Sender<LoadGFAMsg>,
) -> Result<PackedGraph> {
    let mut file = File::open(gfa_path).await?;

    let mut graph = PackedGraph::default();

    read_segments(&mut file, &mut send, &mut graph).await?;

    read_links(&mut file, &mut send, &mut graph).await?;

    read_paths(&mut file, &mut send, &mut graph).await?;

    send.send(LoadGFAMsg::Done).await?;

    Ok(graph)
}
