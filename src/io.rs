use handlegraph::{
    handle::{Edge, Handle},
    mutablehandlegraph::*,
    pathhandlegraph::*,
};

use handlegraph::packedgraph::PackedGraph;

use succinct::SpaceUsage;

use gfa::gfa::Line;

use anyhow::Result;

use fxhash::FxHashMap;

#[allow(unused_imports)]
use crate::{
    interface::{LoadGFAMsg, LoadGFAView},
    mmap_gfa::{LineIndices, LineType, MmapGFA},
};

#[allow(unused_imports)]
use tokio::{
    fs::File,
    io::{
        self, AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncSeek,
        AsyncSeekExt, BufReader,
    },
    sync::mpsc,
    time::sleep,
};

pub fn packed_graph_from_mmap(mmap_gfa: &mut MmapGFA) -> Result<PackedGraph> {
    let indices = mmap_gfa.build_index()?;

    // let mut graph =
    //     PackedGraph::with_expected_node_count(indices.segments.len());

    let mut graph = PackedGraph::default();
    eprintln!("empty space usage: {} bytes", graph.total_bytes());

    let mut min_id = std::usize::MAX;
    let mut max_id = 0;

    for &offset in indices.segments.iter() {
        let _line = mmap_gfa.read_line_at(offset.0)?;
        let name = mmap_gfa.current_line_name().unwrap();
        let name_str = std::str::from_utf8(name).unwrap();
        let id = name_str.parse::<usize>().unwrap();

        min_id = id.min(min_id);
        max_id = id.max(max_id);
    }

    let id_offset = if min_id == 0 { 1 } else { 0 };

    eprintln!("adding nodes");
    for &offset in indices.segments.iter() {
        let _line = mmap_gfa.read_line_at(offset.0)?;
        let segment = mmap_gfa.parse_current_line()?;

        if let gfa::gfa::Line::Segment(segment) = segment {
            let id = (segment.name + id_offset) as u64;
            graph.create_handle(&segment.sequence, id);
        }
    }
    eprintln!(
        "after segments - space usage: {} bytes",
        graph.total_bytes()
    );

    eprintln!("adding edges");

    let edges_iter = indices.links.iter().filter_map(|&offset| {
        let _line = mmap_gfa.read_line_at(offset).ok()?;
        let link = mmap_gfa.parse_current_line().ok()?;

        if let gfa::gfa::Line::Link(link) = link {
            let from_id = (link.from_segment + id_offset) as u64;
            let to_id = (link.to_segment + id_offset) as u64;

            let from = Handle::new(from_id, link.from_orient);
            let to = Handle::new(to_id, link.to_orient);
            Some(Edge(from, to))
        } else {
            None
        }
    });

    graph.create_edges_iter(edges_iter);

    /*
    for &offset in indices.links.iter() {
        let _line = mmap_gfa.read_line_at(offset)?;
        let link = mmap_gfa.parse_current_line()?;

        if let gfa::gfa::Line::Link(link) = link {
            let from_id = (link.from_segment + id_offset) as u64;
            let to_id = (link.to_segment + id_offset) as u64;

            let from = Handle::new(from_id, link.from_orient);
            let to = Handle::new(to_id, link.to_orient);
            graph.create_edge(Edge(from, to));
        }
    }
    */
    eprintln!(
        "after edges    - space usage: {} bytes",
        graph.total_bytes()
    );

    let mut path_ids: FxHashMap<PathId, (usize, usize)> = FxHashMap::default();
    path_ids.reserve(indices.paths.len());

    eprintln!("adding paths");
    for &offset in indices.paths.iter() {
        let line = mmap_gfa.read_line_at(offset)?;
        let length = line.len();
        if let Some(path_name) = mmap_gfa.current_line_name() {
            let path_id = graph.create_path(path_name, false).unwrap();
            path_ids.insert(path_id, (offset, length));
        }
    }

    eprintln!("created path handles");

    let mmap_gfa_bytes = mmap_gfa.get_ref();

    let parser = mmap_gfa.get_parser();

    graph.with_all_paths_mut_ctx_chn_new(|path_id, sender, path_ref| {
        let &(offset, length) = path_ids.get(&path_id).unwrap();
        let end = offset + length;
        let line = &mmap_gfa_bytes[offset..end];
        if let Some(Line::Path(path)) = parser.parse_gfa_line(line).ok() {
            path_ref.append_handles_iter_chn(
                sender,
                path.iter().map(|(node, orient)| {
                    let node = node + id_offset;
                    Handle::new(node, orient)
                }),
            );
        }
    });

    /*
    graph.with_all_paths_mut_ctx_chn(|path_id, path_ref| {
        let &(offset, length) = path_ids.get(&path_id).unwrap();
        let end = offset + length;
        let line = &mmap_gfa_bytes[offset..end];
        if let Some(Line::Path(path)) = parser.parse_gfa_line(line).ok() {
            path_ref.append_steps_iter(path.iter().map(|(node, orient)| {
                let node = node + id_offset;
                Handle::new(node, orient)
            }))
        } else {
            Vec::new()
        }
    });
    */

    eprintln!(
        "after paths    - space usage: {} bytes",
        graph.total_bytes()
    );

    Ok(graph)
}

/*
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

    let paths: HashMap<PathId, gfa_types::Path<usize, ()>> = HashMap::default();

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

pub fn make_diagnostics_dir<P: AsRef<std::path::Path>>(
    gfa_path: P,
) -> std::io::Result<std::path::PathBuf> {
    use std::fs::DirBuilder;
    use std::path::PathBuf;

    let mut dir_path = PathBuf::new();

    let stem = gfa_path
        .as_ref()
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap();

    dir_path.push(stem);
    DirBuilder::new().create(&dir_path)?;

    Ok(dir_path)
}

pub fn node_diagnostics_path(
    dir: &std::path::Path,
    id: &str,
) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    let mut path: PathBuf = dir.to_owned();

    let file_name = format!("nodes.{}.csv", id);
    path.push(file_name);

    Some(path)
}

pub fn edge_diagnostics_path(
    dir: &std::path::Path,
    id: &str,
) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    let mut path: PathBuf = dir.to_owned();

    let file_name = format!("edges.{}.csv", id);
    path.push(file_name);

    Some(path)
}
*/

/*
pub fn packed_graph_diagnostics(
    gfa_path: &str,
    mmap_gfa: &mut MmapGFA,
) -> Result<()> {
    let dir = make_diagnostics_dir(gfa_path)?;

    let mut graph = PackedGraph::default();
    let indices = mmap_gfa.build_index()?;

    let diag_frequency = if indices.links.len() < 10 {
        2
    } else {
        indices.links.len() / 10
    };

    for &offset in indices.segments.iter() {
        let _line = mmap_gfa.read_line_at(offset.0)?;
        let segment = mmap_gfa.parse_current_line()?;

        if let gfa::gfa::Line::Segment(segment) = segment {
            graph.create_handle(&segment.sequence, segment.name as u64);
        }
    }

    for (i, &offset) in indices.links.iter().enumerate() {
        let _line = mmap_gfa.read_line_at(offset)?;
        let link = mmap_gfa.parse_current_line()?;

        if let gfa::gfa::Line::Link(link) = link {
            let from = Handle::new(link.from_segment as u64, link.from_orient);
            let to = Handle::new(link.to_segment as u64, link.to_orient);
            graph.create_edge(Edge(from, to));
        }

        if i % diag_frequency == 0 {
            let node_path =
                node_diagnostics_path(&dir, &i.to_string()).unwrap();
            let node_path = node_path.to_str().unwrap();
            // graph.nodes.save_diagnostics(node_path)?;

            let edge_path =
                edge_diagnostics_path(&dir, &i.to_string()).unwrap();
            let edge_path = edge_path.to_str().unwrap();
            // graph.edges.save_diagnostics(edge_path)?;
        }
    }

    let node_path = node_diagnostics_path(&dir, "final").unwrap();
    let node_path = node_path.to_str().unwrap();
    // graph.nodes.save_diagnostics(node_path)?;

    let edge_path = edge_diagnostics_path(&dir, "final").unwrap();
    let edge_path = edge_path.to_str().unwrap();

    // graph.edges.save_diagnostics(edge_path)?;

    eprintln!("final space usage: {} bytes", graph.total_bytes());

    Ok(())
}
*/
