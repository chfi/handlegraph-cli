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

use bstr::{ByteSlice, ByteVec, B};

use rayon::prelude::*;

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    let file_name = if let Some(name) = args.get(1) {
        name
    } else {
        eprintln!("provide a file name");
        exit(1);
    };

    let mut mmap_gfa = MmapGFA::new(file_name)?;

    eprintln!("parsing GFA");
    let mut graph = packed_graph_from_mmap(&mut mmap_gfa)?;
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

    println!("graph stats");
    let length = graph.total_length();

    println!("  length: {}", length);
    println!("  nodes:  {}", graph.node_count());
    println!("  edges:  {}", graph.edge_count());
    println!("  paths:  {}", graph.path_count());

    // println!(
    //     "  total path steps: {}",
    //     path_lengths.into_iter().sum::<usize>()
    // );

    let graph_path_names = graph
        .path_ids()
        .filter_map(|path| graph.get_path_name_vec(path))
        .collect::<Vec<_>>();

    let cons_path_names = graph_path_names;

    let consensus = handlegraph::consensus::create_consensus_graph(
        &graph,
        &cons_path_names,
        10,
    );

    println!("  consensus graph");
    println!("  nodes:  {}", consensus.node_count());
    println!("  edges:  {}", consensus.edge_count());
    println!("  paths:  {}", consensus.path_count());

    // let cons_gfa = handlegraph::conversion::to_gfa(&consensus);

    Ok(())
}
