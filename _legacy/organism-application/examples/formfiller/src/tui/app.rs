use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::models::{FormConfig, Profile};
use crate::storage;

use super::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Main,
    Profiles,
    ProfileEdit,
    Forms,
    FormEdit,
    Fill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    pub screen: Screen,
    pub input_mode: InputMode,
    pub profiles: Vec<Profile>,
    pub forms: Vec<FormConfig>,
    pub selected_profile: usize,
    pub selected_form: usize,
    pub selected_field: usize,
    pub input_buffer: String,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let profiles = storage::load_profiles().unwrap_or_default();
        let forms = storage::load_form_configs().unwrap_or_default();

        Ok(Self {
            screen: Screen::Main,
            input_mode: InputMode::Normal,
            profiles,
            forms,
            selected_profile: 0,
            selected_form: 0,
            selected_field: 0,
            input_buffer: String::new(),
            status_message: None,
            should_quit: false,
        })
    }

    pub fn current_profile(&self) -> Option<&Profile> {
        self.profiles.get(self.selected_profile)
    }

    pub fn current_form(&self) -> Option<&FormConfig> {
        self.forms.get(self.selected_form)
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    pub fn save_all(&mut self) -> Result<()> {
        storage::save_profiles(&self.profiles)?;
        storage::save_form_configs(&self.forms)?;
        self.set_status("Saved!");
        Ok(())
    }

    fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::Editing => self.handle_editing_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyCode) -> Result<()> {
        match self.screen {
            Screen::Main => match key {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('p') => self.screen = Screen::Profiles,
                KeyCode::Char('f') => self.screen = Screen::Forms,
                KeyCode::Char('r') => self.screen = Screen::Fill,
                KeyCode::Char('s') => self.save_all()?,
                _ => {}
            },
            Screen::Profiles => match key {
                KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::Main,
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_profile > 0 {
                        self.selected_profile -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected_profile < self.profiles.len().saturating_sub(1) {
                        self.selected_profile += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char('e') => {
                    if !self.profiles.is_empty() {
                        self.screen = Screen::ProfileEdit;
                        self.selected_field = 0;
                    }
                }
                KeyCode::Char('n') => {
                    self.profiles.push(Profile::new("New Profile".into()));
                    self.selected_profile = self.profiles.len() - 1;
                    self.screen = Screen::ProfileEdit;
                }
                KeyCode::Char('d') => {
                    if !self.profiles.is_empty() {
                        self.profiles.remove(self.selected_profile);
                        if self.selected_profile >= self.profiles.len() && self.selected_profile > 0
                        {
                            self.selected_profile -= 1;
                        }
                    }
                }
                _ => {}
            },
            Screen::ProfileEdit => match key {
                KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::Profiles,
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_field > 0 {
                        self.selected_field -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.selected_field += 1; // Will be clamped by UI
                }
                KeyCode::Enter => {
                    self.input_mode = InputMode::Editing;
                    self.input_buffer = self.get_current_field_value();
                }
                _ => {}
            },
            Screen::Forms => match key {
                KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::Main,
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.selected_form > 0 {
                        self.selected_form -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.selected_form < self.forms.len().saturating_sub(1) {
                        self.selected_form += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char('e') => {
                    if !self.forms.is_empty() {
                        self.screen = Screen::FormEdit;
                        self.selected_field = 0;
                    }
                }
                KeyCode::Char('n') => {
                    self.forms
                        .push(FormConfig::new("New Form".into(), "https://".into()));
                    self.selected_form = self.forms.len() - 1;
                    self.screen = Screen::FormEdit;
                }
                KeyCode::Char('d') => {
                    if !self.forms.is_empty() {
                        self.forms.remove(self.selected_form);
                        if self.selected_form >= self.forms.len() && self.selected_form > 0 {
                            self.selected_form -= 1;
                        }
                    }
                }
                _ => {}
            },
            Screen::FormEdit => match key {
                KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::Forms,
                _ => {}
            },
            Screen::Fill => match key {
                KeyCode::Esc | KeyCode::Char('q') => self.screen = Screen::Main,
                KeyCode::Enter => {
                    self.set_status("Form filling triggered! (WebDriver required)");
                }
                _ => {}
            },
        }
        Ok(())
    }

    fn handle_editing_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
            }
            KeyCode::Enter => {
                self.set_current_field_value(self.input_buffer.clone());
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn get_current_field_value(&self) -> String {
        if let Some(profile) = self.profiles.get(self.selected_profile) {
            match self.selected_field {
                0 => profile.name.clone(),
                1 => profile.personal.first_name.clone(),
                2 => profile.personal.last_name.clone(),
                3 => profile.contact.email.clone(),
                4 => profile.contact.phone.clone(),
                5 => profile.address.street.clone(),
                6 => profile.address.city.clone(),
                7 => profile.address.postal_code.clone(),
                8 => profile.address.country.clone(),
                _ => String::new(),
            }
        } else {
            String::new()
        }
    }

    fn set_current_field_value(&mut self, value: String) {
        if let Some(profile) = self.profiles.get_mut(self.selected_profile) {
            match self.selected_field {
                0 => profile.name = value,
                1 => profile.personal.first_name = value,
                2 => profile.personal.last_name = value,
                3 => profile.contact.email = value,
                4 => profile.contact.phone = value,
                5 => profile.address.street = value,
                6 => profile.address.city = value,
                7 => profile.address.postal_code = value,
                8 => profile.address.country = value,
                _ => {}
            }
        }
    }
}

pub fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new()?;

    // Main loop
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code)?;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
