#[cfg(not(any(test, doctest, feature = "doctests")))]
use std::io::stdout;

#[cfg(not(any(test, doctest, feature = "doctests")))]
use ratatui::backend::CrosstermBackend;

#[cfg(any(test, doctest, feature = "doctests"))]
use ratatui::backend::TestBackend;

use ratatui::Terminal;

#[cfg(not(any(test, doctest, feature = "doctests")))]
use std::io::Stdout;

pub struct AppConfig {
    #[cfg(any(test, doctest, feature = "doctests"))]
    pub(crate) terminal: Terminal<TestBackend>,

    #[cfg(not(any(test, doctest, feature = "doctests")))]
    pub(crate) terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl AppConfig {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(not(any(test, doctest, feature = "doctests")))]
    pub fn with_backend(mut self, backend: CrosstermBackend<Stdout>) -> Self {
        self.terminal = Terminal::new(backend).unwrap();
        self
    }

    #[cfg(any(test, doctest, feature = "doctests"))]
    pub fn with_backend(mut self, backend: TestBackend) -> Self {
        self.terminal = Terminal::new(backend).unwrap();
        self
    }

    #[cfg(any(test, doctest, feature = "doctests"))]
    pub(crate) fn terminal_mut(&mut self) -> &mut Terminal<TestBackend> {
        &mut self.terminal
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        #[cfg(not(any(test, doctest, feature = "doctests")))]
        let backend = CrosstermBackend::new(stdout()); // TODO handle errors...

        #[cfg(any(test, doctest, feature = "doctests"))]
        let backend = TestBackend::new(80, 40);

        let terminal = Terminal::new(backend).unwrap();

        Self { terminal }
    }
}
