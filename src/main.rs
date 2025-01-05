use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rodio::{Decoder, OutputStream, Sink};
use std::{fs::File, io::BufReader, path::PathBuf, sync::Arc, time::Duration};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

struct App {
    songs: Vec<PathBuf>,
    current_song: Option<usize>,
    playing: bool,
    sink: Option<Sink>,
    stream: Option<OutputStream>,
    stream_handle: Option<Arc<rodio::OutputStreamHandle>>,
}

impl App {
    fn new() -> Result<Self> {
        // Initialize audio stream at startup
        let (stream, stream_handle) = OutputStream::try_default()?;
        Ok(Self {
            songs: Vec::new(),
            current_song: None,
            playing: false,
            sink: None,
            stream: Some(stream),
            stream_handle: Some(Arc::new(stream_handle)),
        })
    }

    fn play(&mut self) -> Result<()> {
        if let Some(index) = self.current_song {
            if let Some(song_path) = self.songs.get(index) {
                // Stop current playback if any
                if let Some(sink) = self.sink.take() {
                    sink.stop();
                }

                // Create new sink and start playback
                if let Some(handle) = &self.stream_handle {
                    let file = BufReader::new(File::open(song_path)?);
                    let source = Decoder::new(file)?;
                    let sink = Sink::try_new(&**handle)?;
                    sink.append(source);
                    sink.play();
                    self.sink = Some(sink);
                    self.playing = true;
                }
            }
        }
        Ok(())
    }

    fn pause(&mut self) {
        if let Some(sink) = &self.sink {
            if self.playing {
                sink.pause();
            } else {
                sink.play();
            }
            self.playing = !self.playing;
        }
    }

    fn next_song(&mut self) {
        if !self.songs.is_empty() {
            self.current_song = Some(match self.current_song {
                Some(current) => (current + 1) % self.songs.len(),
                None => 0,
            });
        }
    }

    fn previous_song(&mut self) {
        if !self.songs.is_empty() {
            self.current_song = Some(match self.current_song {
                Some(current) => (current + self.songs.len() - 1) % self.songs.len(),
                None => 0,
            });
        }
    }

    fn select_first_song(&mut self) {
        if !self.songs.is_empty() && self.current_song.is_none() {
            self.current_song = Some(0);
        }
    }
}

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new()?;

    // Example: Add some songs to the playlist
    app.songs.push(PathBuf::from("src/Mixdown_toska(6).mp3"));
    app.songs.push(PathBuf::from("src/forlorad for alltid.mp3"));
    app.select_first_song();

    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: tui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(f.size());

            // Now Playing
            let now_playing = if let Some(current) = app.current_song {
                if let Some(path) = app.songs.get(current) {
                    format!(
                        "Now Playing: {} [{}]",
                        path.file_name().unwrap().to_string_lossy(),
                        if app.playing { "Playing" } else { "Paused" }
                    )
                } else {
                    String::from("No song selected")
                }
            } else {
                String::from("No song selected")
            };

            let current_status = Paragraph::new(now_playing)
                .block(Block::default().borders(Borders::ALL).title("Status"));
            f.render_widget(current_status, chunks[0]);

            // Playlist
            let songs: Vec<ListItem> = app
                .songs
                .iter()
                .enumerate()
                .map(|(i, path)| {
                    let content = if Some(i) == app.current_song {
                        vec![Spans::from(vec![
                            Span::raw("▶ "),
                            Span::styled(
                                path.file_name().unwrap().to_string_lossy(),
                                Style::default().add_modifier(Modifier::BOLD),
                            ),
                        ])]
                    } else {
                        vec![Spans::from(vec![
                            Span::raw("  "),
                            Span::raw(path.file_name().unwrap().to_string_lossy()),
                        ])]
                    };
                    ListItem::new(content)
                })
                .collect();

            let songs = List::new(songs)
                .block(Block::default().borders(Borders::ALL).title("Playlist"))
                .highlight_style(Style::default().fg(Color::Yellow));

            f.render_widget(songs, chunks[1]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char(' ') => app.pause(),
                    KeyCode::Right => {
                        app.next_song();
                        app.play()?;
                    }
                    KeyCode::Left => {
                        app.previous_song();
                        app.play()?;
                    }
                    KeyCode::Enter => {
                        app.play()?;
                    }
                    _ => {}
                }
            }
        }
    }
}
