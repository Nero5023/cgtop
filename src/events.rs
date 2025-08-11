/// Events sent to the main thread, inspired by Bottom's event system
#[derive(Debug)]
pub enum CGroupEvent {
    /// Terminal resize event
    Resize,
    /// Key input event  
    KeyInput(crossterm::event::KeyEvent),
    /// Mouse input event
    MouseInput(crossterm::event::MouseEvent),
    /// Data update from collection thread
    Update(Box<crate::collection::CGroupMetrics>),
    /// Clean old data
    Clean,
    /// Terminate the application
    Terminate,

    // Dummy event to trigger a data update, TODO: remove this once we move collection to a separate thread
    UpdateDummy,
}

impl CGroupEvent {
    /// Check if this event should cause the application to terminate
    pub fn is_terminate(&self) -> bool {
        matches!(self, CGroupEvent::Terminate)
    }

    /// Check if this is a key event matching the given key code
    pub fn is_key(&self, key_code: crossterm::event::KeyCode) -> bool {
        if let CGroupEvent::KeyInput(key_event) = self {
            key_event.code == key_code
        } else {
            false
        }
    }

    /// Check if this is a quit key (q or Esc)
    pub fn is_quit_key(&self) -> bool {
        self.is_key(crossterm::event::KeyCode::Char('q'))
            || self.is_key(crossterm::event::KeyCode::Esc)
    }
}
