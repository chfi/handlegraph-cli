use crossterm::{
    cursor,
    event::{Event, EventStream, KeyCode},
    execute, style, terminal,
    terminal::ClearType,
};

use tokio::{io, sync::mpsc};

#[derive(Debug, Default)]
pub struct LoadGFAView {
    file_name: String,
    seconds_elapsed: usize,
    nodes_added: usize,
    edges_added: usize,
    paths_added: usize,
    bytes_used: usize,
    // events_input: Option<mpsc::Receiver<LoadGFAMsg>>,
}

impl LoadGFAView {
    pub fn new(file: &str) -> Self {
        Self {
            file_name: file.to_string(),
            ..Default::default()
        }
    }

    pub(crate) fn apply_message(&mut self, msg: LoadGFAMsg) {
        match msg {
            LoadGFAMsg::Node => self.nodes_added += 1,
            LoadGFAMsg::Edge => self.edges_added += 1,
            LoadGFAMsg::Path => self.paths_added += 1,
            LoadGFAMsg::Bytes(bytes) => self.bytes_used = bytes,
            LoadGFAMsg::Done => (),
        }
    }

    // pub(crate) async fn render_loop<W: std::io::Write>(&mut self, write: &mut W) -> mpsc::Sender<LoadGFAMsg> {
    pub async fn render_loop<W: std::io::Write>(
        &mut self,
        write: &mut W,
        mut recv: mpsc::Receiver<LoadGFAMsg>,
    ) {
        // use tokio::time;

        // let mut interval = time::interval(time::Duration::from_secs(1));
        let mut instant = std::time::Instant::now();

        'render: loop {
            // let now = std::time::Instant::now();
            // interval.tick().await;
            while let Ok(msg) = recv.try_recv() {
                if msg == LoadGFAMsg::Done {
                    break 'render;
                }
                self.apply_message(msg);
            }
            if instant.elapsed().as_millis() >= 1000 {
                self.seconds_elapsed += 1;
                instant = std::time::Instant::now();
                self.render(write).unwrap();
            }
        }
    }

    pub(crate) fn render<W: std::io::Write>(
        &self,
        write: &mut W,
    ) -> crossterm::Result<()> {
        execute!(write, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
        execute!(write, cursor::MoveTo(5, 3))?;
        println!("{}", self.file_name);

        execute!(write, cursor::MoveTo(7, 6))?;
        println!("Seconds: {}", self.seconds_elapsed);

        execute!(write, cursor::MoveTo(28, 6))?;
        println!("Bytes: {}", self.bytes_used);

        execute!(write, cursor::MoveTo(8, 9))?;
        println!("Nodes: {}", self.nodes_added);

        execute!(write, cursor::MoveTo(8, 10))?;
        println!("Edges: {}", self.edges_added);

        execute!(write, cursor::MoveTo(8, 11))?;
        println!("Paths: {}", self.paths_added);

        Ok(())
    }

    // pub(crate) fn set_receiver(&mut self, recv: mpsc::Receiver<LoadGFAMsg>) {
    //     self.events_input = Some(recv);
    // }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadGFAMsg {
    Node,
    Edge,
    Path,
    Bytes(usize),
    Done,
}
