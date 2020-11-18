use crossterm::{
    cursor,
    event::{Event, EventStream, KeyCode},
    execute, style, terminal,
    terminal::ClearType,
};

use tokio::{io, sync::mpsc};

pub struct LoadGFAView {
    file_name: String,
    seconds_elapsed: usize,
    nodes_added: usize,
    edges_added: usize,
    paths_added: usize,
    // events_input:
}
