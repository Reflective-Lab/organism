# University Course Application

CLI application for preparing reviewable, governed course applications.

## What it does
- Reads a PDF schema fixture (JSON) for a course application form.
- Builds a deterministic fill plan highlighting missing required fields.
- Writes the plan as JSON for review.

## Usage
```bash
cargo run -- analyze-pdf fixtures/sample.json
cargo run -- plan-pdf fixtures/sample.json out/plan.json
```

## Fixtures
See `fixtures/sample.json` for the schema format.
