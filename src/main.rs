#[allow(unused_imports)]
use handlegraph_cli::{
    interface::{LoadGFAMsg, LoadGFAView},
    io::packed_graph_from_mmap,
    mmap_gfa::{LineIndices, LineType, MmapGFA},
};

use std::env;
use std::process::exit;

use anyhow::Result;

#[allow(unused_imports)]
use succinct::SpaceUsage;

#[allow(unused_imports)]
use handlegraph::{
    handle::{Direction, Edge, Handle, NodeId},
    handlegraph::*,
    mutablehandlegraph::*,
    packed::*,
    pathhandlegraph::*,
};

#[allow(unused_imports)]
use handlegraph::hashgraph::HashGraph;
#[allow(unused_imports)]
use handlegraph::packedgraph::PackedGraph;

#[allow(unused_imports)]
use bstr::{ByteSlice, ByteVec, B};

#[allow(unused_imports)]
use rayon::prelude::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace};

fn main() -> Result<()> {
    let mut builder = pretty_env_logger::formatted_builder();
    // builder.filter_level(log::LevelFilter::Info);
    builder.filter_level(log::LevelFilter::Debug);
    builder.init();

    let args = env::args().collect::<Vec<_>>();
    let file_name = if let Some(name) = args.get(1) {
        name
    } else {
        eprintln!("provide a file name");
        exit(1);
    };

    let cons_jump_max = args.get(2).and_then(|arg| arg.parse::<usize>().ok());

    let mut mmap_gfa = MmapGFA::new(file_name)?;

    eprintln!("parsing GFA");
    let graph = packed_graph_from_mmap(&mut mmap_gfa)?;
    eprintln!("PackedGraph constructed");

    /*
    // `handles()` comes from the `handlegraph::IntoHandles` trait,
    // and iterates through the graph's handles in an
    // implementation-specific order
    println!("Handles");
    for handle in graph.handles() {
        let id = handle.id();

        // `sequence()` comes from the `handlegraph::IntoSequences`
        // trait, and returns an iterator over the bases in the
        // sequence as `u8`s. We collect it into a Vec<u8> (could also
        // have used the `sequence_vec` method which does that for us)
        let seq = graph.sequence(handle).collect::<Vec<_>>();

        // The `.as_bstr()` method comes from `bstr`'s `ByteSlice`
        // trait. It casts a `&[u8]` into a `BStr`; `BStr` is a
        // newtype wrapper over `&[u8]` that implements `Display`,
        // which lets us print byteslices without having to first
        // transform them into a `&str`.
        println!("{} - {}", id, seq.as_bstr());
    }

    // `neighbors()` comes from the `handlegraph::IntoNeighbors` trait
    // and returns an iterator over the adjacent handles of a given
    // handle, in the specified direction
    println!("Neighbors");
    for handle in graph.handles() {
        println!("  Neighbors of {}", handle.id());
        for left in graph.neighbors(handle, Direction::Left) {
            println!("  {:^5} <- {:<5}", left.id(), handle.id());
        }
        for right in graph.neighbors(handle, Direction::Right) {
            println!("        {:^5} -> {:<5}", handle.id(), right.id());
        }
    }

    // Right now the only public parallel path interface is
    // `with_all_paths_mut_ctx`, which is quite limited at the moment
    // -- the closure it takes is an `Fn(..)`, not an `FnMut(..)`, so
    // there's no way to use it to update any state, other than the
    // paths themselves
    graph.with_all_paths_mut_ctx(|path_id, path_ref| {
        // we use Write to build the string before printing it all at
        // once, so the output doesn't get jumbled due to concurrent
        // printing
        use std::fmt::Write;
        // i like prettily structured output~~
        let mut to_print = String::from("Path ");
        write!(to_print, "{:<9}", format!("{}: ", path_id.0)).unwrap();

        for (ix, step) in path_ref.steps().enumerate() {
            if ix != 0 {
                write!(to_print, ", ").unwrap();
            }
            let id = step.handle().id();
            let orient = if step.handle().is_reverse() { "-" } else { "+" };
            write!(to_print, "{}{}", id, orient).unwrap();
        }
        println!("{}", to_print);
        // the way `with_all_paths_mut_ctx` currently works is that
        // the closure must produce a list of changes to apply to the
        // node occurrences... so this is hacky but w/e
        Vec::new()
    });

    // The serial, immutable path iterator comes from the
    // `pathhandlegraph::embedded_paths::IntoPathIds` trait, using the
    // `path_ids` method

    // We can use rayon's ParallelBridge to transform this serial
    // iterator into a parallel one
    let path_lengths = graph
        .path_ids()
        .par_bridge()
        .filter_map(|path_id| {
            // `get_path_ref` returns a shared (immutable) reference
            // to a path, wrapped in an `Option<_>`. We use
            // `filter_map` together with the `?` syntax to neatly
            // unwrap it

            // there's no way `get_path_ref` will return `None` in
            // this context, but AFAIK this way makes it easier for
            // the compiler to optimize, since using `filter_map` and
            // `?` makes it impossible for a panic to occur, compared
            // to if we would use `unwrap()`
            let path_ref = graph.get_path_ref(path_id)?;
            Some(path_ref.steps().count())
        })
        .collect::<Vec<_>>();

    */

    eprintln!("input graph");
    eprintln!("  length: {}", graph.total_length());
    eprintln!("  nodes:  {}", graph.node_count());
    eprintln!("  edges:  {}", graph.edge_count());
    eprintln!("  paths:  {}", graph.path_count());

    eprintln!();

    eprintln!("getting path names");

    let mut cons_path_names = Vec::with_capacity(graph.path_count());

    let mut buf: Vec<u8> = Vec::with_capacity(256);
    for path_id in graph.path_ids() {
        buf.clear();
        if let Some(name_iter) = graph.get_path_name(path_id) {
            buf.extend(name_iter);
            if buf.starts_with(b"Consensus") {
                let mut new_buf = Vec::with_capacity(buf.capacity());
                std::mem::swap(&mut buf, &mut new_buf);
                new_buf.shrink_to_fit();
                cons_path_names.push(new_buf);
            }
        }
    }

    let cons_jump_max = cons_jump_max.unwrap_or_else(|| 10);
    let cons_jump_limit = cons_jump_max * 10;

    eprintln!("starting consensus");
    let consensus = handlegraph::consensus::create_consensus_graph(
        &graph,
        &cons_path_names,
        cons_jump_max,
        cons_jump_limit,
    );

    let mut stdout = std::io::stdout();

    handlegraph::conversion::write_as_gfa(&consensus, &mut stdout)?;

    eprintln!();

    eprintln!("consensus graph");
    eprintln!("  length: {}", consensus.total_length());
    eprintln!("  nodes:  {}", consensus.node_count());
    eprintln!("  edges:  {}", consensus.edge_count());
    eprintln!("  paths:  {}", consensus.path_count());

    Ok(())
}
