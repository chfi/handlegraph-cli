use handlegraph::{
    handle::{Edge, Handle, NodeId},
    handlegraph::HandleGraphRef,
    mutablehandlegraph::*,
    pathhandlegraph::*,
    // pathgraph::PathHandleGraph,
};

use handlegraph::packedgraph::{
    paths::{PackedGraphPaths, PackedPath, StepUpdate},
    PackedGraph,
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
use crate::mmap_gfa::{LineIndices, LineType, MmapGFA};

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

    use std::collections::HashMap;

    let mut paths: HashMap<PathId, gfa_types::Path<usize, ()>> =
        HashMap::default();

    loop {
        buf.clear();
        let res = reader.read_until(0xA, &mut buf).await?;
        if res == 0 {
            break;
        }

        match gfa_parser.parse_gfa_line(&buf[0..res]) {
            Ok(parsed) => match parsed {
                Line::Path(path) => {
                    send.send(LoadGFAMsg::Path).await?;
                    let path_id =
                        graph.create_path(&path.path_name, false).unwrap();
                    // paths.insert(path_id, path);

                    graph.with_path_mut_ctx(path_id, |path_ref| {
                        path.iter()
                            .map(|(node, orient)| {
                                let handle = Handle::new(node as u64, orient);
                                path_ref.append_step(handle)
                            })
                            .collect()
                    });
                }
                _ => (),
            },
            Err(err) => bail!("Cannot parse GFA file: {:?}", err),
        }

        let bytes = graph.total_bytes();
        send.send(LoadGFAMsg::Bytes(bytes)).await?;
    }

    /*
    graph.with_all_paths_mut_ctx_chn(|path_id, path_ref| {
        let gfa_path = paths.get(&path_id).unwrap();
        gfa_path
            .iter()
            .map(|(node, orient)| {
                let handle = Handle::new(node as u64, orient);
                path_ref.append_step(handle)
            })
            .collect()
    });
    */

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

pub fn packed_graph_from_mmap(mmap_gfa: &mut MmapGFA) -> Result<PackedGraph> {
    let mut graph = PackedGraph::default();

    let indices = mmap_gfa.build_index()?;

    println!("adding nodes");
    for (ix, &offset) in indices.segments.iter().enumerate() {
        if ix % (indices.segments.len() / 100).max(1) == 0 {
            println!("{:6} - {} bytes", ix, graph.total_bytes());
        }
        let _line = mmap_gfa.read_line_at(offset)?;
        let segment = mmap_gfa.parse_current_line()?;

        if let gfa::gfa::Line::Segment(segment) = segment {
            graph.create_handle(&segment.sequence, segment.name as u64);
        }
    }
    println!("space usage: {} bytes", graph.total_bytes());

    println!("adding edges");
    for &offset in indices.links.iter() {
        let _line = mmap_gfa.read_line_at(offset)?;
        let link = mmap_gfa.parse_current_line()?;

        if let gfa::gfa::Line::Link(link) = link {
            let from = Handle::new(link.from_segment as u64, link.from_orient);
            let to = Handle::new(link.to_segment as u64, link.to_orient);
            graph.create_edge(Edge(from, to));
        }
    }
    println!("space usage: {} bytes", graph.total_bytes());

    println!("adding paths");
    for &offset in indices.paths.iter() {
        let _line = mmap_gfa.read_line_at(offset)?;
        let path = mmap_gfa.parse_current_line()?;

        if let gfa::gfa::Line::Path(path) = path {
            let path_id = graph.create_path(&path.path_name, false).unwrap();

            graph.with_path_mut_ctx(path_id, |path_ref| {
                path.iter()
                    .map(|(node, orient)| {
                        let handle = Handle::new(node as u64, orient);
                        path_ref.append_step(handle)
                    })
                    .collect()
            });
        }
    }
    println!("space usage: {} bytes", graph.total_bytes());

    /*
    let mut path_ids = Vec::with_capacity(indices.paths.len());

    for &offset in indices.paths.iter() {
        let _line = mmap_gfa.read_line_at(offset)?;
        if let Some(path_name) = mmap_gfa.current_line_name() {
            let path_id = graph.create_path(path_name, false).unwrap();
            path_ids.push((path_id, offset));
        }
    }
    */

    Ok(graph)
}
