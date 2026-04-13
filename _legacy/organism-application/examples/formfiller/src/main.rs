mod analyzer;
mod filler;
mod learner;
mod models;
mod pdf;
mod storage;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use models::{Priority, Task, TaskStatus};

#[derive(Parser)]
#[command(name = "formfiller")]
#[command(about = "Fast form filler with TUI for time-sensitive applications")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the interactive TUI (default)
    Tui,

    /// Learn from logging server and update form configs
    Learn {
        /// Logging server URL
        #[arg(short, long, default_value = "http://localhost:3001")]
        server: String,

        /// Base URL for forms
        #[arg(short, long, default_value = "http://localhost:8080")]
        base_url: String,
    },

    /// Analyze a URL and discover form fields
    Analyze {
        /// URL to analyze
        url: String,

        /// WebDriver URL
        #[arg(short, long, default_value = "http://localhost:9515")]
        webdriver: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Analyze a PDF schema fixture (JSON)
    AnalyzePdf {
        /// Path to schema fixture JSON
        input: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Build a fill plan from a PDF schema fixture (JSON)
    PlanPdf {
        /// Path to schema fixture JSON
        input: String,

        /// Output path for the fill plan JSON
        output: String,
    },

    /// Fill a form using profile data
    Fill {
        /// Form config name or URL
        form: String,

        /// Profile name to use
        #[arg(short, long)]
        profile: Option<String>,

        /// WebDriver URL
        #[arg(short, long, default_value = "http://localhost:9515")]
        webdriver: String,

        /// Don't submit, just fill
        #[arg(long)]
        no_submit: bool,
    },

    /// Show validation errors from logs and suggest fixes
    Errors {
        /// Logging server URL
        #[arg(short, long, default_value = "http://localhost:3001")]
        server: String,
    },

    /// List all form configs
    List,

    /// List all profiles
    Profiles,

    /// Manage task queue
    Tasks {
        #[command(subcommand)]
        action: Option<TaskAction>,
    },
}

#[derive(Subcommand)]
enum TaskAction {
    /// List all tasks (default)
    List {
        /// Show only tasks with this status
        #[arg(short, long)]
        status: Option<String>,
    },

    /// Add a new task
    Add {
        /// Task name
        name: String,

        /// Form config name
        #[arg(short, long)]
        form: String,

        /// Profile name (defaults to first)
        #[arg(short, long)]
        profile: Option<String>,

        /// Priority: low, normal, high, urgent, critical
        #[arg(long, default_value = "normal")]
        priority: String,
    },

    /// Show next task to run
    Next,

    /// Run the next task in queue
    Run {
        /// WebDriver URL
        #[arg(short, long, default_value = "http://localhost:9515")]
        webdriver: String,
    },

    /// Mark a task as completed
    Complete {
        /// Task name or ID prefix
        task: String,
    },

    /// Cancel a task
    Cancel {
        /// Task name or ID prefix
        task: String,
    },

    /// Show task queue summary with priority beads
    Summary,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Tui) => {
            // Run TUI (blocking)
            tui::run()
        }

        Some(Commands::Learn { server, base_url }) => {
            println!("Learning from logging server...");
            let result = learner::learn_and_update(Some(&server), &base_url).await?;
            println!("{}", result);
            Ok(())
        }

        Some(Commands::Analyze { url, webdriver, json }) => {
            println!("Analyzing form at {}...", url);

            let caps = thirtyfour::DesiredCapabilities::chrome();
            let driver = thirtyfour::WebDriver::new(&webdriver, caps).await?;

            let analysis = analyzer::analyze_form(&driver, &url).await?;

            driver.quit().await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&analysis)?);
            } else {
                println!("\nForm Analysis: {}", analysis.title.as_deref().unwrap_or("Unknown"));
                println!("URL: {}", analysis.url);
                println!("Multi-step: {}", analysis.is_multi_step);
                println!("\nFields found: {}", analysis.fields.len());
                println!("{:-<60}", "");

                for field in &analysis.fields {
                    let source = field.guessed_source
                        .as_ref()
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_else(|| "???".to_string());

                    let conf = if field.confidence > 0.0 {
                        format!(" ({:.0}%)", field.confidence * 100.0)
                    } else {
                        String::new()
                    };

                    println!(
                        "{:20} {:15} -> {}{}",
                        field.selector.chars().take(20).collect::<String>(),
                        field.label.as_deref().unwrap_or("-").chars().take(15).collect::<String>(),
                        source,
                        conf
                    );
                }

                if let Some(submit) = &analysis.submit_selector {
                    println!("\nSubmit button: {}", submit);
                }

                // Generate config
                let config = analysis.to_form_config("Analyzed Form".to_string());
                println!("\n--- Generated FormConfig ---");
                println!("{}", serde_json::to_string_pretty(&config)?);
            }

            Ok(())
        }

        Some(Commands::AnalyzePdf { input, json }) => {
            let schema = pdf::load_fixture(&input)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&schema)?);
            } else {
                println!("PDF Form Schema: {}", schema.title.as_deref().unwrap_or("Untitled"));
                println!("Form ID: {}", schema.form_id);
                println!("Fields: {}", schema.fields.len());
                for field in &schema.fields {
                    let label = field.label.as_deref().unwrap_or("-");
                    println!(
                        "  - {} ({}){}",
                        field.field_id,
                        label,
                        if field.required { " [required]" } else { "" }
                    );
                }
            }

            Ok(())
        }

        Some(Commands::PlanPdf { input, output }) => {
            let schema = pdf::load_fixture(&input)?;
            let plan = pdf::build_plan(&schema);
            pdf::write_plan(&plan, &output)?;
            println!("Wrote fill plan: {}", output);
            Ok(())
        }

        Some(Commands::Fill { form, profile, webdriver, no_submit }) => {
            let configs = storage::load_form_configs()?;
            let profiles = storage::load_profiles()?;

            // Find form config
            let config = configs
                .iter()
                .find(|c| c.name.to_lowercase().contains(&form.to_lowercase()) || c.url == form)
                .ok_or_else(|| anyhow::anyhow!("Form config not found: {}", form))?;

            // Find profile
            let profile_data = if let Some(name) = profile {
                profiles
                    .iter()
                    .find(|p| p.name.to_lowercase().contains(&name.to_lowercase()))
                    .ok_or_else(|| anyhow::anyhow!("Profile not found: {}", name))?
            } else {
                profiles.first().ok_or_else(|| anyhow::anyhow!("No profiles found"))?
            };

            println!("Filling form: {}", config.name);
            println!("Using profile: {}", profile_data.name);
            println!("URL: {}", config.url);

            let filler_instance = filler::Filler::new(&webdriver).await?;
            let result = filler_instance.fill_form(profile_data, config).await?;

            println!("\nFill result:");
            println!("  Fields filled: {}", result.filled);
            println!("  Success: {}", result.success);

            if !result.errors.is_empty() {
                println!("  Errors:");
                for err in &result.errors {
                    println!("    - {}", err);
                }
            }

            if !no_submit {
                if let Some(_) = &config.submit_selector {
                    println!("\nSubmitting form...");
                    filler_instance.submit(config).await?;
                    println!("Submitted!");
                }
            }

            filler_instance.quit().await?;

            Ok(())
        }

        Some(Commands::Errors { server }) => {
            println!("Fetching validation errors...\n");

            let errors = learner::analyze_errors(Some(&server)).await?;

            if errors.is_empty() {
                println!("No validation errors found!");
            } else {
                for (i, err) in errors.iter().enumerate() {
                    println!("{}. {}", i + 1, err.pathname.as_deref().unwrap_or("Unknown form"));
                    println!("   Field: {}", err.field.as_deref().unwrap_or("?"));
                    println!("   Error: {}", err.message.as_deref().unwrap_or("?"));
                    if let Some(suggestion) = &err.suggestion {
                        println!("   Fix: {}", suggestion);
                    }
                    println!();
                }
            }

            Ok(())
        }

        Some(Commands::List) => {
            let configs = storage::load_form_configs()?;

            if configs.is_empty() {
                println!("No form configs found.");
            } else {
                println!("Form Configs ({}):\n", configs.len());
                for config in &configs {
                    let step_info = if config.is_multi_step() {
                        format!(" [{} steps]", config.steps.len())
                    } else {
                        format!(" [{} fields]", config.fields.len())
                    };
                    println!("  {} {}", config.name, step_info);
                    println!("    {}", config.url);
                }
            }

            Ok(())
        }

        Some(Commands::Profiles) => {
            let profiles = storage::load_profiles()?;

            if profiles.is_empty() {
                println!("No profiles found.");
            } else {
                println!("Profiles ({}):\n", profiles.len());
                for profile in &profiles {
                    println!("  {}", profile.name);
                    println!("    {} {}", profile.personal.first_name, profile.personal.last_name);
                    println!("    {}", profile.contact.email);
                }
            }

            Ok(())
        }

        Some(Commands::Tasks { action }) => {
            handle_tasks(action).await
        }
    }
}

async fn handle_tasks(action: Option<TaskAction>) -> Result<()> {
    let mut queue = storage::load_tasks()?;
    let configs = storage::load_form_configs()?;
    let profiles = storage::load_profiles()?;

    match action {
        None | Some(TaskAction::List { status: None }) => {
            if queue.tasks.is_empty() {
                println!("No tasks in queue.");
            } else {
                println!("Task Queue ({}):\n", queue.tasks.len());
                print_task_list(&queue, &configs);
            }
        }

        Some(TaskAction::List { status: Some(status_str) }) => {
            let status = parse_status(&status_str)?;
            let tasks = queue.by_status(status);

            if tasks.is_empty() {
                println!("No tasks with status {:?}.", status);
            } else {
                println!("Tasks ({:?}): {}\n", status, tasks.len());
                for task in tasks {
                    print_task(task, &configs);
                }
            }
        }

        Some(TaskAction::Add { name, form, profile, priority }) => {
            let form_config = configs
                .iter()
                .find(|c| c.name.to_lowercase().contains(&form.to_lowercase()))
                .ok_or_else(|| anyhow::anyhow!("Form config not found: {}", form))?;

            let profile_data = if let Some(p) = profile {
                profiles
                    .iter()
                    .find(|pr| pr.name.to_lowercase().contains(&p.to_lowercase()))
                    .ok_or_else(|| anyhow::anyhow!("Profile not found: {}", p))?
            } else {
                profiles.first().ok_or_else(|| anyhow::anyhow!("No profiles found"))?
            };

            let prio = parse_priority(&priority)?;

            let task = Task::new(name.clone(), form_config.id, profile_data.id)
                .with_priority(prio);

            println!("{} Added task: {}", prio.bead(), name);
            println!("  Form: {}", form_config.name);
            println!("  Profile: {}", profile_data.name);
            println!("  Priority: {}", prio.label());

            queue.add(task);
            storage::save_tasks(&queue)?;
        }

        Some(TaskAction::Next) => {
            if let Some(task) = queue.next_ready() {
                let form_name = configs
                    .iter()
                    .find(|c| c.id == task.form_config_id)
                    .map(|c| c.name.as_str())
                    .unwrap_or("Unknown");

                println!("{} Next task: {}", task.effective_priority().bead(), task.name);
                println!("  Form: {}", form_name);
                println!("  Priority: {} (effective: {})",
                    task.priority.label(),
                    task.effective_priority().label()
                );
            } else {
                println!("No tasks ready to run.");
            }
        }

        Some(TaskAction::Run { webdriver }) => {
            let next_task = queue.next_ready().map(|t| t.id);

            if let Some(task_id) = next_task {
                let task = queue.get_mut(task_id).unwrap();
                let form_config = configs
                    .iter()
                    .find(|c| c.id == task.form_config_id)
                    .ok_or_else(|| anyhow::anyhow!("Form config not found"))?;
                let profile_data = profiles
                    .iter()
                    .find(|p| p.id == task.profile_id)
                    .ok_or_else(|| anyhow::anyhow!("Profile not found"))?;

                println!("{} Running task: {}", task.effective_priority().bead(), task.name);
                task.start();
                storage::save_tasks(&queue)?;

                let filler_instance = filler::Filler::new(&webdriver).await?;
                let result = filler_instance.fill_form(profile_data, form_config).await?;

                if result.success {
                    let task = queue.get_mut(task_id).unwrap();
                    task.complete();
                    println!("✓ Task completed: {} fields filled", result.filled);
                } else {
                    let task = queue.get_mut(task_id).unwrap();
                    let error = result.errors.join("; ");
                    task.fail(error.clone());
                    println!("✗ Task failed: {}", error);
                }

                filler_instance.quit().await?;
                storage::save_tasks(&queue)?;
            } else {
                println!("No tasks ready to run.");
            }
        }

        Some(TaskAction::Complete { task: task_ref }) => {
            if let Some(task) = find_task_mut(&mut queue, &task_ref) {
                task.complete();
                println!("✓ Marked as completed: {}", task.name);
                storage::save_tasks(&queue)?;
            } else {
                println!("Task not found: {}", task_ref);
            }
        }

        Some(TaskAction::Cancel { task: task_ref }) => {
            if let Some(task) = find_task_mut(&mut queue, &task_ref) {
                task.cancel();
                println!("⊘ Cancelled: {}", task.name);
                storage::save_tasks(&queue)?;
            } else {
                println!("Task not found: {}", task_ref);
            }
        }

        Some(TaskAction::Summary) => {
            let summary = queue.summary();
            println!("Task Queue Summary");
            println!("{:-<40}", "");
            println!();

            // Priority beads visualization
            print!("Priority: ");
            for _ in 0..summary.critical { print!("◆ "); }
            for _ in 0..summary.urgent { print!("◈ "); }
            for _ in 0..summary.high { print!("◉ "); }
            let normal = summary.total - summary.critical - summary.urgent - summary.high;
            for _ in 0..normal { print!("● "); }
            println!();
            println!();

            println!("  {} Critical  {} Urgent  {} High",
                summary.critical, summary.urgent, summary.high);
            println!();

            // Status breakdown
            println!("Status:");
            println!("  ⏳ Pending:     {}", summary.pending);
            println!("  ▶  In Progress: {}", summary.in_progress);
            println!("  ✓  Completed:   {}", summary.completed);
            println!("  ✗  Failed:      {}", summary.failed);
            println!("  ⊘  Cancelled:   {}", summary.cancelled);
            println!("  ⊟  Blocked:     {}", summary.blocked);
            println!();
            println!("Total: {}", summary.total);
        }
    }

    Ok(())
}

fn print_task_list(queue: &models::TaskQueue, configs: &[models::FormConfig]) {
    for task in queue.sorted_by_priority() {
        print_task(task, configs);
    }
}

fn print_task(task: &Task, configs: &[models::FormConfig]) {
    let form_name = configs
        .iter()
        .find(|c| c.id == task.form_config_id)
        .map(|c| c.name.as_str())
        .unwrap_or("Unknown");

    let prio = task.effective_priority();
    println!(
        "{} {} {} {}",
        prio.bead(),
        task.status.symbol(),
        task.name,
        format!("[{}]", prio.label())
    );
    println!("    Form: {}", form_name);
    if let Some(deadline) = task.deadline {
        println!("    Deadline: {}", deadline.format("%Y-%m-%d %H:%M"));
    }
    if let Some(err) = &task.last_error {
        println!("    Error: {}", err);
    }
    println!();
}

fn find_task_mut<'a>(queue: &'a mut models::TaskQueue, reference: &str) -> Option<&'a mut Task> {
    // Try to find by name (partial match) or ID prefix
    queue.tasks.iter_mut().find(|t| {
        t.name.to_lowercase().contains(&reference.to_lowercase())
            || t.id.to_string().starts_with(reference)
    })
}

fn parse_priority(s: &str) -> Result<Priority> {
    match s.to_lowercase().as_str() {
        "low" | "l" | "1" => Ok(Priority::Low),
        "normal" | "n" | "2" => Ok(Priority::Normal),
        "high" | "h" | "3" => Ok(Priority::High),
        "urgent" | "u" | "4" => Ok(Priority::Urgent),
        "critical" | "c" | "5" => Ok(Priority::Critical),
        _ => Err(anyhow::anyhow!("Invalid priority: {}. Use: low, normal, high, urgent, critical", s)),
    }
}

fn parse_status(s: &str) -> Result<TaskStatus> {
    match s.to_lowercase().as_str() {
        "pending" | "p" => Ok(TaskStatus::Pending),
        "in_progress" | "progress" | "running" => Ok(TaskStatus::InProgress),
        "completed" | "done" | "c" => Ok(TaskStatus::Completed),
        "failed" | "f" => Ok(TaskStatus::Failed),
        "cancelled" | "canceled" => Ok(TaskStatus::Cancelled),
        "blocked" | "b" => Ok(TaskStatus::Blocked),
        _ => Err(anyhow::anyhow!("Invalid status: {}. Use: pending, in_progress, completed, failed, cancelled, blocked", s)),
    }
}
