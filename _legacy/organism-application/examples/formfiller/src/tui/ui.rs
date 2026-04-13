use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use super::app::{App, InputMode, Screen};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Status/help
        ])
        .split(f.area());

    draw_title(f, chunks[0], app);

    match app.screen {
        Screen::Main => draw_main(f, chunks[1], app),
        Screen::Profiles => draw_profiles(f, chunks[1], app),
        Screen::ProfileEdit => draw_profile_edit(f, chunks[1], app),
        Screen::Forms => draw_forms(f, chunks[1], app),
        Screen::FormEdit => draw_form_edit(f, chunks[1], app),
        Screen::Fill => draw_fill(f, chunks[1], app),
    }

    draw_status(f, chunks[2], app);
}

fn draw_title(f: &mut Frame, area: Rect, _app: &App) {
    let title = Paragraph::new("FormFiller")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &App) {
    let items = vec![
        ListItem::new(format!(
            "[p] Profiles ({})",
            app.profiles.len()
        )),
        ListItem::new(format!("[f] Forms ({})", app.forms.len())),
        ListItem::new("[r] Run form fill"),
        ListItem::new("[s] Save all"),
        ListItem::new("[q] Quit"),
    ];

    let list = List::new(items)
        .block(Block::default().title("Main Menu").borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(list, area);
}

fn draw_profiles(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .profiles
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == app.selected_profile {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!(
                "{} - {} {}",
                p.name, p.personal.first_name, p.personal.last_name
            ))
            .style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Profiles [n]ew [e]dit [d]elete [q]back")
            .borders(Borders::ALL),
    );

    f.render_widget(list, area);
}

fn draw_profile_edit(f: &mut Frame, area: Rect, app: &App) {
    let profile = match app.current_profile() {
        Some(p) => p,
        None => return,
    };

    let fields = vec![
        ("Profile Name", &profile.name),
        ("First Name", &profile.personal.first_name),
        ("Last Name", &profile.personal.last_name),
        ("Email", &profile.contact.email),
        ("Phone", &profile.contact.phone),
        ("Street", &profile.address.street),
        ("City", &profile.address.city),
        ("Postal Code", &profile.address.postal_code),
        ("Country", &profile.address.country),
    ];

    let items: Vec<ListItem> = fields
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let display_value = if app.input_mode == InputMode::Editing && i == app.selected_field {
                &app.input_buffer
            } else {
                value.as_str()
            };

            let style = if i == app.selected_field {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{:15}", label), Style::default().fg(Color::Cyan)),
                Span::raw(": "),
                Span::styled(display_value, style),
            ]))
        })
        .collect();

    let title = if app.input_mode == InputMode::Editing {
        "Edit Profile [Enter]save [Esc]cancel"
    } else {
        "Edit Profile [Enter]edit [j/k]navigate [q]back"
    };

    let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));

    f.render_widget(list, area);
}

fn draw_forms(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .forms
        .iter()
        .enumerate()
        .map(|(i, form)| {
            let style = if i == app.selected_form {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{} - {}", form.name, form.url)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("Forms [n]ew [e]dit [d]elete [q]back")
            .borders(Borders::ALL),
    );

    f.render_widget(list, area);
}

fn draw_form_edit(f: &mut Frame, area: Rect, app: &App) {
    let form = match app.current_form() {
        Some(f) => f,
        None => return,
    };

    let text = format!(
        "Name: {}\nURL: {}\nFields: {}\nSubmit selector: {:?}",
        form.name,
        form.url,
        form.fields.len(),
        form.submit_selector
    );

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .title("Form Config (edit coming soon) [q]back")
            .borders(Borders::ALL),
    );

    f.render_widget(paragraph, area);
}

fn draw_fill(f: &mut Frame, area: Rect, app: &App) {
    let profile_name = app
        .current_profile()
        .map(|p| p.name.as_str())
        .unwrap_or("None selected");
    let form_name = app
        .current_form()
        .map(|f| f.name.as_str())
        .unwrap_or("None selected");

    let text = format!(
        "Ready to fill form!\n\n\
         Profile: {}\n\
         Form: {}\n\n\
         Press [Enter] to start filling\n\
         Press [q] to go back\n\n\
         Note: Requires chromedriver running on http://localhost:9515",
        profile_name, form_name
    );

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("Fill Form").borders(Borders::ALL));

    f.render_widget(paragraph, area);
}

fn draw_status(f: &mut Frame, area: Rect, app: &App) {
    let status = app.status_message.as_deref().unwrap_or("");
    let paragraph = Paragraph::new(status)
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(paragraph, area);
}
