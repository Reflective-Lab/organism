mod pdf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "university-course-application")]
#[command(about = "Prepare governed university course applications")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a PDF schema fixture (JSON)
    AnalyzePdf {
        /// Path to schema fixture JSON
        input: String,
    },

    /// Build a fill plan from a PDF schema fixture (JSON)
    PlanPdf {
        /// Path to schema fixture JSON
        input: String,

        /// Output path for the fill plan JSON
        output: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::AnalyzePdf { input } => {
            let schema = pdf::load_fixture(&input)?;
            println!("PDF Form Schema: {}", schema.title.as_deref().unwrap_or("Untitled"));
            println!("Form ID: {}", schema.form_id);
            println!("Fields: {}", schema.fields.len());
            for field in &schema.fields {
                let label = field.label.as_deref().unwrap_or("-");
                let required = if field.required { " [required]" } else { "" };
                println!("  - {} ({}){}", field.field_id, label, required);
            }
        }
        Commands::PlanPdf { input, output } => {
            let schema = pdf::load_fixture(&input)?;
            let plan = pdf::build_plan(&schema);
            pdf::write_plan(&plan, &output)?;
            println!("Wrote fill plan: {}", output);
        }
    }

    Ok(())
}
