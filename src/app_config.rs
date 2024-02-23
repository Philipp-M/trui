#[cfg(not(any(test, doctest, feature = "doctests")))]
use std::io::stdout;

#[cfg(not(any(test, doctest, feature = "doctests")))]
use ratatui::backend::CrosstermBackend;

#[cfg(any(test, doctest, feature = "doctests"))]
use ratatui::backend::TestBackend;

use ratatui::Terminal;

#[cfg(not(any(test, doctest, feature = "doctests")))]
use std::io::Stdout;

/// Configuration to create a customized [`App`](crate::App) instance
pub struct AppConfig {
    /// The terminal used when run from unit or integration tests
    #[cfg(any(test, doctest, feature = "doctests"))]
    pub(crate) terminal: Terminal<TestBackend>,

    /// The terminal backend use to render the output to
    #[cfg(not(any(test, doctest, feature = "doctests")))]
    pub(crate) terminal: Terminal<CrosstermBackend<Stdout>>,

    runtime: RuntimeOrHandle,
}

impl AppConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Provide a custom backend to render the output to
    #[cfg(not(any(test, doctest, feature = "doctests")))]
    pub fn with_backend(mut self, backend: CrosstermBackend<Stdout>) -> Self {
        self.terminal = Terminal::new(backend).unwrap();
        self
    }

    /// Provide a custom backend to render the output to
    #[cfg(any(test, doctest, feature = "doctests"))]
    pub fn with_backend(mut self, backend: TestBackend) -> Self {
        self.terminal = Terminal::new(backend).unwrap();
        self
    }

    #[cfg(any(test, doctest, feature = "doctests"))]
    pub(crate) fn terminal_mut(&mut self) -> &mut Terminal<TestBackend> {
        &mut self.terminal
    }

    /// Returns a [`Handle`](tokio::runtime::Handle) referring to the configured
    /// [`Runtime`](tokio::runtime::Runtime)
    ///
    /// If a runtime was explicitely provided, then a handle for this runtime,
    /// otherwise a handle for the runtime found in the context is returned.
    /// If no runtime was provided or found, a new runtime instance was created
    /// and the returned handle is for this runtime.
    pub fn runtime_handle(&self) -> tokio::runtime::Handle {
        match &self.runtime {
            RuntimeOrHandle::Runtime(rt) => rt.handle().clone(),
            RuntimeOrHandle::Handle(h) => h.clone(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        #[cfg(not(any(test, doctest, feature = "doctests")))]
        let backend = CrosstermBackend::new(stdout()); // TODO handle errors...

        #[cfg(any(test, doctest, feature = "doctests"))]
        let backend = TestBackend::new(80, 40);

        let terminal = Terminal::new(backend).unwrap();

        let runtime = match tokio::runtime::Handle::try_current() {
            Ok(handle) => RuntimeOrHandle::Handle(handle),
            Err(_) => RuntimeOrHandle::Runtime(tokio::runtime::Runtime::new().unwrap()),
        };

        Self { terminal, runtime }
    }
}

enum RuntimeOrHandle {
    Runtime(tokio::runtime::Runtime),
    Handle(tokio::runtime::Handle),
}
